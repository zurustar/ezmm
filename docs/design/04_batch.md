# 書き出しエンジン

1プロジェクト = 1本の動画ファイルを書き出す処理の実行方式・進捗・キャンセル・ログ・スリープ抑制。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [03_renderer.md](03_renderer.md)（レンダリング）、[05_ipc.md](05_ipc.md)（イベント定義）

---

## 基本方針

- **出力**: 1プロジェクト = 1本の動画ファイルを出力する
- **同時実行**: 1書き出しのみ。既に書き出し中に `start_export` が呼ばれた場合は `"export_already_running:"` エラーを返す。UI 側でも書き出し中は「書き出し」ボタンを無効化する
- **エラー時**: エラー発生時に書き出しを中断し、UIにエラー内容を表示

---

## 実行フロー

```
start_export IPC
  └─ 書き出し中フラグを true にセット
  └─ ffprobe で全映像ファイルを probe（ProbeResult キャッシュに保存）
  └─ build_filter_graph() で FFmpeg フィルタグラフを生成
  └─ FFmpeg サブプロセスを起動（-progress pipe:1 オプション付き）
  └─ stdout を読んで export:progress イベントを emit
  └─ FFmpeg 終了 → export:done / export:error イベントを emit
```

---

## キャンセル

- `cancel_export` IPC が呼ばれると `cancel_requested` フラグを `true` にセットし、`ffmpeg_child.kill()` で現在の FFmpeg プロセスを即時終了させる
- 不完全な出力ファイルを削除してから `export:cancelled` イベントを emit する

### キャンセル確認ダイアログ

書き出し中のキャンセルボタン押下時は確認ダイアログを表示:
「書き出しをキャンセルしますか？ [キャンセルする] [続行する]」

---

## 進捗

- FFmpeg の `-progress pipe:1` から `out_time_ms` を取得し、全シーン合計時間に対する % を計算
- `export:progress` イベントで `progress: 0.0–1.0` をフロントエンドへ通知

---

## ログ

書き出しごとに `{output_folder}/ezmm-YYYYMMDD-HHMMSS.log` を生成。書き出し開始・完了・エラー・FFmpegコマンドを記録。

`output_folder = ""` の場合はバリデーションで書き出しがブロックされるため、ログ未生成の状況は発生しない。ログ書き込み失敗は tracing で警告を記録し、書き出しは中断しない（サイレントに続行）。

---

## 完了後動作

完了ダイアログ:「書き出し完了 / 出力先: /path/to/output.mp4 / 経過時間: N秒 / [フォルダを開く] [OK]」

**「フォルダを開く」の実装**:
- `tauri-plugin-opener` の `openPath(path)` を呼ぶ
- macOS: Finder でフォルダ表示 / Windows: エクスプローラでフォルダ表示
- `src-tauri/capabilities/default.json` の `permissions` に `"opener:allow-open-path"` を追加する必要あり

---

## 長時間動作対応（スリープ抑制）

書き出し中はシステムスリープを抑制する:

**macOS**: `/usr/bin/caffeinate -i` を書き出し開始時に `std::process::Command` で起動し、完了時に kill する。macOS アプリバンドル起動時は PATH 環境変数が不定のため、必ず**絶対パス**で指定する:
```rust
let caffeinate = Command::new("/usr/bin/caffeinate").arg("-i").spawn()?;
// 完了後: caffeinate.kill();
```

**Windows**: `windows` クレートから `SetThreadExecutionState` を呼ぶ（`unsafe` が必要）:
```rust
#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

// 書き出し開始時
unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED); }
// 書き出し完了時
unsafe { SetThreadExecutionState(ES_CONTINUOUS); }
```

`windows` クレートは `Cargo.toml` で Windows 限定依存として宣言:
```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_System_Power"] }
```

### RAII パターン（SleepGuard）

スリープ抑制は RAII パターンで実装し、`Drop` トレイトで自動解除する:

```rust
#[cfg(target_os = "macos")]
pub struct SleepGuard { caffeinate: std::process::Child }

#[cfg(target_os = "windows")]
pub struct SleepGuard; // Windows 側は保持データなし、Drop の副作用のみ利用

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct SleepGuard; // その他 OS は no-op
```

`start_export` 内では `let _sleep_guard = SleepGuard::new()?;`（macOS）または `let _sleep_guard = SleepGuard::new();`（Windows/その他）として確保し、関数終了時の自動 Drop に委ねる。

---

## 書き出し中の編集制限

書き出し中はエディタUIを**読み取り専用**にする。プロパティパネルへの入力・シーン/オブジェクトの追加・削除を無効化。「書き出し中はプロジェクトを編集できません」のバナーを表示。

`settings.json` の書き込みは許可（プロジェクトデータと別ファイルで競合しない）。

---

## 出力ファイル検証

書き出し完了後:
1. 出力ファイルが存在するか確認
2. ファイルサイズ > 0 か確認

FFmpeg による再デコード確認は行わない（起動コスト過大）。

---

## ディスク空き容量チェック

事前チェックなし（FFmpegがディスクフル時にエラーを出すため任せる）。
