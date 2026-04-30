use std::path::Path;
use crate::project::{Codec, Format, Project, SceneObject};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidationResult {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { errors: vec![], warnings: vec![] }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    fn push_error(&mut self, code: &str, message: &str,
                  scene_id: Option<&str>, object_id: Option<&str>) {
        self.errors.push(ValidationIssue {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.to_string(),
            scene_id: scene_id.map(|s| s.to_string()),
            object_id: object_id.map(|s| s.to_string()),
        });
    }

    fn push_warning(&mut self, code: &str, message: &str,
                    scene_id: Option<&str>, object_id: Option<&str>) {
        self.warnings.push(ValidationIssue {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.to_string(),
            scene_id: scene_id.map(|s| s.to_string()),
            object_id: object_id.map(|s| s.to_string()),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub scene_id: Option<String>,
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

const ALLOWED_FONTS: &[&str] = &["NotoSansCJK-Regular", "NotoSansCJK-Bold"];

pub fn validate_project(project: &Project) -> ValidationResult {
    let mut result = ValidationResult::new();
    validate_output_settings(project, &mut result);
    validate_scenes(project, &mut result);
    result
}

fn validate_output_settings(project: &Project, result: &mut ValidationResult) {
    if project.output_folder.trim().is_empty() {
        result.push_error("output_folder_invalid", "出力フォルダが設定されていません。", None, None);
    }

    if project.output.output_name.trim().is_empty() {
        result.push_error("output_name_invalid", "出力ファイル名が設定されていません。", None, None);
    }

    let combo_ok = matches!(
        (&project.output.codec, &project.output.format),
        (Codec::H264, Format::Mp4)
        | (Codec::H264, Format::Mov)
        | (Codec::H265, Format::Mp4)
        | (Codec::H265, Format::Mov)
        | (Codec::Vp9,  Format::Webm)
    );
    if !combo_ok {
        result.push_error("codec_format_mismatch", "コーデックとフォーマットの組み合わせが不正です。", None, None);
    }

    let crf_max = match project.output.codec {
        Codec::H264 | Codec::H265 => 51,
        Codec::Vp9 => 63,
    };
    if project.output.crf > crf_max {
        result.push_error(
            "crf_out_of_range",
            &format!("CRF の値が範囲外です（最大: {}）。", crf_max),
            None, None,
        );
    }
}

fn validate_scenes(project: &Project, result: &mut ValidationResult) {
    let mut seen_scene_ids = std::collections::HashSet::new();
    let mut global_seen_obj_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for scene in &project.scenes {
        if !seen_scene_ids.insert(scene.id.clone()) {
            result.push_error(
                "scene_id_duplicate",
                &format!("シーン ID \"{}\" が重複しています。", scene.id),
                Some(&scene.id), None,
            );
        }

        // scene_no_duration: no video objects (implicit duration from file) and no
        // objects with explicit duration > 0, and scene.duration not set
        if scene.duration.map(|d| d <= 0.0).unwrap_or(true) {
            let has_video_file = scene.objects.iter().any(|o| matches!(o, SceneObject::Video(v) if v.file.is_some()));
            let has_explicit_duration = scene.objects.iter().any(|o| match o {
                SceneObject::Image(img) => img.duration > 0.0,
                SceneObject::Text(txt) => txt.duration > 0.0,
                SceneObject::Audio(a) => a.duration > 0.0,
                SceneObject::Video(_) => false,
            });
            if !has_video_file && !has_explicit_duration {
                result.push_error(
                    "scene_no_duration",
                    &format!("シーン \"{}\" に有限長を定める要素がありません。scene.duration を設定してください。", scene.id),
                    Some(&scene.id), None,
                );
            }
        }

        let mut seen_obj_ids = std::collections::HashSet::new();

        for obj in &scene.objects {
            let (id, file_opt) = object_core_fields(obj);

            if !seen_obj_ids.insert(id.to_string()) {
                result.push_error(
                    "object_id_duplicate",
                    &format!("オブジェクト ID \"{}\" が重複しています。", id),
                    Some(&scene.id), Some(id),
                );
            }
            if let Some(prev_scene) = global_seen_obj_ids.get(id) {
                if prev_scene != &scene.id {
                    result.push_error(
                        "object_id_duplicate",
                        &format!(
                            "オブジェクト ID \"{}\" が複数シーンで使用されています（シーン \"{}\"、\"{}\"）。",
                            id, prev_scene, scene.id
                        ),
                        Some(&scene.id), Some(id),
                    );
                }
            } else {
                global_seen_obj_ids.insert(id.to_string(), scene.id.clone());
            }

            // テキスト以外でファイル未指定はエラー
            if let Some(false) = file_opt.map(|f| !f.is_empty()) {
                result.push_error(
                    "object_field_missing",
                    &format!("オブジェクト \"{}\" のファイルが指定されていません。", id),
                    Some(&scene.id), Some(id),
                );
            }
            if matches!(obj, SceneObject::Video(_) | SceneObject::Image(_) | SceneObject::Audio(_))
                && file_opt.is_none()
            {
                result.push_error(
                    "object_field_missing",
                    &format!("オブジェクト \"{}\" のファイルが指定されていません。", id),
                    Some(&scene.id), Some(id),
                );
            }

            // file_not_found: file path is set but does not exist on disk
            if let Some(file) = file_opt {
                if !file.is_empty() && !Path::new(file).exists() {
                    result.push_error(
                        "file_not_found",
                        &format!("オブジェクト \"{}\" のファイルが見つかりません: {}", id, file),
                        Some(&scene.id), Some(id),
                    );
                }
            }

            if let SceneObject::Text(txt) = obj {
                if !ALLOWED_FONTS.contains(&txt.font.as_str()) {
                    result.push_error(
                        "font_not_whitelisted",
                        &format!("フォント \"{}\" は使用できません。使用可能: {:?}", txt.font, ALLOWED_FONTS),
                        Some(&scene.id), Some(id),
                    );
                }
            }

            check_object_bounds(project, scene, obj, result);
        }
    }
}

fn object_core_fields(obj: &SceneObject) -> (&str, Option<&str>) {
    match obj {
        SceneObject::Video(v)   => (&v.id, v.file.as_deref()),
        SceneObject::Image(img) => (&img.id, img.file.as_deref()),
        SceneObject::Text(txt)  => (&txt.id, None),
        SceneObject::Audio(a)   => (&a.id, a.file.as_deref()),
    }
}

fn check_object_bounds(
    project: &Project,
    scene: &crate::project::Scene,
    obj: &SceneObject,
    result: &mut ValidationResult,
) {
    let (id, x, y, w, h) = match obj {
        SceneObject::Video(v)   => (v.id.as_str(), v.x, v.y, v.width, v.height),
        SceneObject::Image(img) => (img.id.as_str(), img.x, img.y, img.width, img.height),
        SceneObject::Text(txt)  => (txt.id.as_str(), txt.x, txt.y, txt.width, txt.height),
        SceneObject::Audio(_)   => return,
    };
    let out_w = project.output.width as i32;
    let out_h = project.output.height as i32;
    let right = x + w as i32;
    let bottom = y + h as i32;
    if x >= out_w || y >= out_h || right <= 0 || bottom <= 0 {
        result.push_warning(
            "object_out_of_bounds",
            &format!("オブジェクト \"{}\" がキャンバス外にあります。", id),
            Some(&scene.id), Some(id),
        );
    }
}
