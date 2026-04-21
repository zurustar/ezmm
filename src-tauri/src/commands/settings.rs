use tauri::{AppHandle, Manager, State};
use std::path::PathBuf;

use crate::settings::{self, AppSettings};
use crate::state::AppState;

/// 設定ファイルのパスを取得するヘルパー
fn get_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("settings.json"))
        .map_err(|e| format!("io_error: app_config_dir 取得失敗: {}", e))
}

/// 設定ファイルを読み込んで返す（存在しない場合はデフォルト値を返す）
#[tauri::command]
pub fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let path = get_settings_path(&app)?;
    Ok(settings::load_settings(&path))
}

/// 設定ファイルを保存し、メモリ内のキャッシュも更新する
#[tauri::command]
pub fn save_settings(
    settings: AppSettings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = get_settings_path(&app)?;

    // 1. ファイルへの安全なアトミック保存
    settings::save_settings_sync(&settings, &path)?;

    // 2. メモリ上キャッシュの更新（Close時などの利用目的）
    let mut current = state.current_settings.lock().unwrap();
    *current = settings;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::async_runtime::block_on;
    // TauriのAppHandleはテストでモックするのが難しいため、内部の純粋な連携のみを手動テストで担保する
}
