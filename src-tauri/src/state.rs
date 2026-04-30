use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing_appender::non_blocking::WorkerGuard;

use crate::renderer::probe::ProbeResult;
use crate::settings::AppSettings;

/// グローバルなアプリケーション状態（TauriのStateとして管理する）
pub struct AppState {
    /// ffprobe 結果のセッションキャッシュ（重複実行回避用）
    pub probe_cache: Mutex<HashMap<PathBuf, ProbeResult>>,

    /// FFmpeg バージョン情報のキャッシュ
    pub ffmpeg_version: Mutex<Option<String>>,

    /// メモリ上に保持する現在の設定
    pub current_settings: Mutex<AppSettings>,

    /// バッチが実行中かどうか
    pub batch_running: Arc<Mutex<bool>>,

    /// バッチのキャンセルフラグ
    pub cancel_requested: Arc<Mutex<bool>>,

    /// 実行中の FFmpeg サブプロセス
    pub ffmpeg_child: Arc<Mutex<Option<std::process::Child>>>,

    /// 同梱されているフォントディレクトリの絶対パス
    pub font_dir: PathBuf,

    /// 同梱されている FFmpeg 実行ファイルの絶対パス
    pub ffmpeg_path: PathBuf,

    /// 同梱されている ffprobe 実行ファイルの絶対パス
    pub ffprobe_path: PathBuf,

    /// 環境変数 `EZMM_DEBUG=1` で有効になるデバッグモードフラグ
    pub debug_mode: bool,

    /// ログファイル出力非同期ワーカのガード（drop するとログ停止するため保持）
    #[allow(dead_code)]
    pub(crate) log_guard: WorkerGuard,
}

impl AppState {
    pub fn new(
        font_dir: PathBuf,
        ffmpeg_path: PathBuf,
        ffprobe_path: PathBuf,
        debug_mode: bool,
        log_guard: WorkerGuard,
        settings: AppSettings,
    ) -> Self {
        Self {
            probe_cache: Mutex::new(HashMap::new()),
            ffmpeg_version: Mutex::new(None),
            current_settings: Mutex::new(settings),
            batch_running: Arc::new(Mutex::new(false)),
            cancel_requested: Arc::new(Mutex::new(false)),
            ffmpeg_child: Arc::new(Mutex::new(None)),
            font_dir,
            ffmpeg_path,
            ffprobe_path,
            debug_mode,
            log_guard,
        }
    }
}

