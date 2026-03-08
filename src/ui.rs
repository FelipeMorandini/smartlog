//! Terminal UI rendering.
//!
//! This module handles all the terminal UI rendering using the Ratatui library.

use crate::app::{App, InputMode};
use crate::parser::{style_log, LogEntry};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Computes the number of visual lines a text occupies when wrapped to a given width.
///
/// Each line of the text is wrapped at `width` characters. Empty lines count as 1.
fn compute_visual_lines(text: &str, width: usize) -> usize {
    if width == 0 {
        return text.lines().count().max(1);
    }
    text.lines()
        .map(|line| {
            let len = line.chars().count();
            if len == 0 {
                1
            } else {
                len.div_ceil(width)
            }
        })
        .sum::<usize>()
        .max(1)
}

/// Counts the number of newline-separated lines in the text (no wrapping).
fn compute_raw_lines(text: &str) -> usize {
    text.lines().count().max(1)
}

/// Finds the index of the first entry to display for auto-scroll.
///
/// Iterates entries in reverse, summing visual lines until the viewport is full.
/// When `line_wrap` is true, uses character-based wrapping; otherwise counts raw lines.
/// Returns the index of the first entry that should be visible. Always ensures
/// at least the last entry is shown, even if it exceeds the viewport height.
fn compute_auto_scroll_entry(
    entries: &[&LogEntry],
    viewport_height: usize,
    viewport_width: usize,
    line_wrap: bool,
) -> usize {
    if entries.is_empty() || viewport_height == 0 {
        return 0;
    }

    let last_index = entries.len() - 1;
    let mut lines_used = 0usize;
    for (i, entry) in entries.iter().enumerate().rev() {
        let entry_lines = if line_wrap {
            compute_visual_lines(&entry.pretty, viewport_width)
        } else {
            compute_raw_lines(&entry.pretty)
        };
        lines_used += entry_lines;
        if lines_used > viewport_height {
            return (i + 1).min(last_index);
        }
    }
    0
}

/// Builds the status bar title string for the input bar.
///
/// When an export feedback message is present, it overrides the normal status
/// display. Otherwise shows scroll state, wrap, level, log count, regex mode,
/// and source label.
fn build_status_title(app: &App, shown: usize, total: usize) -> String {
    // Export feedback takes priority over normal status
    if let Some(ref msg) = app.last_export_message {
        return format!(" {msg} ");
    }

    let status = if app.auto_scroll {
        "FOLLOWING"
    } else {
        "PAUSED"
    };
    let wrap = if app.line_wrap { "WRAP" } else { "NOWRAP" };
    let level = match app.min_log_level {
        Some(l) => format!("≥{}", l.label()),
        None => "ALL".to_string(),
    };
    let regex = if app.use_regex {
        if app.is_regex_invalid() {
            " | INVALID REGEX"
        } else {
            " | REGEX"
        }
    } else {
        ""
    };
    let source = if app.source_label.is_empty() {
        String::new()
    } else {
        format!(" | {}", app.source_label)
    };

    if shown == total {
        format!(" / filter | {status} | {wrap} | {level} | {total} logs{regex}{source} ")
    } else {
        format!(" / filter | {status} | {wrap} | {level} | {shown}/{total} logs{regex}{source} ")
    }
}

/// Renders the application UI to the terminal.
///
/// The UI consists of two sections:
/// - Main log area with filtered and styled logs
/// - Input bar showing the current search query and mode
pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(f.area());

    // In regex mode, skip substring highlighting (regex highlight is tracked in TD-7)
    let highlight_query = if app.use_regex { "" } else { &app.input_buffer };
    let filtered_logs = app.get_filtered_logs();
    let viewport_height = chunks[0].height.saturating_sub(2) as usize;
    let viewport_width = chunks[0].width.saturating_sub(2) as usize;

    let scroll_entry = if app.auto_scroll {
        compute_auto_scroll_entry(
            &filtered_logs,
            viewport_height,
            viewport_width,
            app.line_wrap,
        )
    } else {
        app.scroll.min(filtered_logs.len().saturating_sub(1))
    };

    let styled_logs: Vec<_> = filtered_logs[scroll_entry..]
        .iter()
        .map(|log| style_log(log, highlight_query))
        .collect();

    let mut logs_block = Paragraph::new(styled_logs)
        .block(Block::default().borders(Borders::ALL).title(" SmartLog "));
    if app.line_wrap {
        logs_block = logs_block.wrap(Wrap { trim: false });
    }
    f.render_widget(logs_block, chunks[0]);

    render_input_bar(f, app, filtered_logs.len(), chunks[1]);
}

/// Renders the input bar with search query and status indicators.
fn render_input_bar(f: &mut Frame, app: &App, shown: usize, area: Rect) {
    let (input_style, border_style) = match app.input_mode {
        InputMode::Normal => (Style::default(), Style::default()),
        InputMode::Editing => (
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        ),
    };
    let title = build_status_title(app, shown, app.logs.len());
    let input_block = Paragraph::new(app.input_buffer.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );
    f.render_widget(input_block, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LogLevel;

    fn entry(pretty: &str) -> LogEntry {
        LogEntry {
            raw: pretty.to_string(),
            pretty: pretty.to_string(),
            level: LogLevel::Info,
        }
    }

    // --- compute_visual_lines tests ---

    #[test]
    fn test_visual_lines_single_short_line() {
        assert_eq!(compute_visual_lines("hello", 80), 1);
    }

    #[test]
    fn test_visual_lines_exact_width() {
        assert_eq!(compute_visual_lines("abcde", 5), 1);
    }

    #[test]
    fn test_visual_lines_wraps_once() {
        assert_eq!(compute_visual_lines("abcdef", 5), 2);
    }

    #[test]
    fn test_visual_lines_multiline_text() {
        assert_eq!(compute_visual_lines("aaa\nbbb\nccc", 80), 3);
    }

    #[test]
    fn test_visual_lines_multiline_with_wrapping() {
        assert_eq!(compute_visual_lines("abcdef\ngh", 5), 3);
    }

    #[test]
    fn test_visual_lines_empty_string() {
        assert_eq!(compute_visual_lines("", 80), 1);
    }

    #[test]
    fn test_visual_lines_zero_width() {
        assert_eq!(compute_visual_lines("abc\ndef", 0), 2);
    }

    #[test]
    fn test_visual_lines_pretty_json() {
        let json = "{\n  \"level\": \"ERROR\",\n  \"msg\": \"fail\"\n}";
        assert_eq!(compute_visual_lines(json, 40), 4);
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

    // --- compute_auto_scroll_entry tests ---

    #[test]
    fn test_auto_scroll_empty_entries() {
        let entries: Vec<&LogEntry> = vec![];
        assert_eq!(compute_auto_scroll_entry(&entries, 10, 80, true), 0);
    }

    #[test]
    fn test_auto_scroll_all_fit() {
        let e1 = entry("line1");
        let e2 = entry("line2");
        let entries: Vec<&LogEntry> = vec![&e1, &e2];
        assert_eq!(compute_auto_scroll_entry(&entries, 10, 80, true), 0);
    }

    #[test]
    fn test_auto_scroll_not_all_fit() {
        let e1 = entry("line1");
        let e2 = entry("line2");
        let e3 = entry("line3");
        let e4 = entry("line4");
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &e3, &e4];
        assert_eq!(compute_auto_scroll_entry(&entries, 2, 80, true), 2);
    }

    #[test]
    fn test_auto_scroll_multiline_entries() {
        let json = "{\n  \"level\": \"ERROR\",\n  \"msg\": \"fail\"\n}";
        let e1 = entry(json);
        let e2 = entry(json);
        let e3 = entry(json);
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &e3];
        assert_eq!(compute_auto_scroll_entry(&entries, 5, 80, true), 2);
    }

    #[test]
    fn test_auto_scroll_zero_viewport_height() {
        let e1 = entry("line1");
        let entries: Vec<&LogEntry> = vec![&e1];
        assert_eq!(compute_auto_scroll_entry(&entries, 0, 80, true), 0);
    }

    #[test]
    fn test_auto_scroll_single_oversized_entry() {
        let big = entry("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\nq\nr\ns\nt");
        let entries: Vec<&LogEntry> = vec![&big];
        assert_eq!(compute_auto_scroll_entry(&entries, 3, 80, true), 0);
    }

    #[test]
    fn test_auto_scroll_last_entry_exceeds_viewport() {
        let e1 = entry("short");
        let e2 = entry("short");
        let big = entry("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &big];
        assert_eq!(compute_auto_scroll_entry(&entries, 3, 80, true), 2);
    }

    // --- no-wrap auto-scroll tests ---

    #[test]
    fn test_auto_scroll_no_wrap_ignores_width() {
        // Long line that would wrap at width 5, but with wrap off it's 1 line
        let e1 = entry("abcdefghij");
        let e2 = entry("klmnopqrst");
        let entries: Vec<&LogEntry> = vec![&e1, &e2];
        // With wrap on: each entry is 2 visual lines at width 5, total 4 > viewport 2
        assert_eq!(compute_auto_scroll_entry(&entries, 2, 5, true), 1);
        // With wrap off: each entry is 1 raw line, total 2 fits viewport 2
        assert_eq!(compute_auto_scroll_entry(&entries, 2, 5, false), 0);
    }

    #[test]
    fn test_auto_scroll_no_wrap_multiline_entry() {
        // Entry with newlines: 3 raw lines regardless of width
        let e1 = entry("a\nb\nc");
        let e2 = entry("d");
        let entries: Vec<&LogEntry> = vec![&e1, &e2];
        // 3 + 1 = 4 raw lines, viewport 3 -> start from index 1
        assert_eq!(compute_auto_scroll_entry(&entries, 3, 80, false), 1);
    }

    // --- build_status_title tests ---

    #[test]
    fn test_status_title_default() {
        let app = App::new();
        let title = build_status_title(&app, 0, 0);
        assert!(title.contains("FOLLOWING"));
        assert!(title.contains("WRAP"));
        assert!(title.contains("ALL"));
        assert!(title.contains("0 logs"));
    }

    #[test]
    fn test_status_title_with_source_label() {
        let mut app = App::new();
        app.source_label = "file: app.log".to_string();
        let title = build_status_title(&app, 5, 5);
        assert!(title.contains("file: app.log"));
    }

    #[test]
    fn test_status_title_empty_source_label_omitted() {
        let app = App::new();
        let title = build_status_title(&app, 0, 0);
        // Should not contain a trailing " | " with nothing after it
        assert!(!title.contains("| |"));
    }

    #[test]
    fn test_status_title_filtered_count() {
        let app = App::new();
        let title = build_status_title(&app, 3, 10);
        assert!(title.contains("3/10 logs"));
    }

    #[test]
    fn test_status_title_export_message_overrides() {
        let mut app = App::new();
        app.last_export_message = Some("Exported 5 logs → test.log".to_string());
        let title = build_status_title(&app, 5, 10);
        assert!(title.contains("Exported 5 logs"));
        // Normal status info should not appear when export message is shown
        assert!(!title.contains("FOLLOWING"));
    }
}
