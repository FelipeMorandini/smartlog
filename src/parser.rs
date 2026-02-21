//! Log parsing and styling utilities.
//!
//! This module provides functionality to parse JSON and plain text log entries,
//! detect log levels, and style them for terminal display with syntax highlighting.

use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Line, Span};
use serde_json::Value;

/// Log severity level.
#[derive(PartialEq, Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Unknown,
}

/// A parsed log entry with its original text, pretty-printed version, and severity level.
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// The original raw log line
    pub raw: String,
    /// Pretty-printed version (formatted JSON or original text)
    pub pretty: String,
    /// Detected log level
    pub level: LogLevel,
}

/// Parses a raw log line into a structured `LogEntry`.
///
/// If the line is valid JSON, it will be pretty-printed and the log level
/// will be extracted from common fields (e.g., "level", "severity").
/// Otherwise, the line is treated as plain text and the level is guessed
/// from keywords like "error", "warn", "info".
pub fn parse_log(line: String) -> LogEntry {
    match serde_json::from_str::<Value>(&line) {
        Ok(json) => {
            // Try to guess the level from common JSON fields
            let level_str = json.get("level")
                .or_else(|| json.get("severity"))
                .or_else(|| json.get("lvl"))
                .and_then(|v| v.as_str());

            let level = if let Some(lvl) = level_str {
                match lvl.to_lowercase().as_str() {
                    "error" | "err" | "fatal" => LogLevel::Error,
                    "warn" | "warning" => LogLevel::Warn,
                    "info" | "information" => LogLevel::Info,
                    "debug" | "trace" => LogLevel::Debug,
                    _ => LogLevel::Unknown,
                }
            } else {
                LogLevel::Unknown
            };

            LogEntry {
                raw: line,
                pretty: serde_json::to_string_pretty(&json).unwrap_or_default(),
                level,
            }
        }
        Err(_) => {
            // Check for plain text keywords if not JSON
            let lower = line.to_lowercase();
            let level = if lower.contains("error") || lower.contains("fatal") {
                LogLevel::Error
            } else if lower.contains("warn") {
                LogLevel::Warn
            } else if lower.contains("info") {
                LogLevel::Info
            } else if lower.contains("debug") || lower.contains("trace") {
                LogLevel::Debug
            } else {
                LogLevel::Unknown
            };

            LogEntry {
                raw: line.clone(),
                pretty: line,
                level,
            }
        }
    }
}

/// Styles a log entry for terminal display with syntax highlighting.
///
/// Colors the log based on its severity level and highlights any matches
/// to the search query with a cyan background.
///
/// # Arguments
///
/// * `entry` - The log entry to style
/// * `search_query` - Text to highlight (case-insensitive)
pub fn style_log(entry: &LogEntry, search_query: &str) -> Line<'static> {
    let base_color = match entry.level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::Green,
        LogLevel::Debug => Color::Blue,
        LogLevel::Unknown => Color::White,
    };

    let base_style = Style::default().fg(base_color);

    // If no search query, return the whole line colored
    if search_query.is_empty() {
        return Line::from(Span::styled(entry.pretty.clone(), base_style));
    }

    // Use char-based matching to avoid UTF-8 byte boundary panics.
    // Lowercasing can change byte lengths for certain Unicode chars,
    // so we work with char slices instead of byte slices.
    let content_chars: Vec<char> = entry.pretty.chars().collect();
    let content_lower: Vec<char> = entry.pretty.to_lowercase().chars().collect();
    let query_lower: Vec<char> = search_query.to_lowercase().chars().collect();

    // If lowercasing changed char count (rare Unicode edge case), skip highlighting
    if content_chars.len() != content_lower.len() || query_lower.is_empty() {
        return Line::from(Span::styled(entry.pretty.clone(), base_style));
    }

    let highlight_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();
    let mut last_end: usize = 0;
    let mut i: usize = 0;

    while i + query_lower.len() <= content_lower.len() {
        if content_lower[i..i + query_lower.len()] == query_lower[..] {
            if i > last_end {
                let before: String = content_chars[last_end..i].iter().collect();
                spans.push(Span::styled(before, base_style));
            }
            let matched: String = content_chars[i..i + query_lower.len()].iter().collect();
            spans.push(Span::styled(matched, highlight_style));
            last_end = i + query_lower.len();
            i = last_end;
        } else {
            i += 1;
        }
    }

    if last_end < content_chars.len() {
        let remaining: String = content_chars[last_end..].iter().collect();
        spans.push(Span::styled(remaining, base_style));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_log tests ---

    #[test]
    fn test_parse_json_level_field() {
        let entry = parse_log(r#"{"level": "ERROR", "msg": "fail"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Error);
        assert!(entry.pretty.contains("ERROR"));
    }

    #[test]
    fn test_parse_json_severity_field() {
        let entry = parse_log(r#"{"severity": "WARN", "msg": "caution"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_json_lvl_field() {
        let entry = parse_log(r#"{"lvl": "info", "msg": "hello"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_parse_json_debug_trace_levels() {
        let debug = parse_log(r#"{"level": "debug", "msg": "d"}"#.to_string());
        assert_eq!(debug.level, LogLevel::Debug);
        let trace = parse_log(r#"{"level": "trace", "msg": "t"}"#.to_string());
        assert_eq!(trace.level, LogLevel::Debug);
    }

    #[test]
    fn test_parse_json_fatal_level() {
        let entry = parse_log(r#"{"level": "fatal", "msg": "crash"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Error);
    }

    #[test]
    fn test_parse_json_unknown_level() {
        let entry = parse_log(r#"{"level": "verbose", "msg": "v"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Unknown);
    }

    #[test]
    fn test_parse_json_no_level_field() {
        let entry = parse_log(r#"{"msg": "no level"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Unknown);
    }

    #[test]
    fn test_parse_json_level_priority_over_severity() {
        let entry = parse_log(r#"{"level": "ERROR", "severity": "INFO"}"#.to_string());
        assert_eq!(entry.level, LogLevel::Error);
    }

    #[test]
    fn test_parse_plain_text_error() {
        let entry = parse_log("2024-01-01 ERROR something broke".to_string());
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.pretty, "2024-01-01 ERROR something broke");
    }

    #[test]
    fn test_parse_plain_text_fatal() {
        let entry = parse_log("FATAL: system down".to_string());
        assert_eq!(entry.level, LogLevel::Error);
    }

    #[test]
    fn test_parse_plain_text_warn() {
        let entry = parse_log("WARNING: disk usage high".to_string());
        assert_eq!(entry.level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_plain_text_info() {
        let entry = parse_log("info: server started".to_string());
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_parse_plain_text_debug() {
        let entry = parse_log("DEBUG: variable x = 42".to_string());
        assert_eq!(entry.level, LogLevel::Debug);
    }

    #[test]
    fn test_parse_plain_text_trace() {
        let entry = parse_log("TRACE entering function foo".to_string());
        assert_eq!(entry.level, LogLevel::Debug);
    }

    #[test]
    fn test_parse_plain_text_unknown() {
        let entry = parse_log("just a regular line".to_string());
        assert_eq!(entry.level, LogLevel::Unknown);
    }

    // --- style_log tests ---

    #[test]
    fn test_style_log_no_query_returns_single_span() {
        let entry = LogEntry {
            raw: "test".to_string(),
            pretty: "test".to_string(),
            level: LogLevel::Info,
        };
        let line = style_log(&entry, "");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_style_log_highlight_match() {
        let entry = LogEntry {
            raw: "hello world".to_string(),
            pretty: "hello world".to_string(),
            level: LogLevel::Unknown,
        };
        let line = style_log(&entry, "world");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[1].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_case_insensitive_highlight() {
        let entry = LogEntry {
            raw: "Hello World".to_string(),
            pretty: "Hello World".to_string(),
            level: LogLevel::Unknown,
        };
        let line = style_log(&entry, "HELLO");
        assert!(line.spans.len() >= 1);
        assert_eq!(line.spans[0].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_no_match() {
        let entry = LogEntry {
            raw: "hello".to_string(),
            pretty: "hello".to_string(),
            level: LogLevel::Info,
        };
        let line = style_log(&entry, "xyz");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_style_log_unicode_no_panic() {
        let entry = LogEntry {
            raw: "Hallo Welt! Schon".to_string(),
            pretty: "Hallo Welt! Schon".to_string(),
            level: LogLevel::Info,
        };
        let _line = style_log(&entry, "welt");
    }

    #[test]
    fn test_style_log_multiple_matches() {
        let entry = LogEntry {
            raw: "abcabc".to_string(),
            pretty: "abcabc".to_string(),
            level: LogLevel::Unknown,
        };
        let line = style_log(&entry, "abc");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].style.bg, Some(Color::Cyan));
        assert_eq!(line.spans[1].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_error_color() {
        let entry = LogEntry {
            raw: "err".to_string(),
            pretty: "err".to_string(),
            level: LogLevel::Error,
        };
        let line = style_log(&entry, "");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_style_log_warn_color() {
        let entry = LogEntry {
            raw: "w".to_string(),
            pretty: "w".to_string(),
            level: LogLevel::Warn,
        };
        let line = style_log(&entry, "");
        assert_eq!(line.spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_style_log_debug_color() {
        let entry = LogEntry {
            raw: "d".to_string(),
            pretty: "d".to_string(),
            level: LogLevel::Debug,
        };
        let line = style_log(&entry, "");
        assert_eq!(line.spans[0].style.fg, Some(Color::Blue));
    }
}