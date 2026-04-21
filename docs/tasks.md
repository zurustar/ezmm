# ezmm 実装タスク

設計書: [design.md](design.md)（目次）→ [design/](design/) 配下の各ファイル  
開発プロセス: [CONTRIBUTING.md](../CONTRIBUTING.md)（TDD方針・コードスタイル・コミット規則）

---

## 現在の状態

**フェーズ**: Step 3（`batch` モジュール）完了 → Step 4（`commands` / `state` / `settings`）着手

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

## Step 5: TypeScript 型定義 + Zustand ストア

> 依存: なし（Step 1〜4 と並行して進められる）  
> 参照: [01_project_schema.md](design/01_project_schema.md), [09_store.md](design/09_store.md), [05_ipc.md](design/05_ipc.md)

#### サイクル 5-1: Project 型定義
- `[ ]` 🔴 テスト (Vitest): `isProject(unknown)` 型ガード関数が正しい/不正なオブジェクトを判別する
- `[ ]` 🟢 実装: `src/types/project.ts`（`Project` / `Scene` / `SceneObject` / `Entry` / `VariableValue` 等全型）

#### サイクル 5-2: ProjectStore — loadProject
- `[ ]` 🔴 テスト: `loadProject(project, "/path")` 後に `state.project` / `state.filePath` / `state.dirty=false` が正しくセットされる
- `[ ]` 🟢 実装: `projectStore.ts` – `loadProject` アクション

#### サイクル 5-3: ProjectStore — updateProject
- `[ ]` 🔴 テスト: `updateProject(fn)` 後に `state.dirty === true` になる / 渡した updater が project に適用される
- `[ ]` 🟢 実装: `updateProject` アクション

#### サイクル 5-4: BatchStore — 状態遷移
- `[ ]` 🔴 テスト: `idle` → `startBatch()` → `running` → `cancelBatch()` → `cancelling` → `onCancelled()` → `idle` の遷移が正しい
- `[ ]` 🟢 実装: `batchStore.ts` – status 状態遷移ロジック

#### サイクル 5-5: BatchStore — 進捗更新
- `[ ]` 🔴 テスト: `onProgress(payload)` 後に `currentEntryIndex` / `currentEntryName` / `currentEntryProgress` が更新される
- `[ ]` 🟢 実装: `onProgress` / `onEntryDone` / `onEntryError` / `onDone` ハンドラ

#### サイクル 5-6: SettingsStore
- `[ ]` 🔴 テスト: `setSettings(s)` 後に `state.settings` が更新される
- `[ ]` 🟢 実装: `settingsStore.ts`

---

## Step 6: Canvas プレビューエンジン（TypeScript）

> 依存: Step 5（store）完了後  
> 参照: [07_preview.md](design/07_preview.md)

#### サイクル 6-1: シーン累積時間計算
- `[ ]` 🔴 テスト: `[3s, 5s, 2s]` の3シーン → `totalDuration = 10.0` / シーン長ゼロ混在ケースも確認
- `[ ]` 🟢 実装: `calculateTotalDuration(scenes, entry)` 純粋関数

#### サイクル 6-2: 現在シーン・相対時間の算出
- `[ ]` 🔴 テスト: `currentTime=5.5`, シーン1=3s / シーン2=5s → シーン2のインデックス、relative=2.5s / 境界値（ちょうど3.0s）も確認
- `[ ]` 🟢 実装: `getCurrentScene(currentTime, scenes, entry)` 純粋関数

#### サイクル 6-3: オブジェクト表示判定
- `[ ]` 🔴 テスト: `start=2.0, duration=3.0` → `t=1.9: false`, `t=2.0: true`, `t=4.9: true`, `t=5.0: false` / `duration=0.0`（シーン終端）のケースも確認
- `[ ]` 🟢 実装: `isObjectVisible(obj, relativeTime, sceneLen)` 純粋関数

#### サイクル 6-4: font_size pt → px 変換
- `[ ]` 🔴 テスト: `48pt → 64px` / `24pt → 32px` / `1pt → 1px`（round での端数処理）
- `[ ]` 🟢 実装: `ptToPx(pt: number): number` 純粋関数

#### サイクル 6-5: Canvas 描画ループ（手動テスト）
- `[ ]` 🟢 実装: `requestAnimationFrame` ループ（30fps 上限）
- `[ ]` 🟢 実装: `<video>` + `requestVideoFrameCallback` + `drawImage`
- `[ ]` 🟢 実装: `convertFileSrc` による asset URL 生成
- `[ ]` 🟢 実装: Web Audio API 音声再生（GainNode / フェード / aloop 相当）
- `[ ]` 🟢 実装: シーク・エントリ切り替え処理
- `[ ]` 🟢 実装: AudioContext autoplay ポリシー対応（resume ボタン）
- 手動テスト: `examples/standard.yaml` でプレビュー再生・シーク・エントリ切り替えを確認

---

## Step 7: UI コンポーネント（React）

> 依存: Step 5（store）+ Step 6（preview）完了後  
> 参照: [08_gui.md](design/08_gui.md)

> ※ UI コンポーネントは DOM 依存が強く TDD が困難なため、機能単位で実装→手動テストで進める。

#### 機能 7-1: ツールバー
- `[ ]` 🟢 実装: 新規 / 開く / 保存 / 名前を付けて保存ボタン
- `[ ]` 🟢 実装: 出力先フォルダ入力欄（未設定時の赤枠ハイライト）
- `[ ]` 🟢 実装: バッチ実行 / キャンセルボタン（バッチ中は編集 UI を無効化）
- 手動テスト: ファイル開閉・保存が正常に動作する

#### 機能 7-2: プレビューパネル
- `[ ]` 🟢 実装: Canvas 埋め込み（Step 6 と接続）
- `[ ]` 🟢 実装: 再生 / 一時停止 / 停止ボタン・シークバー
- `[ ]` 🟢 実装: 時刻表示（`MM:SS / MM:SS`）
- 手動テスト: 再生・シーク・エントリ切り替えが正常に動作する

#### 機能 7-3: プロパティパネル
- `[ ]` 🟢 実装: オブジェクト属性編集フォーム（video / image / text / audio 種別ごと）
- `[ ]` 🟢 実装: 可変値編集フォーム（エントリ選択中に表示）
- `[ ]` 🟢 実装: 出力設定フォーム（codec / format / crf / preset）
- 手動テスト: 各フォームの入力が ProjectStore に反映される

#### 機能 7-4: タイムライン
- `[ ]` 🟢 実装: シーン一覧（追加 / 削除 / 並び替え）
- `[ ]` 🟢 実装: オブジェクト一覧（追加 / 削除 / Z 順変更）
- `[ ]` 🟢 実装: `variable: true` の視覚的表示（★ マーク等）
- 手動テスト: シーン・オブジェクトの操作が正常に動作する

#### 機能 7-5: エントリ一覧
- `[ ]` 🟢 実装: エントリカード（チェックボックス / 選択 / 複製 / 削除）
- `[ ]` 🟢 実装: ドラッグ並び替え
- 手動テスト: エントリ操作とプレビュー連動が正常に動作する

#### 機能 7-6: バッチ進捗ダイアログ
- `[ ]` 🟢 実装: 進捗バー（エントリ単位 + エントリ内 % ）
- `[ ]` 🟢 実装: キャンセルボタン（確認ダイアログ付き）
- `[ ]` 🟢 実装: 完了ダイアログ（フォルダを開く / ログを開く）
- 手動テスト: バッチ実行・キャンセル・完了の一連の UX を確認

#### 機能 7-7: その他 UI
- `[ ]` 🟢 実装: キーボードショートカット（Cmd+N/O/S/Shift+S / Space / Escape / Delete / Cmd+D / 矢印）
- `[ ]` 🟢 実装: 起動時モーダル（Recent Files 一覧）
- `[ ]` 🟢 実装: About ダイアログ
- `[ ]` 🟢 実装: 未保存変更の確認ダイアログ（新規・開く・ウィンドウ閉じる時）
- 手動テスト: 各 UI が仕様通りに動作する

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
- [Zustand ストア](design/09_store.md)
- [インフラ・CI/CD](design/10_infra.md)
