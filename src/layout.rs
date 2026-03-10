//! Layout measurement helpers shared between input handling and UI rendering.
//!
//! This module provides text measurement functions used by both `inputs.rs`
//! (for page scrolling calculations) and `ui.rs` (for auto-scroll and rendering).
//! Extracting them here decouples input handling from the UI layer.

use crate::parser::{format_relative_time, LogEntry};
use chrono::{DateTime, Local};
use unicode_width::UnicodeWidthStr;

/// Computes the number of visual lines a text occupies when wrapped to a given width.
///
/// Uses `unicode-width` for accurate display width (CJK, wide glyphs).
/// `prefix_width` is the display width of metadata prepended to the first line
/// (e.g., timestamp and source labels). Only the first line is affected.
pub(crate) fn compute_visual_lines(text: &str, width: usize, prefix_width: usize) -> usize {
    if width == 0 {
        return text.lines().count().max(1);
    }
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            let display_w = UnicodeWidthStr::width(line);
            let total = if i == 0 {
                display_w + prefix_width
            } else {
                display_w
            };
            if total == 0 {
                1
            } else {
                total.div_ceil(width)
            }
        })
        .sum::<usize>()
        .max(1)
}

/// Counts the number of newline-separated lines in the text (no wrapping).
pub(crate) fn compute_raw_lines(text: &str) -> usize {
    text.lines().count().max(1)
}

/// Computes the display width of metadata prefix spans for a log entry.
///
/// This accounts for the `[timestamp] ` and `[source] ` prefixes that
/// `prepend_metadata` inserts before the log content. Accepts `now` so
/// the caller can batch the `Local::now()` call once per frame.
pub(crate) fn metadata_prefix_display_width(
    entry: &LogEntry,
    show_timestamps: bool,
    now: DateTime<Local>,
) -> usize {
    let mut width = 0;
    if let Some(ref src) = entry.source {
        // Prefix format: "[{src}] " -> '[' + src + ']' + ' '
        width += 3 + UnicodeWidthStr::width(&**src);
    }
    if show_timestamps {
        if let Some(ts) = entry.timestamp {
            let relative = format_relative_time(ts, now);
            // Prefix format: "[{relative}] " -> '[' + relative + ']' + ' '
            width += 3 + UnicodeWidthStr::width(relative.as_str());
        }
    }
    width
}

/// Computes how many visual lines a single entry occupies.
///
/// When `line_wrap` is true, uses unicode-width wrapping with metadata prefix
/// width; otherwise counts raw newline-separated lines. This is the shared
/// helper that eliminates duplication between paging and auto-scroll logic.
pub(crate) fn entry_visual_lines(
    entry: &LogEntry,
    width: usize,
    line_wrap: bool,
    show_timestamps: bool,
    now: DateTime<Local>,
) -> usize {
    if line_wrap {
        let pw = metadata_prefix_display_width(entry, show_timestamps, now);
        compute_visual_lines(&entry.pretty, width, pw)
    } else {
        compute_raw_lines(&entry.pretty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LogLevel;
    use std::sync::Arc;

    fn now() -> DateTime<Local> {
        Local::now()
    }

    fn entry(pretty: &str) -> LogEntry {
        LogEntry {
            raw: pretty.to_string(),
            pretty: pretty.to_string(),
            level: LogLevel::Info,
            timestamp: None,
            source: None,
        }
    }

    // --- compute_visual_lines tests ---

    #[test]
    fn test_visual_lines_single_short_line() {
        assert_eq!(compute_visual_lines("hello", 80, 0), 1);
    }

    #[test]
    fn test_visual_lines_exact_width() {
        assert_eq!(compute_visual_lines("abcde", 5, 0), 1);
    }

    #[test]
    fn test_visual_lines_wraps_once() {
        assert_eq!(compute_visual_lines("abcdef", 5, 0), 2);
    }

    #[test]
    fn test_visual_lines_multiline_text() {
        assert_eq!(compute_visual_lines("aaa\nbbb\nccc", 80, 0), 3);
    }

    #[test]
    fn test_visual_lines_multiline_with_wrapping() {
        assert_eq!(compute_visual_lines("abcdef\ngh", 5, 0), 3);
    }

    #[test]
    fn test_visual_lines_empty_string() {
        assert_eq!(compute_visual_lines("", 80, 0), 1);
    }

    #[test]
    fn test_visual_lines_zero_width() {
        assert_eq!(compute_visual_lines("abc\ndef", 0, 0), 2);
    }

    #[test]
    fn test_visual_lines_pretty_json() {
        let json = "{\n  \"level\": \"ERROR\",\n  \"msg\": \"fail\"\n}";
        assert_eq!(compute_visual_lines(json, 40, 0), 4);
    }

    // --- unicode width tests ---

    #[test]
    fn test_visual_lines_cjk_double_width() {
        assert_eq!(compute_visual_lines("你好世界呀", 10, 0), 1);
        assert_eq!(compute_visual_lines("你好世界呀", 6, 0), 2);
    }

    #[test]
    fn test_visual_lines_mixed_ascii_cjk() {
        assert_eq!(compute_visual_lines("hi你好", 6, 0), 1);
        assert_eq!(compute_visual_lines("hi你好", 5, 0), 2);
    }

    // --- prefix width tests ---

    #[test]
    fn test_visual_lines_with_prefix_causes_wrap() {
        assert_eq!(compute_visual_lines("abcde", 5, 0), 1);
        assert_eq!(compute_visual_lines("abcde", 5, 3), 2);
    }

    #[test]
    fn test_visual_lines_prefix_only_affects_first_line() {
        assert_eq!(compute_visual_lines("abcde\nfgh", 5, 3), 3);
        assert_eq!(compute_visual_lines("abcde\nfgh", 5, 0), 2);
    }

    // --- metadata_prefix_display_width tests ---

    #[test]
    fn test_metadata_prefix_display_width_no_metadata() {
        let e = entry("test");
        assert_eq!(metadata_prefix_display_width(&e, false, now()), 0);
    }

    #[test]
    fn test_metadata_prefix_display_width_source_only() {
        let mut e = entry("test");
        e.source = Some(Arc::from("app.log"));
        assert_eq!(metadata_prefix_display_width(&e, false, now()), 10);
    }

    #[test]
    fn test_metadata_prefix_display_width_timestamp_only() {
        let mut e = entry("test");
        e.timestamp = Some(chrono::Local::now() - chrono::Duration::seconds(5));
        let w = metadata_prefix_display_width(&e, true, now());
        assert!(w > 0);
    }

    #[test]
    fn test_metadata_prefix_display_width_timestamp_disabled() {
        let mut e = entry("test");
        e.timestamp = Some(chrono::Local::now());
        assert_eq!(metadata_prefix_display_width(&e, false, now()), 0);
    }

    // --- compute_raw_lines tests ---

    #[test]
    fn test_raw_lines_single() {
        assert_eq!(compute_raw_lines("hello"), 1);
    }

    #[test]
    fn test_raw_lines_multiline() {
        assert_eq!(compute_raw_lines("a\nb\nc"), 3);
    }

    #[test]
    fn test_raw_lines_empty() {
        assert_eq!(compute_raw_lines(""), 1);
    }

    // --- entry_visual_lines tests ---

    #[test]
    fn test_entry_visual_lines_wrap_on() {
        let e = entry("abcdefghij"); // 10 chars
        assert_eq!(entry_visual_lines(&e, 5, true, false, now()), 2);
    }

    #[test]
    fn test_entry_visual_lines_wrap_off() {
        let e = entry("abcdefghij");
        assert_eq!(entry_visual_lines(&e, 5, false, false, now()), 1);
    }

    #[test]
    fn test_entry_visual_lines_with_source_prefix() {
        let mut e = entry("abcdefghij"); // 10 chars
        e.source = Some(Arc::from("app.log")); // adds 10 display width prefix
                                               // wrap on, width=20: 10+10=20 → ceil(20/20) = 1 line
        assert_eq!(entry_visual_lines(&e, 20, true, false, now()), 1);
        // wrap on, width=15: 10+10=20 → ceil(20/15) = 2 lines
        assert_eq!(entry_visual_lines(&e, 15, true, false, now()), 2);
    }

    #[test]
    fn test_entry_visual_lines_multiline_wrap_off() {
        let e = entry("a\nb\nc");
        assert_eq!(entry_visual_lines(&e, 80, false, false, now()), 3);
    }

    #[test]
    fn test_entry_visual_lines_multiline_wrap_on() {
        let e = entry("a\nb\nc");
        assert_eq!(entry_visual_lines(&e, 80, true, false, now()), 3);
    }
}
