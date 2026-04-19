//! project モジュールのユニットテスト
//! TDD: テストを先に書き、すべて green になるまで実装を進める

#[cfg(test)]
mod tests {
    use crate::project::*;

    // ---------------------------------------------------------------
    // サイクル 1-1: 最小 YAML パース
    // ---------------------------------------------------------------

    /// 最小構成の YAML が Project にデシリアライズできる
    #[test]
    fn test_minimal_yaml_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp/out
output:
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
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        assert_eq!(project.version, 1);
        assert_eq!(project.output_folder, "/tmp/out");
        assert_eq!(project.output.width, 1920);
        assert_eq!(project.output.height, 1080);
        assert_eq!(project.output.fps, 30);
        assert!(matches!(project.output.codec, Codec::H264));
        assert!(matches!(project.output.format, Format::Mp4));
        assert_eq!(project.output.crf, 23);
        assert_eq!(project.output.preset, "medium");
        assert!(project.scenes.is_empty());
        assert!(project.entries.is_empty());
    }

    /// 不正な YAML はエラーを返す
    #[test]
    fn test_invalid_yaml_returns_error() {
        let yaml = "not: valid: yaml: [[[";
        let result: Result<Project, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
    }

    /// codec フィールドが h265 / vp9 もパースできる
    #[test]
    fn test_codec_variants() {
        let h265_yaml = r#"
version: 1
output_folder: /tmp
output:
  width: 1920
  height: 1080
  fps: 30
  codec: h265
  format: mp4
  crf: 28
  preset: medium
scenes: []
entries: []
"#;
        let p: Project = serde_yml::from_str(h265_yaml).expect("should parse");
        assert!(matches!(p.output.codec, Codec::H265));

        let vp9_yaml = r#"
version: 1
output_folder: /tmp
output:
  width: 1920
  height: 1080
  fps: 30
  codec: vp9
  format: webm
  crf: 33
  preset: medium
scenes: []
entries: []
"#;
        let p: Project = serde_yml::from_str(vp9_yaml).expect("should parse");
        assert!(matches!(p.output.codec, Codec::Vp9));
        assert!(matches!(p.output.format, Format::Webm));
    }

    // ---------------------------------------------------------------
    // サイクル 1-2: SceneObject 型パース
    // ---------------------------------------------------------------

    /// video オブジェクトが VideoObject にパースされる
    #[test]
    fn test_video_object_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
entries: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        let obj = &project.scenes[0].objects[0];
        match obj {
            SceneObject::Video(v) => {
                assert_eq!(v.id, "v1");
                assert_eq!(v.file.as_deref(), Some("/path/to/video.mp4"));
                assert_eq!(v.x, 0);
                assert_eq!(v.opacity, 100);
                assert_eq!(v.volume, 80);
            }
            _ => panic!("expected VideoObject"),
        }
    }

    /// image オブジェクトが ImageObject にパースされる
    #[test]
    fn test_image_object_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
entries: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Image(img) => {
                assert_eq!(img.id, "img1");
                assert_eq!(img.x, 100);
                assert_eq!(img.duration, 5.0);
                assert_eq!(img.opacity, 80);
            }
            _ => panic!("expected ImageObject"),
        }
    }

    /// text オブジェクトが TextObject にパースされる（デフォルト値含む）
    #[test]
    fn test_text_object_parse() {
        let yaml = r##"
version: 1
output_folder: /tmp
output:
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
entries: []
"##;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Text(txt) => {
                assert_eq!(txt.id, "txt1");
                assert_eq!(txt.text.as_deref(), Some("Hello World"));
                assert_eq!(txt.font, "NotoSansCJK-Bold");
                assert_eq!(txt.font_size, 48);
                assert_eq!(txt.color, "#ffffff");
                assert_eq!(txt.background_color.as_deref(), Some("#00000088"));
                // align のデフォルトは left
                assert!(matches!(txt.align, TextAlign::Left));
            }
            _ => panic!("expected TextObject"),
        }
    }


    /// audio オブジェクトが AudioObject にパースされる（loop のデフォルト値含む）
    #[test]
    fn test_audio_object_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
      - id: bgm1
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0
        volume: 30
        fade_in: 1.0
        fade_out: 2.0
entries: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Audio(a) => {
                assert_eq!(a.id, "bgm1");
                assert_eq!(a.volume, 30);
                assert_eq!(a.fade_in, Some(1.0));
                assert_eq!(a.fade_out, Some(2.0));
                // loop のデフォルトは Loop
                assert!(matches!(a.r#loop, LoopMode::Loop));
            }
            _ => panic!("expected AudioObject"),
        }
    }

    /// audio オブジェクトで loop: silence が指定できる
    #[test]
    fn test_audio_loop_silence() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
      - id: a1
        type: audio
        file: /path/to/sfx.mp3
        start: 0.0
        duration: 10.0
        volume: 100
        loop: silence
entries: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Audio(a) => {
                assert!(matches!(a.r#loop, LoopMode::Silence));
            }
            _ => panic!("expected AudioObject"),
        }
    }

    /// variable: true の映像オブジェクトが file なしでもパースされる
    #[test]
    fn test_variable_video_object_no_file() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
      - id: main_video
        type: video
        variable: true
        x: 0
        y: 0
        width: 1920
        height: 1080
        start: 0.0
        opacity: 100
        volume: 80
entries: []
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        match &project.scenes[0].objects[0] {
            SceneObject::Video(v) => {
                assert!(v.variable);
                assert!(v.file.is_none());
            }
            _ => panic!("expected VideoObject"),
        }
    }

    // ---------------------------------------------------------------
    // サイクル 1-3: Entry・VariableValue パース
    // ---------------------------------------------------------------

    /// Entry の variables が VariableValue の各バリアントに正しくパースされる
    #[test]
    fn test_entry_variables_parse() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
entries:
  - name: tanaka
    variables:
      main_video:
        file: /path/to/tanaka.mp4
        trim_start: 3.0
        trim_end: 2.0
      photo:
        file: /path/to/photo.jpg
      caption:
        text: "田中 太郎"
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        assert_eq!(project.entries.len(), 1);
        let entry = &project.entries[0];
        assert_eq!(entry.name, "tanaka");

        // Media バリアント（trim あり）
        match entry.variables.get("main_video") {
            Some(VariableValue::Media { file, trim_start, trim_end }) => {
                assert_eq!(file, "/path/to/tanaka.mp4");
                assert_eq!(*trim_start, Some(3.0));
                assert_eq!(*trim_end, Some(2.0));
            }
            _ => panic!("expected Media variable for main_video"),
        }

        // Media バリアント（trim なし）
        match entry.variables.get("photo") {
            Some(VariableValue::Media { file, trim_start, trim_end }) => {
                assert_eq!(file, "/path/to/photo.jpg");
                assert_eq!(*trim_start, None);
                assert_eq!(*trim_end, None);
            }
            _ => panic!("expected Media variable for photo"),
        }

        // Text バリアント
        match entry.variables.get("caption") {
            Some(VariableValue::Text { text }) => {
                assert_eq!(text, "田中 太郎");
            }
            _ => panic!("expected Text variable for caption"),
        }
    }

    /// Entry の variables の挿入順序が保持される（IndexMap）
    #[test]
    fn test_entry_variables_order_preserved() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
  width: 1920
  height: 1080
  fps: 30
  codec: h264
  format: mp4
  crf: 23
  preset: medium
scenes: []
entries:
  - name: test
    variables:
      c_key:
        text: "first"
      a_key:
        text: "second"
      b_key:
        text: "third"
"#;
        let project: Project = serde_yml::from_str(yaml).expect("should parse");
        let keys: Vec<&str> = project.entries[0].variables.keys().map(|s| s.as_str()).collect();
        // 挿入順（BTreeMap のようにソートされてはならない）
        assert_eq!(keys, vec!["c_key", "a_key", "b_key"]);
    }

    // ---------------------------------------------------------------
    // サイクル 1-4: シリアライズ round-trip
    // ---------------------------------------------------------------

    /// Project をシリアライズして再デシリアライズすると元と一致する
    #[test]
    fn test_round_trip_serialization() {
        let yaml = r##"
version: 1
output_folder: /tmp/out
output:
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
      - id: txt1
        type: text
        text: Hello
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
      - id: bgm
        type: audio
        file: /path/to/bgm.mp3
        start: 0.0
        duration: 0.0
        volume: 30
entries:
  - name: tanaka
    variables:
      v1:
        file: /path/to/tanaka.mp4
        trim_start: 3.0
      txt1:
        text: "田中 太郎"
"##;
        let original: Project = serde_yml::from_str(yaml).expect("parse ok");
        // YAML にシリアライズして再パース
        let serialized = serde_yml::to_string(&original).expect("serialize ok");
        let restored: Project = serde_yml::from_str(&serialized).expect("re-parse ok");
        assert_eq!(original, restored);
    }

    /// insta スナップショットで YAML シリアライズ結果を固定する
    #[test]
    fn test_round_trip_snapshot() {
        let project = Project {
            version: 1,
            output_folder: "/tmp/out".to_string(),
            output: OutputSettings {
                width: 1920,
                height: 1080,
                fps: 30,
                codec: Codec::H264,
                format: Format::Mp4,
                crf: 23,
                preset: "medium".to_string(),
            },
            scenes: vec![Scene {
                id: "main".to_string(),
                duration: None,
                objects: vec![SceneObject::Audio(AudioObject {
                    id: "bgm".to_string(),
                    variable: false,
                    file: Some("/path/bgm.mp3".to_string()),
                    start: 0.0,
                    duration: 0.0,
                    volume: 30,
                    fade_in: Some(1.0),
                    fade_out: None,
                    r#loop: LoopMode::Loop,
                })],
            }],
            entries: vec![],
        };
        let yaml = serde_yml::to_string(&project).expect("serialize ok");
        insta::assert_snapshot!(yaml);
    }

    // ---------------------------------------------------------------
    // サイクル 1-5: スキーマバージョンチェック
    // ---------------------------------------------------------------

    /// version: 1 は正常に読み込める
    #[test]
    fn test_load_project_version_1_ok() {
        let yaml = r#"
version: 1
output_folder: /tmp
output:
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
        let result = crate::project::migration::load_project(yaml);
        assert!(result.is_ok());
    }

    /// version: 2 はエラーを返す
    #[test]
    fn test_load_project_unsupported_version_returns_error() {
        let yaml = r#"
version: 2
output_folder: /tmp
output:
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
        let result = crate::project::migration::load_project(yaml);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unsupported project version: 2"), "got: {msg}");
    }

    /// version フィールドが欠落しているとエラー
    #[test]
    fn test_load_project_missing_version_returns_error() {
        let yaml = r#"
output_folder: /tmp
output:
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
        // version が必須フィールドなので serde_yml がエラーを返す
        let result = crate::project::migration::load_project(yaml);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // サイクル 1-6: バリデーション（プロジェクト・出力設定レベル）
    // ---------------------------------------------------------------

    fn base_project() -> Project {
        Project {
            version: 1,
            output_folder: "/tmp/out".to_string(),
            output: OutputSettings {
                width: 1920, height: 1080, fps: 30,
                codec: Codec::H264, format: Format::Mp4,
                crf: 23, preset: "medium".to_string(),
            },
            scenes: vec![],
            entries: vec![],
        }
    }

    /// 有効なプロジェクトはエラーなし
    #[test]
    fn test_valid_project_no_errors() {
        let p = base_project();
        let result = crate::project::validation::validate_project(&p);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    /// output_folder が空文字列のときエラー
    #[test]
    fn test_validation_empty_output_folder() {
        let mut p = base_project();
        p.output_folder = "".to_string();
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "output_folder_invalid"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// h264 + webm はコーデック・フォーマット不一致エラー
    #[test]
    fn test_validation_codec_format_mismatch() {
        let mut p = base_project();
        p.output.codec = Codec::H264;
        p.output.format = Format::Webm;
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "codec_format_mismatch"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// vp9 + mp4 はコーデック・フォーマット不一致エラー
    #[test]
    fn test_validation_vp9_mp4_mismatch() {
        let mut p = base_project();
        p.output.codec = Codec::Vp9;
        p.output.format = Format::Mp4;
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "codec_format_mismatch"));
    }

    /// vp9 + webm は有効
    #[test]
    fn test_validation_vp9_webm_ok() {
        let mut p = base_project();
        p.output.codec = Codec::Vp9;
        p.output.format = Format::Webm;
        let result = crate::project::validation::validate_project(&p);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    /// crf が範囲外（h264 で 52）はエラー
    #[test]
    fn test_validation_crf_out_of_range_h264() {
        let mut p = base_project();
        p.output.crf = 52; // h264 は 0-51
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "crf_out_of_range"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// crf が範囲内（h264 で 51）はエラーなし
    #[test]
    fn test_validation_crf_boundary_ok() {
        let mut p = base_project();
        p.output.crf = 51;
        let result = crate::project::validation::validate_project(&p);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    // ---------------------------------------------------------------
    // サイクル 1-7: バリデーション（シーン・オブジェクトレベル）
    // ---------------------------------------------------------------

    fn make_video_obj(id: &str, file: Option<&str>, variable: bool) -> SceneObject {
        SceneObject::Video(VideoObject {
            id: id.to_string(), variable,
            file: file.map(|f| f.to_string()),
            x: 0, y: 0, width: 1920, height: 1080,
            start: 0.0, opacity: 100, volume: 80,
        })
    }

    /// 同一シーン内でオブジェクト ID が重複するとエラー
    #[test]
    fn test_validation_duplicate_object_id() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![
                make_video_obj("v1", Some("/a.mp4"), false),
                make_video_obj("v1", Some("/b.mp4"), false), // 重複
            ],
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "object_id_duplicate"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// variable:false かつ file 未指定はエラー
    #[test]
    fn test_validation_fixed_object_missing_file() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", None, false)],
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "object_field_missing"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// variable:true かつ file 未指定はエラーなし（エントリで補完するため）
    #[test]
    fn test_validation_variable_object_no_file_ok() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", None, true)],
        });
        // entries にもエラーが出るが、object_field_missing は出ないことを確認
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.errors.iter().any(|e| e.code == "object_field_missing"),
            "unexpected object_field_missing");
    }

    /// ホワイトリスト外のフォントはエラー
    #[test]
    fn test_validation_font_not_whitelisted() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![SceneObject::Text(TextObject {
                id: "t1".to_string(), variable: false,
                text: Some("hello".to_string()),
                x: 0, y: 0, width: 400, height: 60,
                start: 0.0, duration: 3.0, opacity: 100,
                font: "Arial".to_string(), // ホワイトリスト外
                font_size: 24,
                color: "#ffffff".to_string(),
                background_color: None,
                align: TextAlign::Left,
            })],
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "font_not_whitelisted"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// 許可フォントはエラーなし
    #[test]
    fn test_validation_font_whitelisted_ok() {
        let mut p = base_project();
        for font in &["NotoSansCJK-Regular", "NotoSansCJK-Bold"] {
            p.scenes.push(Scene {
                id: font.to_string(), duration: None,
                objects: vec![SceneObject::Text(TextObject {
                    id: "t1".to_string(), variable: false,
                    text: Some("hello".to_string()),
                    x: 0, y: 0, width: 400, height: 60,
                    start: 0.0, duration: 3.0, opacity: 100,
                    font: font.to_string(),
                    font_size: 24,
                    color: "#ffffff".to_string(),
                    background_color: None,
                    align: TextAlign::Left,
                })],
            });
        }
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.errors.iter().any(|e| e.code == "font_not_whitelisted"),
            "unexpected font_not_whitelisted");
    }

    // ---------------------------------------------------------------
    // サイクル 1-8: バリデーション（エントリレベル）
    // ---------------------------------------------------------------

    /// variable:true オブジェクトに対応する変数が entries にないとエラー
    #[test]
    fn test_validation_variable_missing_in_entry() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![make_video_obj("v1", None, true)],
        });
        p.entries.push(Entry {
            name: "tanaka".to_string(),
            variables: indexmap::IndexMap::new(), // v1 がない
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "variable_missing"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// エントリ名に禁止文字（/）があるとエラー
    #[test]
    fn test_validation_entry_name_invalid_chars() {
        let mut p = base_project();
        p.entries.push(Entry {
            name: "foo/bar".to_string(),
            variables: indexmap::IndexMap::new(),
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "entry_name_invalid"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    /// エントリ名が空文字列はエラー
    #[test]
    fn test_validation_entry_name_empty() {
        let mut p = base_project();
        p.entries.push(Entry {
            name: "".to_string(),
            variables: indexmap::IndexMap::new(),
        });
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "entry_name_invalid"));
    }

    /// エントリ名の重複はエラー
    #[test]
    fn test_validation_entry_name_duplicate() {
        let mut p = base_project();
        for _ in 0..2 {
            p.entries.push(Entry {
                name: "tanaka".to_string(),
                variables: indexmap::IndexMap::new(),
            });
        }
        let result = crate::project::validation::validate_project(&p);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "entry_name_duplicate"),
            "codes: {:?}", result.errors.iter().map(|e| &e.code).collect::<Vec<_>>());
    }

    // ---------------------------------------------------------------
    // サイクル 1-9: バリデーション（警告）
    // ---------------------------------------------------------------

    /// オブジェクトがキャンバス外（x < 0 で width を超える）は警告
    #[test]
    fn test_validation_object_out_of_bounds_warning() {
        let mut p = base_project();
        p.scenes.push(Scene {
            id: "s1".to_string(), duration: None,
            objects: vec![SceneObject::Video(VideoObject {
                id: "v1".to_string(), variable: false,
                file: Some("/a.mp4".to_string()),
                x: 2000, // 1920 を超える
                y: 0, width: 1920, height: 1080,
                start: 0.0, opacity: 100, volume: 80,
            })],
        });
        let result = crate::project::validation::validate_project(&p);
        // エラーではなく警告
        assert!(result.is_valid(), "should be valid (no errors)");
        assert!(result.warnings.iter().any(|w| w.code == "object_out_of_bounds"),
            "warnings: {:?}", result.warnings.iter().map(|w| &w.code).collect::<Vec<_>>());
    }



}
