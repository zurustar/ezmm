use tauri::State;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::state::AppState;
use crate::renderer::probe::{parse_ffprobe_output, ProbeResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontPaths {
    pub regular: String,
    pub bold: String,
}

/// FFmpeg のバージョンを取得する
#[tauri::command]
pub async fn get_ffmpeg_version(state: State<'_, AppState>) -> Result<String, String> {
    // キャッシュがあるか確認
    {
        let cache = state.ffmpeg_version.lock().unwrap();
        if let Some(version) = &*cache {
            return Ok(version.clone());
        }
    }

    let ffmpeg_path = &state.ffmpeg_path;
    let output = Command::new(ffmpeg_path)
        .arg("-version")
        .output()
        .map_err(|e| format!("io_error: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = output_str.lines().next() {
        if let Some(version_match) = extract_ffmpeg_version(line) {
            let mut cache = state.ffmpeg_version.lock().unwrap();
            *cache = Some(version_match.clone());
            return Ok(version_match);
        }
    }

    Err("ffmpeg_error: バージョン情報が取得できませんでした。".to_string())
}

/// FFmpeg のバージョン行から "ffmpeg version N.N.N" 相当の文字列を抽出する
fn extract_ffmpeg_version(line: &str) -> Option<String> {
    let re = regex::Regex::new(r"ffmpeg version [0-9a-zA-Z\.\-]+").unwrap();
    re.find(line).map(|m| m.as_str().to_string())
}

/// メディアファイルのメタデータを ffprobe で取得する
#[tauri::command]
pub async fn probe_file(path: String, state: State<'_, AppState>) -> Result<ProbeResult, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("io_error: ファイルが見つかりません: {}", path));
    }

    // パスを正規化（キャッシュキーにするため）
    let canonical_path = dunce::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());

    // キャッシュのチェック
    {
        let cache = state.probe_cache.lock().unwrap();
        if let Some(res) = cache.get(&canonical_path) {
            return Ok(res.clone());
        }
    }

    let ffprobe_path = &state.ffprobe_path;
    let output = Command::new(ffprobe_path)
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(&canonical_path)
        .output()
        .map_err(|e| format!("probe_error: {}", e))?;

    if !output.status.success() {
        return Err("probe_error: ffprobe の実行に失敗しました。".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let probe_result = parse_ffprobe_output(&output_str)
        .map_err(|e| format!("probe_error: {}", e))?;

    // キャッシュに保存
    {
        let mut cache = state.probe_cache.lock().unwrap();
        cache.insert(canonical_path, probe_result.clone());
    }

    Ok(probe_result)
}

/// 同梱のフォントパスを取得する
#[tauri::command]
pub fn get_font_paths(state: State<'_, AppState>) -> FontPaths {
    let dir = &state.font_dir;
    FontPaths {
        regular: dir.join("NotoSansCJK-Regular.otf").to_string_lossy().to_string(),
        bold: dir.join("NotoSansCJK-Bold.otf").to_string_lossy().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_from_custom_build() {
        let line = "ffmpeg version 7.1.0-macOS Copyright (c) 2000-2024 the FFmpeg developers";
        assert_eq!(extract_ffmpeg_version(line), Some("ffmpeg version 7.1.0-macOS".to_string()));
    }

    #[test]
    fn extract_version_short() {
        let line = "ffmpeg version 7.1.0";
        assert_eq!(extract_ffmpeg_version(line), Some("ffmpeg version 7.1.0".to_string()));
    }
}
