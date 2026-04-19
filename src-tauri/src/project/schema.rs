// schema.rs — Project データモデル（実装前の空スタブ）
// テストが Red であることを確認するためのスケルトン

use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub version: u32,
    pub output_folder: String,
    pub output: OutputSettings,
    pub scenes: Vec<Scene>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputSettings {
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
pub enum Codec {
    H264,
    H265,
    Vp9,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Mp4,
    Mov,
    Webm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub variable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub start: f64,
    pub opacity: u8,
    pub volume: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub variable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub start: f64,
    pub duration: f64,
    pub opacity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub variable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub start: f64,
    pub duration: f64,
    pub opacity: u8,
    pub font: String,
    pub font_size: u32,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(default = "default_align")]
    pub align: TextAlign,
}

fn default_align() -> TextAlign {
    TextAlign::Left
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioObject {
    pub id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub variable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub start: f64,
    pub duration: f64,
    pub volume: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out: Option<f64>,
    #[serde(rename = "loop", default = "default_loop_mode")]
    pub r#loop: LoopMode,
}

fn default_loop_mode() -> LoopMode {
    LoopMode::Loop
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode {
    #[default]
    Loop,
    Silence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub name: String,
    pub variables: IndexMap<String, VariableValue>,
}

/// VariableValue: serde の untagged で Media と Text を区別する。
/// JSON/YAML 上の形式:
///   Media: { "file": "...", "trim_start"?: ..., "trim_end"?: ... }
///   Text:  { "text": "..." }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum VariableValue {
    // Text を先に書く: serde の untagged は上から順に試すため、
    // Media が先だと { text: "..." } が Media にマッチしてしまう可能性がある。
    // Text は "text" フィールドが必須なので、先に試して一致しなければ Media へ fallback する。
    Text { text: String },
    Media {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trim_start: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trim_end: Option<f64>,
    },
}

fn is_false(b: &bool) -> bool {
    !b
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
