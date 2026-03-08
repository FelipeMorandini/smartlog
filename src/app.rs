//! Application state management and core logic.

use crate::config::MAX_LOG_BUFFER_SIZE;
use crate::parser::{LogEntry, LogLevel};
use regex::Regex;
use std::collections::VecDeque;

/// Internal text matching strategy.
enum TextMatcher {
    /// No text filter active
    None,
    /// Case-insensitive substring match
    Substring(String),
    /// Compiled regex pattern
    Regex(Regex),
    /// Invalid regex (matches nothing)
    Invalid,
}

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
    /// Whether line wrapping is enabled
    pub line_wrap: bool,
    /// Minimum log level filter (None = show all)
    pub min_log_level: Option<LogLevel>,
    /// Whether to use regex for text filtering
    pub use_regex: bool,
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
            line_wrap: true,
            min_log_level: None,
            use_regex: false,
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

    /// Returns the filtered log entries based on text query and log level.
    pub fn get_filtered_logs(&self) -> Vec<&LogEntry> {
        let matcher = self.build_text_matcher();
        self.logs
            .iter()
            .filter(|l| self.matches_level(l) && Self::matches_text(l, &matcher))
            .collect()
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
        self.get_filtered_logs().len()
    }

    /// Cycles the minimum log level filter.
    pub fn cycle_log_level(&mut self) {
        self.min_log_level = match self.min_log_level {
            None => Some(LogLevel::Error),
            Some(level) => level.next_filter(),
        };
    }

    /// Returns true if the compiled regex (when in regex mode) is invalid.
    pub fn is_regex_invalid(&self) -> bool {
        if !self.use_regex || self.input_buffer.is_empty() {
            return false;
        }
        matches!(self.build_text_matcher(), TextMatcher::Invalid)
    }

    /// Builds a text matcher from the current input buffer and mode.
    fn build_text_matcher(&self) -> TextMatcher {
        if self.input_buffer.is_empty() {
            return TextMatcher::None;
        }
        if self.use_regex {
            match Regex::new(&format!("(?i){}", &self.input_buffer)) {
                Ok(re) => TextMatcher::Regex(re),
                Err(_) => TextMatcher::Invalid,
            }
        } else {
            TextMatcher::Substring(self.input_buffer.to_lowercase())
        }
    }

    /// Checks if an entry matches the current log level filter.
    fn matches_level(&self, entry: &LogEntry) -> bool {
        match self.min_log_level {
            None => true,
            Some(min) => entry.level.severity() <= min.severity(),
        }
    }

    /// Checks if an entry matches the text filter.
    fn matches_text(entry: &LogEntry, matcher: &TextMatcher) -> bool {
        match matcher {
            TextMatcher::None => true,
            TextMatcher::Substring(q) => entry.pretty.to_lowercase().contains(q.as_str()),
            TextMatcher::Regex(re) => re.is_match(&entry.pretty),
            TextMatcher::Invalid => false,
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
        assert!(app.line_wrap);
        assert!(app.min_log_level.is_none());
        assert!(!app.use_regex);
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

    // --- Log level filtering tests ---

    #[test]
    fn test_cycle_log_level_sequence() {
        let mut app = App::new();
        assert!(app.min_log_level.is_none());
        app.cycle_log_level();
        assert_eq!(app.min_log_level, Some(LogLevel::Error));
        app.cycle_log_level();
        assert_eq!(app.min_log_level, Some(LogLevel::Warn));
        app.cycle_log_level();
        assert_eq!(app.min_log_level, Some(LogLevel::Info));
        app.cycle_log_level();
        assert_eq!(app.min_log_level, Some(LogLevel::Debug));
        app.cycle_log_level();
        assert!(app.min_log_level.is_none());
    }

    #[test]
    fn test_level_filter_error_only() {
        let mut app = App::new();
        app.on_log(make_entry("err msg", LogLevel::Error));
        app.on_log(make_entry("warn msg", LogLevel::Warn));
        app.on_log(make_entry("info msg", LogLevel::Info));
        app.min_log_level = Some(LogLevel::Error);
        assert_eq!(app.get_filtered_count(), 1);
        assert_eq!(app.get_filtered_logs()[0].pretty, "err msg");
    }

    #[test]
    fn test_level_filter_warn_and_above() {
        let mut app = App::new();
        app.on_log(make_entry("err", LogLevel::Error));
        app.on_log(make_entry("warn", LogLevel::Warn));
        app.on_log(make_entry("info", LogLevel::Info));
        app.on_log(make_entry("debug", LogLevel::Debug));
        app.min_log_level = Some(LogLevel::Warn);
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_level_filter_includes_unknown_only_when_none() {
        let mut app = App::new();
        app.on_log(make_entry("unknown", LogLevel::Unknown));
        app.on_log(make_entry("err", LogLevel::Error));
        // No level filter -> both shown
        assert_eq!(app.get_filtered_count(), 2);
        // Error filter -> only error (Unknown severity=4 > Error severity=0)
        app.min_log_level = Some(LogLevel::Error);
        assert_eq!(app.get_filtered_count(), 1);
    }

    #[test]
    fn test_level_and_text_filter_combined() {
        let mut app = App::new();
        app.on_log(make_entry("error: disk full", LogLevel::Error));
        app.on_log(make_entry("error: network", LogLevel::Error));
        app.on_log(make_entry("warn: disk usage", LogLevel::Warn));
        app.min_log_level = Some(LogLevel::Error);
        app.input_buffer = "disk".to_string();
        assert_eq!(app.get_filtered_count(), 1);
        assert_eq!(app.get_filtered_logs()[0].pretty, "error: disk full");
    }

    // --- Regex filtering tests ---

    #[test]
    fn test_regex_filter_basic() {
        let mut app = App::new();
        app.on_log(make_entry("error 404", LogLevel::Error));
        app.on_log(make_entry("error 500", LogLevel::Error));
        app.on_log(make_entry("info ok", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = r"error \d+".to_string();
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_regex_filter_case_insensitive() {
        let mut app = App::new();
        app.on_log(make_entry("ERROR happened", LogLevel::Error));
        app.on_log(make_entry("error too", LogLevel::Error));
        app.use_regex = true;
        app.input_buffer = "error".to_string();
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_regex_invalid_matches_nothing() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = "[invalid".to_string();
        assert_eq!(app.get_filtered_count(), 0);
    }

    #[test]
    fn test_is_regex_invalid_true() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = "[bad".to_string();
        assert!(app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_valid() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = r"\d+".to_string();
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_empty() {
        let mut app = App::new();
        app.use_regex = true;
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_when_not_regex_mode() {
        let mut app = App::new();
        app.use_regex = false;
        app.input_buffer = "[bad".to_string();
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_regex_empty_buffer_shows_all() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.use_regex = true;
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_regex_and_level_filter_combined() {
        let mut app = App::new();
        app.on_log(make_entry("error: code 42", LogLevel::Error));
        app.on_log(make_entry("warn: code 42", LogLevel::Warn));
        app.on_log(make_entry("info: code 99", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = r"code \d{2}".to_string();
        app.min_log_level = Some(LogLevel::Warn);
        // error + warn match level, all 3 match regex, but info is filtered by level
        assert_eq!(app.get_filtered_count(), 2);
    }
}
