#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use tauri::{Manager, WindowEvent};
use tracing::{info, Level};
use tracing_appender::non_blocking;

use ezmm_lib::state::AppState;
use ezmm_lib::settings;
use ezmm_lib::commands::{batch, infra, project, settings as settings_cmd};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            let config_dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
            let settings_path = config_dir.join("settings.json");
            
            // ログの初期化
            let log_dir = app_handle.path().app_log_dir().unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&log_dir).unwrap_or_default();
            
            let debug_mode = std::env::var("EZMM_DEBUG").unwrap_or_default() == "1";
            let level = if debug_mode { Level::DEBUG } else { Level::INFO };
            
            let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");
            let (non_blocking_appender, log_guard) = non_blocking(file_appender);
            
            tracing_subscriber::fmt()
                .with_writer(non_blocking_appender)
                .with_max_level(level)
                .init();

            info!("Starting ezmm app...");

            // 同梱リソースパスの解決
            let resource_dir = app_handle.path().resource_dir().unwrap_or_else(|_| PathBuf::from("."));
            let font_dir = resource_dir.join("fonts");
            
            #[cfg(target_os = "windows")]
            let ffmpeg_exe = "ffmpeg.exe";
            #[cfg(target_os = "windows")]
            let ffprobe_exe = "ffprobe.exe";
            #[cfg(not(target_os = "windows"))]
            let ffmpeg_exe = "ffmpeg";
            #[cfg(not(target_os = "windows"))]
            let ffprobe_exe = "ffprobe";

            let ffmpeg_path = resource_dir.join("ffmpeg").join(ffmpeg_exe);
            let ffprobe_path = resource_dir.join("ffmpeg").join(ffprobe_exe);

            let current_settings = settings::load_settings(&settings_path);

            let state = AppState::new(
                font_dir,
                ffmpeg_path,
                ffprobe_path,
                debug_mode,
                log_guard,
                current_settings,
            );

            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            project::open_project,
            project::save_project,
            project::validate_project,
            infra::get_ffmpeg_version,
            infra::probe_file,
            infra::get_font_paths,
            settings_cmd::load_settings,
            settings_cmd::save_settings,
            batch::start_batch,
            batch::cancel_batch,
            batch::check_output_conflicts,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                // ウィンドウ終了時に設定を同期保存する
                if let Some(state) = window.try_state::<AppState>() {
                    let settings = state.current_settings.lock().unwrap().clone();
                    if let Ok(config_dir) = window.app_handle().path().app_config_dir() {
                        let settings_path = config_dir.join("settings.json");
                        let _ = settings::save_settings_sync(&settings, &settings_path);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
