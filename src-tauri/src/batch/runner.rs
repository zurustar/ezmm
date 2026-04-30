//! 書き出し実行
//!
//! プロジェクトを元に ffprobe + FFmpeg を実行して1本の動画を出力する。
//! 進捗は Tauri イベント（`export:progress` / `export:done` / `export:error` /
//! `export:cancelled`）で通知する。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use crate::project::Project;
use crate::renderer::codec::build_codec_args;
use crate::renderer::filter::{build_filter_graph, compute_scene_len};
use crate::renderer::probe::{parse_ffprobe_output, ProbeResult};
use crate::batch::log::log_file_path;
use crate::batch::sleep_guard::SleepGuard;

// ─────────────────────────────────────────────
// イベントペイロード型
// ─────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct ExportProgressPayload {
    pub progress: Option<f64>, // 0.0–1.0
}

#[derive(Clone, Serialize)]
pub struct ExportDonePayload {
    pub output_path: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct ExportErrorPayload {
    pub message: String,
    pub ffmpeg_stderr: Option<String>,
}

// ─────────────────────────────────────────────
// 抽象イベントエミッタ
// ─────────────────────────────────────────────

pub trait ExportEventEmitter: Send + Sync {
    fn emit_progress(&self, payload: ExportProgressPayload);
    fn emit_done(&self, payload: ExportDonePayload);
    fn emit_error(&self, payload: ExportErrorPayload);
    fn emit_cancelled(&self);
}

// ─────────────────────────────────────────────
// キャンセルフラグ
// ─────────────────────────────────────────────

pub struct CancelFlag(pub Arc<Mutex<bool>>);

impl CancelFlag {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(false)))
    }

    pub fn is_cancelled(&self) -> bool {
        *self.0.lock().unwrap()
    }

    pub fn request_cancel(&self) {
        *self.0.lock().unwrap() = true;
    }
}

// ─────────────────────────────────────────────
// ffprobe ヘルパー
// ─────────────────────────────────────────────

pub fn probe_file(ffprobe_path: &str, file_path: &str) -> Result<ProbeResult, String> {
    let output = Command::new(ffprobe_path)
        .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", file_path])
        .output()
        .map_err(|e| format!("probe_error: ffprobe 実行に失敗しました: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(format!("probe_error: ffprobe がエラーを返しました: {stderr}"));
    }

    let json = String::from_utf8_lossy(&output.stdout).into_owned();
    parse_ffprobe_output(&json)
        .map_err(|e| format!("probe_error: ffprobe 出力のパースに失敗しました: {e}"))
}

// ─────────────────────────────────────────────
// 書き出し設定
// ─────────────────────────────────────────────

pub struct ExportConfig {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub font_dir: std::path::PathBuf,
    pub timestamp: String,
}

// ─────────────────────────────────────────────
// 書き出しメイン
// ─────────────────────────────────────────────

pub fn run_export(
    project: &Project,
    config: &ExportConfig,
    cancel: &CancelFlag,
    emitter: &dyn ExportEventEmitter,
    ffmpeg_child_slot: &Arc<Mutex<Option<std::process::Child>>>,
) -> Result<(), String> {
    let output_folder = &project.output_folder;
    let format_ext = format!("{}", project.output.format);
    let output_name = &project.output.output_name;

    let out_path = Path::new(output_folder).join(format!("{output_name}.{format_ext}"));
    let out_path_str = out_path.to_string_lossy().into_owned();

    let log_path = log_file_path(output_folder, &config.timestamp);
    let mut log_lines: Vec<String> = vec![format!("=== ezmm export log {} ===", config.timestamp)];

    #[cfg(target_os = "macos")]
    let _sleep_guard = SleepGuard::new().map_err(|e| eprintln!("SleepGuard::new() failed: {e}"));
    #[cfg(not(target_os = "macos"))]
    let _sleep_guard = SleepGuard::new();

    emitter.emit_progress(ExportProgressPayload { progress: None });

    // 前回の子プロセスが残っていれば kill
    {
        let mut slot = ffmpeg_child_slot.lock().unwrap();
        if let Some(mut prev) = slot.take() {
            let _ = prev.kill();
        }
    }

    // ffprobe: プロジェクト内の全ファイルをプローブ
    let mut probes: HashMap<String, ProbeResult> = HashMap::new();
    for scene in &project.scenes {
        for file_path in collect_files(scene) {
            if !probes.contains_key(&file_path) {
                if cancel.is_cancelled() {
                    emitter.emit_cancelled();
                    return Ok(());
                }
                match probe_file(&config.ffprobe_path, &file_path) {
                    Ok(probe) => { probes.insert(file_path, probe); }
                    Err(e) => {
                        let msg = format!("ffprobe エラー ({file_path}): {e}");
                        log_lines.push(format!("[ERROR] {msg}"));
                        write_log(&log_path, &log_lines);
                        emitter.emit_error(ExportErrorPayload { message: msg, ffmpeg_stderr: None });
                        return Err("ffmpeg_error: probe に失敗しました".to_string());
                    }
                }
            }
        }
    }

    // filter_complex グラフ構築
    let graph = match build_filter_graph(project, &probes, &config.font_dir) {
        Ok(g) => g,
        Err(e) => {
            let msg = format!("フィルタグラフ構築エラー: {e}");
            log_lines.push(format!("[ERROR] {msg}"));
            write_log(&log_path, &log_lines);
            emitter.emit_error(ExportErrorPayload { message: msg, ffmpeg_stderr: None });
            return Err("ffmpeg_error: フィルタグラフ構築に失敗しました".to_string());
        }
    };

    // FFmpeg コマンド組み立て
    let mut ffmpeg_args: Vec<String> = Vec::new();
    ffmpeg_args.extend(["-y".into(), "-progress".into(), "pipe:1".into(), "-nostats".into()]);

    for input in &graph.inputs {
        if let Some(flags) = &input.image_flags {
            ffmpeg_args.extend(["-loop".into(), "1".into(), "-t".into(), flags.duration.to_string()]);
        }
        ffmpeg_args.extend(["-i".into(), input.path.clone()]);
    }

    ffmpeg_args.extend(["-filter_complex".into(), graph.filter_complex.clone()]);
    let codec_args = build_codec_args(&project.output, &graph.video_map, &graph.audio_map, &out_path_str);
    ffmpeg_args.extend(codec_args);

    log_lines.push(format!("[CMD] ffmpeg {}", ffmpeg_args.join(" ")));

    // probes を使ったシーン長の合計で進捗を正確に計算する
    let total_duration: f64 = project.scenes.iter()
        .map(|s| compute_scene_len(s, &probes))
        .sum();
    let export_start = Instant::now();

    let mut child = match Command::new(&config.ffmpeg_path)
        .args(&ffmpeg_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("FFmpeg 起動に失敗しました: {e}");
            log_lines.push(format!("[ERROR] {msg}"));
            write_log(&log_path, &log_lines);
            emitter.emit_error(ExportErrorPayload { message: msg, ffmpeg_stderr: None });
            return Err("ffmpeg_error: FFmpeg 起動に失敗しました".to_string());
        }
    };

    let stdout = child.stdout.take().unwrap();
    // slot に格納して cancel_export が即座に kill できるようにする
    {
        let mut slot = ffmpeg_child_slot.lock().unwrap();
        *slot = Some(child);
    }

    let reader = BufReader::new(stdout);

    for line in reader.lines().flatten() {
        if cancel.is_cancelled() {
            let mut slot = ffmpeg_child_slot.lock().unwrap();
            if let Some(mut c) = slot.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
            if out_path.exists() { let _ = std::fs::remove_file(&out_path); }
            emitter.emit_cancelled();
            log_lines.push("[CANCELLED]".to_string());
            write_log(&log_path, &log_lines);
            return Ok(());
        }

        if let Some(ms_str) = line.strip_prefix("out_time_ms=") {
            if let Ok(ms) = ms_str.trim().parse::<u64>() {
                let elapsed_sec = ms as f64 / 1_000_000.0;
                let progress = if total_duration > 0.0 {
                    Some((elapsed_sec / total_duration).min(1.0))
                } else {
                    None
                };
                emitter.emit_progress(ExportProgressPayload { progress });
            }
        }
    }

    // slot から child を取り出して stderr と終了コードを取得
    let (status, ffmpeg_stderr) = {
        let mut slot = ffmpeg_child_slot.lock().unwrap();
        match slot.take() {
            Some(mut c) => {
                let stderr = c.stderr.take().and_then(|mut se| {
                    use std::io::Read;
                    let mut buf = String::new();
                    se.read_to_string(&mut buf).ok()?;
                    if buf.is_empty() { None } else { Some(buf) }
                });
                let status = c.wait().map_err(|e| format!("ffmpeg_error: wait() 失敗: {e}"))?;
                (status, stderr)
            }
            None => {
                // cancel_export がすでに child を kill してスロットから取り出した場合
                if out_path.exists() { let _ = std::fs::remove_file(&out_path); }
                emitter.emit_cancelled();
                log_lines.push("[CANCELLED by cancel_export]".to_string());
                write_log(&log_path, &log_lines);
                return Ok(());
            }
        }
    };

    if !status.success() {
        let msg = format!("FFmpeg がエラーコード {} で終了しました", status.code().unwrap_or(-1));
        log_lines.push(format!("[ERROR] {msg}"));
        if let Some(ref se) = ffmpeg_stderr { log_lines.push(format!("[STDERR] {se}")); }
        if out_path.exists() { let _ = std::fs::remove_file(&out_path); }
        write_log(&log_path, &log_lines);
        emitter.emit_error(ExportErrorPayload { message: msg, ffmpeg_stderr });
        return Err("ffmpeg_error: レンダリングに失敗しました".to_string());
    }

    match validate_output_file(&out_path) {
        Ok(()) => {}
        Err(e) => {
            let msg = format!("出力ファイル検証エラー: {e}");
            log_lines.push(format!("[ERROR] {msg}"));
            write_log(&log_path, &log_lines);
            emitter.emit_error(ExportErrorPayload { message: msg, ffmpeg_stderr });
            return Err("ffmpeg_error: 出力ファイルが無効です".to_string());
        }
    }

    let elapsed_ms = export_start.elapsed().as_millis() as u64;
    log_lines.push(format!("[DONE] {out_path_str}: {elapsed_ms}ms"));
    write_log(&log_path, &log_lines);

    emitter.emit_done(ExportDonePayload { output_path: out_path_str, elapsed_ms });
    Ok(())
}

// ─────────────────────────────────────────────
// ヘルパー
// ─────────────────────────────────────────────

fn collect_files(scene: &crate::project::Scene) -> Vec<String> {
    use crate::project::SceneObject;
    scene.objects.iter().filter_map(|obj| match obj {
        SceneObject::Video(v) => v.file.clone(),
        SceneObject::Image(i) => i.file.clone(),
        SceneObject::Audio(a) => a.file.clone(),
        SceneObject::Text(_) => None,
    }).collect()
}

fn validate_output_file(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("出力ファイルが見つかりません: {e}"))?;
    if meta.len() == 0 {
        return Err("出力ファイルのサイズが 0 バイトです".to_string());
    }
    Ok(())
}

fn write_log(path: &Path, lines: &[String]) {
    let content = lines.join("\n") + "\n";
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("ログ書き込みに失敗しました ({path:?}): {e}");
    }
}
