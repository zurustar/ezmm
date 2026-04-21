use tauri::{AppHandle, Manager, State, Emitter};
use std::sync::Arc;

use crate::batch::runner::{
    self, BatchConfig, BatchDonePayload, BatchEntryDonePayload, BatchEntryErrorPayload,
    BatchEventEmitter, BatchProgressPayload, CancelFlag,
};
use crate::project::Project;
use crate::state::AppState;
use crate::batch::output;

struct TauriBatchEventEmitter {
    app: AppHandle,
}

impl BatchEventEmitter for TauriBatchEventEmitter {
    fn emit_progress(&self, payload: BatchProgressPayload) {
        let _ = self.app.emit("batch:progress", payload);
    }
    fn emit_entry_done(&self, payload: BatchEntryDonePayload) {
        let _ = self.app.emit("batch:entry_done", payload);
    }
    fn emit_entry_error(&self, payload: BatchEntryErrorPayload) {
        let _ = self.app.emit("batch:entry_error", payload);
    }
    fn emit_done(&self, payload: BatchDonePayload) {
        let _ = self.app.emit("batch:done", payload);
    }
    fn emit_cancelled(&self) {
        let _ = self.app.emit("batch:cancelled", ());
    }
}

/// バッチ処理を開始する（非同期実行）
#[tauri::command]
pub async fn start_batch(
    project: Project,
    entry_names: Vec<String>,
    overwrite_policy: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 二重起動チェック
    {
        let mut running = state.batch_running.lock().unwrap();
        if *running {
            return Err("batch_already_running: すでにバッチが実行中です".to_string());
        }
        *running = true;
    }

    // キャンセルフラグをリセット
    {
        let mut cancel = state.cancel_requested.lock().unwrap();
        *cancel = false;
    }

    let batch_running_flag = Arc::clone(&state.batch_running);
    let cancel_flag = CancelFlag(Arc::clone(&state.cancel_requested));
    let ffmpeg_child_slot = Arc::clone(&state.ffmpeg_child);

    let config = BatchConfig {
        ffmpeg_path: state.ffmpeg_path.to_string_lossy().to_string(),
        ffprobe_path: state.ffprobe_path.to_string_lossy().to_string(),
        entry_names,
        overwrite_policy,
        timestamp: chrono::Local::now().format("%Y%m%d-%H%M%S").to_string(),
    };

    let emitter = TauriBatchEventEmitter { app };

    // ブロッキングスレッドで実行
    tauri::async_runtime::spawn_blocking(move || {
        let _ = runner::run_batch(&project, &config, &cancel_flag, &emitter, &ffmpeg_child_slot);
        
        // 終了後に running フラグを降ろす
        let mut running = batch_running_flag.lock().unwrap();
        *running = false;
    });

    Ok(())
}

/// バッチをキャンセルする
#[tauri::command]
pub async fn cancel_batch(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cancel = state.cancel_requested.lock().unwrap();
        *cancel = true;
    }

    // 実行中の ffmpeg があれば kill する
    let mut child_guard = state.ffmpeg_child.lock().unwrap();
    if let Some(child) = child_guard.as_mut() {
        let _ = child.kill();
    }

    Ok(())
}

/// 出力ファイルの衝突を確認する
#[tauri::command]
pub fn check_output_conflicts(
    project: Project,
    entry_names: Vec<String>,
) -> Result<Vec<String>, String> {
    let output_str = project.output_folder.replace('\\', "/");
    let format_str = format!("{}", project.output.format);
    
    // 対象のファイル名を計算
    let target_names: Vec<&str> = if entry_names.is_empty() {
        project.entries.iter().map(|e| e.name.as_str()).collect()
    } else {
        project.entries.iter()
            .filter(|e| entry_names.contains(&e.name))
            .map(|e| e.name.as_str())
            .collect()
    };

    let conflicts = output::check_output_conflicts(&output_str, &target_names, &format_str);
    Ok(conflicts)
}
