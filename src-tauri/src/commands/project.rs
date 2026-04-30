use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::project::{
    migration, validation::{validate_project as core_validate_project, ValidationResult}, Project
};

/// プロジェクトファイルを開く
#[tauri::command]
pub async fn open_project(path: String) -> Result<Project, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("io_error: ファイルが見つかりません: {}", path));
    }
    let yaml = fs::read_to_string(p).map_err(|e| format!("io_error: {}", e))?;
    
    match migration::load_project(&yaml) {
        Ok(project) => Ok(project),
        Err(e) => {
            // migration::load_project はバージョンエラーかもしくは parse エラーを返す
            let msg = e.to_string();
            if msg.contains("unsupported project version") {
                Err(format!("unsupported_version: {}", msg))
            } else {
                Err(format!("invalid_yaml: {}", msg))
            }
        }
    }
}

/// プロジェクトファイルをアトミックに保存する
#[tauri::command]
pub async fn save_project(path: String, mut project: Project) -> Result<(), String> {
    // 内部パス中のバックスラッシュをスラッシュに正規化して保存
    project.output_folder = project.output_folder.replace('\\', "/");
    for scene in &mut project.scenes {
        for obj in &mut scene.objects {
            match obj {
                crate::project::SceneObject::Video(v) => {
                    if let Some(f) = &mut v.file { *f = f.replace('\\', "/"); }
                }
                crate::project::SceneObject::Image(img) => {
                    if let Some(f) = &mut img.file { *f = f.replace('\\', "/"); }
                }
                crate::project::SceneObject::Audio(a) => {
                    if let Some(f) = &mut a.file { *f = f.replace('\\', "/"); }
                }
                crate::project::SceneObject::Text(_) => {}
            }
        }
    }

    let yaml_str = serde_yml::to_string(&project)
        .map_err(|e| format!("invalid_yaml: シリアライズに失敗: {}", e))?;

    let p = PathBuf::from(&path);
    // 保存先の親ディレクトリが存在するか確認
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            return Err(format!("invalid_path: 保存先のディレクトリが存在しません: {:?}", parent));
        }
    }

    // バックアップの作成
    if p.exists() {
        let mut bak_path = p.clone();
        bak_path.set_extension("yaml.bak");
        let _ = fs::copy(&p, &bak_path); // バックアップ失敗は無視
    }

    let mut tmp_path = p.clone();
    tmp_path.set_extension("yaml.tmp");

    {
        let mut tmp_file = fs::File::create(&tmp_path).map_err(|e| format!("io_error: {}", e))?;
        tmp_file.write_all(yaml_str.as_bytes()).map_err(|e| format!("io_error: {}", e))?;
        tmp_file.sync_all().map_err(|e| format!("io_error: {}", e))?;
    } // tmp_file close

    if let Err(e) = fs::rename(&tmp_path, &p) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("io_error: アトミックリネーム失敗: {}", e));
    }

    Ok(())
}

/// プロジェクトのバリデーションのみ実行
#[tauri::command]
pub fn validate_project(project: Project) -> ValidationResult {
    core_validate_project(&project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_open_project_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_proj.yaml");
        let path_str = path.to_string_lossy().to_string();

        let mut proj = Project {
            version: 1,
            output_folder: "C:\\out\\dir".to_string(),
            output: crate::project::OutputSettings {
                output_name: "output".to_string(),
                width: 1920, height: 1080, fps: 30,
                codec: crate::project::Codec::H264,
                format: crate::project::Format::Mp4,
                crf: 23, preset: "medium".to_string(),
            },
            scenes: vec![],
        };

        // サイクル 4-4: アトミック保存とパス正規化
        let res = tauri::async_runtime::block_on(save_project(path_str.clone(), proj.clone()));
        assert!(res.is_ok());
        
        let loaded = tauri::async_runtime::block_on(open_project(path_str.clone())).unwrap();
        // 保存時にスラッシュに正規化されることを確認
        assert_eq!(loaded.output_folder, "C:/out/dir");

        // もう一度保存して bak ファイルができるか
        let res2 = tauri::async_runtime::block_on(save_project(path_str.clone(), proj.clone()));
        assert!(res2.is_ok());

        let bak_path = dir.path().join("test_proj.yaml.bak");
        assert!(bak_path.exists());
    }
}
