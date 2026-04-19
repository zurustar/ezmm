# Tauri IPC コマンド・イベント仕様

フロントエンド ↔ Rustバックエンド 間の通信仕様。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [01_project_schema.md](01_project_schema.md)（Project・ProbeResult・ValidationResult 等の型）

---

## IPC コマンド一覧

原則としてコマンドは Rust 側で `Result<T, String>` を返す（`E = String` はエラーメッセージの文字列）。Tauri はこれを `{ status: "ok", data: T }` / `{ status: "error", error: string }` 形式でシリアライズしてフロントエンドへ渡す。ただし **`validate_project`（失敗しない）** と **`get_font_paths`（失敗しない）** は例外として裸の戻り値型（`ValidationResult` / `FontPaths`）を返す。

| コマンド | 引数 | 成功時の戻り値 | 説明 |
|---------|------|--------|------|
| `open_project` | `path: string` | `Project` | YAMLを読み込みパース |
| `save_project` | `path: string, project: Project` | `null` | プロジェクトをYAMLに保存 |
| `validate_project` | `project: Project` | `ValidationResult` | バリデーションのみ実行。編集中・バッチ前どちらからも呼べる（空プロジェクト〈シーン0件・エントリ0件〉はバッチ実行時のみエラー扱い。`validate_project` IPC は空でもエラーを返さない） |
| `start_batch` | `project: Project, entry_names: string[], overwrite_policy: 'overwrite' \| 'skip'` | `void` | バッチ処理開始（非同期）。`entry_names` は処理対象エントリ名リスト（空配列は全件）。`overwrite_policy` は出力ファイル衝突時の挙動。既にバッチ実行中の場合はエラー。**Project データは stateless**：フロントエンドが毎回送信し、Rust 側はキャッシュしない（ffprobe の ProbeResult キャッシュはセッション内のみ保持） |
| `check_output_conflicts` | `project: Project, entry_names: string[]` | `string[]` | `output_folder` をスキャンし、選択エントリの出力ファイルと衝突する既存ファイル名を返す。バッチ開始前に呼び、1件以上なら確認ダイアログを表示する |
| `cancel_batch` | なし | `void` | `cancel_requested` フラグを `true` にセットし、かつ `ffmpeg_child.kill()` を呼んで現在レンダリング中のFFmpegプロセスを即時終了させる。バッチループが次エントリ開始前にフラグを確認して早期リターンすることで以降のエントリ処理もキャンセルする（同時実行は1バッチのみのため引数不要） |
| `get_ffmpeg_version` | なし | `string` | 同梱FFmpegのバージョン確認。`ffmpeg -version` の先頭行から `ffmpeg version N.N.N` 形式を抽出して返す（例: `"ffmpeg version 7.1.0"`） |
| `probe_file` | `path: string` | `ProbeResult` | ffprobeでファイルメタデータを取得（duration・width・height・fps・音声トラック有無・sample_rate） |
| `get_font_paths` | なし | `FontPaths` (`{ regular: string, bold: string }`) | 同梱フォントの絶対パスを返す（Canvas `@font-face` ロード用）。`AppState.font_dir` から構築。失敗しないため `Result` なし |
| `load_settings` | なし | `AppSettings` | `settings.json` を読み込んで返す。ファイルが存在しない・バージョン不明の場合はデフォルト値を返す |
| `save_settings` | `settings: AppSettings` | `null` | `settings.json` を atomic write で保存（`.tmp` → `rename`） |

---

## Rust コマンドシグネチャ

各コマンドは `Result<T, String>` を返す（`String` はエラーメッセージ）。`app_handle` や `state` は Tauri が引数に注入する。

```rust
// commands/mod.rs（またはコマンドごとのサブモジュール）

#[tauri::command]
pub async fn open_project(path: String) -> Result<Project, String> { ... }

#[tauri::command]
pub async fn save_project(path: String, project: Project) -> Result<(), String> { ... }

#[tauri::command]
pub fn validate_project(project: Project) -> ValidationResult { ... }
// validate_project は常に Ok を返すため Result を使わない。バリデーション結果は ValidationResult の errors/warnings に格納する

#[tauri::command]
pub async fn start_batch(
    project: Project,
    entry_names: Vec<String>,       // 空 Vec は全エントリを対象とする
    overwrite_policy: String,       // "overwrite" | "skip"
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> { ... }

#[tauri::command]
pub async fn cancel_batch(state: tauri::State<'_, AppState>) -> Result<(), String> { ... }

#[tauri::command]
pub fn check_output_conflicts(
    project: Project,
    entry_names: Vec<String>,
) -> Result<Vec<String>, String> { ... }
// 戻り値: output_folder 内に既存する出力ファイル名のリスト（例: ["tanaka.mp4", "suzuki.mp4"]）

#[tauri::command]
pub async fn get_ffmpeg_version(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> { ... }

#[tauri::command]
pub async fn probe_file(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<ProbeResult, String> { ... }

#[tauri::command]
pub fn get_font_paths(
    state: tauri::State<'_, AppState>,
) -> FontPaths {
    // FontPaths は失敗しないため Result でラップしない
    ...
}

// commands/mod.rs（または commands/font.rs）内に定義する
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontPaths {
    pub regular: String,
    pub bold: String,
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<AppSettings, String> { ... }

#[tauri::command]
pub fn save_settings(
    settings: AppSettings,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,  // current_settings キャッシュを更新するために必要
) -> Result<(), String> { ... }
// 処理: (1) current_settings キャッシュを settings で上書き (2) settings.json を atomic write で保存

// settings.rs に定義する内部ヘルパー（IPC コマンドではない）。main.rs からは settings::save_settings_sync(...) で呼ぶ
pub fn save_settings_sync(settings: &AppSettings, app: &tauri::AppHandle) -> Result<(), String> { ... }
```

---

## Tauri イベント（バックエンド → フロントエンド）

| イベント | ペイロード型 | 説明 |
|---------|-----------|------|
| `batch:progress` | `BatchProgressPayload` | エントリ処理開始・内部進捗更新 |
| `batch:entry_done` | `BatchEntryDonePayload` | 1エントリ完了 |
| `batch:entry_error` | `BatchEntryErrorPayload` | エラーで中断 |
| `batch:done` | `BatchDonePayload` | 全バッチ完了 |
| `batch:cancelled` | `null` | キャンセル完了 |

### ペイロード型定義（TypeScript）

```typescript
interface BatchProgressPayload {
  entry_index: number;        // 0始まり
  total: number;              // 総エントリ数
  entry_name: string;
  entry_progress?: number;    // 0.0–1.0、FFmpeg out_time_ms から算出
}

interface BatchEntryDonePayload {
  entry_name: string;
  output_path: string;
  elapsed_ms: number;         // 残り時間予測用
}

interface BatchEntryErrorPayload {
  entry_name: string;
  message: string;            // 日本語サマリー
  ffmpeg_stderr?: string;     // FFmpeg 由来の場合、stderr 原文
}

interface BatchDonePayload {
  success_count: number;
  error_count: number;
  total_elapsed_ms: number;
}
```

### ペイロード型定義（Rust）

バッチイベントは `app_handle.emit("batch:progress", &payload)` 形式のグローバルイベントとして emit する（`tauri::ipc::Channel` は使わない）:

```rust
#[derive(Clone, Serialize)]
pub struct BatchProgressPayload {
    pub entry_index: usize,
    pub total: usize,
    pub entry_name: String,
    pub entry_progress: Option<f64>,  // 0.0–1.0
}

#[derive(Clone, Serialize)]
pub struct BatchEntryDonePayload {
    pub entry_name: String,
    pub output_path: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct BatchEntryErrorPayload {
    pub entry_name: String,
    pub message: String,
    pub ffmpeg_stderr: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct BatchDonePayload {
    pub success_count: usize,
    pub error_count: usize,
    pub total_elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity { Error, Warning }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,  // "error" | "warning" にシリアライズ
    pub code: String,
    pub message: String,
    pub scene_id: Option<String>,
    pub object_id: Option<String>,
    pub entry_name: Option<String>,
}
```

### イベントリスナーの登録ライフサイクル

`App.tsx` マウント時（`useEffect` 初回実行）に `@tauri-apps/api/event` の `listen` でバッチイベント5種を一括登録し、アプリ終了まで保持する（コンポーネントアンマウントや `unlisten` は不要）。多重 mount を防ぐため登録フラグ（ref または module-level singleton）を使い、2回目以降の `listen` 呼び出しをスキップする。

---

## エラーコード体系

すべての `Result::Err(String)` は `"<code>:"` + 詳細メッセージの形式に統一する。

| コード | 使用コマンド | 意味 |
|--------|------------|------|
| `unsupported_version:` | `open_project` | バージョン不一致 |
| `invalid_yaml:` | `open_project` | YAML パース失敗 |
| `io_error:` | `open_project`, `save_project`, `probe_file` | ファイル I/O エラー |
| `invalid_path:` | `save_project` | 保存先パスが書き込み不可など I/O 以外のパス不正 |
| `batch_already_running:` | `start_batch` | 二重起動 |
| `ffmpeg_error:` | `start_batch` | FFmpeg 終了コード非0 |
| `probe_error:` | `probe_file` | ffprobe 実行失敗 |

> **注意**: `validate_project` は `ValidationResult`（`errors` / `warnings` の配列）を返すのみで、`Result::Err(String)` は返さない。上記 IPC エラーコード（`"<code>:"` 接頭辞付き文字列）と `ValidationIssue.code`（接頭辞なし、例: `"file_not_found"`）は別体系。

### `open_project` のエラー検出・バージョン対応

読み込み時に `version` フィールドを最初にチェックする:

| 状態 | 挙動 |
|------|------|
| `version == 1` | 正常読み込み |
| `version > 1` | 「このプロジェクトファイルはより新しいバージョンの ezmm で作成されています（バージョン {N}）。最新版をご利用ください。」のエラーを表示し読み込みを拒否 |
| `version == 0` または負数相当 | 「バージョン情報が無効です。ファイルが破損しているか、ezmm のプロジェクトファイルではない可能性があります。」のエラー（`"unsupported_version:"` コード）|
| `version` なし / 解析不可 | 「バージョン情報が読み取れません。ファイルが破損しているか、ezmm のプロジェクトファイルではない可能性があります。」のエラー |

---

## Tauri IPC のシリアライズ

`Project` オブジェクトは `serde_json` でシリアライズしてIPCバウンダリを越える。標準的なプロジェクト規模（100エントリ・20シーン）のJSONは数百KBであり、転送は数ms以内。問題なし。
