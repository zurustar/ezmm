# プロジェクトファイル スキーマ・データモデル

YAMLスキーマ・Rust/TypeScript型定義・スキーマバージョン管理。

> **参照元**: [設計書インデックス](../design.md)  
> **このモジュールは他のすべてのモジュールから参照される（依存の最上流）**

---

## YAMLスキーマ サンプル

プロジェクトファイルの完全なサンプルを以下に示す。すべての設定項目を含む。

```yaml
version: 1

# 出力フォルダ
output_folder: /path/to/output

# 出力設定
output:
  output_name: my_video      # 出力ファイル名（拡張子なし）
  width: 1920
  height: 1080
  fps: 30
  codec: h264       # h264 / h265 / vp9
  format: mp4       # mp4 / mov / webm
  crf: 23           # 品質値（h264: 0-51 推奨23、h265: 0-51 推奨28、vp9: 0-63 推奨33）
  preset: medium    # エンコード速度と品質のトレードオフ（ultrafast〜veryslow）

# シーン定義（記述順に再生）
scenes:
  - id: intro
    name: イントロ           # オプション。未設定時は GUI で「シーン 1」などと表示
    objects:
      - id: intro_video
        name: イントロ映像    # オプション。未設定時はファイル名から自動生成
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
        file: /path/to/main.mp4
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 80            # 映像の音声音量 0-100
        trim_start: 3.0       # ソースファイルの再生開始位置（秒）。省略時は 0
        trim_end: 30.0        # ソースファイルの再生終了位置（秒）。省略時はファイル末尾

      - id: bgm
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0         # 0 = シーン終端まで
        volume: 30
        fade_in: 1.0
        fade_out: 2.0
        loop: loop            # 音声ファイルがdurationより短い場合: loop=ループ / silence=無音

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

      - id: caption
        type: text
        text: "田中 太郎  代表取締役社長"
        x: 100
        y: 900
        width: 800
        height: 80
        start: 3.0
        duration: 10.0
        opacity: 100
        font: "NotoSansCJK-Bold"
        font_size: 48
        color: "#ffffff"
        background_color: "#00000088"  # 透明度付き背景
        align: left     # left / center / right

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
        font: "NotoSansCJK-Bold"
        font_size: 64
        color: "#ffffff"
        align: center
```

---

## スキーマ補足

| 項目 | 補足 |
|------|------|
| `duration: 0.0` | 画像・テキスト・音声オブジェクトでシーン終端まで表示/再生を継続する。映像オブジェクトには `duration` は存在しない（再生長はファイルの長さで決まる） |
| `trim_start` / `trim_end` | 映像オブジェクト（任意）。ソースファイルの再生開始・終了位置（秒）。`trim_start` 省略時は 0、`trim_end` 省略時はファイル末尾 |
| `volume` | 映像オブジェクト: 埋め込み音声の音量（0-100）。音声オブジェクト: 再生音量（0-100） |
| `fade_in` / `fade_out` | 音声オブジェクトのみ。フェードの秒数 |
| `loop` | 音声オブジェクトのみ。音声ファイルがdurationより短い場合の挙動。`loop` でループ再生、`silence` で残りは無音 |
| `background_color` | アルファ値付きの16進数カラーコード（`#RRGGBBAA`）を使用可。6桁（`#RRGGBB`）の場合は `#RRGGBBFF`（完全不透明）として扱う。Canvas では `rgba(R,G,B,A/255)` に変換、FFmpeg drawtext では `boxcolor=0xRRGGBBAA` に変換して渡す（FFmpeg 7.x では `boxcolor=0xRRGGBBAA` 形式の 8 桁 hex アルファが正式にサポートされており、同梱の GPL ビルドで動作確認済み。4.x 以前では非対応のため本プロジェクトは FFmpeg 7.x 前提とする） |
| シーンの `duration` | 明示的な時間を持つオブジェクト（映像、またはduration > 0の画像/テキスト/音声）が1つも存在しない場合のみ必要 |
| シーン・オブジェクトの `name` | 省略可。未設定時は GUI でシーン番号やファイル名から自動生成 |
| `version` | プロジェクトファイル先頭に記述。スキーマバージョン。現在は `1` |

---

## TypeScript 型定義

```typescript
interface Project {
  version: number;
  output_folder: string;
  output: OutputSettings;
  scenes: Scene[];
}

interface OutputSettings {
  output_name: string;
  width: number;
  height: number;
  fps: number;
  codec: 'h264' | 'h265' | 'vp9';
  format: 'mp4' | 'mov' | 'webm';
  crf: number;
  preset: string;
}

interface Scene {
  id: string;
  name?: string;
  duration?: number;
  objects: SceneObject[];
}

type SceneObject = VideoObject | ImageObject | TextObject | AudioObject;

interface BaseObject {
  id: string;
  name?: string;
  start: number;
}

interface VideoObject extends BaseObject {
  type: 'video';
  file?: string;
  x: number; y: number; width: number; height: number;
  opacity: number;
  volume: number;
  trim_start?: number;
  trim_end?: number;
}

interface ImageObject extends BaseObject {
  type: 'image';
  file?: string;
  x: number; y: number; width: number; height: number;
  duration: number;
  opacity: number;
}

interface TextObject extends BaseObject {
  type: 'text';
  text?: string;
  x: number; y: number; width: number; height: number;
  duration: number;
  opacity: number;
  font: string;
  font_size: number;
  color: string;
  background_color?: string;
  align?: 'left' | 'center' | 'right';
}

interface AudioObject extends BaseObject {
  type: 'audio';
  file?: string;
  duration: number;
  volume: number;
  fade_in?: number;
  fade_out?: number;
  loop: 'loop' | 'silence';  // YAML 省略時は Rust が 'loop' に補完。IPC レスポンスには常に含まれるため TS 型は必須
}

interface ProbeResult {
  duration: number;      // 秒（float）
  width?: number;        // 映像・画像のみ
  height?: number;       // 映像・画像のみ
  fps?: number;          // 映像のみ
  has_audio: boolean;    // 音声トラックの有無
  sample_rate?: number;  // 音声のみ（Hz）。aloop size 計算用: round(duration × sample_rate)
}

interface ValidationResult {
  errors: ValidationIssue[];     // 書き出しをブロック
  warnings: ValidationIssue[];   // 続行可能
}

interface ValidationIssue {
  severity: 'error' | 'warning';
  code: string;                    // 例: 'file_not_found', 'trim_out_of_range'
  message: string;                 // 日本語のユーザー向けメッセージ
  scene_id?: string;               // 対象シーンID（該当する場合）
  object_id?: string;              // 対象オブジェクトID（該当する場合）
}
```

注: TypeScript の `number` 型は整数・小数を区別しない。`x`, `y`, `width`, `height`, `font_size`, `version`, `fps`, `crf` など Rust 側で `u32` / `i32` と定義されたフィールドは、Rust の `validate_project` で整数であることを検証する（小数が渡された場合はエラー）。

---

## Rust 構造体

特記なき限り、すべての struct / enum に共通で `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` を適用する（IPC 送信での `Clone`、ログ出力での `Debug`、スナップショットテストでの `PartialEq` を想定）。

**例外**: Tauri イベントペイロード struct（`ExportProgressPayload` / `ExportDonePayload` / `ExportErrorPayload`）は `emit()` 専用で受信側では使わないため `#[derive(Clone, Serialize)]` のみとする（`Deserialize` / `PartialEq` は不要）。

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub version: u32,
    pub output_folder: String,
    pub output: OutputSettings,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputSettings {
    pub output_name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub codec: Codec,
    pub format: Format,
    pub crf: u32,
    pub preset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Codec { H264, H265, Vp9 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format { Mp4, Mov, Webm }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    pub objects: Vec<SceneObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SceneObject {
    Video(VideoObject),
    Image(ImageObject),
    Text(TextObject),
    Audio(AudioObject),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VideoObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub x: i32, pub y: i32,
    pub width: u32, pub height: u32,
    pub start: f64,
    pub opacity: u8,
    pub volume: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_end: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImageObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub x: i32, pub y: i32,
    pub width: u32, pub height: u32,
    pub start: f64,
    pub duration: f64,
    pub opacity: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub x: i32, pub y: i32,
    pub width: u32, pub height: u32,
    pub start: f64,
    pub duration: f64,
    pub opacity: u8,
    pub font: String,
    pub font_size: u32,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(default)]
    pub align: TextAlign,
}

fn default_align() -> TextAlign { TextAlign::Left }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign { #[default] Left, Center, Right }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AudioObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub start: f64,
    pub duration: f64,
    pub volume: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out: Option<f64>,
    // Rust の `loop` キーワード衝突回避: フィールド名は r#loop、YAML 上は loop
    #[serde(rename = "loop", default = "default_loop_mode")]
    pub r#loop: LoopMode,
}

fn default_loop_mode() -> LoopMode { LoopMode::Loop }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode { #[default] Loop, Silence }

#[derive(Serialize, Deserialize)]
pub struct ProbeResult {
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    pub has_audio: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,  // 音声のみ（Hz）。aloop size = round(duration × sample_rate)
}
```

---

## スキーマ仕様補足

### 座標系

- 原点: 左上 (0, 0)
- 単位: 出力解像度基準のピクセル
- `x`, `y`: オブジェクトのバウンディングボックスの左上座標
- `x`, `y` は負値も許容（画面外からの登場演出等）

### テキストオブジェクトの追加属性

| 属性 | 型 | YAML デフォルト | GUI 新規追加時の値 | 説明 |
|------|-----|---------|---------|------|
| `font` | string | YAML 必須（省略不可） | `NotoSansCJK-Regular` | 同梱フォント名: `NotoSansCJK-Regular` / `NotoSansCJK-Bold`。Rust 側での解決: `font_name + ".otf"` を `AppState.font_dir` に結合。ホワイトリスト外はバリデーションエラー |
| `font_size` | number | YAML 必須 | `24` | フォントサイズ（pt）。`height` はバウンディングボックスの高さ |
| `color` | string | YAML 必須 | `#ffffff` | 文字色（`#RRGGBB`）。FFmpeg drawtext に渡す際は `#RRGGBB` → `0xRRGGBB` 形式に変換する |
| `background_color` | string | 省略可（未指定で背景矩形なし） | 未設定 | 背景色（`#RRGGBBAA`）。指定時のみ背景矩形を描画 |
| `align` | string | 省略可（デフォルト `left`） | `left` | テキスト配置: `left` / `center` / `right` |

**同梱フォント:**
- `NotoSansCJK-Regular.otf`, `NotoSansCJK-Bold.otf`
- 配置先: `src-tauri/fonts/`（Tauri `bundle.resources` でバンドル）
- FFmpeg参照パス: アプリ起動時に `app_handle.path().resource_dir()` でリソースディレクトリを取得し、フォントパスを Rust 側で絶対パスとして保持
- Canvas参照: フロントエンド起動時に `get_font_paths` IPC を呼び、`convertFileSrc(absolutePath)` でプラットフォーム別URLを生成し `@font-face` を `<style>` に注入

### サポートコーデック・フォーマット

| codec | format |
|-------|--------|
| h264 | mp4, mov |
| h265 | mp4, mov |
| vp9 | webm |

### opacity / volume のスケール

- `opacity: 0` = 完全透明、`opacity: 100` = 完全不透明
- `volume: 0` = 無音、`volume: 100` = 元音量（100%）

### ファイルパスの扱い

- 絶対パス・相対パスの両方を受け付ける
- 相対パスはプロジェクトファイル（`.yaml`）の保存場所を起点に解決する
- 推奨: 相対パス（プロジェクトフォルダとメディアをまとめて移動可能）

### 映像ファイル長とシーン長の関係

- 映像オブジェクトの実効長 = `(trim_end ?? file_duration) − (trim_start ?? 0)`
- シーン長 = シーン内オブジェクトの終了時刻の最大値
- `scene.duration` が明示的に設定されている場合はそれを優先する

**循環参照の回避:** 以下に該当するシーンはシーン長が定義不能になるためバリデーションエラーとする:
- 映像オブジェクトがなく、かつ `duration > 0` のオブジェクトが1つも存在せず（画像・テキスト・音声すべて `duration: 0.0`）、かつ `scene.duration` も未指定

---

## 数値型の定義

| 属性 | 型 | 値域 |
|------|-----|------|
| `x`, `y` | 整数（integer、符号付き i32） | 負値も許容（画面外からの登場演出等を想定） |
| `width`, `height` | 整数（integer） | > 0 |
| `opacity` | 整数（integer） | 0–100 |
| `volume` | 整数（integer） | 0–100 |
| `font_size` | 整数（integer、pt 単位） | > 0 |
| `start`, `duration`, `fade_in`, `fade_out`, `trim_start`, `trim_end` | 浮動小数（float64秒） | ≥ 0.0 |

FFmpegへの渡し時は小数点3桁に丸める（例: `3.000`）。YAML上でfloat64秒として記述。内部表現もf64秒。FFmpegへの引数化時は `format!("{:.3}", t)` で小数点3桁に固定。

---

## 空プロジェクト

シーン0件でも有効なプロジェクトとして保存・読み込み可能。書き出し実行時のみ「シーンが存在しません」エラーで弾く。

---

## スキーマバージョン

プロジェクトファイルの先頭に `version: 1` フィールドを配置。スキーマ変更時にインクリメント。アプリは未サポートバージョンを読み込むとエラー表示する。

### マイグレーションロジックの配置

将来的なマイグレーション用モジュール: `src-tauri/src/project/migration.rs`

```rust
pub fn migrate(raw: serde_yml::Value, version: u32) -> Result<Project, Box<dyn std::error::Error>> {
    match version {
        1 => Ok(serde_yml::from_value(raw)?),
        _ => Err(Box::new(UnsupportedVersionError(version))),
    }
}
```

初期バージョン（v1 のみ）ではマイグレーション処理は不要。

---

## 出力解像度変更時の座標の扱い

変更時に座標は自動スケールしない（ユーザーの明示的指定を優先）。GUIで解像度変更時に「オブジェクトの座標・サイズは変更されません」の一時通知を表示する。
