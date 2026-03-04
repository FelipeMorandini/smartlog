//! Terminal UI rendering.
//!
//! This module handles all the terminal UI rendering using the Ratatui library.

use crate::app::{App, InputMode};
use crate::parser::{style_log, LogEntry};
use ratatui::{
    layout::{Constraint, Direction, Layout},
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

/// Finds the index of the first entry to display for auto-scroll.
///
/// Iterates entries in reverse, summing visual lines until the viewport is full.
/// Returns the index of the first entry that should be visible. Always ensures
/// at least the last entry is shown, even if it exceeds the viewport height.
fn compute_auto_scroll_entry(
    entries: &[&LogEntry],
    viewport_height: usize,
    viewport_width: usize,
) -> usize {
    if entries.is_empty() || viewport_height == 0 {
        return 0;
    }

    let last_index = entries.len() - 1;
    let mut lines_used = 0usize;
    for (i, entry) in entries.iter().enumerate().rev() {
        let entry_lines = compute_visual_lines(&entry.pretty, viewport_width);
        lines_used += entry_lines;
        if lines_used > viewport_height {
            // Clamp so at least the last entry is always visible
            return (i + 1).min(last_index);
        }
    }
    0
}

/// Renders the application UI to the terminal.
///
/// The UI consists of two sections:
/// - Main log area with filtered and styled logs
/// - Input bar showing the current search query and mode
///
/// # Arguments
///
/// * `f` - The Ratatui frame to render to
/// * `app` - The current application state
pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main Log Area
            Constraint::Length(3), // Input Bar
        ])
        .split(f.area());

    // --- 1. Filter Logs (using centralized filtering) ---
    let query = &app.input_buffer;
    let filtered_logs = app.get_filtered_logs();

    // --- 2. Compute viewport dimensions (inner area minus borders) ---
    let viewport_height = chunks[0].height.saturating_sub(2) as usize;
    let viewport_width = chunks[0].width.saturating_sub(2) as usize;

    // --- 3. Determine first visible entry ---
    let scroll_entry = if app.auto_scroll {
        compute_auto_scroll_entry(&filtered_logs, viewport_height, viewport_width)
    } else {
        app.scroll.min(filtered_logs.len().saturating_sub(1))
    };

    // --- 4. Slice to visible entries and style them ---
    let visible_logs = &filtered_logs[scroll_entry..];
    let styled_logs: Vec<_> = visible_logs
        .iter()
        .map(|log| style_log(log, query))
        .collect();

    let logs_block = Paragraph::new(styled_logs)
        .block(Block::default().borders(Borders::ALL).title(" SmartLog "))
        .wrap(Wrap { trim: false }); // Don't trim JSON indentation

    f.render_widget(logs_block, chunks[0]);

    // --- 5. Render Input Bar ---
    let (input_style, border_style) = match app.input_mode {
        InputMode::Normal => (Style::default(), Style::default()),
        InputMode::Editing => (
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        ),
    };

    let status_text = if app.auto_scroll {
        "FOLLOWING"
    } else {
        "PAUSED"
    };
    let total = app.logs.len();
    let shown = filtered_logs.len();
    let title = if app.input_buffer.is_empty() {
        format!(" Filter (Press /) | {} | {} logs ", status_text, total)
    } else {
        format!(
            " Filter (Press /) | {} | {}/{} matches ",
            status_text, shown, total
        )
    };

    let input_block = Paragraph::new(app.input_buffer.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );

    f.render_widget(input_block, chunks[1]);
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
        // 3 lines of text, each fitting in width 80
        assert_eq!(compute_visual_lines("aaa\nbbb\nccc", 80), 3);
    }

    #[test]
    fn test_visual_lines_multiline_with_wrapping() {
        // Line 1: "abcdef" -> 2 visual lines at width 5
        // Line 2: "gh" -> 1 visual line
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
        // 4 lines, all short enough to fit in width 40
        assert_eq!(compute_visual_lines(json, 40), 4);
    }

    // --- compute_auto_scroll_entry tests ---

    #[test]
    fn test_auto_scroll_empty_entries() {
        let entries: Vec<&LogEntry> = vec![];
        assert_eq!(compute_auto_scroll_entry(&entries, 10, 80), 0);
    }

    #[test]
    fn test_auto_scroll_all_fit() {
        let e1 = entry("line1");
        let e2 = entry("line2");
        let entries: Vec<&LogEntry> = vec![&e1, &e2];
        // 2 single-line entries, viewport height 10 -> all fit, start from 0
        assert_eq!(compute_auto_scroll_entry(&entries, 10, 80), 0);
    }

    #[test]
    fn test_auto_scroll_not_all_fit() {
        let e1 = entry("line1");
        let e2 = entry("line2");
        let e3 = entry("line3");
        let e4 = entry("line4");
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &e3, &e4];
        // 4 single-line entries, viewport height 2 -> show last 2 (index 2)
        assert_eq!(compute_auto_scroll_entry(&entries, 2, 80), 2);
    }

    #[test]
    fn test_auto_scroll_multiline_entries() {
        // Each entry is 4 visual lines (pretty-printed JSON)
        let json = "{\n  \"level\": \"ERROR\",\n  \"msg\": \"fail\"\n}";
        let e1 = entry(json);
        let e2 = entry(json);
        let e3 = entry(json);
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &e3];
        // 3 entries x 4 lines = 12 visual lines, viewport height 5
        // Last entry (4 lines) fits, second-to-last (4 more = 8) exceeds 5
        // So start from index 2 (only last entry)
        assert_eq!(compute_auto_scroll_entry(&entries, 5, 80), 2);
    }

    #[test]
    fn test_auto_scroll_zero_viewport_height() {
        let e1 = entry("line1");
        let entries: Vec<&LogEntry> = vec![&e1];
        assert_eq!(compute_auto_scroll_entry(&entries, 0, 80), 0);
    }

    #[test]
    fn test_auto_scroll_single_oversized_entry() {
        // Single entry with 20 visual lines, viewport only 3 lines
        // Must still show the last (only) entry, not return entries.len()
        let big = entry("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\nq\nr\ns\nt");
        let entries: Vec<&LogEntry> = vec![&big];
        assert_eq!(compute_auto_scroll_entry(&entries, 3, 80), 0);
    }

    #[test]
    fn test_auto_scroll_last_entry_exceeds_viewport() {
        // 3 entries, last one alone exceeds viewport
        let e1 = entry("short");
        let e2 = entry("short");
        let big = entry("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let entries: Vec<&LogEntry> = vec![&e1, &e2, &big];
        // Last entry is 10 lines, viewport is 3 — should show last entry (index 2)
        assert_eq!(compute_auto_scroll_entry(&entries, 3, 80), 2);
    }
}
