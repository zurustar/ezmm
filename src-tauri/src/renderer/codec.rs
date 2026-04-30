// codec.rs — コーデック → FFmpeg 引数マッピング

use crate::project::{Codec, Format, OutputSettings};

/// FFmpegの最終エンコード引数を生成する。
/// `-pix_fmt`, `-c:v`, `-crf`, `-preset`, `-map`, `-c:a`, `-b:a`, `-ar`, `-ac` など。
///
/// `video_map` / `audio_map` はフィルタグラフの出力ラベル（例: `"[vout]"` / `"[aout]"`）。
pub fn build_codec_args(
    settings: &OutputSettings,
    video_map: &str,
    audio_map: &str,
    output_path: &str,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // pix_fmt は常に yuv420p（アルファ非対応コーデック向け）
    args.push("-pix_fmt".into());
    args.push("yuv420p".into());

    match &settings.codec {
        Codec::H264 => {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-crf".into(),
                settings.crf.to_string(),
                "-preset".into(),
                settings.preset.clone(),
            ]);
        }
        Codec::H265 => {
            args.extend([
                "-c:v".into(),
                "libx265".into(),
                "-crf".into(),
                settings.crf.to_string(),
                "-preset".into(),
                settings.preset.clone(),
                "-tag:v".into(),
                "hvc1".into(),
            ]);
        }
        Codec::Vp9 => {
            args.extend([
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-crf".into(),
                settings.crf.to_string(),
                "-b:v".into(),
                "0".into(),
                "-cpu-used".into(),
                "4".into(),
                "-row-mt".into(),
                "1".into(),
            ]);
        }
    }

    // -map (video)
    args.push("-map".into());
    args.push(video_map.to_string());

    // -map + audio codec (audio)
    args.push("-map".into());
    args.push(audio_map.to_string());

    match &settings.format {
        Format::Mp4 | Format::Mov => {
            args.extend([
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "192k".into(),
                "-ar".into(),
                "44100".into(),
                "-ac".into(),
                "2".into(),
            ]);
        }
        Format::Webm => {
            args.extend([
                "-c:a".into(),
                "libopus".into(),
                "-b:a".into(),
                "128k".into(),
                "-ar".into(),
                "48000".into(),
                "-ac".into(),
                "2".into(),
            ]);
        }
    }

    args.push(output_path.to_string());
    args
}

/// 出力サンプルレートをコンテナ形式から返す。
pub fn output_sample_rate(format: &Format) -> u32 {
    match format {
        Format::Mp4 | Format::Mov => 44100,
        Format::Webm => 48000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h264_settings() -> OutputSettings {
        OutputSettings {
            output_name: "output".into(),
            width: 1920,
            height: 1080,
            fps: 30,
            codec: Codec::H264,
            format: Format::Mp4,
            crf: 23,
            preset: "medium".into(),
        }
    }

    fn h265_settings() -> OutputSettings {
        OutputSettings {
            codec: Codec::H265,
            format: Format::Mp4,
            crf: 28,
            preset: "medium".into(),
            ..h264_settings()
        }
    }

    fn vp9_settings() -> OutputSettings {
        OutputSettings {
            codec: Codec::Vp9,
            format: Format::Webm,
            crf: 33,
            preset: "medium".into(),
            ..h264_settings()
        }
    }

    #[test]
    fn h264_args_snapshot() {
        let args = build_codec_args(&h264_settings(), "[vout]", "[aout]", "/out/test.mp4");
        insta::assert_yaml_snapshot!(args);
    }

    #[test]
    fn h265_args_snapshot() {
        let args = build_codec_args(&h265_settings(), "[vout]", "[aout]", "/out/test.mp4");
        insta::assert_yaml_snapshot!(args);
    }

    #[test]
    fn vp9_args_snapshot() {
        let args = build_codec_args(&vp9_settings(), "[vout]", "[aout]", "/out/test.webm");
        insta::assert_yaml_snapshot!(args);
    }

    #[test]
    fn h264_contains_libx264_and_aac() {
        let args = build_codec_args(&h264_settings(), "[vout]", "[aout]", "/out/test.mp4");
        assert!(args.iter().any(|a| a == "libx264"), "libx264 がない: {args:?}");
        assert!(args.iter().any(|a| a == "aac"), "aac がない: {args:?}");
        assert!(args.iter().any(|a| a == "44100"), "44100 がない: {args:?}");
    }

    #[test]
    fn h265_has_hvc1_tag() {
        let args = build_codec_args(&h265_settings(), "[vout]", "[aout]", "/out/test.mp4");
        assert!(args.iter().any(|a| a == "libx265"));
        assert!(args.iter().any(|a| a == "hvc1"));
    }

    #[test]
    fn vp9_uses_libvpx_and_opus() {
        let args = build_codec_args(&vp9_settings(), "[vout]", "[aout]", "/out/test.webm");
        assert!(args.iter().any(|a| a == "libvpx-vp9"));
        assert!(args.iter().any(|a| a == "libopus"));
        assert!(args.iter().any(|a| a == "48000"));
        // VP9 は -b:v 0
        let bv_idx = args.iter().position(|a| a == "-b:v").unwrap();
        assert_eq!(args[bv_idx + 1], "0");
    }
}
