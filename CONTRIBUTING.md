# ezmm 開発ガイド

このプロジェクトへの貢献・開発に関するルールをまとめます。

---

## 開発環境のセットアップ

依存ツール・バージョン・セットアップ手順: [docs/design/10_infra.md](docs/design/10_infra.md) を参照。

```bash
# FFmpeg バイナリを取得（初回のみ）
bash scripts/download-ffmpeg.sh   # macOS
# powershell scripts/download-ffmpeg.ps1   # Windows

# 開発サーバー起動
pnpm tauri dev
```

---

## 開発プロセス: TDD（テスト駆動開発）

実装はモジュール単位で **TDD サイクル** を回す。

```
Red   — 失敗するテストを先に書く（仕様をテストとして表現する）
Green — テストが通る最小限の実装をする
Refactor — 動作を保ちつつコードを整理する
```

### 鉄則

- **実装はテストの後**。テストなしに実装コードを先に書かない
- **1モジュール完了 = そのモジュールのテストがすべて green** を次のステップへ進む条件とする
- テストは `docs/tasks.md` の実装ステップ順（`project → renderer → batch → commands → store → preview → components`）に対応させる

### テストコマンド

```bash
# Rust
cargo test --manifest-path src-tauri/Cargo.toml

# insta スナップショット更新（ローカルのみ）
INSTA_UPDATE=always cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript
pnpm test

# 両方
cargo test --manifest-path src-tauri/Cargo.toml && pnpm test
```

### CI での制約

CI（GitHub Actions）ではスナップショット更新を行わない（`INSTA_UPDATE=no`）。スナップショットに差分があればテスト失敗とする。

---

## コードスタイル

```bash
# TypeScript: フォーマット・Lint
pnpm format    # Prettier
pnpm lint      # ESLint

# Rust: フォーマット・Lint
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

PR・コミット前に上記をすべてパスさせること。

---

## コミットメッセージ

[Conventional Commits](https://www.conventionalcommits.org/) 形式を使う:

```
feat: 新機能
fix: バグ修正
docs: ドキュメントのみの変更
refactor: 動作を変えないリファクタ
test: テストの追加・修正
chore: ビルドやツール設定の変更
```

---

## ドキュメントの更新ルール

| ドキュメント | 更新タイミング |
|------------|-------------|
| `docs/tasks.md` | 実装ステップ着手時・完了時に進捗を更新。決定・メモ欄に重要な判断を記録 |
| `docs/design/` 各ファイル | 設計変更があったとき（実装プロセスは書かない） |
| `CONTRIBUTING.md`（このファイル） | プロジェクト全体のルールが変わったとき |

> **設計書（`docs/design/`）には「何を作るか」を書く。「どう進めるか」は `CONTRIBUTING.md` と `docs/tasks.md` に書く。**
