# アプリ状態・設定・ファイル I/O

AppState（ランタイム状態）・AppSettings（永続化設定）・ファイル保存の信頼性。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [01_project_schema.md](01_project_schema.md)（Project 型）、[05_ipc.md](05_ipc.md)（IPC コマンド）

---

## AppState（ランタイム状態）

Rust 側のアプリケーション状態を保持する `Mutex` 管理の構造体:

```rust
pub struct AppState {
    pub probe_cache: Mutex<HashMap<PathBuf, ProbeResult>>,  // キーは dunce::canonicalize() で正規化した PathBuf。シンボリックリンク・相対パスの重複キャッシュを防ぐ
    pub ffmpeg_version: Mutex<Option<String>>,  // 初回 get_ffmpeg_version IPC 呼び出し時に遅延キャッシュ
    pub current_settings: Mutex<AppSettings>,   // save_settings IPC 呼び出しごとに更新。CloseRequested ハンドラが参照してウィンドウ状態を追記保存
    pub batch_running: Mutex<bool>,
    pub cancel_requested: Mutex<bool>,  // cancel_batch で true にセット。バッチループが毎エントリ先頭で確認
    pub ffmpeg_child: Mutex<Option<std::process::Child>>,
    pub font_dir: PathBuf,      // .setup() で resource_dir() から初期化
    pub ffmpeg_path: PathBuf,   // .setup() で current_exe().parent() + env!("TARGET") から初期化（externalBin は resource_dir ではなく実行ファイル隣接に配置）
    pub ffprobe_path: PathBuf,  // 同上
    pub debug_mode: bool,       // EZMM_DEBUG=1 で true。.setup() で env::var から初期化
    pub(crate) log_guard: tracing_appender::non_blocking::WorkerGuard,  // pub(crate): main.rs の構造体リテラル構築で必要。crate 外への漏洩は防止。drop するとログ停止
}
```

### ProbeResult キャッシュ戦略

セッション内メモリキャッシュのみ（プロジェクトファイルへの保存なし）。
Rust バックエンドに `HashMap<PathBuf, ProbeResult>` を保持し、同一ファイルの再 probe を避ける。ファイルが外部で変更された場合のキャッシュ無効化は行わない（社内ツールの用途上、問題なしと判断）。

---

## AppSettings（永続化設定）

### settings.json の項目一覧

保存場所: macOS `~/Library/Application Support/io.github.zurustar.ezmm/settings.json` / Windows `%APPDATA%\io.github.zurustar.ezmm\settings.json`

```json
{
  "version": 1,
  "default_crf": 23,
  "default_preset": "medium",
  "preview_resolution_scale": 0.5,
  "last_open_folder": "/path/to/last/folder",
  "recent_files": [
    "/path/to/project1.yaml",
    "/path/to/project2.yaml"
  ],
  "window": {
    "width": 1280,
    "height": 800,
    "x": null,
    "y": null
  }
}
```

### Rust 構造体

```rust
// #[derive(Default)] は仕様値を返せないため手動 impl を用意する
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub version: u32,
    pub default_crf: u32,
    pub default_preset: String,
    pub preview_resolution_scale: f64,
    pub last_open_folder: Option<String>,
    pub recent_files: Vec<String>,
    pub window: WindowSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            default_crf: 23,
            default_preset: "medium".to_string(),
            preview_resolution_scale: 0.5,
            last_open_folder: None,
            recent_files: Vec::new(),
            window: WindowSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self { width: 1280, height: 800, x: None, y: None }
    }
}
```

### TypeScript 型定義

```typescript
interface AppSettings {
  version: number;
  default_crf: number;
  default_preset: string;
  preview_resolution_scale: number;
  last_open_folder: string | null;
  recent_files: string[];
  window: WindowSettings;
}

interface WindowSettings {
  width: number;
  height: number;
  x: number | null;
  y: number | null;
}

interface FontPaths {
  regular: string;
  bold: string;
}
```

---

## settings.json のバージョニング

- `version` フィールド必須。現在は `1`
- アプリ起動時に `version` が未知（現行バージョン `!= 1`、つまり `0`・`2以上`・欠落・非整数を含む）の場合はファイル全体を無視してデフォルト値で起動。破壊的変更を避けるため既存ファイルはバックアップ（`settings.json.bak`）してから上書き
- 将来フィールドを追加する場合は後方互換を維持する形で行い、version は必要に応じてインクリメント

---

## ウィンドウ状態の復元

アプリ終了時に現在のウィンドウサイズ・位置を `settings.json` に保存する。次回起動時に復元する。
記録されたサイズが現在のスクリーン解像度に収まらない場合は OS デフォルト位置にフォールバックする。

**実装方法**: `tauri::Builder::on_window_event` で `WindowEvent::CloseRequested` を受け取り、ネイティブハンドラ内でウィンドウの現在サイズ・位置を取得して `AppState.current_settings` を更新してから `settings.json` に書き込む。`current_settings` は `save_settings` IPC を呼ぶたびにフロントエンドが更新するため、close 時はウィンドウ状態（width / height / x / y）のみ上書きし他フィールドはキャッシュ済み値を使用する。resize / move イベントは即時書き込みをせず、close 時のみ保存する（パフォーマンス上の理由）。

**同期 I/O のブロック影響**: `CloseRequested` ハンドラ内の `save_settings_sync` は同期ファイル書き込みのため、ディスク遅延が大きい場合にウィンドウ close が数百 ms ブロックする可能性がある。社内ツールとして許容範囲とし、`spawn_blocking` は使用しない（ハンドラ内での非同期 spawn は Tauri が対応していない制約のため）。

---

## 最後に開いたフォルダ

ファイル選択ダイアログはフロントエンド（JS）から `@tauri-apps/plugin-dialog` の `open({ defaultPath: lastOpenFolder })` / `save({ defaultPath: lastOpenFolder })` を呼ぶ。ダイアログ成功後に `last_open_folder` を更新して `save_settings` IPC で保存する。Rust 側からダイアログを開く必要はない（`tauri_plugin_dialog::Dialog::file()` ビルダーは使用しない）。

---

## settings.json のパス解決

`load_settings` / `save_settings` / `save_settings_sync` で `settings.json` のパスを解決するには `app.path().app_config_dir()` を使用する:

```rust
let settings_path = app.path().app_config_dir()
    .map_err(|e| format!("app_config_dir failed: {e}"))? 
    .join("settings.json");
```

macOS では `~/Library/Application Support/io.github.zurustar.ezmm/`、Windows では `%APPDATA%\io.github.zurustar.ezmm\` に解決される。

---

## ファイル I/O の信頼性

### Atomic Write（プロジェクトファイル保存の完全手順）

プロジェクトファイルの保存は以下の順番で行う:

0. **パス正規化**: `save_project` IPC の `path` 引数に対して `dunce::canonicalize()` を適用し、内部パス（`output_folder`・`file` 等）の `\` を `/` に正規化した Project を YAML 文字列に変換する（正規化 → シリアライズの順）
1. 既存ファイル `{filename}.yaml` が存在する場合、`{filename}.yaml.bak` にコピーする（バックアップ確保）
2. 同ディレクトリに一時ファイル `{filename}.yaml.tmp` を作成して書き込む。一時ファイル作成失敗（権限エラー等）は即座にエラーを返す
3. `std::fs::rename("{filename}.yaml.tmp", "{filename}.yaml")` でアトミックに置き換える
4. rename が失敗した場合（クロスデバイス等）は `{filename}.yaml.tmp` を削除してエラーを返す（元ファイルは .bak で保全済み）

`.bak` / `.tmp` の作成先は必ず `{filename}` と**同じディレクトリ**（クロスデバイス rename を避けるため）。バックアップは1世代のみ保持。

`settings.json` も同様に atomic write（一時ファイル `.tmp` への書き込み → `rename`）を使用する。

### YAML のコメント・キー順序

`serde_yml` はシリアライズ時にコメントを保持しない。ユーザーが YAML を手編集してコメントを追加しても、アプリで保存し直すと**コメントは失われる**。これは `serde_yml` の制約であり仕様として受け入れる。キー順序は Rust 構造体のフィールド定義順に従う（`serde_yml` の `Mapping` はフィールド順を保持する）。

### 文字コード・改行コード・BOM

- 文字コード: **UTF-8 固定**（BOM なし）
- 改行コード: **LF**（Windows 上でも LF で保存）
- 読み込み時: CRLF / LF 両対応（`serde_yml` が自動処理）

### 拡張子サポート

- 推奨拡張子: `.yaml`
- 読み込み: `.yaml` / `.yml` 両対応（ファイルダイアログのフィルタ: `*.yaml;*.yml`）
- 保存: 常に `.yaml` 拡張子で保存

---

## セキュリティ・堅牢性方針

### パス参照の扱い

社内ツールであるためプロジェクトフォルダ外のファイル参照を禁止はしない。ただし絶対パスが別ユーザーのホームディレクトリ等を指す場合は警告なし（ユーザーの責任）。

### ファイルロック・排他制御

同一プロジェクトファイルを複数プロセスで開いた際のロックは行わない（社内・単一ユーザー使用を前提）。

### 外部書き換えの検知

外部エディタによるプロジェクトファイル変更の検知・リロードはv2以降。

---

## クロスプラットフォームパスの正規化

- YAML保存時: パス区切りを `/` に統一
- 読み込み時: `\` を `/` に正規化（`path.replace('\\', '/')`）
- Rust側: 内部処理は `PathBuf` を lossless に保持。YAML 文字列化・FFmpeg 引数化時は以下の変換:
  - `PathBuf::to_str()`（有効な UTF-8 のみ通し、無効文字があれば `Err` を返す）を使用
  - 無効 UTF-8 の場合（理論上 Windows の極端なケース）はユーザー向けエラー「パスを UTF-8 として扱えません: {debug_path}」を表示しバッチを中断
  - `to_string_lossy()` は**使用禁止**（`U+FFFD` 置換で情報損失し、FFmpeg に渡すと別ファイルを指すか失敗する）

### settings.json 内パスの正規化

`last_open_folder` および `recent_files` に保存するパスはYAMLと同様に `/` 区切りで正規化して保存する（Windows 上でも `\` ではなく `/` を使用）。読み込み時は `\` → `/` 変換を行う。

---

## アプリケーションログ

バッチ実行ログとは別に、アプリ全体の動作ログを出力:
- 場所: `~/Library/Logs/io.github.zurustar.ezmm/`（macOS）/ `%APPDATA%\io.github.zurustar.ezmm\logs\`（Windows）（`app_handle.path().app_log_dir()` の戻り値）
- ライブラリ: `tracing` クレート（INFO以上を記録）
- ローテーション: 日次（`Rotation::DAILY`）、最大5ファイル保持（`tracing-appender` はサイズベース rotation を未サポート）

### クラッシュレポート

パニック時は `std::panic::set_hook` で `tracing::error!` を呼び、既存の tracing ログ（`ezmm.YYYY-MM-DD.log`）にバックトレースを記録する（専用の `app.log` は作成しない）。ユーザーへのダイアログ表示は行わない（社内ツールのため）。
