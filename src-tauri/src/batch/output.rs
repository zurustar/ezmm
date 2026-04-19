//! 出力ファイルパス生成と衝突チェック

use std::path::{Path, PathBuf};

/// エントリ名・フォルダ・フォーマットから出力ファイルパスを生成する。
///
/// # 例
/// ```
/// let path = build_output_path("/out", "tanaka", "mp4");
/// assert_eq!(path, PathBuf::from("/out/tanaka.mp4"));
/// ```
pub fn build_output_path(output_folder: &str, entry_name: &str, format: &str) -> PathBuf {
    Path::new(output_folder)
        .join(format!("{}.{}", entry_name, format))
}

/// 与えられたエントリ名一覧に対し、既に `output_folder` に存在する出力ファイル名を返す。
///
/// 返り値は `"entry_name.format"` 形式のファイル名文字列のリスト（パスではない）。
pub fn check_output_conflicts(
    output_folder: &str,
    entry_names: &[&str],
    format: &str,
) -> Vec<String> {
    entry_names
        .iter()
        .filter_map(|name| {
            let path = build_output_path(output_folder, name, format);
            if path.exists() {
                Some(format!("{}.{}", name, format))
            } else {
                None
            }
        })
        .collect()
}

// ─────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── サイクル 3-1: 出力ファイルパス生成 ────────────────────────────────

    #[test]
    fn build_output_path_basic() {
        let path = build_output_path("/out", "tanaka", "mp4");
        assert_eq!(path, PathBuf::from("/out/tanaka.mp4"));
    }

    #[test]
    fn build_output_path_webm() {
        let path = build_output_path("/videos/output", "suzuki", "webm");
        assert_eq!(path, PathBuf::from("/videos/output/suzuki.webm"));
    }

    #[test]
    fn build_output_path_trailing_slash_folder() {
        // Path::join はトレイリングスラッシュに依存しない
        let path = build_output_path("/out/", "yamada", "mp4");
        assert_eq!(path, PathBuf::from("/out/yamada.mp4"));
    }

    // ── サイクル 3-2: 出力ファイル衝突チェック ────────────────────────────

    #[test]
    fn check_output_conflicts_file_exists() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().to_str().unwrap();
        fs::write(dir.path().join("tanaka.mp4"), b"").unwrap();

        let conflicts = check_output_conflicts(folder, &["tanaka", "suzuki"], "mp4");
        assert_eq!(conflicts, vec!["tanaka.mp4"]);
    }

    #[test]
    fn check_output_conflicts_no_file() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().to_str().unwrap();

        let conflicts = check_output_conflicts(folder, &["tanaka", "suzuki"], "mp4");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn check_output_conflicts_multiple_files() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().to_str().unwrap();
        fs::write(dir.path().join("tanaka.mp4"), b"").unwrap();
        fs::write(dir.path().join("suzuki.mp4"), b"").unwrap();

        let mut conflicts = check_output_conflicts(folder, &["tanaka", "suzuki", "yamada"], "mp4");
        conflicts.sort();
        assert_eq!(conflicts, vec!["suzuki.mp4", "tanaka.mp4"]);
    }
}
