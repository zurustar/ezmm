use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub version: u32,
    pub default_crf: u32,
    pub default_preset: String,
    pub preview_resolution_scale: f64,
    pub last_open_folder: Option<String>,
    pub recent_files: Vec<String>,
    pub window: WindowSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            default_crf: 23,
            default_preset: "medium".to_string(),
            preview_resolution_scale: 0.5,
            last_open_folder: None,
            recent_files: Vec::new(),
            window: WindowSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            x: None,
            y: None,
        }
    }
}

/// JSON文字列からAppSettingsをパースする
///
/// バージョンが `1` 以外の場合やパースエラー時はデフォルト設定を返す。
pub fn settings_from_str(json: &str) -> AppSettings {
    match serde_json::from_str::<AppSettings>(json) {
        Ok(settings) if settings.version == 1 => settings,
        _ => AppSettings::default(),
    }
}

/// AppSettingsをJSON文字列に整形する（整形済み）
pub fn settings_to_string(settings: &AppSettings) -> Result<String, String> {
    serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))
}

/// 指定パスからAppSettingsを読み込む。存在しない・不正な場合はデフォルト値を返す。
pub fn load_settings(path: &Path) -> AppSettings {
    if !path.exists() {
        return AppSettings::default();
    }
    match fs::read_to_string(path) {
        Ok(json) => settings_from_str(&json),
        Err(_) => AppSettings::default(),
    }
}

/// AppSettings をアトミックに保存する（.tmpファイル書き込みからrenameおよび.bakバックアップ）
pub fn save_settings_sync(settings: &AppSettings, path: &Path) -> Result<(), String> {
    // 親ディレクトリが存在しない場合は作成
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
    }

    // 既存ファイルのバックアップ作成
    if path.exists() {
        let mut bak_path = path.to_path_buf();
        bak_path.set_extension("json.bak");
        let _ = fs::copy(path, &bak_path); // バックアップ失敗は無視する
    }

    let mut tmp_path = path.to_path_buf();
    tmp_path.set_extension("json.tmp");

    let json = settings_to_string(settings)?;

    // .tmpファイルへ書き込み（fsync 含む）
    {
        let mut tmp_file = fs::File::create(&tmp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;
        tmp_file.write_all(json.as_bytes()).map_err(|e| format!("Failed to write temp file: {}", e))?;
        tmp_file.sync_all().map_err(|e| format!("Failed to sync temp file: {}", e))?;
    } // tmp_file がスコープを抜けて確実に close される

    // アトミックリネーム
    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path); // 失敗時はゴミを残さない
        return Err(format!("Failed to rename temp file to target settings file: {}", e));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── サイクル 4-1: AppSettings デフォルト値 ─────────────────────────
    #[test]
    fn app_settings_default_values() {
        let settings = AppSettings::default();
        assert_eq!(settings.version, 1);
        assert_eq!(settings.default_crf, 23);
        assert_eq!(settings.default_preset, "medium");
        assert_eq!(settings.preview_resolution_scale, 0.5);
        assert!(settings.last_open_folder.is_none());
        assert!(settings.recent_files.is_empty());

        assert_eq!(settings.window.width, 1280);
        assert_eq!(settings.window.height, 800);
        assert!(settings.window.x.is_none());
        assert!(settings.window.y.is_none());
    }

    // ── サイクル 4-2: settings.json シリアライズ round-trip ──────────
    #[test]
    fn settings_serialization_round_trip() {
        let mut settings = AppSettings::default();
        settings.default_crf = 18;
        settings.default_preset = "fast".to_string();
        settings.window.x = Some(100);

        let json = settings_to_string(&settings).expect("serialization failed");
        let parsed = settings_from_str(&json);

        assert_eq!(settings, parsed);
    }

    #[test]
    fn settings_from_str_invalid_version_returns_default() {
        let json_v2 = r#"{
            "version": 2,
            "default_crf": 18,
            "default_preset": "medium",
            "preview_resolution_scale": 0.5,
            "last_open_folder": null,
            "recent_files": [],
            "window": { "width": 100, "height": 100, "x": null, "y": null }
        }"#;

        let parsed = settings_from_str(json_v2);
        assert_eq!(parsed, AppSettings::default());
    }

    #[test]
    fn settings_from_str_invalid_json_returns_default() {
        let parsed = settings_from_str("not a json string");
        assert_eq!(parsed, AppSettings::default());
    }

    // ── サイクル 4-3: 設定ファイルのアトミック I/O ─────────────
    #[test]
    fn save_and_load_settings_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        // 存在しないときは load が デフォルトを返す
        assert_eq!(load_settings(&path), AppSettings::default());

        // 保存テスト
        let mut settings = AppSettings::default();
        settings.default_crf = 10;
        save_settings_sync(&settings, &path).unwrap();

        // 正常に保存・ロードできるか
        let loaded = load_settings(&path);
        assert_eq!(loaded.default_crf, 10);

        // 再度保存して .bak が作成されるかを確認
        settings.default_preset = "slow".to_string();
        save_settings_sync(&settings, &path).unwrap();

        let loaded2 = load_settings(&path);
        assert_eq!(loaded2.default_preset, "slow");

        let bak_path = dir.path().join("settings.json.bak");
        assert!(bak_path.exists(), "Backup file should be created");

        let loaded_bak = load_settings(&bak_path);
        assert_eq!(loaded_bak.default_crf, 10); // 一つ前の値
        assert_eq!(loaded_bak.default_preset, "medium"); // 一つ前の値
    }
}
