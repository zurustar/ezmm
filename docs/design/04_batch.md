# バッチ実行エンジン

バッチ処理の実行方式・進捗・キャンセル・ログ・スリープ抑制。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [03_renderer.md](03_renderer.md)（1エントリのレンダリング）、[05_ipc.md](05_ipc.md)（イベント定義）

---

## 基本方針

- **実行方式**: 直列（1エントリずつ順番に処理）。並列化なし（低スペックPC対策）
- **同時実行**: **1バッチのみ**。既にバッチ実行中に `start_batch` が呼ばれた場合は `"batch_already_running:"` エラーを返す（エラーコード体系に統一: `"<code>:"` + 詳細メッセージ）。UI 側でもバッチ実行中は「バッチ実行」ボタンを無効化する
- **エラー時**: エラー発生時点でバッチを中断し、UIにエラー内容を表示。再開機能なし（プロジェクトを修正して再実行）
- **ログ**: バッチ実行ごとに `{output_folder}/ezmm-YYYYMMDD-HHMMSS.log` を生成。各エントリの開始・完了・エラー・FFmpegコマンドを記録。`output_folder = ""` の場合はバリデーションでバッチ実行がブロックされるため、ログ未生成の状況は発生しない。ログ書き込み失敗（権限エラー等）は tracing でアプリログに警告を記録し、バッチ実行を中断しない（ログ失敗はサイレントに続行）

---

## 出力ファイル名衝突時の挙動

バッチ開始前に `check_output_conflicts` IPC を呼んで衝突ファイル一覧を取得し、1件以上あればダイアログを表示。ダイアログ:
「以下のファイルが既に存在します: tanaka.mp4, suzuki.mp4 / [すべて上書き] [スキップ] [キャンセル]」

ユーザー選択結果を `overwrite_policy: 'overwrite' | 'skip'` として `start_batch` に渡す。衝突ゼロの場合はダイアログをスキップして `overwrite_policy: 'overwrite'`（事実上無意味）で即座に開始する。

### `start_batch` シグネチャ

```rust
#[tauri::command]
pub async fn start_batch(
    project: Project,
    entry_names: Vec<String>,
    overwrite_policy: String,  // "overwrite" | "skip"
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> { ... }
```

---

## キャンセル時の扱い

- キャンセル時点で**処理済みのエントリの出力ファイルは保持**する
- 現在レンダリング中のエントリの不完全ファイルは削除する

### バッチキャンセル時の確認ダイアログ

- 処理済みエントリが **0 件**: 確認なしでキャンセル
- 処理済みエントリが **1 件以上**: 確認ダイアログを表示:
  「N 件のレンダリングが完了しています。キャンセルしても完了済みのファイルは保持されます。キャンセルしますか？ [キャンセルする] [続行する]」

---

## 特定エントリのみ再実行（部分バッチ）

エントリ一覧に**チェックボックス**を設け、チェックされたエントリのみを対象に処理する。デフォルトは全エントリにチェック済み。

**「バッチ実行」ボタンの動作:** ツールバーの「バッチ実行」ボタンは1つのみで、**チェック済みエントリを対象**に実行する（未チェックは除外）。すべてのチェックが外れている場合はボタンを無効化する。

---

## 進捗の粒度

- エントリ単位: 「N / M 件完了」を常時表示
- エントリ内進捗: FFmpegの `-progress pipe:1` から `out_time_ms` を取得し、エントリ全体の予測時間（全シーン合計）に対する%を表示

### 残り時間予測

最初の 1エントリが完了した時点で実績時間を記録し、「残り約 N 分」を線形予測で表示する:
表示形式: 「残り約 N 分（{完了件数}/{総件数}）」

---

## バッチ完了サマリー

完了ダイアログ:「レンダリング完了 / 成功: N件 / エラー: 0件 / 出力先: /path/to/output / [フォルダを開く] [ログを開く] [OK]」

**「フォルダを開く」「ログを開く」の実装**:
- `tauri-plugin-opener` を使用。フロントエンドから `@tauri-apps/plugin-opener` の名前付きエクスポート `openPath(path)` を呼ぶ（`invoke("plugin:opener|open_path", ...)` の直接呼び出しより型安全で推奨）
- macOS: Finder でフォルダ表示 / ログファイルを標準テキストエディタで開く
- Windows: エクスプローラでフォルダ表示 / ログファイルをメモ帳等で開く
- `src-tauri/capabilities/default.json` の `permissions` に `"opener:allow-open-path"` を追加する必要あり

---

## バッチ実行中の編集制限

バッチ実行中はエディタUIを**読み取り専用**にする。プロパティパネルへの入力・シーン/オブジェクト/エントリの追加・削除を無効化。「バッチ実行中はプロジェクトを編集できません」のバナーを表示。

**`settings.json` の書き込みは許可:** プロジェクトデータの編集は不可だが、ウィンドウサイズ変更・終了時保存等による `settings.json` の書き込みはバッチ実行中でも許可する（プロジェクトデータとは別ファイルで競合しない）。

---

## 出力ファイル検証

エントリのレンダリング完了後:
1. 出力ファイルが存在するか確認
2. ファイルサイズ > 0 か確認

FFmpeg による再デコード確認は行わない（起動コスト過大）。

---

## ディスク空き容量チェック

事前チェックなし（FFmpegがディスクフル時にエラーを出すため任せる）。

---

## 可変値の一括インポート

CSV / JSON でのエントリ一括インポート機能は **v1 では非対応**。
エントリ数が多い場合はプロジェクト YAML をテキストエディタで直接編集することを推奨する。
v2 での対応を検討（CSV: 列 = オブジェクト ID、行 = エントリ）。

---

## 長時間動作対応（スリープ抑制）

バッチ実行中はシステムスリープを抑制する:

**macOS**: `/usr/bin/caffeinate -i` をバッチ開始時に `std::process::Command` で起動し、バッチ完了時に kill する。macOS アプリバンドル起動時は PATH 環境変数が不定のため、必ず**絶対パス**で指定する:
```rust
let caffeinate = Command::new("/usr/bin/caffeinate").arg("-i").spawn()?;
// バッチ完了後: caffeinate.kill();
```

**Windows**: `windows` クレートから `SetThreadExecutionState` を呼ぶ（`unsafe` が必要）:
```rust
#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

// バッチ開始時
unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED); }
// バッチ完了時
unsafe { SetThreadExecutionState(ES_CONTINUOUS); }
```

### RAII パターン（SleepGuard）

スリープ抑制は RAII パターンで実装し、`Drop` トレイトで自動解除する:

```rust
#[cfg(target_os = "macos")]
struct SleepGuard {
    caffeinate: std::process::Child,
}

#[cfg(target_os = "windows")]
struct SleepGuard; // Windows 側は保持データなし、Drop の副作用のみ利用

impl Drop for SleepGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = self.caffeinate.kill() {
                tracing::warn!("caffeinate.kill() failed: {e}");
            }
            let _ = self.caffeinate.wait();
        }
        #[cfg(target_os = "windows")]
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}
```

**`SleepGuard::new()` 実装骨格:**

```rust
#[cfg(target_os = "macos")]
impl SleepGuard {
    pub fn new() -> Result<Self, std::io::Error> {
        let caffeinate = std::process::Command::new("/usr/bin/caffeinate")
            .arg("-i")
            .spawn()?;
        Ok(Self { caffeinate })
    }
}

#[cfg(target_os = "windows")]
impl SleepGuard {
    pub fn new() -> Self {
        unsafe {
            windows::Win32::System::Power::SetThreadExecutionState(
                windows::Win32::System::Power::ES_CONTINUOUS
                | windows::Win32::System::Power::ES_SYSTEM_REQUIRED,
            );
        }
        Self
    }
}
```

`start_batch` 内では `let _sleep_guard = SleepGuard::new()?;`（macOS）または `let _sleep_guard = SleepGuard::new();`（Windows）として確保し、関数終了時の自動 Drop に委ねる。

**AppState との関係:** `SleepGuard` は `start_batch` 関数のローカル変数として確保する。`cancel_batch` は `cancel_requested` フラグを立てると同時に `ffmpeg_child.kill()` を呼んで現在の FFmpeg プロセスを即時終了させる。バッチループが次のイテレーション開始前にフラグを検出して早期リターンすることで `SleepGuard` が自動的に Drop される。`SleepGuard` 自体は `AppState` に格納しない。

**エラー扱いの方針:**
- `caffeinate.kill()` の失敗は `tracing::warn!` でログに記録し、それ以上の処理は行わない（Drop 内でパニックを起こすと未定義動作になる）
- Windows の `SetThreadExecutionState(ES_CONTINUOUS)` はエラーを返さない（戻り値は前回の状態）

パニック時も `Drop` は呼ばれるため、アプリクラッシュ時のスリープ抑制の取り残しを防ぐ。
