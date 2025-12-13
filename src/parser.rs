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
            let level = if let Some(lvl) = json.get("level").and_then(|v| v.as_str()) {
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
            let level = if lower.contains("error") {
                LogLevel::Error
            } else if lower.contains("warn") {
                LogLevel::Warn
            } else if lower.contains("info") {
                LogLevel::Info
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
pub fn style_log<'a>(entry: &'a LogEntry, search_query: &str) -> Line<'a> {
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
        return Line::from(Span::styled(&entry.pretty, base_style));
    }

    // Highlighting Logic
    let mut spans = Vec::new();
    let content = &entry.pretty;
    let lower_content = content.to_lowercase();
    let lower_query = search_query.to_lowercase();

    let mut last_idx = 0;

    for (idx, _) in lower_content.match_indices(&lower_query) {
        // Push text before the match
        if idx > last_idx {
            spans.push(Span::styled(&content[last_idx..idx], base_style));
        }

        // Push the match (Highlighted)
        spans.push(Span::styled(
            &content[idx..idx + lower_query.len()],
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

        last_idx = idx + lower_query.len();
    }

    // Push remaining text
    if last_idx < content.len() {
        spans.push(Span::styled(&content[last_idx..], base_style));
    }

    Line::from(spans)
}