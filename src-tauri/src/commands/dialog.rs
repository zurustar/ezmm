use rfd::AsyncFileDialog;

/// 汎用ファイル選択ダイアログ（kind で種別を指定）
/// kind: "video" | "image" | "audio" | "any"
#[tauri::command]
pub async fn show_open_file_dialog(kind: String) -> Result<Option<String>, String> {
    let mut dialog = AsyncFileDialog::new().set_title("ファイルを選択");
    dialog = match kind.as_str() {
        "video" => dialog.add_filter("動画", &["mp4", "mov", "webm", "avi", "mkv", "m4v"]),
        "image" => dialog.add_filter("画像", &["png", "jpg", "jpeg", "gif", "bmp", "webp"]),
        "audio" => dialog.add_filter("音声", &["mp3", "wav", "aac", "m4a", "flac", "ogg", "opus"]),
        _ => dialog,
    };
    let result = dialog.pick_file().await;
    Ok(result.map(|h| h.path().to_string_lossy().to_string()))
}

/// YAML ファイルを開くダイアログを表示する
#[tauri::command]
pub async fn show_open_yaml_dialog() -> Result<Option<String>, String> {
    let result = AsyncFileDialog::new()
        .add_filter("YAML", &["yaml", "yml"])
        .set_title("プロジェクトを開く")
        .pick_file()
        .await;
    Ok(result.map(|h| h.path().to_string_lossy().to_string()))
}

/// YAML ファイル保存ダイアログを表示する
#[tauri::command]
pub async fn show_save_yaml_dialog(default_name: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = AsyncFileDialog::new()
        .add_filter("YAML", &["yaml", "yml"])
        .set_title("名前を付けて保存");
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }
    let result = dialog.save_file().await;
    Ok(result.map(|h| h.path().to_string_lossy().to_string()))
}

/// フォルダ選択ダイアログを表示する
#[tauri::command]
pub async fn show_folder_dialog() -> Result<Option<String>, String> {
    let result = AsyncFileDialog::new()
        .set_title("出力フォルダを選択")
        .pick_folder()
        .await;
    Ok(result.map(|h| h.path().to_string_lossy().to_string()))
}
