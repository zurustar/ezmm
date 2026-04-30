use tauri::{AppHandle, Emitter, State};
use std::sync::Arc;

use crate::batch::runner::{
    ExportConfig, ExportDonePayload, ExportErrorPayload,
    ExportEventEmitter, ExportProgressPayload, CancelFlag, run_export,
};
use crate::project::Project;
use crate::state::AppState;

struct TauriExportEventEmitter {
    app: AppHandle,
}

impl ExportEventEmitter for TauriExportEventEmitter {
    fn emit_progress(&self, payload: ExportProgressPayload) {
        let _ = self.app.emit("export:progress", payload);
    }
    fn emit_done(&self, payload: ExportDonePayload) {
        let _ = self.app.emit("export:done", payload);
    }
    fn emit_error(&self, payload: ExportErrorPayload) {
        let _ = self.app.emit("export:error", payload);
    }
    fn emit_cancelled(&self) {
        let _ = self.app.emit("export:cancelled", ());
    }
}

/// 書き出しを開始する（非同期実行）
#[tauri::command]
pub async fn start_export(
    project: Project,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut running = state.batch_running.lock().unwrap();
        if *running {
            return Err("export_already_running: すでに書き出しが実行中です".to_string());
        }
        *running = true;
    }

    {
        let mut cancel = state.cancel_requested.lock().unwrap();
        *cancel = false;
    }

    let batch_running_flag = Arc::clone(&state.batch_running);
    let cancel_flag = CancelFlag(Arc::clone(&state.cancel_requested));
    let ffmpeg_child_slot = Arc::clone(&state.ffmpeg_child);

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let config = ExportConfig {
        ffmpeg_path: state.ffmpeg_path.to_string_lossy().to_string(),
        ffprobe_path: state.ffprobe_path.to_string_lossy().to_string(),
        font_dir: state.font_dir.clone(),
        timestamp,
    };

    let emitter = TauriExportEventEmitter { app };

    tauri::async_runtime::spawn_blocking(move || {
        let result = run_export(&project, &config, &cancel_flag, &emitter, &ffmpeg_child_slot);
        {
            let mut running = batch_running_flag.lock().unwrap();
            *running = false;
        }
        if let Err(e) = result {
            tracing::error!("export failed: {e}");
        }
    });

    Ok(())
}

/// 書き出しをキャンセルする
#[tauri::command]
pub async fn cancel_export(state: State<'_, AppState>) -> Result<(), String> {
    let cancel = CancelFlag(Arc::clone(&state.cancel_requested));
    cancel.request_cancel();

    // FFmpeg 子プロセスがあれば即 kill
    let mut slot = state.ffmpeg_child.lock().unwrap();
    if let Some(mut child) = slot.take() {
        let _ = child.kill();
    }

    Ok(())
}
