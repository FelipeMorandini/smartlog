//! Application state management and core logic.

use crate::config::MAX_LOG_BUFFER_SIZE;
use crate::parser::LogEntry;
use std::collections::VecDeque;

/// The input mode for the application.
#[derive(PartialEq, Debug)]
pub enum InputMode {
    /// Normal mode for navigation and viewing
    Normal,
    /// Editing mode for entering search queries
    Editing,
}

/// The main application state.
pub struct App {
    /// Current input mode (Normal or Editing)
    pub input_mode: InputMode,
    /// User input buffer for search queries
    pub input_buffer: String,
    /// Rolling buffer of log entries
    pub logs: VecDeque<LogEntry>,
    /// Current scroll position
    pub scroll: usize,
    /// Whether to automatically scroll to the latest logs
    pub auto_scroll: bool,
    /// Flag to signal the application should quit
    pub should_quit: bool,
    /// Total number of logs processed (for debugging)
    pub logs_processed: usize,
    /// Visible height of the log area (updated each frame from terminal size)
    pub visible_height: u16,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new `App` instance with default values.
    pub fn new() -> App {
        App {
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            logs: VecDeque::with_capacity(MAX_LOG_BUFFER_SIZE),
            scroll: 0,
            auto_scroll: true,
            should_quit: false,
            logs_processed: 0,
            visible_height: 20,
        }
    }

    /// Adds a new log entry to the buffer.
    ///
    /// If the buffer exceeds the configured maximum size, the oldest entry is removed
    /// and the scroll position is adjusted to compensate.
    pub fn on_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= MAX_LOG_BUFFER_SIZE {
            self.logs.pop_front();
            self.scroll = self.scroll.saturating_sub(1);
        }
        self.logs.push_back(entry);
        self.logs_processed += 1;
    }

    /// Scrolls the view up by one line and disables auto-scroll.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    /// Scrolls the view down by one line.
    pub fn scroll_down(&mut self) {
        let max_scroll = self.get_filtered_count().saturating_sub(1);

        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    /// Scrolls up by `n` lines.
    pub fn scroll_up_by(&mut self, n: usize) {
        self.auto_scroll = false;
        self.scroll = self.scroll.saturating_sub(n);
    }

    /// Scrolls down by `n` lines, clamped to the maximum scroll position.
    pub fn scroll_down_by(&mut self, n: usize) {
        let max_scroll = self.get_filtered_count().saturating_sub(1);
        self.scroll = (self.scroll + n).min(max_scroll);
    }

    /// Jumps to the top of the log buffer.
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll = 0;
    }

    /// Jumps to the bottom of the log buffer and re-enables auto-scroll.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.get_filtered_count().saturating_sub(1);
        self.auto_scroll = true;
    }

    /// Returns the filtered log entries based on the current search query.
    pub fn get_filtered_logs(&self) -> Vec<&LogEntry> {
        if self.input_buffer.is_empty() {
            self.logs.iter().collect()
        } else {
            let q = self.input_buffer.to_lowercase();
            self.logs
                .iter()
                .filter(|l| l.pretty.to_lowercase().contains(&q))
                .collect()
        }
    }

    /// Clamps `scroll` so it does not exceed the last filtered entry index.
    pub fn clamp_scroll(&mut self) {
        let count = self.get_filtered_count();
        if count == 0 {
            self.scroll = 0;
        } else {
            self.scroll = self.scroll.min(count - 1);
        }
    }

    /// Returns the number of log entries matching the current filter.
    pub fn get_filtered_count(&self) -> usize {
        if self.input_buffer.is_empty() {
            self.logs.len()
        } else {
            let q = self.input_buffer.to_lowercase();
            self.logs
                .iter()
                .filter(|l| l.pretty.to_lowercase().contains(&q))
                .count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{LogEntry, LogLevel};

    fn make_entry(msg: &str, level: LogLevel) -> LogEntry {
        LogEntry {
            raw: msg.to_string(),
            pretty: msg.to_string(),
            level,
        }
    }

    #[test]
    fn test_new_app_defaults() {
        let app = App::new();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
        assert_eq!(app.logs.len(), 0);
        assert_eq!(app.scroll, 0);
        assert!(app.auto_scroll);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_on_log_adds_entry() {
        let mut app = App::new();
        app.on_log(make_entry("test", LogLevel::Info));
        assert_eq!(app.logs.len(), 1);
        assert_eq!(app.logs_processed, 1);
    }

    #[test]
    fn test_on_log_buffer_overflow_evicts_oldest() {
        let mut app = App::new();
        for i in 0..MAX_LOG_BUFFER_SIZE + 10 {
            app.on_log(make_entry(&format!("log {}", i), LogLevel::Info));
        }
        assert_eq!(app.logs.len(), MAX_LOG_BUFFER_SIZE);
        assert_eq!(app.logs_processed, MAX_LOG_BUFFER_SIZE + 10);
        assert_eq!(app.logs.front().unwrap().pretty, "log 10");
    }

    #[test]
    fn test_on_log_overflow_adjusts_scroll() {
        let mut app = App::new();
        app.auto_scroll = false;
        for _ in 0..MAX_LOG_BUFFER_SIZE {
            app.on_log(make_entry("fill", LogLevel::Info));
        }
        app.scroll = 5;
        app.on_log(make_entry("overflow", LogLevel::Info));
        assert_eq!(app.scroll, 4);
    }

    #[test]
    fn test_scroll_up_disables_auto_scroll() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.scroll = 1;
        app.scroll_up();
        assert!(!app.auto_scroll);
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_scroll_up_saturates_at_zero() {
        let mut app = App::new();
        app.scroll = 0;
        app.scroll_up();
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_scroll_down_does_not_re_enable_auto_scroll() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.auto_scroll = false;
        app.scroll = 0;
        app.scroll_down();
        assert!(!app.auto_scroll);
        assert_eq!(app.scroll, 1);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.scroll = 1;
        app.scroll_to_top();
        assert_eq!(app.scroll, 0);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.scroll = 0;
        app.auto_scroll = false;
        app.scroll_to_bottom();
        assert_eq!(app.scroll, 1);
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_scroll_up_by() {
        let mut app = App::new();
        for _ in 0..20 {
            app.on_log(make_entry("line", LogLevel::Info));
        }
        app.scroll = 15;
        app.scroll_up_by(10);
        assert_eq!(app.scroll, 5);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_scroll_up_by_saturates() {
        let mut app = App::new();
        for _ in 0..10 {
            app.on_log(make_entry("line", LogLevel::Info));
        }
        app.scroll = 3;
        app.scroll_up_by(100);
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_scroll_down_by_clamps() {
        let mut app = App::new();
        for _ in 0..5 {
            app.on_log(make_entry("line", LogLevel::Info));
        }
        app.auto_scroll = false;
        app.scroll = 0;
        app.scroll_down_by(100);
        assert_eq!(app.scroll, 4);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_clamp_scroll_reduces_beyond_filtered() {
        let mut app = App::new();
        app.on_log(make_entry("alpha", LogLevel::Info));
        app.on_log(make_entry("beta", LogLevel::Info));
        app.on_log(make_entry("gamma", LogLevel::Info));
        app.scroll = 2;
        app.input_buffer = "alpha".to_string();
        // Only 1 match, so scroll should clamp to 0
        app.clamp_scroll();
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_clamp_scroll_no_matches() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.scroll = 5;
        app.input_buffer = "zzz".to_string();
        app.clamp_scroll();
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_clamp_scroll_within_range_unchanged() {
        let mut app = App::new();
        for _ in 0..10 {
            app.on_log(make_entry("line", LogLevel::Info));
        }
        app.scroll = 5;
        app.clamp_scroll();
        assert_eq!(app.scroll, 5);
    }

    #[test]
    fn test_scroll_down_empty_filter_no_auto_scroll() {
        let mut app = App::new();
        app.input_buffer = "nonexistent".to_string();
        app.auto_scroll = false;
        app.scroll = 0;
        app.scroll_down();
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_scroll_down_by_empty_filter_no_auto_scroll() {
        let mut app = App::new();
        app.input_buffer = "nonexistent".to_string();
        app.auto_scroll = false;
        app.scroll = 0;
        app.scroll_down_by(10);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_get_filtered_count_no_filter() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.on_log(make_entry("world", LogLevel::Warn));
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_get_filtered_count_with_filter() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.on_log(make_entry("world", LogLevel::Warn));
        app.input_buffer = "hello".to_string();
        assert_eq!(app.get_filtered_count(), 1);
    }

    #[test]
    fn test_get_filtered_logs_returns_correct_entries() {
        let mut app = App::new();
        app.on_log(make_entry("error occurred", LogLevel::Error));
        app.on_log(make_entry("info message", LogLevel::Info));
        app.input_buffer = "error".to_string();
        let filtered = app.get_filtered_logs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pretty, "error occurred");
    }

    #[test]
    fn test_get_filtered_logs_case_insensitive() {
        let mut app = App::new();
        app.on_log(make_entry("Hello World", LogLevel::Info));
        app.on_log(make_entry("goodbye", LogLevel::Info));
        app.input_buffer = "HELLO".to_string();
        assert_eq!(app.get_filtered_logs().len(), 1);
    }
}
