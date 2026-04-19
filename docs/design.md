# 設計ドキュメント インデックス

社内向け動画バッチ編集ツール **ezmm** の設計書インデックス。
各トピックの詳細は以下のファイルを参照。

---

## アーキテクチャ概要

### 技術スタック

| 役割 | 技術 |
|------|------|
| デスクトップアプリ基盤 | Tauri 2.x（Rust バックエンド＋システム WebView） |
| フロントエンド（GUI） | React + TypeScript |
| リアルタイムプレビュー | Canvas API（WebView 内でフレーム合成） |
| 動画レンダリング | FFmpeg（Rust のサブプロセスとして呼び出し） |
| プロジェクトファイル形式 | YAML |
| 配布形式 | シングルバイナリ（Tauri ビルド） |

### モジュール依存グラフ

依存は**一方向のみ**。下位モジュールから先に実装・テストできる。

```
【Rust バックエンド】

  project（データモデル・YAML スキーマ・バリデーション）
      ↓ 依存
  renderer（FFmpeg コマンド生成）
      ↓ 依存
  batch（バッチ実行エンジン・進捗通知）
      ↓ 依存
  commands（Tauri IPC エントリポイント）

  ※ state / settings（AppState・AppSettings）は横断的関心事として全層から参照される

【フロントエンド（React + TypeScript）】

  types（TypeScript 型定義）
      ↓ 依存
  store（Zustand 状態管理）
      ↓ 依存
  preview（Canvas プレビューエンジン）
      ↓ 依存
  components（UI コンポーネント）

フロント ⇄ Rust は Tauri IPC のみで通信（境界明確）
```

### 推奨実装順序

| ステップ | モジュール | 前提 |
|--------|----------|------|
| 1 | `project`（Rust） | なし |
| 2 | `renderer`（Rust） | project |
| 3 | `batch`（Rust） | renderer |
| 4 | `commands`（Rust） | batch |
| 5 | `types` + `store`（TS） | Rust と並行可 |
| 6 | `preview`（TS） | store |
| 7 | `components`（TS） | preview + store |

---

## ディレクトリ構造（概要）

```
ezmm/
├── src-tauri/src/
│   ├── project/    # スキーマ・バリデーション（最上流）
│   ├── renderer/   # FFmpeg コマンド生成
│   ├── batch/      # バッチ実行エンジン
│   ├── commands/   # Tauri IPC
│   ├── state.rs    # AppState
│   └── settings.rs # AppSettings
├── src/
│   ├── types/      # TypeScript 型定義
│   ├── store/      # Zustand ストア
│   ├── preview/    # Canvas プレビューエンジン
│   └── components/ # UI コンポーネント
└── docs/
    ├── requirements.md
    ├── design.md       ← このファイル（インデックス）
    └── design/         ← 詳細設計書（各トピック別）
```

---

## 設計書ファイル一覧

| ファイル | 内容 |
|--------|------|
| [01_project_schema.md](design/01_project_schema.md) | YAML スキーマ・Rust/TS データモデル・スキーマバージョン・マイグレーション |
| [02_validation.md](design/02_validation.md) | バリデーション仕様・ValidationIssue コード一覧・エラーと警告の区別 |
| [03_renderer.md](design/03_renderer.md) | FFmpeg レンダリングパイプライン・filter_complex 構築・音声ミキシング・エスケープ |
| [04_batch.md](design/04_batch.md) | バッチ実行エンジン・進捗・キャンセル・ログ・スリープ抑制 |
| [05_ipc.md](design/05_ipc.md) | Tauri IPC コマンド一覧・イベント定義・エラーコード体系 |
| [06_state.md](design/06_state.md) | AppState・AppSettings・settings.json・ファイル I/O 信頼性 |
| [07_preview.md](design/07_preview.md) | リアルタイムプレビュー（Canvas・Web Audio API・メモリ管理） |
| [08_gui.md](design/08_gui.md) | GUI 設計・画面レイアウト・UX 詳細・キーボードショートカット |
| [09_store.md](design/09_store.md) | Zustand 状態管理（ProjectStore・PreviewStore・BatchStore・SettingsStore） |
| [10_infra.md](design/10_infra.md) | 技術スタック・開発環境・CI/CD・配布・テスト戦略・main.rs 骨格 |

---

## 変更履歴

| 日付 | 内容 |
|------|------|
| 2026-04-15 | 初版作成 |
| 2026-04-18 | 「テンプレート」「ジョブ」を廃止し「プロジェクト」に統合。全オブジェクト対等・可変モデル（映像・画像・テキスト・音声の4種別）。出力設定・オブジェクトID・Z順を追加。BGMを廃止し音声オブジェクトとして統合 |
| 2026-04-19 | アーキテクチャ整理に伴い design.md を目次化し design/ サブディレクトリに分割 |
