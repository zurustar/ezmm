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
| `validate_project` | `project: Project` | `ValidationResult` | バリデーションのみ実行。書き出し前にも呼べる（空プロジェクト〈シーン0件〉は書き出し時のみエラー扱い。`validate_project` IPC は空でもエラーを返さない） |
| `start_export` | `project: Project` | `void` | 書き出し開始（非同期）。1プロジェクト = 1本の動画を出力。既に書き出し中の場合はエラー。**Project データは stateless**：フロントエンドが毎回送信し、Rust 側はキャッシュしない（ffprobe の ProbeResult キャッシュはセッション内のみ保持） |
| `cancel_export` | なし | `void` | `cancel_requested` フラグを `true` にセットし、かつ `ffmpeg_child.kill()` を呼んで現在レンダリング中のFFmpegプロセスを即時終了させる（同時実行は1書き出しのみのため引数不要） |
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
pub async fn start_export(
    project: Project,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> { ... }

#[tauri::command]
pub async fn cancel_export(state: tauri::State<'_, AppState>) -> Result<(), String> { ... }

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
| `export:progress` | `ExportProgressPayload` | 書き出し進捗更新（FFmpeg out_time_ms から算出） |
| `export:done` | `ExportDonePayload` | 書き出し完了 |
| `export:error` | `ExportErrorPayload` | エラーで中断 |
| `export:cancelled` | `null` | キャンセル完了 |

### ペイロード型定義（TypeScript）

```typescript
interface ExportProgressPayload {
  progress?: number;    // 0.0–1.0、未確定時は undefined
}

interface ExportDonePayload {
  output_path: string;
  elapsed_ms: number;
}

interface ExportErrorPayload {
  message: string;            // 日本語サマリー
  ffmpeg_stderr?: string;     // FFmpeg 由来の場合、stderr 原文
}
```

### ペイロード型定義（Rust）

書き出しイベントは `app_handle.emit("export:progress", &payload)` 形式のグローバルイベントとして emit する（`tauri::ipc::Channel` は使わない）:

```rust
#[derive(Clone, Serialize)]
pub struct ExportProgressPayload {
    pub progress: Option<f64>,  // 0.0–1.0
}

#[derive(Clone, Serialize)]
pub struct ExportDonePayload {
    pub output_path: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct ExportErrorPayload {
    pub message: String,
    pub ffmpeg_stderr: Option<String>,
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

`+layout.svelte` の `onMount` 時に `@tauri-apps/api/event` の `listen` でエクスポートイベント4種（`export:progress`, `export:done`, `export:error`, `export:cancelled`）を一括登録し、アプリ終了まで保持する。

---

## エラーコード体系

すべての `Result::Err(String)` は `"<code>:"` + 詳細メッセージの形式に統一する。

| コード | 使用コマンド | 意味 |
|--------|------------|------|
| `unsupported_version:` | `open_project` | バージョン不一致 |
| `invalid_yaml:` | `open_project` | YAML パース失敗 |
| `io_error:` | `open_project`, `save_project`, `probe_file` | ファイル I/O エラー |
| `invalid_path:` | `save_project` | 保存先パスが書き込み不可など I/O 以外のパス不正 |
| `export_already_running:` | `start_export` | 二重起動 |
| `ffmpeg_error:` | `start_export` | FFmpeg 終了コード非0 |
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
