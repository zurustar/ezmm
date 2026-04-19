// probe.rs — ffprobe JSON 出力のパース

use serde::Deserialize;

/// ffprobe の出力から取り出したメタデータ
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeResult {
    /// ファイル長（秒）
    pub duration: f64,
    /// 映像ストリームの幅（映像なし時は None）
    pub width: Option<u32>,
    /// 映像ストリームの高さ（映像なし時は None）
    pub height: Option<u32>,
    /// フレームレート（映像なし・不明時は None）
    pub fps: Option<f64>,
    /// 音声トラックの有無
    pub has_audio: bool,
    /// 音声サンプルレート（音声なし時は None）
    pub sample_rate: Option<u32>,
}

// ---- serde 用の内部構造体 ----

#[derive(Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    format: FfprobeFormat,
    #[serde(default)]
    streams: Vec<FfprobeStream>,
}

#[derive(Deserialize, Default)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    sample_rate: Option<String>,
    duration: Option<String>,
}

// ---- 公開 API ----

/// ffprobe が出力した JSON 文字列を `ProbeResult` にパースする。
///
/// # Errors
/// JSON パース失敗時や duration が取得できない場合は `Err` を返す。
pub fn parse_ffprobe_output(json: &str) -> Result<ProbeResult, String> {
    let raw: FfprobeOutput =
        serde_json::from_str(json).map_err(|e| format!("ffprobe JSON parse error: {e}"))?;

    // duration 優先順位: format.duration → video stream → audio stream
    let duration = parse_duration_priority(&raw)
        .ok_or_else(|| "ffprobe: duration が取得できませんでした".to_string())?;

    let video_stream = raw.streams.iter().find(|s| {
        s.codec_type.as_deref() == Some("video")
    });
    let audio_stream = raw.streams.iter().find(|s| {
        s.codec_type.as_deref() == Some("audio")
    });

    let width = video_stream.and_then(|s| s.width);
    let height = video_stream.and_then(|s| s.height);
    let fps = video_stream.and_then(|s| parse_r_frame_rate(s.r_frame_rate.as_deref()));
    let has_audio = audio_stream.is_some();
    let sample_rate = audio_stream.and_then(|s| {
        s.sample_rate.as_deref().and_then(|r| r.parse::<u32>().ok())
    });

    Ok(ProbeResult {
        duration,
        width,
        height,
        fps,
        has_audio,
        sample_rate,
    })
}

/// "30000/1001" 形式の r_frame_rate 文字列を f64 に変換する。
/// "0/0" や parse 失敗時は None を返す。
fn parse_r_frame_rate(s: Option<&str>) -> Option<f64> {
    let s = s?;
    let mut parts = s.splitn(2, '/');
    let num: f64 = parts.next()?.parse().ok()?;
    let den: f64 = parts.next()?.parse().ok()?;
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

/// duration を優先順位に従って取得する。
fn parse_duration_priority(raw: &FfprobeOutput) -> Option<f64> {
    // 1. format.duration
    if let Some(d) = raw.format.duration.as_deref().and_then(|s| s.parse::<f64>().ok()) {
        if d > 0.0 {
            return Some(d);
        }
    }
    // 2. video stream duration
    if let Some(d) = raw.streams.iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .and_then(|s| s.duration.as_deref())
        .and_then(|s| s.parse::<f64>().ok())
    {
        if d > 0.0 {
            return Some(d);
        }
    }
    // 3. audio stream duration
    if let Some(d) = raw.streams.iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"))
        .and_then(|s| s.duration.as_deref())
        .and_then(|s| s.parse::<f64>().ok())
    {
        if d > 0.0 {
            return Some(d);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 映像＋音声を含む典型的な ffprobe 出力
    const TYPICAL_JSON: &str = r#"{
        "streams": [
            {
                "codec_type": "video",
                "width": 1920,
                "height": 1080,
                "r_frame_rate": "30000/1001",
                "duration": "10.010000"
            },
            {
                "codec_type": "audio",
                "sample_rate": "44100",
                "duration": "10.000000"
            }
        ],
        "format": {
            "duration": "10.010000"
        }
    }"#;

    #[test]
    fn parses_typical_video_with_audio() {
        let result = parse_ffprobe_output(TYPICAL_JSON).unwrap();
        assert_eq!(result.duration, 10.01);
        assert_eq!(result.width, Some(1920));
        assert_eq!(result.height, Some(1080));
        assert!(result.has_audio);
        assert_eq!(result.sample_rate, Some(44100));
        // fps: 30000/1001 ≈ 29.97...
        let fps = result.fps.unwrap();
        assert!((fps - 29.97).abs() < 0.01, "fps={fps}");
    }

    #[test]
    fn parses_integer_fps() {
        let json = r#"{
            "streams": [
                {
                    "codec_type": "video",
                    "width": 1280,
                    "height": 720,
                    "r_frame_rate": "30/1"
                }
            ],
            "format": { "duration": "5.0" }
        }"#;
        let result = parse_ffprobe_output(json).unwrap();
        assert_eq!(result.fps, Some(30.0));
        assert!(!result.has_audio);
        assert_eq!(result.sample_rate, None);
    }

    #[test]
    fn zero_over_zero_fps_returns_none() {
        let json = r#"{
            "streams": [
                {
                    "codec_type": "video",
                    "width": 640,
                    "height": 480,
                    "r_frame_rate": "0/0"
                }
            ],
            "format": { "duration": "3.0" }
        }"#;
        let result = parse_ffprobe_output(json).unwrap();
        assert_eq!(result.fps, None);
    }

    #[test]
    fn audio_only_has_no_video_fields() {
        let json = r#"{
            "streams": [
                {
                    "codec_type": "audio",
                    "sample_rate": "48000",
                    "duration": "60.0"
                }
            ],
            "format": { "duration": "60.0" }
        }"#;
        let result = parse_ffprobe_output(json).unwrap();
        assert_eq!(result.duration, 60.0);
        assert_eq!(result.width, None);
        assert_eq!(result.height, None);
        assert_eq!(result.fps, None);
        assert!(result.has_audio);
        assert_eq!(result.sample_rate, Some(48000));
    }

    #[test]
    fn duration_fallback_to_stream() {
        // format.duration がない場合、video stream.duration を使う
        let json = r#"{
            "streams": [
                {
                    "codec_type": "video",
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "25/1",
                    "duration": "7.5"
                }
            ],
            "format": {}
        }"#;
        let result = parse_ffprobe_output(json).unwrap();
        assert_eq!(result.duration, 7.5);
    }

    #[test]
    fn returns_error_for_invalid_json() {
        let result = parse_ffprobe_output("not json");
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_when_duration_missing() {
        let json = r#"{ "streams": [], "format": {} }"#;
        let result = parse_ffprobe_output(json);
        assert!(result.is_err());
    }
}
