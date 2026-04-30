# インフラ・開発環境・CI/CD・配布

技術スタック・ディレクトリ構造・クレート依存・開発環境・CI・配布・テスト戦略。

> **参照元**: [設計書インデックス](../design.md)  
> **このモジュールは他モジュールに依存せず、横断的なインフラ情報をまとめる**

---

## 技術スタック

| 役割 | 技術 |
|------|------|
| デスクトップアプリ基盤 | Tauri 2.x（Rustバックエンド＋システムWebView） |
| フロントエンド（GUI） | React + TypeScript |
| リアルタイムプレビュー | Canvas API（WebView内でフレーム合成） |
| 動画レンダリング | FFmpeg（Rustのサブプロセスとして呼び出し） |
| プロジェクトファイル形式 | YAML |
| 配布形式 | シングルバイナリ（Tauriビルド） |

### 選定理由

- **Tauri**: 社内配布PCが低スペックのため、メモリ使用量を最小化する必要がある。ElectronのようにChromiumを同梱せずシステムのWebViewを使うため軽量。シングルバイナリ配布も容易。
- **React + TypeScript**: WebViewベースのUIをコンポーネント指向で開発できる。Canvas APIとの親和性が高く、リアルタイムプレビューの実装がしやすい。
- **FFmpeg**: 動画エンコード・デコードのデファクトスタンダード。対応フォーマットが広く、コーデック変換・フィルタ処理が高品質。Rustのサブプロセスとして呼び出す。
- **Canvas API**: フロントエンドがWebViewベースのため、追加ライブラリなしにリアルタイムフレーム合成が実現できる。FFmpegによる事前レンダリング不要でプレビューが可能。
- **YAML**: 人間が読み書きしやすく、プロジェクトファイルをテキストエディタで確認・共有しやすい。コメント記述も可能。
- **FFmpeg同梱**: ユーザーの追加インストール不要でシングルバイナリ配布を実現。FFmpegはGPLのためソフトウェア全体もGPLライセンスとする。

---

## アーキテクチャ責任分担

**Rustバックエンド（`src-tauri/`）**
- YAMLパース・シリアライズ（`serde_yml`）
- プロジェクトファイルの読み書き・パス解決
- プロジェクトバリデーション
- FFmpegサブプロセス管理・コマンド構築
- バッチ実行エンジン
- 進捗・エラー通知（Tauriイベント経由）

**Reactフロントエンド（`src/`）**
- プロジェクトエディタGUI（シーン・オブジェクト・エントリ・出力設定）
- リアルタイムプレビュー（Canvas + HTMLVideoElement + Web Audio API）
- バッチ実行の起動・進捗表示・キャンセル

---

## ディレクトリ構造

```
ezmm/
├── .nvmrc                  # Node.js バージョン固定
├── rust-toolchain.toml     # Rust バージョン固定
├── package.json            # Node 依存・scripts
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── README.md               # ユーザー向けドキュメント
├── LICENSE                 # GPL-3.0
├── CHANGELOG.md            # リリースノート
├── src-tauri/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── tauri.conf.json     # Tauri 設定（bundle/security/windows 等）
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs
│   │   ├── state.rs        # AppState 構造体（Mutex 管理ランタイム状態）
│   │   ├── settings.rs     # AppSettings / WindowSettings 構造体・impl Default・load/save ロジック
│   │   ├── commands/       # Tauri IPCコマンド
│   │   ├── project/        # YAMLスキーマ・Rust構造体・バリデーション
│   │   │   ├── migration.rs
│   │   │   └── snapshots/  # insta スナップショット（git コミット対象）
│   │   ├── renderer/       # FFmpegコマンド生成
│   │   └── batch/          # バッチ実行エンジン
│   ├── binaries/           # 同梱FFmpeg・ffprobe（.gitignore対象、CIで取得）
│   ├── fonts/              # 同梱フォント（NotoSansCJK-Regular.otf / Bold.otf）
│   ├── icons/              # アプリアイコン（.icns / .ico / PNG各サイズ）
│   └── capabilities/       # Tauri Capability 定義（default.json 等）
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/         # UIコンポーネント
│   ├── hooks/              # Reactカスタムフック
│   ├── store/              # Zustandストア
│   ├── types/              # TypeScript型定義
│   └── preview/            # Canvasプレビューエンジン
├── scripts/
│   ├── download-ffmpeg.sh  # FFmpeg/ffprobe ダウンロード（macOS）
│   └── download-ffmpeg.ps1 # FFmpeg/ffprobe ダウンロード（Windows）
├── examples/               # サンプルプロジェクトYAML（minimal/standard/audio-rich/text-heavy）
├── docs/
│   ├── requirements.md
│   ├── design.md
│   ├── design/             # 設計書（本ドキュメント群）
│   └── tasks.md
└── .github/
    └── workflows/          # CI 定義（GitHub Actions）
```

---

## バージョン固定

| ツール | バージョン | 設定ファイル |
|-------|---------|------------|
| Node.js | 20.x LTS | `.nvmrc` |
| pnpm | 9.x | `package.json#packageManager` |
| Rust | 1.78以上（stable） | `rust-toolchain.toml` |

---

## Rust クレート依存（`src-tauri/Cargo.toml`）

```toml
[dependencies]
tauri = { version = "2", features = [] }              # Tauri 2.x: asset protocol 等は tauri.conf.json で制御。システムトレイ不使用のため features 空で可
tauri-plugin-dialog = "2"             # Rust 側からダイアログを開く必要はないが、JS 側 API（@tauri-apps/plugin-dialog）を有効化するためプラグイン初期化が必要
tauri-plugin-opener = "2"    # フォルダ・ファイルをOS標準アプリで開く
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"          # serde_yaml（archived 2024）の後継フォーク。API 互換（旧: `serde_yaml::` → 新: `serde_yml::`）
indexmap = { version = "2", features = ["serde"] }  # Entry.variables 用
dunce = "1"                   # Windows UNC パスなし canonicalize
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-appender = "0.2.3"    # ログローテーション（max_log_files は 0.2.3 以降が必要）
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1"               # コマンドビルダー等の内部エラー型定義に使用
anyhow = "1"                  # main.rs .setup() クロージャの Box<dyn Error> を簡略化

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_System_Power"] }  # SetThreadExecutionState

[dev-dependencies]
insta = "1"                   # スナップショットテスト

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

---

## Node.js 依存（`package.json`）

```json
{
  "name": "ezmm",
  "private": true,
  "version": "0.1.0",
  "packageManager": "pnpm@9.0.0",
  "engines": { "node": ">=20" },
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "lint": "eslint src --ext ts,tsx",
    "format": "prettier --write src"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "react": "^18",
    "react-dom": "^18",
    "zustand": "^4"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^18",
    "@types/react-dom": "^18",
    "@typescript-eslint/eslint-plugin": "^7",
    "@typescript-eslint/parser": "^7",
    "@vitejs/plugin-react": "^4",
    "eslint": "^8",
    "prettier": "^3",
    "typescript": "^5",
    "vite": "^5",
    "vitest": "^1"
  }
}
```

---

## `src-tauri/build.rs` 骨格

`env!("TARGET")` をメインクレートで使うには、`build.rs` が `TARGET` 環境変数を `cargo:rustc-env` で伝搬する必要がある:

```rust
fn main() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").expect("TARGET env var not set")
    );
    tauri_build::build()
}
```

---

## `src-tauri/src/main.rs` 骨格

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;  // app.manage() / app.path() に必要

mod commands;
mod state;      // AppState 構造体
mod settings;   // AppSettings / WindowSettings 構造体・impl Default
mod project;    // YAML スキーマ・Rust 構造体・バリデーション
mod renderer;   // FFmpeg コマンド生成
mod batch;      // バッチ実行エンジン
use state::AppState;
use settings::AppSettings;

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder, PredefinedMenuItem};
    let file_menu = SubmenuBuilder::new(app, "ファイル")
        .item(&MenuItemBuilder::with_id("new", "新規").accelerator("CmdOrCtrl+N").build(app)?)
        .item(&MenuItemBuilder::with_id("open", "開く...").accelerator("CmdOrCtrl+O").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("save", "保存").accelerator("CmdOrCtrl+S").build(app)?)
        .item(&MenuItemBuilder::with_id("save_as", "名前を付けて保存...").accelerator("CmdOrCtrl+Shift+S").build(app)?)
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "編集")
        .item(&MenuItemBuilder::with_id("undo", "取り消し").accelerator("CmdOrCtrl+Z").enabled(false).build(app)?)
        .item(&MenuItemBuilder::with_id("redo", "やり直し").accelerator("CmdOrCtrl+Shift+Z").enabled(false).build(app)?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .separator()
        .item(&MenuItemBuilder::with_id("duplicate", "選択を複製").accelerator("CmdOrCtrl+D").build(app)?)
        .build()?;
    let help_menu = SubmenuBuilder::new(app, "ヘルプ")
        .item(&MenuItemBuilder::with_id("about", "ezmm について").build(app)?)
        .item(&MenuItemBuilder::with_id("open_log_dir", "ログフォルダを開く").build(app)?)
        .build()?;
    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&help_menu)
        .build()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .menu(build_menu)
        .setup(|app| {
            // tracing 初期化: 日次ローテーション、最大5ファイル保持
            let log_dir = app.path().app_log_dir()
                .map_err(|e| format!("app_log_dir failed: {e}"))?;
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| format!("create_dir_all failed: {e}"))?;
            let file_appender = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .max_log_files(5)
                .filename_prefix("ezmm")
                .filename_suffix("log")
                .build(&log_dir)
                .map_err(|e| format!("log appender build failed: {e}"))?;
            let (non_blocking, log_guard) = tracing_appender::non_blocking(file_appender);
            let debug_mode = std::env::var("EZMM_DEBUG").is_ok();
            let log_filter = if debug_mode { "debug" } else { "info" };
            tracing_subscriber::fmt()
                .with_writer(non_blocking)
                .with_env_filter(log_filter)
                .init();

            // AppState 初期化（ffmpeg/ffprobe パスをここで解決して保持）
            let resource_dir = app.path().resource_dir()
                .expect("resource_dir not found");
            let font_dir = resource_dir.join("fonts");
            let target = env!("TARGET");
            let bin_dir = std::env::current_exe()
                .expect("current_exe failed")
                .parent()
                .expect("no parent dir")
                .to_owned();
            let ffmpeg_bin = if cfg!(target_os = "windows") {
                format!("ffmpeg-{target}.exe")
            } else {
                format!("ffmpeg-{target}")
            };
            let ffprobe_bin = if cfg!(target_os = "windows") {
                format!("ffprobe-{target}.exe")
            } else {
                format!("ffprobe-{target}")
            };
            // 設定ファイルを読み込んで current_settings を初期化
            let initial_settings = {
                let settings_path = app.path().app_config_dir()
                    .map(|d| d.join("settings.json"))
                    .ok();
                if let Some(ref path) = settings_path {
                    if let Ok(raw) = std::fs::read_to_string(path) {
                        if let Ok(parsed) = serde_json::from_str::<AppSettings>(&raw) {
                            if parsed.version != 1 {
                                let bak = path.with_extension("json.bak");
                                let _ = std::fs::rename(path, &bak);
                                tracing::warn!("settings.json version {} unsupported, backed up to {:?}", parsed.version, bak);
                            }
                        }
                    }
                }
                settings_path
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
                    .filter(|s| s.version == 1)
                    .unwrap_or_default()
            };
            app.manage(AppState {
                probe_cache: Mutex::new(HashMap::new()),
                ffmpeg_version: Mutex::new(None),
                current_settings: Mutex::new(initial_settings),
                batch_running: Mutex::new(false),
                cancel_requested: Mutex::new(false),
                ffmpeg_child: Mutex::new(None),
                font_dir,
                ffmpeg_path: bin_dir.join(&ffmpeg_bin),
                ffprobe_path: bin_dir.join(&ffprobe_bin),
                debug_mode,
                log_guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_project,
            commands::save_project,
            commands::validate_project,
            commands::start_batch,
            commands::cancel_batch,
            commands::check_output_conflicts,
            commands::get_ffmpeg_version,
            commands::probe_file,
            commands::get_font_paths,
            commands::load_settings,
            commands::save_settings,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let pos = window.outer_position().ok();
                let size = window.outer_size().ok();
                let mut settings = state.current_settings.lock().unwrap();
                if let Some(s) = size {
                    settings.window.width = s.width;
                    settings.window.height = s.height;
                }
                if let Some(p) = pos {
                    settings.window.x = Some(p.x);
                    settings.window.y = Some(p.y);
                }
                let _ = settings::save_settings_sync(&*settings, &app);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Tauri Capability（必要権限一覧）

`src-tauri/capabilities/default.json`:
```json
{
  "identifier": "default",
  "description": "default capability",
  "windows": ["main"],
  "permissions": [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "dialog:allow-open",
    "dialog:allow-save",
    "opener:allow-open-path"
  ]
}
```

---

## CSP（Content Security Policy）

```
default-src 'self' http://asset.localhost https://asset.localhost;
script-src 'self' 'unsafe-inline';
style-src 'self' 'unsafe-inline';
media-src 'self' http://asset.localhost https://asset.localhost;
img-src 'self' data: http://asset.localhost https://asset.localhost;
font-src 'self' http://asset.localhost https://asset.localhost;
connect-src 'self' ipc: http://ipc.localhost https://ipc.localhost;
```

---

## tauri.conf.json 統合サンプル

```json
{
  "productName": "ezmm",
  "identifier": "io.github.zurustar.ezmm",
  "version": "../Cargo.toml",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "security": {
      "csp": "default-src 'self' http://asset.localhost https://asset.localhost; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; media-src 'self' http://asset.localhost https://asset.localhost; img-src 'self' data: http://asset.localhost https://asset.localhost; font-src 'self' http://asset.localhost https://asset.localhost; connect-src 'self' ipc: http://ipc.localhost https://ipc.localhost",
      "assetProtocol": {
        "enable": true,
        "scope": ["$RESOURCE/fonts/**", "**"]
      }
    },
    "windows": [
      {
        "title": "ezmm",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 640,
        "resizable": true,
        "dragDropEnabled": false
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "msi"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": ["fonts/NotoSansCJK-Regular.otf", "fonts/NotoSansCJK-Bold.otf"],
    "externalBin": [
      "binaries/ffmpeg",
      "binaries/ffprobe"
    ],
    "windows": {
      "webviewInstallMode": {
        "type": "embedBootstrapper",
        "silent": false
      }
    }
  }
}
```

---

## Tauri アプリ基本設定

### Bundle Identifier

`io.github.zurustar.ezmm`

### アプリ名

- **表示名**: `ezmm`
- tabBar: `ezmm — {ファイル名}` 形式（詳細は [08_gui.md](08_gui.md) を参照）

### アプリアイコン

- **ソース**: 1024×1024 PNG（シンプルな映像フィルム＋歯車アイコン）
- **生成**: `pnpm tauri icon <source.png>` で各サイズを自動生成
- **配置**: `src-tauri/icons/`

### ウィンドウサイズ

| 項目 | 値 |
|------|-----|
| 初期サイズ | 1280 × 800 |
| 最小サイズ | 1024 × 640 |
| 初期位置 | OS デフォルト（中央配置） |
| 状態復元 | アプリ終了時のサイズ・位置を `settings.json` に保存し、次回起動時に復元 |

### macOS メニューバー構成

| メニュー | 項目 |
|--------|------|
| ezmm | ezmm について |
| ファイル | 新規 (⌘N) / 開く... (⌘O) / — / 保存 (⌘S) / 名前を付けて保存... (⌘⇧S) / — / 最近開いたファイル ▶ |
| 編集 | 取り消し (⌘Z、グレーアウト) / やり直し (⌘⇧Z、グレーアウト) / — / カット / コピー / ペースト / — / 選択を複製 (⌘D) |
| ヘルプ | アプリログフォルダを開く / バッチログフォルダを開く |

### Windows メニューバー

なし（macOS のようなメニューバーは実装しない）。操作はすべてツールバーまたはコンテキストメニューから行う。

---

## Lint / Formatter

- **TypeScript**: ESLint（`@typescript-eslint/recommended`） + Prettier
- **Rust**: `rustfmt`（`cargo fmt`） + Clippy（`cargo clippy -- -D warnings`）

---

## テストフレームワーク

- **Rust**: 標準の `#[test]` + `insta`（スナップショットテスト、filter_complexコマンド生成の検証に使用）。スナップショットファイルは git にコミットする。CI ではスナップショットの更新を行わず（`INSTA_UPDATE=no`）、差分があればテスト失敗とする
- **TypeScript**: Vitest

---

## テスト戦略

概要:
- **Rustユニットテスト**: YAMLパース、バリデーション、filter_complexコマンド生成（`insta` スナップショット）
- **TypeScriptユニットテスト**: 型ガード、Zustandストアのロジック（Vitest）
- **統合テスト**: ローカル手動確認のみ（CIでは除外）
- **レンダリング品質確認**: 目視確認（自動ピクセルdiffは初期バージョンでは対象外）

---

## CI（GitHub Actions）

```
jobs:
  test-mac:
    runs-on: macos-latest
    steps: [checkout, setup-rust, setup-node, install-deps, download-ffmpeg, cargo-test, vitest]
  test-win:
    runs-on: windows-latest
    steps: [checkout, setup-rust, setup-node, install-deps, download-ffmpeg, cargo-test, vitest]
  build-mac:
    runs-on: macos-latest
    needs: test-mac
    steps: [..., download-ffmpeg, pnpm tauri build]
  build-win:
    runs-on: windows-latest
    needs: test-win
    steps: [..., download-ffmpeg, pnpm tauri build]
```

```yaml
- name: Download FFmpeg
  run: bash scripts/download-ffmpeg.sh  # macOS
  # run: powershell scripts/download-ffmpeg.ps1  # Windows
```

CI では Rust ユニットテストと Vitest のみ実行する（FFmpeg 実行を必要とする統合テストは CI 対象外）。

---

## 配布・ビルド

| 項目 | 方針 |
|------|------|
| Mac配布形式 | `.dmg`（**2バイナリ別配布**: arm64 用と x86_64 用それぞれ個別にビルド・配布。Tauri 2.x の `externalBin` はターゲット別サフィックス付きバイナリを要求するため、単一 Universal Binary での配布はせず、`aarch64-apple-darwin` / `x86_64-apple-darwin` の .dmg を2本リリース） |
| Windows配布形式 | `.msi` インストーラ |
| 署名・公証 | 社内配布のため不要 |
| FFmpeg同梱 | Tauriサイドカー（`src-tauri/binaries/`）でバンドル |
| 初回起動 | ログディレクトリを自動作成（Mac: `~/Library/Logs/io.github.zurustar.ezmm/`、Win: `%APPDATA%\io.github.zurustar.ezmm\logs\`） |
| アップデート | 手動配布（自動アップデートなし、初期バージョン） |

**ビルドコマンド:**

```bash
pnpm tauri dev    # 開発
pnpm tauri build  # 本番ビルド
```

---

## 開発時のFFmpeg入手

`scripts/download-ffmpeg.sh`（macOS）/ `scripts/download-ffmpeg.ps1`（Windows）を用意。
スタティックビルド済みバイナリを `src-tauri/binaries/` に配置する手順を自動化。

**ダウンロード元:**
- **macOS (x86_64)**: [evermeet.cx](https://evermeet.cx/ffmpeg/) の GPL スタティックビルド（ffmpeg / ffprobe を個別 ZIP で提供）
- **macOS (arm64)**: [martin-riedl/ffmpeg-static](https://github.com/martin-riedl/ffmpeg-static) の macOS arm64 GPL ビルド
- **Windows**: [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) の `ffmpeg-master-win64-gpl` ビルド

---

## リリース手順

1. `CHANGELOG.md` を更新
2. `git tag v0.1.0` でタグを打つ
3. GitHub Actions が自動でビルドし GitHub Releases にアーティファクトを添付

---

## セマンティックバージョニング

`v0.x.y` から開始。スキーマ変更（`version` フィールドのインクリメント）は `MINOR` バージョンアップ。初回リリースは `v0.1.0`。

**バージョン文字列の同期:**
- `src-tauri/Cargo.toml` の `[package] version` を**唯一の正**とする
- `src-tauri/tauri.conf.json` の `version` は `"../Cargo.toml"` を指定して Cargo から自動取得
- `package.json` の `version` は同期対象外

---

## GPL遵守・ライセンス表記

- `cargo-about` で Rust 依存ライブラリのライセンスを収集
- `license-checker` で Node.js 依存ライブラリのライセンスを収集
- 両者をまとめた `THIRD_PARTY_LICENSES.txt` をビルド成果物に同梱
- NotoSansCJKのSIL OFL 1.1ライセンスも同ファイルに記載

---

## ドキュメント・配布物計画

| 成果物 | タイミング | 内容 |
|--------|----------|------|
| README.md | v0.1.0リリース前 | 概要・インストール・使い方（日本語）・スクリーンショット |
| ユーザーマニュアル | v0.1.0リリース後 | 操作詳細・FAQ（日本語） |
| CHANGELOG.md | 各リリース | ConventionalCommits形式で手動メンテ |
| THIRD_PARTY_LICENSES.txt | ビルド時自動生成 | GPL同梱要件・OSSライセンス一覧 |
| `examples/minimal.yaml` | 実装中 | 1シーン・1映像オブジェクト（固定）・1エントリ。スキーマの最小構成確認用 |
| `examples/standard.yaml` | 実装中 | 3シーン（intro/main/outro）。main に可変映像+固定BGM+固定ロゴ+可変テキスト。エントリ2件 |
| `examples/audio-rich.yaml` | 実装中 | 2シーン。複数音声オブジェクト、`loop: loop` と `loop: silence` の両方、フェードあり |
| `examples/text-heavy.yaml` | 実装中 | 2シーン。複数テキストオブジェクト、全 `align` パターン、`background_color` あり/なし両方 |

---

## examples/ に置くファイル

`examples/` ディレクトリには **YAML ファイルのみ**を置く。
メディアファイル（mp4/png/mp3）は Git リポジトリに含めない（バイナリによるリポジトリ肥大化を避けるため）。Git LFS は使用しない。

---

## デバッグモード

**有効化方法**: 環境変数 `EZMM_DEBUG=1` を設定して起動する。

**実装**: `.setup()` 内で `std::env::var("EZMM_DEBUG").is_ok()` を判定し、`AppState.debug_mode: bool` に保持する。

**デバッグモード時の挙動**:
- バッチ実行画面の「詳細」欄に FFmpeg へ渡すコマンドライン引数を表示する
- FFmpeg の `-loglevel` を `verbose` に切り替える
- アプリログのレベルを DEBUG に引き上げる（通常は INFO）

アプリ設定画面からの切り替えは v2 以降。
