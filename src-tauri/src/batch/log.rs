//! バッチログファイル名生成

use std::path::{Path, PathBuf};

/// タイムスタンプ（`YYYYMMDD-HHMMSS` 文字列）からログファイル名を生成する。
///
/// # 例
/// ```
/// # use ezmm_lib::batch::log::log_filename;
/// let name = log_filename("20260419-134500");
/// assert_eq!(name, "ezmm-20260419-134500.log");
/// ```
pub fn log_filename(timestamp: &str) -> String {
    format!("ezmm-{}.log", timestamp)
}

/// `output_folder` とタイムスタンプからログファイルの完全パスを返す。
pub fn log_file_path(output_folder: &str, timestamp: &str) -> PathBuf {
    Path::new(output_folder).join(log_filename(timestamp))
}

// ─────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── サイクル 3-3: バッチログファイル名生成 ────────────────────────────

    #[test]
    fn log_filename_format() {
        let name = log_filename("20260419-134500");
        assert_eq!(name, "ezmm-20260419-134500.log");
    }

    #[test]
    fn log_filename_matches_pattern() {
        // 形式 ezmm-YYYYMMDD-HHMMSS.log を正規表現で検証
        let ts = "20261231-235959";
        let name = log_filename(ts);
        let re = regex::Regex::new(r"^ezmm-\d{8}-\d{6}\.log$").unwrap();
        assert!(re.is_match(&name), "ファイル名が形式に一致しない: {name}");
    }

    #[test]
    fn log_file_path_joins_correctly() {
        let path = log_file_path("/out", "20260419-134500");
        assert_eq!(path, std::path::PathBuf::from("/out/ezmm-20260419-134500.log"));
    }
}
