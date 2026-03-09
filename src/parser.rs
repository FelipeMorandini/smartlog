//! Log parsing and styling utilities.
//!
//! This module provides functionality to parse JSON and plain text log entries,
//! detect log levels, extract timestamps, and style them for terminal display
//! with syntax highlighting and theme support.

use crate::theme::Theme;
use chrono::{DateTime, Local, NaiveDateTime};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

/// Log severity level.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Unknown,
}

impl LogLevel {
    /// Returns a numeric severity for ordering (lower = more severe).
    pub fn severity(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Debug => 3,
            Self::Unknown => 4,
        }
    }

    /// Cycles to the next level filter: Error → Warn → Info → Debug → None (wraps).
    pub fn next_filter(self) -> Option<Self> {
        match self {
            Self::Error => Some(Self::Warn),
            Self::Warn => Some(Self::Info),
            Self::Info => Some(Self::Debug),
            Self::Debug => None,
            Self::Unknown => None,
        }
    }

    /// Returns a human-readable label for the level.
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// A parsed log entry with its original text, pretty-printed version, severity level,
/// optional parsed timestamp, and optional source identifier.
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// The original raw log line
    pub raw: String,
    /// Pretty-printed version (formatted JSON or original text)
    pub pretty: String,
    /// Detected log level
    pub level: LogLevel,
    /// Parsed timestamp from the log entry (if detected)
    pub timestamp: Option<DateTime<Local>>,
    /// Source identifier (e.g., filename for multi-file tailing)
    pub source: Option<String>,
}

/// Timestamp field names to look for in JSON log entries.
const JSON_TIMESTAMP_FIELDS: &[&str] =
    &["timestamp", "ts", "time", "@timestamp", "datetime", "date"];

/// Formats for parsing timestamps with timezone information.
const TZ_FORMATS: &[&str] = &[
    // With colon in offset, e.g. 2024-01-01T12:34:56.789+05:30
    "%Y-%m-%dT%H:%M:%S%.f%:z",
    "%Y-%m-%dT%H:%M:%S%:z",
    // Without colon in offset, e.g. 2024-01-01T12:34:56.789+0530
    "%Y-%m-%dT%H:%M:%S%.f%z",
    "%Y-%m-%dT%H:%M:%S%z",
];

/// Formats for parsing timestamps without timezone (assumed local).
const NAIVE_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S",
    "%Y/%m/%d %H:%M:%S%.f",
    "%Y/%m/%d %H:%M:%S",
];

/// Parses a timestamp string into a local DateTime.
///
/// Tries RFC 3339 first, then common formats with timezone, then without.
fn parse_timestamp_str(s: &str) -> Option<DateTime<Local>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Local));
    }
    for fmt in TZ_FORMATS {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Some(dt.with_timezone(&Local));
        }
    }
    for fmt in NAIVE_FORMATS {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return naive.and_local_timezone(Local).earliest();
        }
    }
    None
}

/// Parses a numeric epoch value into a local DateTime.
///
/// Detects seconds vs. milliseconds vs. microseconds based on magnitude.
fn parse_epoch(value: f64) -> Option<DateTime<Local>> {
    let abs = value.abs();
    let seconds = if abs > 1e15 {
        // Microseconds
        value / 1_000_000.0
    } else if abs > 1e12 {
        // Milliseconds
        value / 1000.0
    } else {
        // Seconds
        value
    };

    let mut secs = seconds.floor() as i64;
    let frac = seconds - (secs as f64);
    let mut nanos = (frac * 1_000_000_000.0).round() as u32;

    // Normalize so nanos is always < 1_000_000_000
    if nanos >= 1_000_000_000 {
        let carry = (nanos / 1_000_000_000) as i64;
        secs = secs.checked_add(carry)?;
        nanos %= 1_000_000_000;
    }

    DateTime::from_timestamp(secs, nanos).map(|dt| dt.with_timezone(&Local))
}

/// Extracts a timestamp from a JSON object by checking common field names.
fn extract_json_timestamp(json: &Value) -> Option<DateTime<Local>> {
    for &field in JSON_TIMESTAMP_FIELDS {
        if let Some(ts_value) = json.get(field) {
            let parsed = match ts_value {
                Value::String(s) => parse_timestamp_str(s),
                Value::Number(n) => n.as_f64().and_then(parse_epoch),
                _ => None,
            };
            if parsed.is_some() {
                return parsed;
            }
        }
    }
    None
}

/// Returns the compiled regex for extracting timestamps from plain text log lines.
///
/// Returns `None` if the regex fails to compile (should never happen with a
/// static pattern, but avoids `.expect()` per project convention).
fn plain_text_timestamp_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        // Match hyphen-separated dates with T or space (with optional TZ suffix),
        // slash-separated dates with space only (no TZ — no chrono format for that combo).
        Regex::new(
            r"^((?:\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)|(?:\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?))",
        )
        .ok()
    })
    .as_ref()
}

/// Extracts a timestamp from the beginning of a plain text log line.
fn extract_plain_text_timestamp(line: &str) -> Option<DateTime<Local>> {
    let re = plain_text_timestamp_regex()?;
    let cap = re.captures(line)?;
    parse_timestamp_str(cap.get(1)?.as_str())
}

/// Formats a timestamp as a human-readable relative time string (e.g., "3s ago", "2m ago").
pub fn format_relative_time(dt: DateTime<Local>) -> String {
    let now = Local::now();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();
    if secs < 0 {
        return "future".to_string();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

/// Parses a raw log line into a structured `LogEntry`.
///
/// If the line is valid JSON, it will be pretty-printed and the log level
/// will be extracted from common fields (e.g., "level", "severity").
/// Timestamps are extracted from JSON fields or plain text patterns.
/// Otherwise, the line is treated as plain text and the level is guessed
/// from keywords like "error", "warn", "info".
pub fn parse_log(line: String) -> LogEntry {
    match serde_json::from_str::<Value>(&line) {
        Ok(json) => {
            let level_str = json
                .get("level")
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

            let timestamp = extract_json_timestamp(&json);

            LogEntry {
                raw: line,
                pretty: serde_json::to_string_pretty(&json).unwrap_or_default(),
                level,
                timestamp,
                source: None,
            }
        }
        Err(_) => {
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

            let timestamp = extract_plain_text_timestamp(&line);

            LogEntry {
                raw: line.clone(),
                pretty: line,
                level,
                timestamp,
                source: None,
            }
        }
    }
}

/// Styles a log entry for terminal display with syntax highlighting.
///
/// Colors the log based on its severity level using the provided theme,
/// and highlights any matches to the search query.
///
/// # Arguments
///
/// * `entry` - The log entry to style
/// * `search_query` - Text to highlight (case-insensitive)
/// * `theme` - Color theme to use for styling
pub fn style_log(entry: &LogEntry, search_query: &str, theme: &Theme) -> Line<'static> {
    let base_color = match entry.level {
        LogLevel::Error => theme.error,
        LogLevel::Warn => theme.warn,
        LogLevel::Info => theme.info,
        LogLevel::Debug => theme.debug,
        LogLevel::Unknown => theme.unknown,
    };

    let base_style = Style::default().fg(base_color);

    if search_query.is_empty() {
        return Line::from(Span::styled(entry.pretty.clone(), base_style));
    }

    // Use char-based matching to avoid UTF-8 byte boundary panics.
    let content_chars: Vec<char> = entry.pretty.chars().collect();
    let content_lower: Vec<char> = entry.pretty.to_lowercase().chars().collect();
    let query_lower: Vec<char> = search_query.to_lowercase().chars().collect();

    if content_chars.len() != content_lower.len() || query_lower.is_empty() {
        return Line::from(Span::styled(entry.pretty.clone(), base_style));
    }

    let highlight_style = Style::default()
        .fg(theme.highlight_fg)
        .bg(theme.highlight_bg)
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
    use ratatui::style::Color;

    fn test_entry(pretty: &str, level: LogLevel) -> LogEntry {
        LogEntry {
            raw: pretty.to_string(),
            pretty: pretty.to_string(),
            level,
            timestamp: None,
            source: None,
        }
    }

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

    // --- Timestamp parsing tests ---

    #[test]
    fn test_parse_json_timestamp_rfc3339() {
        let entry = parse_log(
            r#"{"level": "INFO", "timestamp": "2024-06-15T10:30:45Z", "msg": "test"}"#.to_string(),
        );
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_timestamp_ts_field() {
        let entry = parse_log(
            r#"{"level": "INFO", "ts": "2024-06-15T10:30:45.123Z", "msg": "test"}"#.to_string(),
        );
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_timestamp_fallback_to_later_field() {
        // "timestamp" is present but unparseable; "@timestamp" is valid — should use the latter
        let entry = parse_log(
            r#"{"level": "INFO", "timestamp": "n/a", "@timestamp": "2024-06-15T10:30:45Z", "msg": "test"}"#.to_string(),
        );
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_timestamp_at_timestamp_field() {
        let entry = parse_log(
            r#"{"level": "INFO", "@timestamp": "2024-06-15T10:30:45Z", "msg": "test"}"#.to_string(),
        );
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_timestamp_epoch_seconds() {
        let entry =
            parse_log(r#"{"level": "INFO", "timestamp": 1718447445, "msg": "test"}"#.to_string());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_timestamp_epoch_millis() {
        let entry = parse_log(
            r#"{"level": "INFO", "timestamp": 1718447445123, "msg": "test"}"#.to_string(),
        );
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_json_no_timestamp() {
        let entry = parse_log(r#"{"level": "INFO", "msg": "no time"}"#.to_string());
        assert!(entry.timestamp.is_none());
    }

    #[test]
    fn test_parse_plain_text_iso_timestamp() {
        let entry = parse_log("2024-06-15T10:30:45 INFO server started".to_string());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_plain_text_space_timestamp() {
        let entry = parse_log("2024-06-15 10:30:45 ERROR disk full".to_string());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_plain_text_no_timestamp() {
        let entry = parse_log("just a plain log line".to_string());
        assert!(entry.timestamp.is_none());
    }

    #[test]
    fn test_parse_plain_text_slash_t_not_matched() {
        // Slash-separated date with T separator has no chrono parser, should not match
        let entry = parse_log("2024/06/15T10:30:45Z some log".to_string());
        assert!(entry.timestamp.is_none());
    }

    #[test]
    fn test_parse_plain_text_slash_space_timestamp() {
        // Slash-separated date with space separator should parse
        let entry = parse_log("2024/06/15 10:30:45 some log".to_string());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_plain_text_slash_tz_stripped() {
        // Slash-separated regex branch excludes TZ suffix, so "Z" is not captured.
        // The remaining "2024/06/15 10:30:45" parses as a naive local time.
        let entry = parse_log("2024/06/15 10:30:45Z some log".to_string());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_rfc3339() {
        let result = parse_timestamp_str("2024-06-15T10:30:45Z");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_with_offset() {
        let result = parse_timestamp_str("2024-06-15T10:30:45+05:30");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_with_offset_no_colon() {
        let result = parse_timestamp_str("2024-06-15T10:30:45+0530");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_with_offset_no_colon_fractional() {
        let result = parse_timestamp_str("2024-06-15T10:30:45.123+0530");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_naive_datetime() {
        let result = parse_timestamp_str("2024-06-15 10:30:45");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_str_invalid() {
        let result = parse_timestamp_str("not a timestamp");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_epoch_seconds() {
        let result = parse_epoch(1718447445.0);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_epoch_negative_fractional() {
        // -1.2 seconds before epoch should produce a valid timestamp
        let result = parse_epoch(-1.2);
        assert!(result.is_some());
        let dt = result.unwrap();
        // Should be 1969-12-31T23:59:58.8 UTC (epoch - 1.2s)
        let utc = dt.with_timezone(&chrono::Utc);
        assert_eq!(utc.timestamp(), -2);
        assert_eq!(utc.timestamp_subsec_nanos() / 100_000_000, 8); // ~800ms
    }

    #[test]
    fn test_parse_epoch_millis() {
        let result = parse_epoch(1718447445123.0);
        assert!(result.is_some());
    }

    // --- format_relative_time tests ---

    #[test]
    fn test_format_relative_seconds() {
        let dt = Local::now() - chrono::Duration::seconds(30);
        let result = format_relative_time(dt);
        assert!(result.contains("s ago"), "Expected seconds, got: {result}");
    }

    #[test]
    fn test_format_relative_minutes() {
        let dt = Local::now() - chrono::Duration::minutes(5);
        let result = format_relative_time(dt);
        assert!(result.contains("m ago"), "Expected minutes, got: {result}");
    }

    #[test]
    fn test_format_relative_hours() {
        let dt = Local::now() - chrono::Duration::hours(3);
        let result = format_relative_time(dt);
        assert!(result.contains("h ago"), "Expected hours, got: {result}");
    }

    #[test]
    fn test_format_relative_days() {
        let dt = Local::now() - chrono::Duration::days(2);
        let result = format_relative_time(dt);
        assert!(result.contains("d ago"), "Expected days, got: {result}");
    }

    #[test]
    fn test_format_relative_future() {
        let dt = Local::now() + chrono::Duration::hours(1);
        let result = format_relative_time(dt);
        assert_eq!(result, "future");
    }

    // --- style_log tests ---

    #[test]
    fn test_style_log_no_query_returns_single_span() {
        let entry = test_entry("test", LogLevel::Info);
        let line = style_log(&entry, "", &Theme::DARK);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_style_log_highlight_match() {
        let entry = test_entry("hello world", LogLevel::Unknown);
        let line = style_log(&entry, "world", &Theme::DARK);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[1].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_case_insensitive_highlight() {
        let entry = test_entry("Hello World", LogLevel::Unknown);
        let line = style_log(&entry, "HELLO", &Theme::DARK);
        assert!(!line.spans.is_empty());
        assert_eq!(line.spans[0].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_no_match() {
        let entry = test_entry("hello", LogLevel::Info);
        let line = style_log(&entry, "xyz", &Theme::DARK);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_style_log_unicode_no_panic() {
        let entry = test_entry("Hallo Welt! Schon", LogLevel::Info);
        let _line = style_log(&entry, "welt", &Theme::DARK);
    }

    #[test]
    fn test_style_log_multiple_matches() {
        let entry = test_entry("abcabc", LogLevel::Unknown);
        let line = style_log(&entry, "abc", &Theme::DARK);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].style.bg, Some(Color::Cyan));
        assert_eq!(line.spans[1].style.bg, Some(Color::Cyan));
    }

    #[test]
    fn test_style_log_error_color() {
        let entry = test_entry("err", LogLevel::Error);
        let line = style_log(&entry, "", &Theme::DARK);
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_style_log_warn_color() {
        let entry = test_entry("w", LogLevel::Warn);
        let line = style_log(&entry, "", &Theme::DARK);
        assert_eq!(line.spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_style_log_debug_color() {
        let entry = test_entry("d", LogLevel::Debug);
        let line = style_log(&entry, "", &Theme::DARK);
        assert_eq!(line.spans[0].style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_style_log_with_light_theme() {
        let entry = test_entry("test", LogLevel::Unknown);
        let line = style_log(&entry, "", &Theme::LIGHT);
        assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_style_log_highlight_with_dracula_theme() {
        let entry = test_entry("hello world", LogLevel::Info);
        let line = style_log(&entry, "world", &Theme::DRACULA);
        assert_eq!(line.spans[1].style.bg, Some(Color::LightMagenta));
    }

    // --- LogLevel method tests ---

    #[test]
    fn test_severity_ordering() {
        assert!(LogLevel::Error.severity() < LogLevel::Warn.severity());
        assert!(LogLevel::Warn.severity() < LogLevel::Info.severity());
        assert!(LogLevel::Info.severity() < LogLevel::Debug.severity());
        assert!(LogLevel::Debug.severity() < LogLevel::Unknown.severity());
    }

    #[test]
    fn test_next_filter_cycle() {
        assert_eq!(LogLevel::Error.next_filter(), Some(LogLevel::Warn));
        assert_eq!(LogLevel::Warn.next_filter(), Some(LogLevel::Info));
        assert_eq!(LogLevel::Info.next_filter(), Some(LogLevel::Debug));
        assert_eq!(LogLevel::Debug.next_filter(), None);
        assert_eq!(LogLevel::Unknown.next_filter(), None);
    }

    #[test]
    fn test_level_labels() {
        assert_eq!(LogLevel::Error.label(), "ERROR");
        assert_eq!(LogLevel::Warn.label(), "WARN");
        assert_eq!(LogLevel::Info.label(), "INFO");
        assert_eq!(LogLevel::Debug.label(), "DEBUG");
        assert_eq!(LogLevel::Unknown.label(), "UNKNOWN");
    }
}
