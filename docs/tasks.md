# タスク

## BGMをオブジェクトモデルに統合

- [x] requirements.md: オブジェクトの種別に「音声」を追加（映像・画像・テキスト・音声の4種別）
- [x] requirements.md: 音声オブジェクトの属性を定義（ファイル、音量、フェードイン/アウト、ループ、表示開始時間、表示時間）
- [x] requirements.md: 映像オブジェクトに「音量」属性を追加（映像自体に埋め込まれた音声のボリューム制御）
- [x] requirements.md: BGMセクションを廃止
- [x] requirements.md: プロジェクトファイル構成表からBGM設定を削除
- [x] design.md: YAMLスキーマを更新（BGMセクション廃止、音声オブジェクトに変更）
- [x] design.md: スキーマ補足テーブルを更新
- [x] requirements.md: シーンの長さ計算に音声オブジェクトが含まれていない（「画像・テキスト」→「画像・テキスト・音声」）
- [x] requirements.md: 変更履歴にBGM廃止・音声オブジェクト統合を反映

## 完了済み

<details>
<summary>過去のタスク</summary>

- [x] BGMが動画全体より短い場合の挙動をrequirements.mdとdesign.mdに反映（ループ/無音を選択可能）
- [x] 映像オブジェクトに `duration` 属性は存在しない
- [x] requirements.md: オブジェクト属性テーブルに `type`（種別）と `variable`（可変フラグ）を追加
- [x] requirements.md: 未決定事項の空テーブルを削除
- [x] requirements.md: 対応フォーマット（FFmpegが対応する形式すべて）を記載
- [x] requirements.md: BGMの挙動を記載
- [x] design.md: 技術スタック表の「テンプレートファイル形式」→「プロジェクトファイル形式」に修正
- [x] design.md: スキーマ補足のシーン `duration` の記述を正確にする
- [x] CLAUDE.md: 「テンプレート」の用語を修正
- [x] design.md: スキーマ補足テーブルに `loop` の説明を追加
- [x] requirements.md: BGMのフェードアウトはループ時でも動画全体の最後にのみ適用されることを明記
- [x] requirements.md / design.md: 映像オブジェクトから `duration` を除外

</details>

## 実装

上記がすべて完了したら実装に進む。
