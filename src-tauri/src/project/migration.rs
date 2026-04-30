use crate::project::Project;

#[derive(Debug, thiserror::Error)]
#[error("unsupported project version: {0}")]
pub struct UnsupportedVersionError(pub u32);

/// YAML 文字列からプロジェクトを読み込み、バージョンを検証して返す。
pub fn load_project(yaml: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let project: Project = serde_yml::from_str(yaml)?;
    if project.version != 1 {
        return Err(Box::new(UnsupportedVersionError(project.version)));
    }
    Ok(project)
}
