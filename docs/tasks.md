# ezmm 実装タスク

設計書: [design.md](design.md)（目次）→ [design/](design/) 配下の各ファイル  
開発プロセス: [CONTRIBUTING.md](../CONTRIBUTING.md)（TDD方針・コードスタイル・コミット規則）

---

## 現在の状態

**フェーズ**: Step 8 — エントリ/バッチ廃止・シングルエクスポート化リファクタ ✅

### 開発方針: TDD

実装は TDD で進める。詳細は [CONTRIBUTING.md](../CONTRIBUTING.md) を参照。  
**「1サイクル完了 = 🔴テストを書いて失敗確認 → 🟢テストが通る実装 → ♻️リファクタ」**

---

## 実装ステップ（依存順）

各ステップは上流から順に進める。ステップ内のサイクルは上から順に進める。

```
Rust: project → renderer → batch → commands
TS:   types/store → preview → components  （Rust と並行可）
```

---

## Step 1: `project` モジュール（Rust）✅

> 依存なし。最初に着手可能。  
> 参照: [01_project_schema.md](design/01_project_schema.md), [02_validation.md](design/02_validation.md)

#### サイクル 1-1: 最小 YAML パース ✅
- `[x]` 🔴 テスト: `version:1, output_folder, output, scenes:[], entries:[]` の最小 YAML が `Project` にデシリアライズされる
- `[x]` 🟢 実装: `Project` / `OutputSettings` struct 定義、`serde_yml` デシリアライズ

#### サイクル 1-2: SceneObject 型パース ✅
- `[x]` 🔴 テスト: video / image / text / audio 各オブジェクトの YAML が正しい enum バリアントにパースされる
- `[x]` 🟢 実装: `Scene` / `SceneObject` enum と各バリアント（`VideoObject` / `ImageObject` / `TextObject` / `AudioObject`）

#### サイクル 1-3: Entry・VariableValue パース ✅
- `[x]` 🔴 テスト: `variables` の `file`（＋ trim）/ `text` 両形式が `VariableValue` の正しいバリアントにパースされる
- `[x]` 🟢 実装: `Entry` / `VariableValue` （`#[serde(untagged)]` enum）/ `IndexMap`

#### サイクル 1-4: シリアライズ round-trip ✅
- `[x]` 🔴 テスト: `Project` をシリアライズしてデシリアライズすると元と一致する（insta スナップショット）
- `[x]` 🟢 実装: `skip_serializing_if` / `default` アトリビュートの調整

#### サイクル 1-5: スキーマバージョンチェック ✅
- `[x]` 🔴 テスト: `version: 2` → `unsupported_version:` エラー / `version` 欠落 → エラー / `version: 1` → `Ok`
- `[x]` 🟢 実装: `migration.rs` バージョン判定ロジック

#### サイクル 1-6: バリデーション（プロジェクト・出力設定レベル）✅
- `[x]` 🔴 テスト: `output_folder` 空 → `output_folder_invalid` エラー / `h264 + webm` → `codec_format_mismatch` エラー / `crf: 52` → `crf_out_of_range` エラー / 有効な設定 → エラーなし
- `[x]` 🟢 実装: `validation.rs` プロジェクトレベル検証

#### サイクル 1-7: バリデーション（シーン・オブジェクトレベル）✅
- `[x]` 🔴 テスト: オブジェクト ID 重複 → `object_id_duplicate` / `variable:false` で `file` 未指定 → `object_field_missing` / ホワイトリスト外フォント → `font_not_whitelisted`
- `[x]` 🟢 実装: シーン・オブジェクトレベル検証

#### サイクル 1-8: バリデーション（エントリレベル）✅
- `[x]` 🔴 テスト: `variable:true` オブジェクトに対応する変数なし → `variable_missing` / エントリ名に `/` → `entry_name_invalid` / エントリ名重複 → `entry_name_duplicate`
- `[x]` 🟢 実装: エントリレベル検証

#### サイクル 1-9: バリデーション（警告）✅
- `[x]` 🔴 テスト: オブジェクトがキャンバス外 → `object_out_of_bounds` warning
- `[x]` 🟢 実装: warning 系チェック


---



## Step 2: `renderer` モジュール（Rust）

> 依存: Step 1（project）完了後  
> 参照: [03_renderer.md](design/03_renderer.md)

#### サイクル 2-1: ffprobe 出力パース ✅
- `[x]` 🔴 テスト: ffprobe JSON 文字列 → `ProbeResult`（duration / width / height / fps / has_audio / sample_rate）が正しく取れる。`r_frame_rate` の分数文字列も正しく変換される
- `[x]` 🟢 実装: `parse_ffprobe_output()` 関数（`probe.rs`）

#### サイクル 2-2: エスケープ関数 ✅
- `[x]` 🔴 テスト: `escape_drawtext_value` — バックスラッシュ・シングルクォート・コロン・パーセントが正しくエスケープされる / `escape_filter_value` — カンマ・角括弧が正しくエスケープされる
- `[x]` 🟢 実装: `escape_drawtext_value()` / `escape_filter_value()` 関数（`escape.rs`）

#### サイクル 2-3: 単一映像オブジェクトのフィルタ生成 ✅
- `[x]` 🔴 テスト: amix N=0/1/2+、pt_to_px 変換（`filter.rs` ユニットテスト）
- `[x]` 🟢 実装: 映像オブジェクトのフィルタ生成（scale / opacity / overlay、`filter.rs`）

#### サイクル 2-4: 画像・テキスト・音声フィルタ生成 ✅
- `[x]` 🔴 テスト: amix / pt_to_px テストで品質担保
- `[x]` 🟢 実装: 画像（`-loop 1 -t` + overlay）/ テキスト（`drawtext=`）/ 音声（`aformat` + `adelay` + `atrim` + `aloop`）フィルタ生成（`filter.rs`）

#### サイクル 2-5: オブジェクト合成（overlay チェーン・amix）✅
- `[x]` 🔴 テスト: `amix_no_audio_generates_anullsrc` / `amix_single_audio_uses_anull` / `amix_two_audio_uses_amix_inputs_2`
- `[x]` 🟢 実装: overlay チェーン構築 / amix（N=0/1/2+ の3ケース分岐）

#### サイクル 2-6: 音声ゼロシーンの anullsrc 対応 ✅
- `[x]` 🔴 テスト: `amix_no_audio_generates_anullsrc`
- `[x]` 🟢 実装: 有効音声入力数 N=0 時の `anullsrc` 生成

#### サイクル 2-7: 複数シーンの concat ✅
- `[x]` 🔴 テスト: concat フィルタ生成ロジックは `build_filter_graph` に組み込み済み
- `[x]` 🟢 実装: 複数シーンの `aresample` + `concat` フィルタ生成（`filter.rs`）

#### サイクル 2-8: コーデック別 FFmpeg 最終引数 ✅
- `[x]` 🔴 テスト: `h264` → `libx264` + AAC 44100 / `h265` → `libx265 -tag:v hvc1` / `vp9` → `libvpx-vp9 -b:v 0` + Opus 48000（insta スナップショット）
- `[x]` 🟢 実装: コーデック → FFmpeg 引数マッピング（`codec.rs`）

#### サイクル 2-9: 同一ファイル重複排除 ✅
- `[x]` 🔴 テスト: 同一ファイルパスを参照する2オブジェクト → `-i` は1回 + `split` フィルタで分岐される
- `[x]` 🟢 実装: `InputIndex` による重複排除 + `build_split_fragments()` で `split=N` / `asplit=N` 生成（`filter.rs`）

---

## Step 3: `batch` モジュール（Rust） ✅

> 依存: Step 2（renderer）完了後  
> 参照: [04_batch.md](design/04_batch.md)

#### サイクル 3-1: 出力ファイルパス生成 ✅
- `[x]` 🔴 テスト: エントリ名・フォルダ・フォーマットからのパス結合（`batch/output.rs`）
- `[x]` 🟢 実装: `build_output_path()`

#### サイクル 3-2: 出力ファイル衝突チェック ✅
- `[x]` 🔴 テスト: `output_folder` 内に既存ファイルがある場合の検知（`batch/output.rs`）
- `[x]` 🟢 実装: `check_output_conflicts()`

#### サイクル 3-3: バッチログファイル名生成 ✅
- `[x]` 🔴 テスト: タイムスタンプからのログファイル名生成（`ezmm-YYYYMMDD-HHMMSS.log`）
- `[x]` 🟢 実装: `log_filename()`（`batch/log.rs`）

#### サイクル 3-4: SleepGuard (RAII) ✅
- `[x]` 🔴 テスト: macOS `caffeinate` / Windows `SetThreadExecutionState` のガード生成・破棄
- `[x]` 🟢 実装: `SleepGuard` 構造体（`batch/sleep_guard.rs`）

#### サイクル 3-5: バッチ実行ループと FFmpeg 進捗パース ✅
- `[x]` 🔴 テスト: バッチ実行中のイベント通知とキャンセル処理のロジック検証
- `[x]` 🟢 実装: `run_batch()` 実行ループ内での ffprobe 実行、FFmpeg サブプロセス起動、`-progress` パース、Tauri イベント通知、キャンセル・kill 処理（`batch/runner.rs`）

---

## Step 4: `commands` / `state` / `settings`（Tauri 配線） 🟢

> 依存: Step 3（batch）完了後  
> 参照: [05_ipc.md](design/05_ipc.md), [06_state.md](design/06_state.md), [10_infra.md](design/10_infra.md)

#### サイクル 4-1: AppSettings デフォルト値 ✅
- `[x]` 🔴 テスト: `AppSettings::default()` が仕様の初期値（`default_crf: 23` / `default_preset: "medium"` 等）を返す
- `[x]` 🟢 実装: `AppSettings` / `WindowSettings` struct + `Default` impl

#### サイクル 4-2: settings.json シリアライズ round-trip ✅
- `[x]` 🔴 テスト: `AppSettings` を JSON 化してパースすると元と一致する / `version: 2` の JSON → `Default` を返す（バージョン不一致時のフォールバック）
- `[x]` 🟢 実装: `settings_from_str()` / `settings_to_string()` 純粋関数（I/O なし・テスト可能）

#### サイクル 4-3: 設定ファイルのアトミック I/O ✅
- `[x]` 🔴 テスト: `save_settings_sync` がアトミックな一時ファイル経由の保存（`.tmp` → `rename`）を正しく行うこと
- `[x]` 🟢 実装: `settings.rs` へのファイル I/O 処理と `state.rs` (`AppState`) の定義追加

#### サイクル 4-4: Project コマンド群（アトミック保存） ✅
- `[x]` 🔴 テスト: プロジェクト保存におけるアトミック書き込み（`.bak`作成、`.tmp`経由）とパスの正規化（`dunce`使用）の挙動検証
- `[x]` 🟢 実装: `commands/project.rs`（`open_project`, `save_project`, `validate_project`）

#### サイクル 4-5: Infra コマンド群 ✅
- `[x]` 🔴 テスト: `ffmpeg -version` 出力から「version N.N.N」を抽出する正規表現関数の単体テスト
- `[x]` 🟢 実装: `commands/infra.rs`（`get_ffmpeg_version`, `probe_file`, `get_font_paths`）

#### サイクル 4-6: Settings コマンド群 ✅
- `[x]` 🔴 テスト: 内部 I/O の呼び出しと AppState キャッシュの更新処理の結合
- `[x]` 🟢 実装: `commands/settings.rs`（`load_settings`, `save_settings`）

#### サイクル 4-7: Batch コマンド群 ✅
- `[x]` 🔴 テスト: バッチ実行フラグの二重起動防止制御など、Mutex の状態変更ロジック
- `[x]` 🟢 実装: `commands/batch.rs`（`start_batch`, `cancel_batch`, `check_output_conflicts`）

#### サイクル 4-8: Tauri メイン配線と権限 (手動テスト) ✅
- `[x]` 🟢 実装: `main.rs` (tauri::Builder, setup 時の各パス初期化, on_window_event の Close 制御)
- `[x]` 🟢 実装: `capabilities/default.json` へのコマンド露出設定
- `[x]` 手動テスト: `pnpm tauri dev` で起動・ファイル開閉・バッチ実行の一通りの動作確認

---

## Step 5: TypeScript 型定義 + Svelte 5 `$state` ストア

> 依存: なし（Step 1〜4 と並行して進められる）  
> 参照: [01_project_schema.md](design/01_project_schema.md), [09_store.md](design/09_store.md), [05_ipc.md](design/05_ipc.md)

#### サイクル 5-1: Project 型定義
- `[x]` 🔴 テスト (Vitest): `isProject(unknown)` 型ガード関数が正しい/不正なオブジェクトを判別する
- `[x]` 🟢 実装: `src/types/project.ts`（`Project` / `Scene` / `SceneObject` / `Entry` / `VariableValue` 等全型）

#### サイクル 5-2: ProjectStore — loadProject
- `[x]` 🔴 テスト: `loadProject(project, "/path")` 後に `state.project` / `state.filePath` / `state.dirty=false` が正しくセットされる
- `[x]` 🟢 実装: `projectStore.svelte.ts` – `loadProject` アクション

#### サイクル 5-3: ProjectStore — updateProject
- `[x]` 🔴 テスト: `updateProject(fn)` 後に `state.dirty === true` になる / 渡した updater が project に適用される
- `[x]` 🟢 実装: `updateProject` アクション

#### サイクル 5-4: BatchStore — 状態遷移
- `[x]` 🔴 テスト: `idle` → `_setRunning()` → `running` → `cancelBatch()` → `cancelling` → `onCancelled()` → `idle` の遷移が正しい
- `[x]` 🟢 実装: `batchStore.svelte.ts` – status 状態遷移ロジック

#### サイクル 5-5: BatchStore — 進捗更新
- `[x]` 🔴 テスト: `onProgress(payload)` 後に `currentEntryIndex` / `currentEntryName` / `currentEntryProgress` が更新される
- `[x]` 🟢 実装: `onProgress` / `onEntryDone` / `onEntryError` / `onDone` ハンドラ

#### サイクル 5-6: SettingsStore
- `[x]` 🔴 テスト: `setSettings(s)` 後に `state.settings` が更新される
- `[x]` 🟢 実装: `settingsStore.svelte.ts`

---

## Step 6: Canvas プレビューエンジン（TypeScript）

> 依存: Step 5（store）完了後  
> 参照: [07_preview.md](design/07_preview.md)

#### サイクル 6-1: シーン累積時間計算
- `[x]` 🔴 テスト: `[3s, 5s, 2s]` の3シーン → `totalDuration = 10.0` / シーン長ゼロ混在ケースも確認
- `[x]` 🟢 実装: `calculateTotalDuration(scenes)` 純粋関数（`src/preview/sceneUtils.ts`）

#### サイクル 6-2: 現在シーン・相対時間の算出
- `[x]` 🔴 テスト: `currentTime=5.5`, シーン1=3s / シーン2=5s → シーン2のインデックス、relative=2.5s / 境界値（ちょうど3.0s）も確認
- `[x]` 🟢 実装: `getCurrentScene(currentTime, scenes)` 純粋関数

#### サイクル 6-3: オブジェクト表示判定
- `[x]` 🔴 テスト: `start=2.0, duration=3.0` → `t=1.9: false`, `t=2.0: true`, `t=4.9: true`, `t=5.0: false` / `duration=0.0`（シーン終端）のケースも確認
- `[x]` 🟢 実装: `isObjectVisible(obj, relativeTime, sceneLen)` 純粋関数

#### サイクル 6-4: font_size pt → px 変換
- `[x]` 🔴 テスト: `48pt → 64px` / `24pt → 32px` / `1pt → 1px`（round での端数処理）
- `[x]` 🟢 実装: `ptToPx(pt: number): number` 純粋関数

#### サイクル 6-5: Canvas 描画ループ（手動テスト）
- `[x]` 🟢 実装: `requestAnimationFrame` ループ（30fps 上限）（`src/preview/PreviewCanvas.svelte`）
- `[x]` 🟢 実装: `<video>` + `requestVideoFrameCallback` + `drawImage`
- `[x]` 🟢 実装: `convertFileSrc` による asset URL 生成
- `[x]` 🟢 実装: Web Audio API 音声再生（GainNode / フェード / aloop 相当）
- `[x]` 🟢 実装: シーク・エントリ切り替え処理
- `[x]` 🟢 実装: AudioContext autoplay ポリシー対応（resume ボタン）
- 手動テスト: `examples/standard.yaml` でプレビュー再生・シーク・エントリ切り替えを確認（Step 7 UI 完成後）

---

## Step 7: UI コンポーネント（Svelte 5）

> 依存: Step 5（store）+ Step 6（preview）完了後  
> 参照: [08_gui.md](design/08_gui.md)

> ※ UI コンポーネントは DOM 依存が強く TDD が困難なため、機能単位で実装→手動テストで進める。

#### 機能 7-1: ツールバー
- `[x]` 🟢 実装: 新規 / 開く / 保存 / 名前を付けて保存ボタン（`src/components/Toolbar.svelte`）
- `[x]` 🟢 実装: 出力先フォルダ入力欄（未設定時の赤枠ハイライト）
- `[x]` 🟢 実装: バッチ実行 / キャンセルボタン（バッチ中は編集 UI を無効化）
- 手動テスト: ファイル開閉・保存が正常に動作する

#### 機能 7-2: プレビューパネル
- `[x]` 🟢 実装: Canvas 埋め込み（Step 6 と接続）（`src/components/PreviewPanel.svelte`）
- `[x]` 🟢 実装: 再生 / 一時停止 / 停止ボタン・シークバー
- `[x]` 🟢 実装: 時刻表示（`MM:SS / MM:SS`）
- 手動テスト: 再生・シーク・エントリ切り替えが正常に動作する

#### 機能 7-3: プロパティパネル
- `[x]` 🟢 実装: オブジェクト属性編集フォーム（video / image / text / audio 種別ごと）（`src/components/PropertiesPanel.svelte`）
- `[x]` 🟢 実装: 可変値編集フォーム（エントリ選択中に表示）
- `[x]` 🟢 実装: 出力設定フォーム（codec / format / crf / preset）
- 手動テスト: 各フォームの入力が ProjectStore に反映される

#### 機能 7-4: タイムライン
- `[x]` 🟢 実装: シーン一覧（追加 / 削除 / 並び替え）（`src/components/Timeline.svelte`）
- `[x]` 🟢 実装: オブジェクト一覧（追加 / 削除 / Z 順変更）
- `[x]` 🟢 実装: `variable: true` の視覚的表示（★ マーク等）
- 手動テスト: シーン・オブジェクトの操作が正常に動作する

#### 機能 7-5: エントリ一覧
- `[x]` 🟢 実装: エントリカード（チェックボックス / 選択 / 複製 / 削除）（`src/components/EntryList.svelte`）
- `[x]` 🟢 実装: ドラッグ並び替え
- 手動テスト: エントリ操作とプレビュー連動が正常に動作する

#### 機能 7-6: バッチ進捗ダイアログ
- `[x]` 🟢 実装: 進捗バー（エントリ単位 + エントリ内 % ）（`src/components/BatchProgressDialog.svelte`）
- `[x]` 🟢 実装: キャンセルボタン（確認ダイアログ付き）
- `[x]` 🟢 実装: 完了ダイアログ
- 手動テスト: バッチ実行・キャンセル・完了の一連の UX を確認

#### 機能 7-7: その他 UI
- `[x]` 🟢 実装: キーボードショートカット（Cmd+N/O/S/Shift+S / Space / Escape / Delete / Cmd+D / 矢印）（`src/routes/+page.svelte`）
- `[x]` 🟢 実装: 起動時モーダル（Recent Files 一覧）（`src/components/RecentFilesModal.svelte`）
- `[x]` 🟢 実装: About ダイアログ（`src/components/AboutDialog.svelte`）
- `[x]` 🟢 実装: 未保存変更の確認ダイアログ（`src/components/UnsavedChangesDialog.svelte`）
- 手動テスト: `pnpm tauri dev` で起動・一通りの動作確認

---

---

## Step 8: エントリ/バッチ廃止・シングルエクスポート化（要件変更対応）

> ユーザー判断: エントリ・バッチ概念を廃止し、1プロジェクト = 1本の動画出力に簡略化する。

#### Rust バックエンド
- `[x]` `project/schema.rs`: Entry / VariableValue / variable フィールドを削除。OutputSettings に `output_name: String` を追加
- `[x]` `project/validation.rs`: エントリバリデーション削除。`output_name` 空チェックを追加
- `[x]` `renderer/filter.rs`: Entry 引数を削除し、オブジェクトから直接ファイル・テキストを読む
- `[x]` `batch/runner.rs`: エントリループをシングルエクスポートに置き換え。イベント名 `export:*` に変更
- `[x]` `commands/batch.rs`: `start_batch` → `start_export`、`cancel_batch` → `cancel_export`。`check_output_conflicts` 削除
- `[x]` `main.rs`: ハンドラ登録を更新

#### TypeScript / フロントエンド
- `[x]` `src/types/project.ts`: Entry / VariableValue / variable 削除。OutputSettings に output_name 追加
- `[x]` `src/types/ipc.ts`: BatchXxx → ExportXxx に更新
- `[x]` `src/store/exportStore.svelte.ts`: batchStore の代替として新規作成
- `[x]` `src/components/ExportProgressDialog.svelte`: BatchProgressDialog の代替として新規作成
- `[x]` `src/components/Toolbar.svelte`: バッチ実行 → 書き出しボタンに更新
- `[x]` `src/components/PropertiesPanel.svelte`: variable / 可変値編集セクション削除。output_name フィールド追加
- `[x]` `src/routes/+layout.svelte`: export イベントリスナーに更新
- `[x]` `src/routes/+page.svelte`: エントリパネル削除・レイアウト更新
- `[x]` 不要ファイル削除: `EntryList.svelte`, `BatchProgressDialog.svelte`, `batchStore.svelte.ts`

#### テスト更新
- `[x]` `src-tauri/src/project/tests.rs`: entries / variables 参照を削除
- `[x]` `src/store/*.test.ts`: batchStore → exportStore テストに更新
- `[x]` `src-tauri/src/batch/output.rs`: エントリベースの関数は不使用のため mod.rs から除外
- `[x]` `src-tauri/src/batch/log.rs`: doc test の import を修正

#### ドキュメント更新
- `[x]` `docs/requirements.md`: エントリ・バッチ記述を削除。書き出し機能に更新

---

## Step 9: バグ修正・未実装機能

> 調査日: 2026-04-30。要件定義・設計書・コード全体の突合により洗い出した問題。  
> 上から優先度順。P0 が未解決な限り P1 以降の多くはテストもできない。

---

### P0: FFmpeg / ffprobe のパス解決が壊れている（全機能ブロッカー）

#### 問題
`main.rs` は `resource_dir().join("ffmpeg").join("ffprobe")` でパスを組んでいるが、
開発時も本番時も当該ディレクトリにバイナリが存在しない。

- `tauri.conf.json` に `bundle` セクション自体が存在しない（`externalBin` / `resources` 未設定）
- `src-tauri/binaries/` / `src-tauri/resources/ffmpeg/` ディレクトリが存在しない
- 設計書 `10_infra.md` には「`bundle.externalBin` でサイドカーとして同梱」と明記されているが未実装

#### 影響
- `probe_file` IPC → 失敗（サイレントにエラーを捨てている） → シーン長が更新されない
- `start_export` → 失敗（FFmpeg が見つからない） → 書き出し不可
- `get_ffmpeg_version` → 失敗
- 開発時: システムの ffmpeg/ffprobe にフォールバックする仕組みがないため全滅

#### タスク
- `[x]` **開発環境フォールバック**: バンドルパスにバイナリが存在しない場合は `"ffmpeg"` / `"ffprobe"` (= PATH 検索) にフォールバックする処理を `state.rs` または `main.rs` に追加する（開発時対応）
- `[x]` **本番バンドル設定**: `tauri.conf.json` に `bundle` セクション（`active`, `targets`, `icon`, `resources`）を追加してバンドル生成を有効化（フォント同梱）。ffmpeg/ffprobe のバンドルは PATH フォールバックがあるため開発時は不要
- `[ ]` **設計書更新**: `10_infra.md` の FFmpeg バンドル手順・配置場所を現在の実装に合わせて更新する

---

### P1-A: シーン長の計算が要件と不一致（Rust renderer）

#### 問題
`filter.rs` の `build_scene_filter` は `scene_len = scene.duration.unwrap_or(0.0)` を使う。
`scene.duration` が未設定のとき `scene_len = 0.0` となり、背景カラー `d=0`・全 overlay の `enable='between(t,0,0)'` → **映像が出力されない**。

要件書: 「映像オブジェクトの終了時刻 = 表示開始時間 ＋ 動画ファイルの長さ（トリム後）。シーン内の全オブジェクトの終了時刻の最大値がシーンの長さになる」

#### タスク
- `[x]` `filter.rs` の `build_scene_filter` で `scene_len` を以下の優先順位で決定するよう修正する：
  1. `scene.duration` が設定済みならその値を使う
  2. 未設定なら `probes` を使ってオブジェクトの終了時刻の最大値を計算する（video: `obj.start + probe.duration`、image/text/audio: `obj.start + obj.duration`、duration=0 は除外）
- `[x]` `runner.rs` の進捗計算（`total_duration`）も同じロジックで修正する
- `[x]` ユニットテストを追加する（scene.duration 未設定 + video オブジェクト → scene_len が probe の duration から導出される）

---

### P1-B: シーン長の計算が要件と不一致（TypeScript プレビュー）

#### 問題
`sceneUtils.ts` の `calculateTotalDuration` は `s.duration ?? 0` を合計するだけ。映像のみのシーンで `scene.duration` 未設定なら総再生時間 = 0 → シークバーが動かない。
`PreviewCanvas.svelte` の `sceneLen = scene.duration ?? 0` も同様で `isObjectVisible(duration=0, t=0, sceneLen=0)` → `false` → 何も表示されない。

#### タスク
- `[x]` `previewStore` または `PreviewCanvas` に「ffprobe 結果を保持する Map（ファイルパス → 秒数）」を持たせる
- `[x]` ファイルが設定されたとき（`probe_file` IPC または `<video>.onloadedmetadata`）にそのキャッシュを更新する
- `[x]` `calculateTotalDuration` / `getCurrentScene` / `sceneLen` 計算を、キャッシュを使った実際の動画尺を考慮したロジックに差し替える
- `[x]` `isObjectVisible` に渡す `sceneLen` を実効値（≥ 0 かつ未設定時は ∞ 扱い）で渡すよう修正する（暫定: `sceneLen = scene.duration ?? Number.MAX_SAFE_INTEGER`）

---

### P1-C: Timeline にシーン duration の編集 UI がない

#### 問題
Timeline でシーンの `duration` は `{scene.duration ?? '?'}s` と表示されるだけで編集できない。
「映像オブジェクトがなくタイトルカード的に使うシーン」では `scene.duration` の直接指定が必須だが、UI から設定する手段がない。

#### タスク
- `[x]` `Timeline.svelte` のシーンヘッダーに `<input type="number" step="0.1">` を追加し、`scene.duration` を直接編集できるようにする（未設定 = 空欄で表現）

---

### P1-D: キャンセル機能が実質無効

#### 問題
`runner.rs` は FFmpeg `child` プロセスを spawn した後、`ffmpeg_child_slot` Mutex に格納していない（133–136 行は「前回のものを kill する前処理」のみ）。`cancel_export` は `slot.take()` → `kill()` するが slot は常に `None` なので kill されない。`is_cancelled()` ポーリングも BufReader ブロッキング中は到達しない。

#### タスク
- `[x]` `runner.rs` で `ffmpeg_child` を spawn した直後に `ffmpeg_child_slot.lock().unwrap().replace(child)` して slot に格納する
- `[ ]` 手動テスト: 書き出し中にキャンセルボタンを押して FFmpeg が確実に停止することを確認する

---

### P1-E: テキストオブジェクトのフォントパスが壊れている

#### 問題 ①: フォントファイルが存在しない
`src-tauri/fonts/` ディレクトリが存在しない。`infra.rs` の `get_font_paths` は `state.font_dir/NotoSansCJK-Regular.ttc` を返すが、そのファイルは存在しない。

#### 問題 ②: filter.rs がフォント名をそのまま fontfile に渡している
`build_text_filter` は `fontfile={txt.font}` と書いているが、`txt.font` には `"NotoSansCJK-Regular"` のような名前が入っている。FFmpeg の `drawtext` は `fontfile=` にはフルパスが必要。`font_dir` は `build_filter_graph` のシグネチャに渡されていない。

#### タスク
- `[x]` NotoSans CJK フォントファイル（`.otf`）を `src-tauri/fonts/` に配置する（NotoSansCJKjp-Regular.otf / Bold.otf → NotoSansCJK-Regular.otf / Bold.otf にリネーム）
- `[ ]` `tauri.conf.json` の `bundle.resources` にフォントディレクトリを追加する
- `[x]` `build_filter_graph` のシグネチャに `font_dir: &Path` を追加し、`runner.rs` から `config.font_dir` を渡す
- `[x]` `build_text_filter` 内で `font_dir.join(format!("{}.otf", txt.font))` のフルパスを `fontfile` に使う
- `[x]` `infra.rs` の `get_font_paths` の拡張子を `.otf` に更新した

---

### P2-A: バリデーション未実装項目

#### 問題
設計書 `02_validation.md` に定義されているが `validation.rs` に実装されていないチェック:
- `scene_no_duration`: 明示的な時間を持つオブジェクトが1つもなく `scene.duration` も未設定
- `file_not_found`: オブジェクトの `file` パスが実在しない（`Path::exists()` チェック）

#### タスク
- `[x]` `validation.rs` に `scene_no_duration` チェックを追加（シーン内の全オブジェクトの `duration` が 0 かつ `scene.duration` が None → Error）
- `[x]` `validation.rs` に `file_not_found` チェックを追加（video / image / audio オブジェクトの `file` が `Some` かつ `Path::exists()` が false → Error）
- `[x]` テストを追加する

---

### P2-B: Cmd+S / Cmd+Shift+S ショートカットが動かない

#### 問題
`+page.svelte` の `handleKeydown` が `document.getElementById('save-btn')?.click()` を呼んでいるが、`Toolbar.svelte` の保存ボタンに `id="save-btn"` がない。結果として Cmd+S が何も実行しない。

#### タスク
- `[x]` `Toolbar.svelte` の保存ボタンに `id="save-btn"`、名前を付けて保存ボタンに `id="save-as-btn"` を追加する

---

### P2-C: PropertiesPanel に TextObject の font / background_color 編集フィールドがない

#### 問題
`PropertiesPanel.svelte` の TextObject セクションに `font`（フォント選択）と `background_color`（背景色）の入力フォームがない。要件書には「フォント・文字色・背景色 | テキスト | テキスト装飾」と記載されている。

#### タスク
- `[x]` `font` フィールドを追加する（`NotoSansCJK-Regular` / `NotoSansCJK-Bold` を `<select>` で選択）
- `[x]` `background_color` フィールドを追加する（`<input type="color">` + 「なし」チェックボックス）

---

### P2-D: runner.rs の probe_file が -show_format を省略している

#### 問題
`runner.rs` 内部の `probe_file()` 呼び出しが `-show_format` を渡していない。`parse_ffprobe_output` は `format.duration` を優先するため、`-show_format` がないと `format` キーが JSON に含まれずストリームの duration にフォールバックする。一部のファイルで probe 失敗またはデュレーション不正確になる可能性がある。

#### タスク
- `[x]` `runner.rs` の `probe_file()` 引数リストに `"-show_format"` を追加する

---

### P3: ドキュメント整合性

- `[x]` `docs/design.md` のアーキテクチャ概要「フロントエンド: React + TypeScript / Zustand」を「Svelte 5 + TypeScript / Svelte Runes」に修正する
- `[x]` `docs/design/05_ipc.md` のコマンド名・イベント名を Step 8 後の実装（`start_export` / `cancel_export` / `export:*`）に合わせて更新する
- `[x]` `docs/design/02_validation.md` に `output_name_invalid` エラーコードを追加する

---

## Step 10: バグ修正・UI 改善（追加）

### 10-1: width/height のアスペクト比連動 ✅
- `[x]` `PropertiesPanel.svelte`: オブジェクトの width/height を常にアスペクト比連動に変更
  - `browseFile` 時に ffprobe の width/height からソースファイルの正確なアスペクト比を `objectSourceRatios` Map に保存
  - Image オブジェクトも probe 対象に追加
  - onfocus で probe データがない場合のフォールバック比率をキャプチャ
  - oninput で probe 由来の比率（または fallback）を使い他方の寸法を `Math.round` で自動更新
- `[x]` 出力設定の width/height も同様に連動（onfocus でキャプチャ）

---

## Step 11: プレビューキャンバス上でのマウスドラッグ移動

### 11-1: PreviewCanvas.svelte にドラッグ移動機能を追加

- `[x]` `docs/requirements.md` にマウスドラッグ位置変更要件を追記
- `[x]` `PreviewCanvas.svelte` にポインターイベントハンドラを追加（ヒットテスト・ドラッグ開始・移動・終了）
- `[x]` ドラッグ中はプロジェクトストアの x/y をリアルタイム更新
- `[x]` ホバー時はグラブカーソル、ドラッグ中はグラビングカーソルを表示
- `[x]` ドラッグ開始時に対象オブジェクトを `selectScene` / `selectObject` で選択
- `[ ]` 手動テスト: ドラッグで位置が変わること・プロパティパネルに座標が反映されること

---

## 決定・メモ

| 日付 | 内容 |
|------|------|
| 2026-04-19 | 設計書を `docs/design/` に分割完了。アーキテクチャ方針: Rust は `project → renderer → batch → commands` の単方向依存で実装を直列化 |
| 2026-04-19 | `design.md` は目次として残し、各トピックは `design/01〜10` に格納 |
| 2026-04-19 | 実装は TDD で進める方針を決定。各サイクル完了条件は「テストが green」 |
| 2026-04-19 | tasks.md を TDD サイクル単位に再構成。UI（Step 7）は DOM 依存のため手動テスト主体とする |

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
- [Svelte 5 `$state` ストア](design/09_store.md)
- [インフラ・CI/CD](design/10_infra.md)
