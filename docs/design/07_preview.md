# リアルタイムプレビュー仕様

Canvas APIを使ったリアルタイム合成再生の詳細設計。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [01_project_schema.md](01_project_schema.md)（Project 型）、[09_store.md](09_store.md)（PreviewStore）

---

## 概要

- 各映像オブジェクト → 非表示の `<video>` 要素（1オブジェクトにつき1要素）。`src` には `convertFileSrc(absolutePath)` で生成したプラットフォーム別 URL を設定する（macOS: `https://asset.localhost/...`、Windows: `http://asset.localhost/...`）。`blob:` URL は使用しない
- 各音声オブジェクト → Web Audio API `AudioBufferSourceNode`
- Canvasは `requestAnimationFrame` で毎フレーム再描画（最大30fps制限）
- プレビュー解像度: `AppSettings.preview_resolution_scale`（デフォルト 0.5）を出力解像度に乗じた値（例: 1920×1080 × 0.5 → 960×540）

---

## フレームレンダリングループ

毎フレームの処理:

1. 現在の再生位置（秒）から現在シーンとシーン内相対時間を計算
2. Canvasをクリア（黒背景で塗りつぶす）:
   ```typescript
   ctx.clearRect(0, 0, canvas.width, canvas.height);
   ctx.fillStyle = '#000000';
   ctx.fillRect(0, 0, canvas.width, canvas.height);
   ```
3. 現在シーン内のオブジェクトをYAML記述順（= Z順）に描画:
   - **映像**: `ctx.globalAlpha = opacity/100` → `ctx.drawImage(videoEl, x, y, w, h)`。`<video>` 要素は `requestVideoFrameCallback` でフレーム更新を検知し、コールバック内で最新フレームを drawImage する（**対応環境**: WKWebView 14+・WebView2 90+）
   - **画像**: プリロード済みの `HTMLImageElement` を `drawImage`
   - **テキスト**: 背景矩形（`background_color` がある場合） + `ctx.fillText`
   - **音声**: Canvas描画なし（Web Audio APIで再生）
   - **描画合成モード**: `ctx.globalCompositeOperation = 'source-over'`（デフォルト値。明示セットで他モードが残らないことを保証）
4. 表示条件: `scene_time >= start && (duration === 0 || scene_time < start + duration)`

---

## トリム・フェード・音量

### 映像トリム

シーン切り替え時（メディアロード完了後）に `video.currentTime = trim_start` をセット。再生中は毎フレーム `video.currentTime >= file_duration - trim_end` を監視してポーズ。シーク時も `seek_pos + trim_start` に currentTime を設定する。

### 映像音量

`createMediaElementSource(videoEl)` → `GainNode`（`gain.value = volume/100`）→ `AudioContext.destination`

### 音声オブジェクト再生手順

1. `fetch(convertFileSrc(absolutePath))` → `response.arrayBuffer()` → `AudioContext.decodeAudioData(buffer)` で `AudioBuffer` を取得
2. `AudioBufferSourceNode.buffer = audioBuffer` / `.loop = (loop === 'loop')` で再生ノードを生成
3. `GainNode`（`gain.value = volume/100`）を経由して `AudioContext.destination` に接続
4. シーン内 `start` 秒後に `AudioBufferSourceNode.start(audioCtx.currentTime + offset)` で遅延開始
5. `loop: silence` の場合: `AudioBufferSourceNode.loop = false`（ファイル長以降は自動無音）
6. フェード: `GainNode.gain.setValueAtTime(0, startTime)` + `linearRampToValueAtTime(target, startTime + fadeIn)` で fade-in、fade-out は同様に逆方向で設定

### 透明度

`ctx.globalAlpha` を毎フレーム設定

---

## シーン間遷移

- シーンは記述順に連続再生。シーン終了で自動的に次へ
- シークバーはプロジェクト全体の累積時間で表示
- シーン切り替え時に前シーンのメディアを停止し、次シーンをロード

---

## プレビュー解像度・フレームレート

- 描画解像度: `AppSettings.preview_resolution_scale`（デフォルト 0.5）を出力解像度に乗じた値
  - 例: 1920×1080 × 0.5 → 960×540
  - アプリ起動時に `load_settings` IPC で取得した値を SettingsStore に保持し、PreviewStore がその値を参照して Canvas サイズを決定する
  - v1 では固定 0.5 のまま変更 UI はない
- 最大フレームレート: 30fps（出力FPSが30以上の場合もプレビューは30fps上限）
- 低スペックPCでコマ落ちが発生する場合は `requestAnimationFrame` 間引きにより自動的に低下する（実装上の自然な挙動）

---

## メモリ管理

- アクティブなシーンのみ映像・音声要素をロード
- 前後のシーンは `<video>` の `src` を解放してメモリを回収
- 次のシーンへの遷移前に次シーンのメディアを prefetch — 現シーン残り3秒のタイミングで `preload="metadata"` を設定、再生直前（現シーン残り0.5秒）で `preload="auto"` に切り替える

---

## AudioContext autoplay ポリシー対応

ブラウザ（WebView）はユーザージェスチャなしに AudioContext を開始できない。対応方針:
- 初回の任意クリックイベントで `AudioContext.resume()` を呼び出す
- `AudioContext.state === 'suspended'` の間は再生ボタンを無効化し「クリックして再生を有効化」の案内を表示

---

## 欠落ファイルのプレースホルダ表示

映像・画像・音声が未指定またはファイルが存在しない場合:
- 映像・画像: グレーの矩形 + 中央にファイル名（または「未設定」）を表示
- 音声: 再生時に無音。タイムライン上でオブジェクトを警告色（オレンジ）に表示

---

## プレビューと最終出力の差異について

プレビューエリアの下部に常時注記を表示:「プレビューは参考表示です。テキスト描画・フォントメトリクスは最終出力（FFmpeg）と異なる場合があります。」

---

## テキスト y 座標の基準統一

`y` はバウンディングボックスの**上端**を基準とする:
- **Canvas**: `ctx.textBaseline = 'top'` を設定して `ctx.fillText(text, x, y)` を呼び出す
- **FFmpeg drawtext**: `y=<value>` はデフォルトでテキストの上端（ascender line）基準。実装上は一致とみなす

---

## align と x 座標の意味

| align | x の意味 | Canvas | FFmpeg |
|-------|---------|--------|--------|
| left | テキストの左端 = `x` | `textAlign='left'`、x をそのまま渡す | `x=<x>` |
| center | バウンディングボックスの中央 = `x + width/2` | `textAlign='center'`、`x + width/2` を渡す | `x=<x+width/2>-text_w/2`（drawtext の `x` は常に左端のため `text_w` を引いて中央揃え） |
| right | バウンディングボックスの右端 = `x + width` | `textAlign='right'`、`x + width` を渡す | `x=<x+width>-text_w`（同様に `text_w` を引いて右端揃え） |

---

## font_size の単位

YAML の `font_size` は**ポイント（pt）**として扱う。

レンダリング時は `px = round(font_size × 96 / 72)` で換算してから描画に渡す（96 DPI 基準: 1pt = 1.333...px）:

- **Canvas（プレビュー）**: `Math.round(font_size * 96 / 72)` px を算出し、さらにプレビュースケール（0.5）を乗じた値を `ctx.font` に渡す
  - 例: `font_size: 48`（48pt） → 64px → プレビュー 32px
- **FFmpeg drawtext**: `fontsize=Math.round(font_size * 96 / 72)` として渡す
  - 例: `font_size: 48`（48pt） → `fontsize=64`

両実装で同一の px 値を使うため、プレビューと最終出力の文字サイズは一致する。

---

## プレビュー精度に関する制約

- **フレームスナップ**: `start` / `duration` は 1/fps の整数倍に自動スナップしない。float 秒のまま保存・使用する。GUI での「fps 整列」ボタンは v2 以降
- **テキスト折り返し**: `width` を超えるテキストはクリップ（自動折り返しなし、初期バージョン）
- **縁取り・影**: 初期バージョンでは非対応
- **`height` の使途**: テキストオブジェクトの `height` はバウンディングボックスの高さ。Canvas では `background_color` がある場合の背景矩形の縦サイズに使う。テキストが `height` をはみ出しても**クリップしない**（初期バージョン）

---

## シーク操作

再生中・一時停止中を問わず、シークバークリックで即座にシークする。再生中にシークした場合、シーク後も再生状態を維持する。

## エントリ切り替え時の挙動

別エントリを選択した場合、プレビューを**停止**して先頭（0秒）に戻る。自動再生はしない（ユーザーが ▶ で再生を開始する）。

## フレーム単位移動

一時停止中に `←` `→` キーで 1/fps 秒（1フレーム）移動する（float 秒単位で加減算）。

---

## エントリごとのシーン長変動

可変映像オブジェクトが含まれるシーンは、エントリによってシーン長が変わる（映像ファイルの長さが異なるため）。

**プレビューの扱い:**
- 選択中エントリの可変映像ファイルを ffprobe で取得した duration を使ってシーン長を計算する
- プレビューのシークバーは選択中エントリのシーン長を表示する（エントリ切り替えで更新）
- 可変映像が未指定の場合はシーン長不明として扱い、シークバーを「-- : --」表示にする

---

## WebViewのコーデック対応差

| コーデック | macOS WKWebView | Windows WebView2 |
|-----------|----------------|-----------------|
| H.264 | ✅ | ✅ |
| H.265 | ✅（macOS 11+） | △（Microsoft Store の「HEVC ビデオ拡張機能」または OEM プリインストールの HEVC コーデックが必要。未インストール時は再生不可） |
| VP9 | ✅ | ✅ |

プレビューで H.265 ファイルを `<video>` で再生できない場合は「このファイルはプレビューで再生できません。バッチ実行で出力を確認してください」と表示。
