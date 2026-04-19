# ezmm 実装タスク

設計書: [docs/design.md](design.md)（目次）→ [docs/design/](design/) 配下の各ファイル

---

## 現在の状態

**フェーズ**: 設計完了・実装未着手

設計書は `docs/design/` に分割済み（2026-04-19）。
次は設計書に沿って実装を開始する。

### 開発方針: TDD

実装は **TDD（テスト駆動開発）** で進める。

```
Red（失敗するテストを先に書く）→ Green（最小実装）→ Refactor（整理）
```

- **「1ステップ完了 = そのモジュールのテストがすべて green」** を条件に次のステップへ進む
- 各 Step の最初のタスクはテスト作成。実装は必ずテストの後

---

## 実装ステップ（依存順）

依存方針: 上流から実装することでステップごとにテスト可能。

### Step 1: `project` モジュール（Rust）
> 依存なし。最初に着手可能。

- `[ ]` `src-tauri/src/project/mod.rs` — `Project` / `Scene` / `SceneObject` / `Entry` Rust 構造体
- `[ ]` YAML デシリアライズ（`serde_yml`）・シリアライズ
- `[ ]` `src-tauri/src/project/migration.rs` — バージョンチェック
- `[ ]` `src-tauri/src/project/validation.rs` — `validate_project()` → `ValidationResult`
- `[ ]` ユニットテスト（YAML パース・バリデーション・insta スナップショット）

参照: [01_project_schema.md](design/01_project_schema.md), [02_validation.md](design/02_validation.md)

---

### Step 2: `renderer` モジュール（Rust）
> 依存: project

- `[ ]` `src-tauri/src/renderer/mod.rs` — `build_filter_complex()` 関数
- `[ ]` 単一シーンの filter_complex 生成（映像・画像・テキスト・音声）
- `[ ]` 複数シーンの concat 対応
- `[ ]` drawtext エスケープ（`escape_drawtext_value()`）
- `[ ]` ffprobe 呼び出し・`ProbeResult` パース
- `[ ]` ユニットテスト（insta スナップショットで FFmpeg コマンドを検証）

参照: [03_renderer.md](design/03_renderer.md)

---

### Step 3: `batch` モジュール（Rust）
> 依存: renderer

- `[ ]` `src-tauri/src/batch/mod.rs` — バッチ実行ループ
- `[ ]` FFmpeg サブプロセス起動・進捗パース（`-progress pipe:1`）
- `[ ]` キャンセル対応（`cancel_requested` フラグ + `ffmpeg_child.kill()`）
- `[ ]` `SleepGuard`（RAII スリープ抑制）
- `[ ]` バッチログ出力（`{output_folder}/ezmm-YYYYMMDD-HHMMSS.log`）

参照: [04_batch.md](design/04_batch.md)

---

### Step 4: `commands` + `state` / `settings`（Rust）
> 依存: batch

- `[ ]` `src-tauri/src/state.rs` — `AppState` 構造体
- `[ ]` `src-tauri/src/settings.rs` — `AppSettings` / `load` / `save` / `save_settings_sync`
- `[ ]` `src-tauri/src/commands/` — IPC コマンド実装（全 11 コマンド）
- `[ ]` `src-tauri/src/main.rs` — Tauri builder / 権限設定 / `on_window_event`
- `[ ]` `src-tauri/capabilities/default.json` — Capability 定義

参照: [05_ipc.md](design/05_ipc.md), [06_state.md](design/06_state.md), [10_infra.md](design/10_infra.md)

---

### Step 5: TypeScript 型定義 + Zustand ストア
> 依存: なし（Rust と並行可能）

- `[ ]` `src/types/` — `Project` / `ProbeResult` / `ValidationResult` / `AppSettings` 等
- `[ ]` `src/store/projectStore.ts` — `ProjectStore`
- `[ ]` `src/store/previewStore.ts` — `PreviewStore`
- `[ ]` `src/store/batchStore.ts` — `BatchStore`（イベントリスナー登録含む）
- `[ ]` `src/store/settingsStore.ts` — `SettingsStore`

参照: [01_project_schema.md](design/01_project_schema.md), [09_store.md](design/09_store.md), [05_ipc.md](design/05_ipc.md)

---

### Step 6: Canvas プレビューエンジン（TypeScript）
> 依存: store

- `[ ]` `src/preview/` — `requestAnimationFrame` ループ
- `[ ]` `<video>` 要素によるフレーム描画（`convertFileSrc` + `requestVideoFrameCallback`）
- `[ ]` 音声再生（Web Audio API）
- `[ ]` フォント読み込み（`get_font_paths` IPC → `@font-face` 注入）
- `[ ]` シーク・エントリ切り替え
- `[ ]` AudioContext autoplay ポリシー対応

参照: [07_preview.md](design/07_preview.md)

---

### Step 7: UI コンポーネント（React）
> 依存: store + preview

- `[ ]` ツールバー（新規・開く・保存・出力先・バッチ実行）
- `[ ]` プレビューパネル（Canvas + シークバー + 再生コントロール）
- `[ ]` プロパティパネル（オブジェクト属性編集・可変値編集）
- `[ ]` タイムライン（シーン・オブジェクト一覧）
- `[ ]` エントリ一覧（チェックボックス・ドラッグ並び替え）
- `[ ]` バッチ進捗ダイアログ
- `[ ]` キーボードショートカット
- `[ ]` 起動時モーダル（Recent Files）
- `[ ]` About ダイアログ

参照: [08_gui.md](design/08_gui.md)

---

## 決定・メモ

| 日付 | 内容 |
|------|------|
| 2026-04-19 | 設計書を `docs/design/` に分割完了。アーキテクチャ方針: Rust は `project → renderer → batch → commands` の単方向依存で実装を直列化 |
| 2026-04-19 | `design.md` は目次として残し、各トピックは `design/01〜10` に格納 |
| 2026-04-19 | 実装はTDDで進める方針を決定。各ステップ完了条件は「テストがすべてgreen」 |

---

## 参考リンク

- [要件定義](requirements.md)
- [設計書インデックス](design.md)
- [プロジェクトスキーマ](design/01_project_schema.md)
- [バリデーション](design/02_validation.md)
- [レンダラー](design/03_renderer.md)
- [バッチ実行](design/04_batch.md)
- [IPC](design/05_ipc.md)
- [AppState / 設定](design/06_state.md)
- [プレビュー](design/07_preview.md)
- [GUI](design/08_gui.md)
- [Zustand ストア](design/09_store.md)
- [インフラ・CI/CD](design/10_infra.md)
