# 設計ドキュメント

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
- **FFmpeg同梱**: ユーザーの追加インストール不要でシングルバイナリ配布を実現。FFmpegはGPLのためソフトウェア全体もGPLライセンスとする。ソースコードはGitHubで公開済みのため要件を満たす。

---

## プロジェクトファイルのYAMLスキーマ

プロジェクトファイルの完全なサンプルを以下に示す。すべての設定項目を含む。

```yaml
# 出力フォルダ
output_folder: /path/to/output

# 出力設定
output:
  width: 1920
  height: 1080
  fps: 30
  codec: h264       # h264 / h265 / vp9 など
  format: mp4       # mp4 / mov / webm など

# シーン定義（記述順に再生）
scenes:
  - id: intro
    objects:
      - id: intro_video
        type: video
        file: /path/to/intro.mp4
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 100

  - id: main
    objects:
      # 記述順がZ順（後が前面）
      - id: main_video
        type: video
        variable: true  # 可変：エントリごとにファイルを指定
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 80        # 映像の音声音量 0-100

      - id: bgm
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0     # 0 = シーン終端まで
        volume: 30
        fade_in: 1.0
        fade_out: 2.0
        loop: true        # 音声が短い場合: true=ループ / false=無音

      - id: logo
        type: image
        file: /path/to/logo.png
        x: 1720
        y: 40
        width: 160
        height: 80
        start: 0.0
        duration: 0.0
        opacity: 80

      - id: photo
        type: image
        variable: true  # 可変：エントリごとにファイルを指定
        x: 1600
        y: 820
        width: 200
        height: 200
        start: 3.0
        duration: 10.0
        opacity: 100

      - id: caption
        type: text
        variable: true  # 可変：エントリごとにテキストを指定
        x: 100
        y: 900
        width: 800
        height: 80
        start: 3.0
        duration: 10.0
        opacity: 100
        font: NotoSansCJK-Bold
        color: "#ffffff"
        background_color: "#00000088"  # 透明度付き背景

  - id: outro
    objects:
      - id: outro_video
        type: video
        file: /path/to/outro.mp4
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 100

  # 映像オブジェクトがないシーン（固定秒数を指定）
  - id: title_card
    duration: 3.0
    objects:
      - id: title_text
        type: text
        text: "役員メッセージ"
        x: 760
        y: 490
        width: 400
        height: 100
        start: 0.0
        duration: 0.0
        opacity: 100
        font: NotoSansCJK-Bold
        color: "#ffffff"

# エントリリスト
entries:
  - name: tanaka          # 出力ファイル名（tanaka.mp4）
    variables:
      main_video:
        file: /path/to/tanaka.mp4
        trim_start: 3.0   # 冒頭3秒カット
        trim_end: 2.0     # 末尾2秒カット
      photo:
        file: /path/to/tanaka_photo.jpg
      caption:
        text: "田中 太郎  代表取締役社長"

  - name: suzuki
    variables:
      main_video:
        file: /path/to/suzuki.mp4
        trim_start: 5.0
        trim_end: 0.0
      photo:
        file: /path/to/suzuki_photo.jpg
      caption:
        text: "鈴木 花子  取締役CFO"
```

### スキーマ補足

| 項目 | 補足 |
|------|------|
| `variable: true` | そのオブジェクトをエントリごとに差し替え可能にするフラグ。映像・画像・テキスト・音声すべてに使用可 |
| `duration: 0.0` | 画像・テキスト・音声オブジェクトでシーン終端まで表示/再生を継続する。映像オブジェクトには `duration` は存在しない（再生長はファイルの長さで決まる） |
| `trim_start` / `trim_end` | 可変映像オブジェクトのみ。エントリで指定。省略時は0 |
| `volume` | 映像オブジェクト: 埋め込み音声の音量（0-100）。音声オブジェクト: 再生音量（0-100） |
| `fade_in` / `fade_out` | 音声オブジェクトのみ。フェードの秒数 |
| `loop` | 音声オブジェクトのみ。音声ファイルがdurationより短い場合の挙動。`true` でループ再生、`false` で残りは無音 |
| `background_color` | アルファ値付きの16進数カラーコード（`#RRGGBBAA`）を使用可 |
| シーンの `duration` | 明示的な時間を持つオブジェクト（映像、またはduration > 0の画像/テキスト/音声）が1つも存在しない場合のみ必要 |
