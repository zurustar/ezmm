use crate::project::{Codec, Entry, Format, Project, SceneObject};

use serde::Serialize;

/// バリデーション結果
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
                  scene_id: Option<&str>, object_id: Option<&str>, entry_name: Option<&str>) {
        self.errors.push(ValidationIssue {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.to_string(),
            scene_id: scene_id.map(|s| s.to_string()),
            object_id: object_id.map(|s| s.to_string()),
            entry_name: entry_name.map(|s| s.to_string()),
        });
    }

    fn push_warning(&mut self, code: &str, message: &str,
                    scene_id: Option<&str>, object_id: Option<&str>, entry_name: Option<&str>) {
        self.warnings.push(ValidationIssue {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.to_string(),
            scene_id: scene_id.map(|s| s.to_string()),
            object_id: object_id.map(|s| s.to_string()),
            entry_name: entry_name.map(|s| s.to_string()),
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
    pub entry_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

/// 許可フォント一覧（設計書 01_project_schema.md 準拠）
const ALLOWED_FONTS: &[&str] = &["NotoSansCJK-Regular", "NotoSansCJK-Bold"];

/// エントリ名の禁止文字（OS 共通）
const INVALID_NAME_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// プロジェクト全体を検証して ValidationResult を返す。
pub fn validate_project(project: &Project) -> ValidationResult {
    let mut result = ValidationResult::new();

    validate_output_settings(project, &mut result);
    validate_scenes(project, &mut result);
    validate_entries(project, &mut result);

    result
}

// ---------------------------------------------------------------------------
// サイクル 1-6: プロジェクト・出力設定レベル
// ---------------------------------------------------------------------------

fn validate_output_settings(project: &Project, result: &mut ValidationResult) {
    // output_folder
    if project.output_folder.trim().is_empty() {
        result.push_error(
            "output_folder_invalid",
            "出力フォルダが設定されていません。",
            None, None, None,
        );
    }

    // codec + format の組み合わせ
    let combo_ok = matches!(
        (&project.output.codec, &project.output.format),
        (Codec::H264, Format::Mp4)
        | (Codec::H264, Format::Mov)
        | (Codec::H265, Format::Mp4)
        | (Codec::H265, Format::Mov)
        | (Codec::Vp9,  Format::Webm)
    );
    if !combo_ok {
        result.push_error(
            "codec_format_mismatch",
            "コーデックとフォーマットの組み合わせが不正です。",
            None, None, None,
        );
    }

    // crf 範囲
    let crf_max = match project.output.codec {
        Codec::H264 | Codec::H265 => 51,
        Codec::Vp9 => 63,
    };
    if project.output.crf > crf_max {
        result.push_error(
            "crf_out_of_range",
            &format!("CRF の値が範囲外です（最大: {}）。", crf_max),
            None, None, None,
        );
    }
}

// ---------------------------------------------------------------------------
// サイクル 1-7: シーン・オブジェクトレベル
// ---------------------------------------------------------------------------

fn validate_scenes(project: &Project, result: &mut ValidationResult) {
    let mut seen_scene_ids = std::collections::HashSet::new();
    // オブジェクト ID はプロジェクト全体でグローバルに一意（02_validation.md §追加チェック）
    let mut global_seen_obj_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for scene in &project.scenes {
        // シーン ID 重複チェック
        if !seen_scene_ids.insert(scene.id.clone()) {
            result.push_error(
                "scene_id_duplicate",
                &format!("シーン ID \"{}\" が重複しています。", scene.id),
                Some(&scene.id), None, None,
            );
        }

        let mut seen_obj_ids = std::collections::HashSet::new();

        for obj in &scene.objects {
            let (id, file_opt, variable) = object_core_fields(obj);

            // オブジェクト ID 重複チェック（同一シーン内）
            if !seen_obj_ids.insert(id.to_string()) {
                result.push_error(
                    "object_id_duplicate",
                    &format!("オブジェクト ID \"{}\" が重複しています。", id),
                    Some(&scene.id), Some(id), None,
                );
            }
            // オブジェクト ID グローバル重複チェック（別シーン間）
            if let Some(prev_scene) = global_seen_obj_ids.get(id) {
                if prev_scene != &scene.id {
                    result.push_error(
                        "object_id_duplicate",
                        &format!(
                            "オブジェクト ID \"{}\" が複数シーンで使用されています（シーン \"{}\"、\"{}\"）。",
                            id, prev_scene, scene.id
                        ),
                        Some(&scene.id), Some(id), None,
                    );
                }
            } else {
                global_seen_obj_ids.insert(id.to_string(), scene.id.clone());
            }

            // variable:false かつ file 未指定
            if !variable && file_opt.is_none() {
                result.push_error(
                    "object_field_missing",
                    &format!("オブジェクト \"{}\" のファイルが指定されていません。", id),
                    Some(&scene.id), Some(id), None,
                );
            }

            // フォント ホワイトリスト
            if let SceneObject::Text(txt) = obj {
                if !ALLOWED_FONTS.contains(&txt.font.as_str()) {
                    result.push_error(
                        "font_not_whitelisted",
                        &format!("フォント \"{}\" は使用できません。使用可能: {:?}", txt.font, ALLOWED_FONTS),
                        Some(&scene.id), Some(id), None,
                    );
                }
            }

            // 警告: オブジェクトがキャンバス外
            check_object_bounds(project, scene, obj, result);
        }
    }
}

/// オブジェクトの (id, file, variable) を取り出す共通ヘルパー
fn object_core_fields(obj: &SceneObject) -> (&str, Option<&str>, bool) {
    match obj {
        SceneObject::Video(v)  => (&v.id, v.file.as_deref(), v.variable),
        SceneObject::Image(img) => (&img.id, img.file.as_deref(), img.variable),
        SceneObject::Text(txt) => (&txt.id, None, txt.variable), // text は file なし
        SceneObject::Audio(a)  => (&a.id, a.file.as_deref(), a.variable),
    }
}

fn check_object_bounds(
    project: &Project,
    scene: &crate::project::Scene,
    obj: &SceneObject,
    result: &mut ValidationResult,
) {
    let (id, x, y, w, h) = match obj {
        SceneObject::Video(v)  => (v.id.as_str(), v.x, v.y, v.width, v.height),
        SceneObject::Image(img) => (img.id.as_str(), img.x, img.y, img.width, img.height),
        SceneObject::Text(txt) => (txt.id.as_str(), txt.x, txt.y, txt.width, txt.height),
        SceneObject::Audio(_)  => return, // 音声は座標なし
    };
    let out_w = project.output.width as i32;
    let out_h = project.output.height as i32;
    let right = x + w as i32;
    let bottom = y + h as i32;
    if x >= out_w || y >= out_h || right <= 0 || bottom <= 0 {
        result.push_warning(
            "object_out_of_bounds",
            &format!("オブジェクト \"{}\" がキャンバス外にあります。", id),
            Some(&scene.id), Some(id), None,
        );
    }
}

// ---------------------------------------------------------------------------
// サイクル 1-8: エントリレベル
// ---------------------------------------------------------------------------

fn validate_entries(project: &Project, result: &mut ValidationResult) {
    // エントリ名 重複チェック
    let mut seen_names = std::collections::HashSet::new();
    for entry in &project.entries {
        // エントリ名 文字制約
        validate_entry_name(&entry.name, result);

        if !entry.name.is_empty() && !seen_names.insert(entry.name.clone()) {
            result.push_error(
                "entry_name_duplicate",
                &format!("エントリ名 \"{}\" が重複しています。", entry.name),
                None, None, Some(&entry.name),
            );
        }

        // variable:true オブジェクトに対応する変数がエントリにあるか
        validate_variables_present(project, entry, result);
    }
}

fn validate_entry_name(name: &str, result: &mut ValidationResult) {
    if name.is_empty() {
        result.push_error(
            "entry_name_invalid",
            "エントリ名が空です。",
            None, None, Some(name),
        );
        return;
    }
    // 制御文字チェック
    let has_control = name.chars().any(|c| c.is_control());
    let has_invalid = name.chars().any(|c| INVALID_NAME_CHARS.contains(&c));
    if has_control || has_invalid {
        result.push_error(
            "entry_name_invalid",
            &format!("エントリ名 \"{}\" に使用できない文字が含まれています。", name),
            None, None, Some(name),
        );
    }
}

fn validate_variables_present(project: &Project, entry: &Entry, result: &mut ValidationResult) {
    for scene in &project.scenes {
        for obj in &scene.objects {
            let (id, _, variable) = object_core_fields(obj);
            if variable && !entry.variables.contains_key(id) {
                result.push_error(
                    "variable_missing",
                    &format!(
                        "エントリ \"{}\" に可変オブジェクト \"{}\" の値がありません。",
                        entry.name, id
                    ),
                    Some(&scene.id), Some(id), Some(&entry.name),
                );
            }
        }
    }
}
