# FFmpeg レンダリングパイプライン

FFmpegコマンド生成・filter_complex構築・音声ミキシング・エスケープの詳細設計。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [01_project_schema.md](01_project_schema.md)（Project 型・ProbeResult 型）

---

## FFmpeg呼び出し方式

Rustから `std::process::Command::args()` で引数配列としてCLIサブプロセス呼び出しする（シェル経由禁止）。同梱バイナリ（Tauriサイドカー）を使用。

`std::process::Command` の `.args()` で引数配列として渡す。シェル（`/bin/sh -c`）経由の呼び出しは禁止。

### FFmpeg / ffprobe バイナリのフルパス取得

`externalBin` でバンドルされたバイナリは Tauri がターゲットトリプルサフィックスを付与して**実行ファイル隣接ディレクトリ**に配置する（macOS バンドルでは `Contents/MacOS/`、Windows では exe と同じフォルダ）。`resource_dir()` 配下ではないため、パスは `std::env::current_exe()?.parent()?.join(&ffmpeg_bin)` で解決する。ターゲットトリプルの取得には Cargo がビルド時に設定する `env!("TARGET")` を使用する。実行時パスは `.setup()` で `AppState` に解決・格納する（`main.rs` 骨格を参照）。Windows では `std::process::Command` に絶対パスを渡しても `.exe` は自動補完されないため、`.setup()` で `cfg!(target_os = "windows")` 分岐を使い `.exe` を付与する。

---

## 1エントリのレンダリング手順

1. バッチ開始時に `std::fs::create_dir_all(&project.output_folder)` を呼ぶ（既存なら無操作、存在しなければ中間ディレクトリも含めて作成）
2. 可変値をプロジェクトに展開してエントリ固有の完全な構造を生成
3. 全入力ファイルを `-i` として列挙
4. `filter_complex` グラフを構築
5. FFmpegを起動し `-progress pipe:1` で進捗を取得
6. 完了後、出力ファイルを `{output_folder}/{entry_name}.{format}` に保存

---

## オブジェクトの表示タイミング制御

YAML のオブジェクト `start` / `duration` は**シーン内の相対時刻**を示す。FFmpeg 側では以下のフィルタで制御する:

- **映像オブジェクト**: `setpts=PTS-STARTPTS+<start>/TB` で start だけオフセット、`trim=duration=<effective_duration>` で終了時刻を制限
- **画像オブジェクト**: `-loop 1 -t <duration> -itsoffset <start> -i image.png` で start オフセットと duration 制限
- **overlay による合成**: `overlay=X:Y:enable='between(t,<start>,<start>+<duration>)'` で表示時間帯を制御（座標は位置引数形式 `overlay=X:Y` に統一）
- **テキスト**: `drawtext=...:enable='between(t,<start>,<start>+<duration>)'` で同様に制御
- **音声**: `aformat=sample_rates=<out_sample_rate>:channel_layouts=stereo` でサンプルレート統一とステレオ変換を行ってから、`adelay=<round(start*1000)>|<round(start*1000)>` で start 秒だけ遅延、`atrim=duration=<duration>` で duration 制限（`adelay` は整数ミリ秒のみ許容のため `round()` で丸める）。`aformat` により amix での入力サンプルレート不一致を防ぐ
- `duration: 0.0`（シーン終端まで）の場合、`<duration>` にはシーン長を代入

---

## filter_complex の構築戦略

### 1シーン内の合成例（映像 + 画像オーバーレイ + テキスト + 音声ミックス）

入力インデックスの前提（`-i` 引数の順番）:
- `[0]` → main_video（可変映像ファイル）
- `[1]` → logo.png（固定画像）
- `[2]` → bgm.mp3（固定音声）

```
[0:v]trim=start=3:duration=25,setpts=PTS-STARTPTS,scale=1920:1080:flags=lanczos,
     format=yuva420p[v0];  # main_video は opacity:100 のため colorchannelmixer 省略
[1:v]scale=160:80:flags=lanczos,format=yuva420p,colorchannelmixer=aa=0.8[v1];  # logo: opacity:80 → aa=0.8
[v0][v1]overlay=1720:40[vtmp];  # logo は start=0・duration=0（シーン全域表示）のため enable 省略
[vtmp]drawtext=fontfile=<フォントディレクトリ>/NotoSansCJK-Bold.otf:text='田中 太郎':
      x=100:y=900:fontsize=64:fontcolor=0xffffff:
      box=1:boxcolor=0x00000088:
      enable='between(t,<start>,<start>+<duration>)'[vout];
# ※ font_size: 48（pt）→ round(48 × 96/72) = 64px に換算して fontsize= へ渡す
[0:a]volume=0.8[a0];
[2:a]volume=0.3,afade=in:st=<start>:d=<fade_in>,afade=out:st=<scene_len - fade_out>:d=<fade_out>,
     aloop=loop=-1:size=<sample_count>[a1];
# ※ size = round(ファイル長 × サンプルレート)
[a0][a1]amix=inputs=2:duration=longest[aout]
```

### 映像オブジェクトの scale 汎用形

- 出力解像度全体を覆う main_video は `scale=<output_width>:<output_height>`
- ピクチャ・イン・ピクチャ等オブジェクト固有サイズを持つ映像は `scale=<object.width>:<object.height>:flags=lanczos`
- Rust 実装では映像オブジェクトの `width` / `height` フィールドを参照し、未指定（`null`）の場合は出力解像度を使用する

### `colorchannelmixer` 省略ロジック

`opacity == 100` の場合は `aa=1.0`（デフォルト）と等価なため `colorchannelmixer` フィルタ自体を省略する。`opacity < 100` の場合のみ `colorchannelmixer=aa=<opacity/100>` を追加する。

### 複数シーンの連結（concatフィルタ）

```
[s0a]aresample=<sample_rate>[s0ar];
[s1a]aresample=<sample_rate>[s1ar];
[s2a]aresample=<sample_rate>[s2ar];
[s0v][s0ar][s1v][s1ar][s2v][s2ar]concat=n=3:v=1:a=1[vfinal][afinal]
```

**シーン終端ラベルの命名規則**: 単一シーンの合成終端ラベルは `[vout]` / `[aout]` とする。複数シーンの場合は各シーンの `[vout]` / `[aout]` を `null` / `anull` フィルタで `[s<n>v]` / `[s<n>a]` にリラベルしてから `concat` フィルタに渡す。例: `[vout]null[s0v]; [aout]anull[s0a]`。単一シーン（concat 不要）の最終 `-map` は `"[vout]"` / `"[aout]"`、複数シーンは `"[vfinal]"` / `"[afinal]"` を使用する。

各シーンの音声ストリームは concat フィルタ前に `aresample=<sample_rate>` で出力サンプルレートに統一する（mp4/mov は 44100 Hz、webm は 48000 Hz）。サンプルレートが既に一致している場合も `aresample` を通すことで concat の安定性を確保する。

---

## 中間ファイル

- シーン数 > 15 または総オブジェクト数 > 150 の場合、各シーンを一時ファイルに出力し concat demuxer で結合
- 一時ファイルは `std::env::temp_dir()` 配下に作成。ファイル名形式: `ezmm-{uuid}-scene{n}.mp4`
- 成功・エラー・キャンセルいずれの場合も終了時に一時ファイルを削除

---

## 進捗取得

`-progress pipe:1` オプションで標準出力に進捗情報を出力。`out_time_ms=NNNNNN` を毎フレームパースし、エントリ全体の予測時間（全シーン合計 = `entry_progress` の分母）と比較して進捗%を算出。Tauriイベント `batch:progress` でフロントエンドに通知。

---

## エラー検出・キャンセル

- 終了コード非0 → エラー
- stderrの `Error` / `Invalid` / `No such file` パターンでユーザー向けメッセージを生成
- キャンセル: `child.kill()` → 出力先の不完全ファイルを削除

---

## 同梱FFmpegバイナリ

- **バージョン**: FFmpeg 7.x（GPLスタティックビルド）
- **配置**: `src-tauri/binaries/`（`.gitignore` 対象）
- **Mac**: `ffmpeg-aarch64-apple-darwin`（arm64）+ `ffmpeg-x86_64-apple-darwin`（Intel）
- **Windows**: `ffmpeg-x86_64-pc-windows-msvc.exe`
- Tauriサイドカー機能（`tauri.conf.json` の `bundle.externalBin`）で同梱

---

## コーデック名マッピング

YAML の `codec` 値から FFmpeg エンコーダ名への変換（ハードウェアエンコーダは使用しない）:

| YAML `codec` 値 | FFmpeg `-c:v` 引数 |
|----------------|-------------------|
| `h264` | `libx264` |
| `h265` | `libx265` |
| `vp9` | `libvpx-vp9` |

---

## 出力画質の既定値

| コーデック | CRF | preset / cpu-used |
|-----------|-----|-------------------|
| H.264 (libx264) | 23 | medium |
| H.265 (libx265) | 28 | medium |
| VP9 (libvpx-vp9) | 33 | cpu-used=4, row-mt=1 |

---

## 音声コーデック・パラメータ

| コンテナ | 音声コーデック | ビットレート | サンプルレート | チャンネル |
|--------|------------|------------|------------|---------|
| mp4 / mov | AAC (aac) | 192 kbps | 44100 Hz | ステレオ (2ch) |
| webm | Opus (libopus) | 128 kbps | 48000 Hz | ステレオ (2ch) |

**FFmpeg コマンドラインフラグ（最終エンコード時）:**

- **H.264**: `ffmpeg ... -pix_fmt yuv420p -c:v libx264 -crf <crf> -preset <preset> -map "[vout]" -map "[aout]" -c:a aac -b:a 192k -ar 44100 -ac 2 <output>`
- **H.265 + mp4/mov**: `ffmpeg ... -pix_fmt yuv420p -c:v libx265 -crf <crf> -preset <preset> -tag:v hvc1 -map "[vout]" -map "[aout]" -c:a aac -b:a 192k -ar 44100 -ac 2 <output>`（`-tag:v hvc1` は QuickTime / Apple 環境での再生互換性確保のため必須）
- **VP9**: `ffmpeg ... -pix_fmt yuv420p -c:v libvpx-vp9 -crf <crf> -b:v 0 -cpu-used 4 -row-mt 1 -map "[vout]" -map "[aout]" -c:a libopus -b:a 128k -ar 48000 -ac 2 <output>`（VP9 は `-preset` 非対応のため省略）

`-pix_fmt yuv420p` は必須（filter_complex は `yuva420p` でアルファチャンネルを保持するが、H.264/H.265/VP9 はアルファ非対応のため最終出力前に変換が必要）。

**VP9 の `preset` フィールドの扱い:** VP9 は `preset` の代わりに `cpu-used`（0〜5）を使用する。`OutputSettings.preset` フィールドは H.264/H.265 専用とし、VP9 選択時は無効化（グレーアウト）する。VP9 の `cpu-used` はデフォルト値 `4`・`row-mt=1` を固定で使用し、GUI での変更は v2 以降。

---

## filter_complex 詳細仕様（追補）

### 画像オブジェクトの扱い

```
ffmpeg -loop 1 -t <duration> -i image.png ...
```

- `-loop 1`: 静止画を繰り返す
- `-t <duration>`: 表示期間。`duration == 0.0` の場合はシーン長を使用

### シーン背景色

各シーンの描画されない領域の背景は**黒**（`#000000`）。明示的な背景色指定機能は v1 では非対応（全シーン黒背景固定）。映像オブジェクトが画面全体を覆わない場合、`color=black:s=<output_width>x<output_height>:d=<scene_len>,format=yuva420p[bg]` を最下層として生成し overlay の base に使用する。

### 映像オブジェクトがないシーンの映像トラック生成

音声のみ・テキストのみ・画像のみのシーンで映像オブジェクトが存在しない場合:

```
color=black:s=1920x1080:d=5.0,format=yuva420p[bgv];
[bgv][overlay1]overlay=...[vtmp];
...
```

### 音声トラック欠落時の amix 対応

映像オブジェクトの音声トラック有無を ffprobe で事前確認し、シーン内の有効音声入力数（N）に応じて3ケースで分岐する:

- **N = 0（音声なし）**: `amix` を使わず `anullsrc=r=<sample_rate>:cl=stereo,atrim=duration=<scene_len>[aout]` を生成する
- **N = 1（1入力のみ）**: `amix=inputs=1` は FFmpeg が拒否するため `[a0][aout]` のように直接ラベルを付け替える
- **N ≥ 2（2入力以上）**: `[a0][a1]...[aN-1]amix=inputs=N:duration=longest[aout]` を使用する

### aloop の size パラメータ計算

```
aloop=loop=-1:size=<sample_count>
```

`size` = ファイルの総サンプル数 = `round(ファイル長(秒) × サンプルレート(Hz))`
ffprobe の `duration` と `sample_rate` から計算する。

### loop: silence の実装

```
[N:a]atrim=end=<file_duration>,apad=whole_dur=<scene_len>[audio_out]
```

### 同一ファイルの重複排除

複数オブジェクトが同一ファイルパスを参照する場合、`-i` フラグは1回だけ指定し、`split` フィルタで分岐する:

```
# N 個の参照先があれば split=N、出力ラベルも N 個
[0:v]split=N[v_ref1][v_ref2]...[v_refN];
[v_ref1]scale=...,trim=...[v_obj1];
[v_ref2]scale=...,trim=...[v_obj2];
```

音声ストリームも同様に `asplit=N[a_ref1][a_ref2]...[a_refN]` を使用する。

同一パスのファイルを識別するキーは `dunce::canonicalize()` で正規化した絶対パスを使用する。

---

## concat フィルタ vs concat demuxer の使い分け

- シーン数 ≤ 15 かつ総オブジェクト数 ≤ 150: 一括 `filter_complex` で処理
- 上記を超える場合: 各シーンを中間ファイルに出力してから `concat demuxer` で結合

中間ファイルのコーデックは **H.264 lossless** (`-c:v libx264 -qp 0`) を使用。

**VP9/webm 出力時の注意**: 中間ファイルの音声は AAC 44100Hz で固定だが、webm 最終エンコードは libopus 48000Hz 必須のため、最終エンコードコマンドに `-ar 48000` を加え FFmpeg の内部リサンプラーで変換する。

**concat demuxer の手順**:
1. 各シーンを `filter_complex`（単一シーンモード）でレンダリングし、一時ファイル `ezmm-{uuid}-scene{n}.mp4` に出力
2. 全シーンの一時ファイルを列挙した `concat.txt` を作成:
   ```
   file '/absolute/path/to/ezmm-xxx-scene0.mp4'
   file '/absolute/path/to/ezmm-xxx-scene1.mp4'
   ```
3. concat demuxer で結合し最終エンコード:
   `ffmpeg -f concat -safe 0 -i concat.txt -c:v libx264 -crf 23 ... <output>`

---

## drawtext 特殊文字エスケープ規則

FFmpeg の `drawtext` フィルタでは、フィルタグラフ内の特殊文字を二段階でエスケープする:

1. **drawtext レベル** (`key=value` 内):
   - `\` → `\\`
   - `'` → `'\''`（シングルクォート終端 + エスケープ + 再開）
   - `:` → `\:`
   - `%` → `%%`

2. **filter_complex レベル**（文字列全体）:
   - `,` → `\,`（フィルタ区切りとの混同回避）
   - `[` `]` → `\[` `\]`（ストリームラベルとの混同回避）

Rust実装では `escape_drawtext_value(s: &str) -> String` 関数を作成し、テキストオブジェクトの文字列をすべて通してから filter_complex に埋め込む。

### フィルタ文字列エスケープ関数

- `escape_drawtext_value(s: &str) -> String`: `drawtext` の `text=` 値に埋め込むテキスト（2段階エスケープ）
- `escape_filter_value(s: &str) -> String`: `filter_complex` 内に埋め込むファイルパスやその他の文字列（`,` `[` `]` `'` `\` のみエスケープ）

---

## アスペクト比不整合時の扱い

デフォルトは**引き伸ばし**（`scale=<output_width>:<output_height>` でそのまま指定サイズへ変換）。ユーザーがオブジェクトの `width`/`height` で意図的にサイズを指定するため、アプリ側での自動補正は行わない。

---

## 画像リサイズアルゴリズム

`scale` フィルタに `flags=lanczos` を使用（高品質縮小）。

---

## 座標・サイズのはみ出し

オブジェクトの座標・サイズが出力解像度をはみ出す場合、バリデーションで**警告**を出す（エラーとしてブロックはしない）。FFmpegは自動クリップするため映像出力は可能。

---

## FFmpegフィルタへの値埋め込みとインジェクション対策

`std::process::Command::args()` で引数配列渡しを使用するためシェルインジェクションは発生しない。ただし `filter_complex` 文字列は単一の文字列引数に結合されるため、フィルタ文法上の特殊文字は上記エスケープ規則に従い処理する。

---

## H.265 + MP4 の QuickTime 互換

H.265 を MP4 / MOV にパッケージする際は `-tag:v hvc1` を付与してQuickTime / Apple環境での再生互換性を確保する。

---

## 入力ファイルの正規化

入力映像の解像度・FPSが出力設定と異なる場合:
- **解像度**: `scale=<output_width>:<output_height>:flags=lanczos` フィルタで出力解像度に変換
- **FPS**: 基本的に出力FPSに合わせる処理はしない（trim後のPTS調整のみ行う）。入力FPSが出力FPSと**2倍以上異なる場合**はFFmpegが内部補間するが、アーティファクトが出る可能性をバリデーション警告として提示

---

## FFmpeg ランタイム制御方針

### ハードウェアアクセラレーション（hwaccel）

**使用しない（CPU エンコードのみ）**。理由:
- GPU エンコーダの利用可否が環境依存（NVIDIA/Intel/Apple Silicon で異なる API）
- GPU エンコードは CRF 相当の品質制御が CPU と異なる
- 社内配布の低スペック PC では GPU エンコーダが存在しない場合がある

### threads

`-threads` オプションは指定しない（FFmpeg のデフォルト自動調整に任せる）。

### loglevel

`-loglevel warning` を常に指定する。デバッグモード時は `-loglevel verbose` に切り替える。

### FFmpeg ビルドのライセンス種別

**GPL ビルドを使用**する:
- libx264 (GPL)・libx265 (GPL) を含むため LGPL ビルドでは利用不可
- アプリ全体も GPL となるのは要件通り

---

## ffprobe と入力ファイルメタデータ取得

### メタデータ取得コマンド

```bash
ffprobe -v quiet -print_format json -show_format -show_streams <file>
```

### 取得する情報

| ffprobe フィールド | `ProbeResult` フィールド | 用途 |
|-----------|------|------|
| `format.duration` | `duration: f64` | シーン長計算・trim バリデーション用 |
| `streams[video].width` / `height` | `width?: u32` / `height?: u32` | 新規追加時のデフォルトサイズ設定用 |
| `streams[video].r_frame_rate` | `fps?: f64` | FPS確認（バリデーション警告用） |
| `streams[audio]` の有無 | `has_audio: bool` | 音声トラック有無（amix 入力除外判定用） |
| `streams[audio].sample_rate` | `sample_rate?: u32` | `aloop size = round(duration × sample_rate)` 計算用 |

**duration の取得先:** `-show_format -show_streams` を併用し、優先順位は `format.duration` → 映像ストリームの `stream.duration` → 音声ストリームの `stream.duration` の順で採用する。

**r_frame_rate の変換:** ffprobe は `"30000/1001"` 形式の分数文字列を返す。変換: 文字列を `/` で分割し `num.parse::<f64>() / den.parse::<f64>()` で f64 化。`"0/0"` や parse 失敗時は `None` を設定。

### ffprobe の同梱

FFmpeg と同様に ffprobe を Tauri サイドカーとして同梱する:
- 配置: `src-tauri/binaries/` に FFmpeg と並べて配置
- `tauri.conf.json` の `bundle.externalBin` に ffprobe も追加

### ffprobe 失敗時の扱い

- ファイルが存在しない場合: バリデーションエラー（ファイル存在チェックで検出済み）
- ffprobe が対応していないフォーマット: 警告表示（シーン長不明）。バッチ実行時に FFmpeg がエラーを出すため最終的に検出される
- ffprobe 実行自体が失敗（バイナリ不存在等）: バッチ実行前の初期化チェックでエラーとして表示し起動を止める

---

## プレースホルダ表記ルール

本ドキュメント内のプレースホルダは文脈により2通りの表記を使い分ける:
- **FFmpeg コマンド・フィルタ例**（コードフェンスまたは `` ` `` 内のフィルタ文字列）: `<placeholder>` 形式。例: `atrim=duration=<scene_len>`、`size=<sample_count>`
- **プロース・パス参照**: `{placeholder}` 形式。例: `{output_folder}/{entry_name}.{format}`、`ezmm-{uuid}-scene{n}.mp4`
