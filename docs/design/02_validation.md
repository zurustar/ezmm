# バリデーション仕様

プロジェクトの整合性検証ルール。エラーコード一覧。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [01_project_schema.md](01_project_schema.md)（Project 型・ValidationResult 型）

---

## 概要

バリデーションは**バッチ実行前**に全件チェック。GUIでの編集中はリアルタイムバリデーション（入力欄の即時エラー表示）と組み合わせる。

---

## チェック項目

### プロジェクト全体

- `output_folder` が存在し書き込み可能
- `output.fps` > 0、`output.width`/`height` > 0
- コーデック・フォーマットの組合せが有効（h264/h265→mp4/mov、vp9→webm）
- `output.crf` が有効範囲内（h264/h265: 0–51、vp9: 0–63）
- `output.preset` が有効値のいずれか: `ultrafast` / `superfast` / `veryfast` / `faster` / `fast` / `medium` / `slow` / `slower` / `veryslow`（VP9 選択時はホワイトリスト検証をスキップする。GUI では VP9 時に preset セレクトをグレーアウトするため通常は変更不可だが、YAML 手編集時に任意の文字列が入っていてもバリデーションエラーにしない。YAML 保存値は VP9 時でも直前の設定値をそのまま保持する）
- シーンが1件以上、エントリが1件以上（空プロジェクトはバッチ実行時のみエラー。`validate_project` IPC は編集中にも呼ばれるが、空プロジェクトはエラーではなく単に実行対象が0件として扱う。詳細は「空プロジェクト」節を参照）

### シーン

- 明示的な時間を持つオブジェクトが1つもない場合、`duration` > 0 が必須
- シーン ID はプロジェクト内で一意
- オブジェクトIDはプロジェクト内で一意
- 有限長を定める要素がないシーン（例: `loop: loop` の音声オブジェクトのみ、全オブジェクトが `duration: 0.0` で映像なし）は `scene.duration` 必須

### オブジェクト

- `type` が有効値（`video` / `image` / `text` / `audio`）
- 各タイプの必須フィールドが存在
- `opacity: 0–100`、`volume: 0–100`、`start >= 0`
- `variable: false` の場合、`file` または `text` が存在し、ファイルは実在する
- テキストオブジェクトの `font` は同梱フォントのホワイトリスト（`NotoSansCJK-Regular` / `NotoSansCJK-Bold`）のいずれかでなければエラー

### エントリ

- `name` がOS共通のファイル名禁止文字（`/ \ : * ? " < > |` および制御文字）を含まない・空文字でない（詳細は [01_project_schema.md](01_project_schema.md) の「エントリ `name` の文字制約」を参照）
- エントリ名がプロジェクト内で一意
- `variable: true` の全オブジェクトに対応する変数がエントリ内に存在（ただしオブジェクト側にデフォルト値が記述されていれば省略可）
- 可変ファイル参照が実在する
- 映像オブジェクトの `trim_start + trim_end < 映像ファイル長` を満たす
- `VariableValue` の形状検証: `file` と `text` の**両方**が同一エントリ項目に存在する場合はエラー（Rust `#[serde(untagged)]` enum はデシリアライズ時に片方を破棄するため `Project` 構造体に変換した後では検出不可。**実装方針**: `serde_yml::from_str::<serde_yml::Value>()` で生パース → 各 `variables` マッピングを走査して `file` と `text` キーの同時存在を検出 → エラーがなければ `serde_yml::from_value::<Project>()` で再デシリアライズする二段階処理とする）
- 対応オブジェクトの `type` と `VariableValue` の形状整合: video/image/audio 対応エントリは `file` 必須、text 対応エントリは `text` 必須
- `VariableValue::Media` の `trim_start`/`trim_end` は video オブジェクト対応エントリにのみ有効。image/audio 対応エントリで `trim_start`/`trim_end` が指定されていた場合は**警告**（`ValidationResult.warnings` に追加）としてバッチ実行は継続するが値を無視する

---

## エラーと警告の区別

### エラー（バッチ実行をブロック）

- 必須フィールドの欠損
- 参照ファイルが存在しない
- コーデック・フォーマットの組み合わせ不正
- `trim_start + trim_end >= 映像ファイル長`
- エントリ名の禁止文字・重複
- オブジェクト ID 重複

### 警告（続行可能）

- オブジェクトの座標・サイズが出力解像度をはみ出す
- 入力 FPS が出力 FPS と 2倍以上異なる（判定式: `max(in_fps / out_fps, out_fps / in_fps) >= 2.0`。`ProbeResult.fps` が `None` の場合は FPS 不明として本警告をスキップする）
- エントリの `variables` に不明なオブジェクト ID が含まれる

---

## `ValidationIssue.code` 一覧

| `code` 値 | 対象 | severity |
|-----------|------|----------|
| `output_folder_invalid` | project | error |
| `output_settings_invalid` | project | error |
| `codec_format_mismatch` | project | error |
| `crf_out_of_range` | project | error |
| `preset_invalid` | project | error |
| `empty_project` | project | error（バッチ実行時のみ発行） |
| `scene_no_duration` | scene | error |
| `scene_id_duplicate` | scene | error |
| `object_id_duplicate` | object | error |
| `object_type_invalid` | object | error |
| `object_field_missing` | object | error |
| `object_value_out_of_range` | object | error |
| `file_not_found` | object/entry | error |
| `font_not_whitelisted` | object | error |
| `entry_name_invalid` | entry | error |
| `entry_name_duplicate` | entry | error |
| `variable_missing` | entry | error |
| `variable_type_mismatch` | entry | error |
| `trim_out_of_range` | entry | error |
| `variable_value_both_fields` | entry | error |
| `trim_on_non_video` | entry | warning |
| `fps_mismatch` | object | warning（映像オブジェクトの入力 FPS が出力 FPS と 2 倍以上異なる） |
| `object_out_of_bounds` | object | warning（オブジェクトの座標・サイズがキャンバス範囲外） |
| `probe_duration_missing` | entry | warning（ffprobe でファイル長を取得できなかったため trim 検証をスキップ） |
| `unknown_variable_key` | entry | warning（`variables` に定義されているキーがいずれのオブジェクト ID にも対応しない） |

---

## エラー通知

- バリデーションエラーはダイアログで一覧表示（対象エントリ/オブジェクトIDを明示）
- FFmpegエラーは元のstderrメッセージとともにUIに表示

### 警告の GUI 提示方法

- バッチ実行前のバリデーションで検出された警告は、確認ダイアログで一覧表示（「警告N件。続行しますか？」[続行] [キャンセル]）
- 編集中にリアルタイム検出された警告は、対象要素（タイムラインのオブジェクトブロック・エントリカード等）に**オレンジの警告マーク（⚠）** を表示。マーク ホバーで警告メッセージをツールチップ表示
- プロパティパネルで対象属性を編集している時は入力欄の下に赤字（警告色）でメッセージ表示

---

## 追加チェック（エントリ・ファイル参照整合性）

### variables 内の不明オブジェクト ID

エントリの `variables` に、プロジェクト内に存在しないオブジェクト ID が含まれる場合:
- **扱い**: 警告（バッチ実行をブロックしない）
- 対象キーはレンダリング時に無視する
- GUI 表示: 「エントリ '{name}' に不明な変数 '{id}' が含まれています」

### 同一オブジェクト ID が複数シーンに存在

プロジェクト内でオブジェクト ID はグローバルに一意であること（バリデーションエラー）:
「オブジェクト ID '{id}' が複数のシーンで使用されています（シーン '{s1}'、'{s2}'）」

### trim_start + trim_end ≥ 映像ファイル長

`trim_start + trim_end >= ファイル長` の場合はバリデーション**エラー**（バッチ実行をブロック）:
「'{entry_name}' の '{object_id}': トリム量（{trim_start}+{trim_end}秒）が映像ファイルの長さ（{duration}秒）以上です」

---

## `validate_project` IPC の仕様

- 原則としてコマンドは `Result<T, String>` を返すが、`validate_project` は**例外**として裸の戻り値型（`ValidationResult`）を返す（失敗しない）
- バリデーション結果は `ValidationResult` の `errors`/`warnings` に格納する
- 空プロジェクト（シーン0件・エントリ0件）はバッチ実行時のみ `empty_project` エラーを発行。`validate_project` IPC は空でもエラーを返さない

> 注意: IPC エラーコード（`"<code>:"` 接頭辞付き文字列）と `ValidationIssue.code`（接頭辞なし）は別体系。前者は IPC 通信レベルの致命的エラー、後者はプロジェクト内容の論理的な検証結果。詳細は [05_ipc.md](05_ipc.md) を参照。
