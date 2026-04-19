//! バッチ実行ループ
//!
//! エントリを直列に処理し、各エントリに対して ffprobe + FFmpeg を実行する。
//! 進捗は Tauri イベント（`batch:progress` / `batch:entry_done` / `batch:entry_error` /
//! `batch:done` / `batch:cancelled`）で通知する。
//!
//! サイクル 3-5: 手動テスト（`examples/minimal.yaml` で実際のレンダリング確認）

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use crate::project::{Entry, Project};
use crate::renderer::codec::build_codec_args;
use crate::renderer::filter::build_filter_graph;
use crate::renderer::probe::{parse_ffprobe_output, ProbeResult};
use crate::batch::output::build_output_path;
use crate::batch::log::log_file_path;
use crate::batch::sleep_guard::SleepGuard;

// ─────────────────────────────────────────────
// イベントペイロード型
// ─────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct BatchProgressPayload {
    pub entry_index: usize,
    pub total: usize,
    pub entry_name: String,
    pub entry_progress: Option<f64>, // 0.0–1.0
}

#[derive(Clone, Serialize)]
pub struct BatchEntryDonePayload {
    pub entry_name: String,
    pub output_path: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct BatchEntryErrorPayload {
    pub entry_name: String,
    pub message: String,
    pub ffmpeg_stderr: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct BatchDonePayload {
    pub success_count: usize,
    pub error_count: usize,
    pub total_elapsed_ms: u64,
}

// ─────────────────────────────────────────────
// 抽象イベントエミッタ（Tauri 非依存 / テスト可能）
// ─────────────────────────────────────────────

/// バッチ実行中に発火するイベントのエミッタ trait。
///
/// 本番では Tauri AppHandle を、テストではモック実装を使う。
pub trait BatchEventEmitter: Send + Sync {
    fn emit_progress(&self, payload: BatchProgressPayload);
    fn emit_entry_done(&self, payload: BatchEntryDonePayload);
    fn emit_entry_error(&self, payload: BatchEntryErrorPayload);
    fn emit_done(&self, payload: BatchDonePayload);
    fn emit_cancelled(&self);
}

// ─────────────────────────────────────────────
// 抽象キャンセルフラグ（テスト可能）
// ─────────────────────────────────────────────

/// バッチキャンセルフラグ。`cancel_batch` コマンドが `true` にセットする。
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

/// `ffprobe` を呼び出してファイルのメタデータを取得する。
///
/// `ffprobe_path` は実行ファイルの絶対パス（Tauri アプリバンドル同梱 ffprobe）。
pub fn probe_file(ffprobe_path: &str, file_path: &str) -> Result<ProbeResult, String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            file_path,
        ])
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
// バッチ実行ループ本体
// ─────────────────────────────────────────────

/// バッチ設定
pub struct BatchConfig {
    /// ffmpeg の絶対パス
    pub ffmpeg_path: String,
    /// ffprobe の絶対パス
    pub ffprobe_path: String,
    /// 処理対象エントリ名リスト（空 = 全エントリ）
    pub entry_names: Vec<String>,
    /// 出力ファイル衝突時のポリシー（"overwrite" | "skip"）
    pub overwrite_policy: String,
    /// ログファイルのタイムスタンプ文字列（"YYYYMMDD-HHMMSS" 形式）
    pub timestamp: String,
}

/// バッチ実行ループ。
///
/// 各エントリを直列に処理する。キャンセル要求があれば現在エントリ完了後に終了する。
pub fn run_batch(
    project: &Project,
    config: &BatchConfig,
    cancel: &CancelFlag,
    emitter: &dyn BatchEventEmitter,
    // FFmpeg 子プロセスを外から kill できるよう共有参照を渡す
    ffmpeg_child_slot: &Arc<Mutex<Option<std::process::Child>>>,
) -> Result<(), String> {
    let output_folder = &project.output_folder;
    let format_ext = format!("{}", project.output.format);

    // 対象エントリを解決
    let target_entries: Vec<&Entry> = if config.entry_names.is_empty() {
        project.entries.iter().collect()
    } else {
        project
            .entries
            .iter()
            .filter(|e| config.entry_names.contains(&e.name))
            .collect()
    };

    let total = target_entries.len();
    let batch_start = Instant::now();
    let mut success_count = 0;
    let mut error_count = 0;

    // スリープ抑制開始（RAII: run_batch リターン時に自動解除）
    #[cfg(target_os = "macos")]
    let _sleep_guard = SleepGuard::new().map_err(|e| {
        eprintln!("SleepGuard::new() failed: {e}");
    });
    #[cfg(not(target_os = "macos"))]
    let _sleep_guard = SleepGuard::new();

    // ログファイル初期化
    let log_path = log_file_path(output_folder, &config.timestamp);
    let mut log_lines: Vec<String> = Vec::new();
    log_lines.push(format!("=== ezmm batch log {} ===", config.timestamp));

    for (entry_idx, entry) in target_entries.iter().enumerate() {
        // キャンセル確認
        if cancel.is_cancelled() {
            emitter.emit_cancelled();
            write_log(&log_path, &log_lines);
            return Ok(());
        }

        let entry_name = &entry.name;
        let out_path = build_output_path(output_folder, entry_name, &format_ext);
        let out_path_str = out_path.to_string_lossy().into_owned();

        // skip ポリシー: 既存ファイルはスキップ
        if config.overwrite_policy == "skip" && out_path.exists() {
            log_lines.push(format!("[SKIP] {entry_name}: ファイルが既に存在します"));
            success_count += 1;
            emitter.emit_entry_done(BatchEntryDonePayload {
                entry_name: entry_name.clone(),
                output_path: out_path_str,
                elapsed_ms: 0,
            });
            continue;
        }

        // 進捗通知（エントリ開始）
        emitter.emit_progress(BatchProgressPayload {
            entry_index: entry_idx,
            total,
            entry_name: entry_name.clone(),
            entry_progress: None,
        });
        log_lines.push(format!("[START] {entry_name}"));

        let entry_start = Instant::now();

        // ffprobe: エントリ内の全ファイルをプローブ
        let mut probes: HashMap<String, ProbeResult> = HashMap::new();
        for scene in &project.scenes {
            for obj_file in collect_files(project, entry, scene) {
                if !probes.contains_key(&obj_file) {
                    match probe_file(&config.ffprobe_path, &obj_file) {
                        Ok(probe) => {
                            probes.insert(obj_file, probe);
                        }
                        Err(e) => {
                            let msg = format!("ffprobe エラー ({obj_file}): {e}");
                            log_lines.push(format!("[ERROR] {entry_name}: {msg}"));
                            write_log(&log_path, &log_lines);
                            emitter.emit_entry_error(BatchEntryErrorPayload {
                                entry_name: entry_name.clone(),
                                message: msg,
                                ffmpeg_stderr: None,
                            });
                            return Err(format!("ffmpeg_error: {entry_name} の probe に失敗しました"));
                        }
                    }
                }
            }
        }

        // filter_complex グラフ構築
        let graph = match build_filter_graph(project, entry, &probes) {
            Ok(g) => g,
            Err(e) => {
                let msg = format!("フィルタグラフ構築エラー: {e}");
                log_lines.push(format!("[ERROR] {entry_name}: {msg}"));
                write_log(&log_path, &log_lines);
                emitter.emit_entry_error(BatchEntryErrorPayload {
                    entry_name: entry_name.clone(),
                    message: msg,
                    ffmpeg_stderr: None,
                });
                return Err(format!("ffmpeg_error: {entry_name} のフィルタグラフ構築に失敗しました"));
            }
        };

        // FFmpeg コマンド組み立て
        let mut ffmpeg_args: Vec<String> = Vec::new();
        ffmpeg_args.extend(["-y".into(), "-progress".into(), "pipe:1".into(), "-nostats".into()]);

        for input in &graph.inputs {
            if let Some(flags) = &input.image_flags {
                ffmpeg_args.extend([
                    "-loop".into(), "1".into(),
                    "-t".into(), flags.duration.to_string(),
                ]);
            }
            ffmpeg_args.extend(["-i".into(), input.path.clone()]);
        }

        ffmpeg_args.extend(["-filter_complex".into(), graph.filter_complex.clone()]);

        let codec_args = build_codec_args(
            &project.output,
            &graph.video_map,
            &graph.audio_map,
            &out_path_str,
        );
        ffmpeg_args.extend(codec_args);

        log_lines.push(format!(
            "[CMD] ffmpeg {}",
            ffmpeg_args.join(" ")
        ));

        // FFmpeg サブプロセス起動（-progress pipe:1 で進捗を stdout に出力）
        let total_duration: f64 = project.scenes.iter().map(|s| s.duration.unwrap_or(0.0)).sum();

        let mut child = match Command::new(&config.ffmpeg_path)
            .args(&ffmpeg_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("FFmpeg 起動に失敗しました: {e}");
                log_lines.push(format!("[ERROR] {entry_name}: {msg}"));
                write_log(&log_path, &log_lines);
                emitter.emit_entry_error(BatchEntryErrorPayload {
                    entry_name: entry_name.clone(),
                    message: msg,
                    ffmpeg_stderr: None,
                });
                return Err(format!("ffmpeg_error: FFmpeg 起動に失敗しました"));
            }
        };

        // 子プロセスをスロットに書き込む（cancel_batch からの kill 用）
        {
            let mut slot = ffmpeg_child_slot.lock().unwrap();
            // 以前のプロセスが残っていれば kill（念のため）
            if let Some(mut prev) = slot.take() {
                let _ = prev.kill() // ignore error
                    .inspect_err(|e| eprintln!("prev child kill failed: {e}"));
            }
        }

        // stdout から -progress pipe:1 の出力を読んで進捗を emit
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        for line in reader.lines().flatten() {
            // キャンセル確認
            if cancel.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                // 不完全ファイルを削除
                if out_path.exists() {
                    let _ = std::fs::remove_file(&out_path);
                }
                emitter.emit_cancelled();
                log_lines.push(format!("[CANCELLED] {entry_name}"));
                write_log(&log_path, &log_lines);
                return Ok(());
            }

            // `out_time_ms=\d+` を解析して進捗を計算
            if let Some(ms_str) = line.strip_prefix("out_time_ms=") {
                if let Ok(ms) = ms_str.trim().parse::<u64>() {
                    let elapsed_sec = ms as f64 / 1_000_000.0; // μs → s
                    let progress = if total_duration > 0.0 {
                        Some((elapsed_sec / total_duration).min(1.0))
                    } else {
                        None
                    };
                    emitter.emit_progress(BatchProgressPayload {
                        entry_index: entry_idx,
                        total,
                        entry_name: entry_name.clone(),
                        entry_progress: progress,
                    });
                }
            }
        }

        // stderr は stdout ループ完了後（子プロセス終了前）に読み出す
        let ffmpeg_stderr: Option<String> = child.stderr.take().and_then(|mut se| {
            use std::io::Read;
            let mut buf = String::new();
            se.read_to_string(&mut buf).ok()?;
            if buf.is_empty() { None } else { Some(buf) }
        });

        // FFmpeg 終了待ち
        let status = child.wait().map_err(|e| format!("ffmpeg_error: wait() 失敗: {e}"))?;

        if !status.success() {
            let msg = format!(
                "FFmpeg がエラーコード {} で終了しました",
                status.code().unwrap_or(-1)
            );
            log_lines.push(format!("[ERROR] {entry_name}: {msg}"));
            if let Some(ref se) = ffmpeg_stderr {
                log_lines.push(format!("[STDERR] {se}"));
            }
            // 不完全ファイルを削除
            if out_path.exists() {
                let _ = std::fs::remove_file(&out_path);
            }
            write_log(&log_path, &log_lines);
            emitter.emit_entry_error(BatchEntryErrorPayload {
                entry_name: entry_name.clone(),
                message: msg,
                ffmpeg_stderr,
            });
            return Err(format!("ffmpeg_error: {entry_name} のレンダリングに失敗しました"));
        }

        // 出力ファイル検証（存在 + サイズ > 0）
        match validate_output_file(&out_path) {
            Ok(()) => {}
            Err(e) => {
                let msg = format!("出力ファイル検証エラー: {e}");
                log_lines.push(format!("[ERROR] {entry_name}: {msg}"));
                write_log(&log_path, &log_lines);
                emitter.emit_entry_error(BatchEntryErrorPayload {
                    entry_name: entry_name.clone(),
                    message: msg,
                    ffmpeg_stderr,
                });
                return Err(format!("ffmpeg_error: {entry_name} の出力ファイルが無効です"));
            }
        }

        let elapsed_ms = entry_start.elapsed().as_millis() as u64;
        success_count += 1;
        log_lines.push(format!("[DONE] {entry_name}: {elapsed_ms}ms"));

        emitter.emit_entry_done(BatchEntryDonePayload {
            entry_name: entry_name.clone(),
            output_path: out_path_str,
            elapsed_ms,
        });

        // スロットをクリア
        {
            let mut slot = ffmpeg_child_slot.lock().unwrap();
            *slot = None;
        }
    }

    let total_elapsed_ms = batch_start.elapsed().as_millis() as u64;
    log_lines.push(format!(
        "[BATCH_DONE] success={success_count} error={error_count} elapsed={total_elapsed_ms}ms"
    ));
    write_log(&log_path, &log_lines);

    emitter.emit_done(BatchDonePayload {
        success_count,
        error_count,
        total_elapsed_ms,
    });

    Ok(())
}

// ─────────────────────────────────────────────
// ヘルパー
// ─────────────────────────────────────────────

/// シーン内の全ファイルパスを収集する（ffprobe 対象）。
fn collect_files(
    _project: &Project,
    entry: &Entry,
    scene: &crate::project::Scene,
) -> Vec<String> {
    use crate::project::{SceneObject, VariableValue};

    let mut files = Vec::new();
    for obj in &scene.objects {
        let (obj_id, static_file) = match obj {
            SceneObject::Video(v) => (v.id.as_str(), v.file.as_deref()),
            SceneObject::Image(i) => (i.id.as_str(), i.file.as_deref()),
            SceneObject::Audio(a) => (a.id.as_str(), a.file.as_deref()),
            SceneObject::Text(_) => continue, // テキストはファイルなし
        };

        // variable=true ならエントリから解決
        if let Some(vv) = entry.variables.get(obj_id) {
            if let VariableValue::Media { file, .. } = vv {
                files.push(file.clone());
                continue;
            }
        }
        if let Some(f) = static_file {
            files.push(f.to_string());
        }
    }
    files
}

/// 出力ファイルが存在し、サイズ > 0 であることを検証する。
fn validate_output_file(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("出力ファイルが見つかりません: {e}"))?;
    if meta.len() == 0 {
        return Err("出力ファイルのサイズが 0 バイトです".to_string());
    }
    Ok(())
}

/// ログ行リストをファイルに追記する（失敗しても警告のみ）。
fn write_log(path: &Path, lines: &[String]) {
    let content = lines.join("\n") + "\n";
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("ログ書き込みに失敗しました ({path:?}): {e}");
    }
}
