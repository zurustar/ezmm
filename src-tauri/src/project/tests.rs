//! project モジュールのユニットテスト

#[cfg(test)]
mod tests {
    use crate::project::*;

    // ---------------------------------------------------------------
    // サイクル 1-1: 最小 YAML パース
    // ---------------------------------------------------------------

    #[test]
    fn test_minimal_yaml_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp/out
output:
  output_name: my_video
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        assert_eq!(project.version, 1);
        assert_eq!(project.output_folder, "/tmp/out");
        assert_eq!(project.output.output_name, "my_video");
        assert_eq!(project.output.width, 1920);
        assert!(matches!(project.output.codec, Codec::H264));
        assert!(matches!(project.output.format, Format::Mp4));
        assert!(project.scenes.is_empty());
    }

    /// 旧 YAML（entries フィールドあり）は entries を無視して読み込める
    #[test]
    fn test_legacy_yaml_with_entries_parses_ok() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
entries: []
"#;
        let result: Result<Project, _> = serde_yml::from_str(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_yaml_returns_error() {
        let yaml = "not: valid: yaml: [[[";
        let result: Result<Project, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_codec_variants() {
        let yaml_base = |codec: &str, format: &str| format!(r#"
version: 1
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: {codec}
  format: {format}
  crf: 28
  preset: medium
scenes: []
"#);
        let p: Project = serde_yml::from_str(&yaml_base("h265", "mp4")).unwrap();
        assert!(matches!(p.output.codec, Codec::H265));
        let p: Project = serde_yml::from_str(&yaml_base("vp9", "webm")).unwrap();
        assert!(matches!(p.output.codec, Codec::Vp9));
        assert!(matches!(p.output.format, Format::Webm));
    }

    // ---------------------------------------------------------------
    // サイクル 1-2: SceneObject 型パース
    // ---------------------------------------------------------------

    fn minimal_project_yaml(objects_yaml: &str) -> String {
        format!(r#"
version: 1
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes:
  - id: s1
    objects:
{objects_yaml}
"#)
    }

    #[test]
    fn test_video_object_parse() {
        let yaml = minimal_project_yaml(r#"
      - id: v1
        type: video
        file: /path/to/video.mp4
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 80
"#);
        let project: Project = serde_yml::from_str(&yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Video(v) => {
                assert_eq!(v.id, "v1");
                assert_eq!(v.file.as_deref(), Some("/path/to/video.mp4"));
                assert_eq!(v.opacity, 100);
                assert_eq!(v.volume, 80);
            }
            _ => panic!("expected VideoObject"),
        }
    }

    #[test]
    fn test_image_object_parse() {
        let yaml = minimal_project_yaml(r#"
      - id: img1
        type: image
        file: /path/to/logo.png
        x: 100
        y: 200
        width: 160
        height: 80
        start: 1.0
        duration: 5.0
        opacity: 80
"#);
        let project: Project = serde_yml::from_str(&yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Image(img) => {
                assert_eq!(img.id, "img1");
                assert_eq!(img.duration, 5.0);
                assert_eq!(img.opacity, 80);
            }
            _ => panic!("expected ImageObject"),
        }
    }

    #[test]
    fn test_text_object_parse() {
        let yaml = minimal_project_yaml(r##"
      - id: txt1
        type: text
        text: "Hello World"
        x: 100
        y: 900
        width: 800
        height: 80
        start: 2.0
        duration: 10.0
        opacity: 100
        font: NotoSansCJK-Bold
        font_size: 48
        color: "#ffffff"
        background_color: "#00000088"
"##);
        let project: Project = serde_yml::from_str(&yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Text(txt) => {
                assert_eq!(txt.text.as_deref(), Some("Hello World"));
                assert_eq!(txt.font_size, 48);
                assert!(matches!(txt.align, TextAlign::Left));
            }
            _ => panic!("expected TextObject"),
        }
    }

    #[test]
    fn test_audio_object_parse() {
        let yaml = minimal_project_yaml(r#"
      - id: bgm1
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0
        volume: 30
        fade_in: 1.0
        fade_out: 2.0
"#);
        let project: Project = serde_yml::from_str(&yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Audio(a) => {
                assert_eq!(a.volume, 30);
                assert_eq!(a.fade_in, Some(1.0));
                assert!(matches!(a.r#loop, LoopMode::Loop));
            }
            _ => panic!("expected AudioObject"),
        }
    }

    #[test]
    fn test_audio_loop_silence() {
        let yaml = minimal_project_yaml(r#"
      - id: a1
        type: audio
        file: /path/to/sfx.mp3
        start: 0.0
        duration: 10.0
        volume: 100
        loop: silence
"#);
        let project: Project = serde_yml::from_str(&yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Audio(a) => {
                assert!(matches!(a.r#loop, LoopMode::Silence));
            }
            _ => panic!("expected AudioObject"),
        }
    }

    // ---------------------------------------------------------------
    // サイクル 1-4: シリアライズ round-trip
    // ---------------------------------------------------------------

    #[test]
    fn test_round_trip_serialization() {
        let yaml = r##"
version: 1
output_folder: /tmp/out
output:
  output_name: my_video
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes:
  - id: main
    objects:
      - id: v1
        type: video
        file: /path/to/video.mp4
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 80
      - id: bgm
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0
        volume: 30
"##;
        let original: Project = serde_yml::from_str(yaml).expect("parse ok");
        let serialized = serde_yml::to_string(&original).expect("serialize ok");
        let restored: Project = serde_yml::from_str(&serialized).expect("re-parse ok");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_round_trip_snapshot() {
        let project = Project {
            version: 1,
            output_folder: "/tmp/out".to_string(),
            output: OutputSettings {
                output_name: "my_video".to_string(),
                width: 1920, height: 1080, fps: 30,
                codec: Codec::H264, format: Format::Mp4,
                crf: 23, preset: "medium".to_string(),
            },
            scenes: vec![Scene {
                id: "main".to_string(),
                duration: None,
                objects: vec![SceneObject::Audio(AudioObject {
                    id: "bgm".to_string(),
                    file: Some("/path/bgm.mp3".to_string()),
                    start: 0.0, duration: 0.0, volume: 30,
                    fade_in: Some(1.0), fade_out: None,
                    r#loop: LoopMode::Loop,
                    ..Default::default()
                })],
                ..Default::default()
            }],
        };
        let yaml = serde_yml::to_string(&project).expect("serialize ok");
        insta::assert_snapshot!(yaml);
    }

    // ---------------------------------------------------------------
    // サイクル 1-5: スキーマバージョンチェック
    // ---------------------------------------------------------------

    #[test]
    fn test_load_project_version_1_ok() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
"#;
        assert!(crate::project::migration::load_project(yaml).is_ok());
    }

    #[test]
    fn test_load_project_unsupported_version_returns_error() {
        let yaml = r#"
version: 2
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
"#;
        let result = crate::project::migration::load_project(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported project version: 2"));
    }

    #[test]
    fn test_load_project_missing_version_returns_error() {
        let yaml = r#"
output_folder: /tmp
output:
  output_name: out
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
"#;
        assert!(crate::project::migration::load_project(yaml).is_err());
    }

    // ---------------------------------------------------------------
    // サイクル 1-6: バリデーション（プロジェクト・出力設定レベル）
    // ---------------------------------------------------------------

    fn base_project() -> Project {
        Project {
            version: 1,
            output_folder: "/tmp/out".to_string(),
            output: OutputSettings {
                output_name: "my_video".to_string(),
                width: 1920, height: 1080, fps: 30,
                codec: Codec::H264, format: Format::Mp4,
                crf: 23, preset: "medium".to_string(),
            },
            scenes: vec![],
        }
    }

    #[test]
    fn test_valid_project_no_errors() {
        let p = base_project();
        let result = crate::project::validation::validate_project(&p);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validation_empty_output_folder() {
        let mut p = base_project();
        p.output_folder = "".to_string();
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "output_folder_invalid"));
    }

    #[test]
    fn test_validation_empty_output_name() {
        let mut p = base_project();
        p.output.output_name = "".to_string();
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "output_name_invalid"));
    }

    #[test]
    fn test_validation_codec_format_mismatch() {
        let mut p = base_project();
        p.output.codec = Codec::H264;
        p.output.format = Format::Webm;
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "codec_format_mismatch"));
    }

    #[test]
    fn test_validation_vp9_webm_ok() {
        let mut p = base_project();
        p.output.codec = Codec::Vp9;
        p.output.format = Format::Webm;
        assert!(crate::project::validation::validate_project(&p).is_valid());
    }

    #[test]
    fn test_validation_crf_out_of_range_h264() {
        let mut p = base_project();
        p.output.crf = 52;
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "crf_out_of_range"));
    }

    // ---------------------------------------------------------------
    // サイクル 1-7: バリデーション（シーン・オブジェクトレベル）
    // ---------------------------------------------------------------

    fn make_video_obj(id: &str, file: Option<&str>) -> SceneObject {
        SceneObject::Video(VideoObject {
            id: id.to_string(),
            file: file.map(|f| f.to_string()),
            x: 0, y: 0, width: 1920, height: 1080,
            start: 0.0, opacity: 100, volume: 80,
            ..Default::default()
        })
    }

    /// テスト用: 必ず存在するファイルパス（/dev/null はmacOS/Linux で常に存在）
    fn existing_file() -> &'static str { "/dev/null" }

    #[test]
    fn test_validation_duplicate_object_id() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![
                make_video_obj("v1", Some(existing_file())),
                make_video_obj("v1", Some(existing_file())),
            ],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "object_id_duplicate"));
    }

    #[test]
    fn test_validation_fixed_object_missing_file() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", None)],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "object_field_missing"));
    }

    #[test]
    fn test_validation_font_not_whitelisted() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![SceneObject::Text(TextObject {
                id: "t1".to_string(),
                text: Some("hello".to_string()),
                x: 0, y: 0, width: 400, height: 60,
                start: 0.0, duration: 3.0, opacity: 100,
                font: "Arial".to_string(),
                font_size: 24,
                color: "#ffffff".to_string(),
                background_color: None,
                align: TextAlign::Left,
                ..Default::default()
            })],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "font_not_whitelisted"));
    }

    // ---------------------------------------------------------------
    // P2-A: scene_no_duration / file_not_found バリデーション
    // ---------------------------------------------------------------

    #[test]
    fn test_validation_scene_no_duration_no_objects() {
        let mut p = base_project();
        p.scenes.push(Scene { id: "s1".to_string(), duration: None, objects: vec![], ..Default::default() });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "scene_no_duration"),
            "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validation_scene_no_duration_audio_loop_only() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![SceneObject::Audio(AudioObject {
                id: "a1".to_string(), file: Some(existing_file().to_string()),
                start: 0.0, duration: 0.0, volume: 100,
                fade_in: None, fade_out: None, r#loop: LoopMode::Loop,
                ..Default::default()
            })],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "scene_no_duration"),
            "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validation_scene_with_video_file_no_duration_ok() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", Some(existing_file()))],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.errors.iter().any(|e| e.code == "scene_no_duration"),
            "should not have scene_no_duration when video file is set");
    }

    #[test]
    fn test_validation_scene_with_explicit_duration_ok() {
        let mut p = base_project();
        p.scenes.push(Scene { id: "s1".to_string(), duration: Some(5.0), objects: vec![], ..Default::default() });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.errors.iter().any(|e| e.code == "scene_no_duration"),
            "should not have scene_no_duration when scene.duration is set");
    }

    #[test]
    fn test_validation_file_not_found() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", Some("/definitely/nonexistent/video_xyz.mp4"))],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.errors.iter().any(|e| e.code == "file_not_found"),
            "errors: {:?}", result.errors);
    }

    // ---------------------------------------------------------------
    // サイクル 1-9: バリデーション（警告）
    // ---------------------------------------------------------------

    #[test]
    fn test_validation_object_out_of_bounds_warning() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![SceneObject::Video(VideoObject {
                id: "v1".to_string(),
                file: Some(existing_file().to_string()),
                x: 2000, y: 0, width: 1920, height: 1080,
                start: 0.0, opacity: 100, volume: 80,
                ..Default::default()
            })],
            ..Default::default()
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(result.is_valid(), "should be valid (no errors)");
        assert!(result.warnings.iter().any(|w| w.code == "object_out_of_bounds"));
    }
}
