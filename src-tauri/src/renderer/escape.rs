// escape.rs — FFmpeg filter_complex / drawtext 文字列エスケープ

/// `drawtext` フィルタの `text=` 値に埋め込むテキストをエスケープする（2段階）。
///
/// **drawtext レベル**:
/// - `\` → `\\`
/// - `'` → `'\''`（シングルクォート終端 + エスケープ + 再開）
/// - `:` → `\:`
/// - `%` → `%%`
///
/// **filter_complex レベル**（drawtext 値は単一引数文字列内に存在する）:
/// - `,` → `\,`
/// - `[` → `\[`
/// - `]` → `\]`
pub fn escape_drawtext_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '\'' => out.push_str(r"'\''"),
            ':' => out.push_str(r"\:"),
            '%' => out.push_str("%%"),
            ',' => out.push_str(r"\,"),
            '[' => out.push_str(r"\["),
            ']' => out.push_str(r"\]"),
            other => out.push(other),
        }
    }
    out
}

/// `filter_complex` 内に埋め込むファイルパスやその他の文字列をエスケープする。
///
/// エスケープ対象: `\` `'` `,` `[` `]`
pub fn escape_filter_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '\'' => out.push_str(r"'\''"),
            ',' => out.push_str(r"\,"),
            '[' => out.push_str(r"\["),
            ']' => out.push_str(r"\]"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- escape_drawtext_value ----

    #[test]
    fn drawtext_escapes_backslash() {
        assert_eq!(escape_drawtext_value(r"a\b"), r"a\\b");
    }

    #[test]
    fn drawtext_escapes_single_quote() {
        assert_eq!(escape_drawtext_value("it's"), r"it'\''s");
    }

    #[test]
    fn drawtext_escapes_colon() {
        assert_eq!(escape_drawtext_value("a:b"), r"a\:b");
    }

    #[test]
    fn drawtext_escapes_percent() {
        assert_eq!(escape_drawtext_value("50%"), "50%%");
    }

    #[test]
    fn drawtext_escapes_comma() {
        assert_eq!(escape_drawtext_value("a,b"), r"a\,b");
    }

    #[test]
    fn drawtext_escapes_brackets() {
        assert_eq!(escape_drawtext_value("[tag]"), r"\[tag\]");
    }

    #[test]
    fn drawtext_plain_text_unchanged() {
        assert_eq!(escape_drawtext_value("Hello World"), "Hello World");
    }

    #[test]
    fn drawtext_combined_special_chars() {
        // バックスラッシュ + コロン + シングルクォート の複合
        assert_eq!(escape_drawtext_value(r"a\:b'c"), r"a\\\:b'\''c");
    }

    // ---- escape_filter_value ----

    #[test]
    fn filter_escapes_comma() {
        assert_eq!(escape_filter_value("a,b"), r"a\,b");
    }

    #[test]
    fn filter_escapes_brackets() {
        assert_eq!(escape_filter_value("[in]"), r"\[in\]");
    }

    #[test]
    fn filter_plain_path_unchanged() {
        assert_eq!(
            escape_filter_value("/Users/foo/bar.mp4"),
            "/Users/foo/bar.mp4"
        );
    }

    #[test]
    fn filter_path_with_spaces_unchanged() {
        // スペースはエスケープ不要（引数配列渡しのため）
        assert_eq!(
            escape_filter_value("/path/to/my video.mp4"),
            "/path/to/my video.mp4"
        );
    }
}
