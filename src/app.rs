//! Application state management and core logic.

use crate::config::MAX_LOG_BUFFER_SIZE;
use crate::parser::{LogEntry, LogLevel};
use crate::theme::Theme;
use chrono::Local;
use regex::Regex;
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

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
    pub logs_processed: u64,
    /// Visible height of the log area in lines (updated each frame from terminal size)
    pub visible_height: u16,
    /// Visible width of the log area in columns (updated each frame from terminal size)
    pub visible_width: u16,
    /// Whether line wrapping is enabled
    pub line_wrap: bool,
    /// Minimum log level filter (None = show all)
    pub min_log_level: Option<LogLevel>,
    /// Whether to use regex for text filtering
    pub use_regex: bool,
    /// Label describing the log source (e.g., "file: app.log", "stdin", "demo")
    pub source_label: String,
    /// Directory for exported log files
    pub export_dir: PathBuf,
    /// Transient feedback message from the last export operation
    pub last_export_message: Option<String>,
    /// Whether to display relative timestamps before log entries
    pub show_timestamps: bool,
    /// Current color theme
    pub theme: Theme,
    /// Cached text matcher, rebuilt when `input_buffer` or `use_regex` changes.
    cached_matcher: TextMatcher,
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
            visible_width: 80,
            line_wrap: true,
            min_log_level: None,
            use_regex: false,
            source_label: String::new(),
            export_dir: PathBuf::from("."),
            last_export_message: None,
            show_timestamps: false,
            theme: Theme::DARK,
            cached_matcher: TextMatcher::None,
        }
    }

    /// Rebuilds the cached text matcher from current input state.
    ///
    /// Must be called after any mutation to `input_buffer` or `use_regex`.
    pub fn rebuild_matcher(&mut self) {
        self.cached_matcher = self.compile_matcher();
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
        self.logs
            .iter()
            .filter(|l| self.matches_level(l) && Self::matches_text(l, &self.cached_matcher))
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
    ///
    /// Uses a direct `.filter().count()` instead of allocating a `Vec` via
    /// `get_filtered_logs()`.
    pub fn get_filtered_count(&self) -> usize {
        self.logs
            .iter()
            .filter(|l| self.matches_level(l) && Self::matches_text(l, &self.cached_matcher))
            .count()
    }

    /// Cycles the minimum log level filter.
    pub fn cycle_log_level(&mut self) {
        self.min_log_level = match self.min_log_level {
            None => Some(LogLevel::Error),
            Some(level) => level.next_filter(),
        };
    }

    /// Exports currently filtered logs to a timestamped file in the export directory.
    ///
    /// Uses a buffered writer to stream entries directly to disk, avoiding a
    /// large intermediate `String` allocation (TD-17).
    /// Sets `last_export_message` with the result (success path or error).
    pub fn export_logs(&mut self) {
        static EXPORT_COUNTER: AtomicU32 = AtomicU32::new(0);

        let filtered = self.get_filtered_logs();
        let timestamp = Local::now().format("%Y%m%dT%H%M%S%.3f");
        let seq = EXPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let filename = format!("smartlog_export_{}_{}.log", timestamp, seq);
        let path = self.export_dir.join(&filename);

        let count = filtered.len();
        let label = if count == 1 { "log" } else { "logs" };

        let result = (|| -> std::io::Result<()> {
            let file = std::fs::File::create(&path)?;
            let mut writer = std::io::BufWriter::new(file);
            if filtered.is_empty() {
                writer.write_all(b"# No log entries matched the current filter.\n")?;
            } else {
                for (i, e) in filtered.iter().enumerate() {
                    if let Some(ref src) = e.source {
                        write!(writer, "[{src}] ")?;
                    }
                    writer.write_all(e.pretty.as_bytes())?;
                    if i + 1 < count {
                        writer.write_all(b"\n")?;
                    }
                }
            }
            writer.flush()?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                tracing::debug!(path = %path.display(), count, "Exported logs");
                self.last_export_message =
                    Some(format!("Exported {count} {label} → {}", path.display()));
            }
            Err(e) => {
                tracing::warn!(error = %e, "Export failed");
                self.last_export_message = Some(format!("Export failed: {}", e));
            }
        }
    }

    /// Clears the transient export feedback message.
    pub fn clear_export_message(&mut self) {
        self.last_export_message = None;
    }

    /// Returns true if the cached text matcher is an invalid regex.
    pub fn is_regex_invalid(&self) -> bool {
        matches!(self.cached_matcher, TextMatcher::Invalid)
    }

    /// Returns a reference to the cached regex, if the matcher is in regex mode.
    pub fn highlight_regex(&self) -> Option<&Regex> {
        match &self.cached_matcher {
            TextMatcher::Regex(re) => Some(re),
            _ => None,
        }
    }

    /// Compiles a text matcher from the current input buffer and mode.
    fn compile_matcher(&self) -> TextMatcher {
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
    use crate::theme::Theme;

    fn make_entry(msg: &str, level: LogLevel) -> LogEntry {
        LogEntry {
            raw: msg.to_string(),
            pretty: msg.to_string(),
            level,
            timestamp: None,
            source: None,
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
        assert!(app.source_label.is_empty());
        assert_eq!(app.export_dir, PathBuf::from("."));
        assert!(app.last_export_message.is_none());
        assert!(!app.show_timestamps);
        assert_eq!(app.theme, Theme::DARK);
    }

    #[test]
    fn test_on_log_adds_entry() {
        let mut app = App::new();
        app.on_log(make_entry("test", LogLevel::Info));
        assert_eq!(app.logs.len(), 1);
        assert_eq!(app.logs_processed, 1);
    }

    #[test]
    fn test_logs_processed_is_u64() {
        let mut app = App::new();
        // Verify the counter is u64, not usize — can hold values beyond u32::MAX
        app.logs_processed = u64::from(u32::MAX) + 1;
        app.on_log(make_entry("test", LogLevel::Info));
        assert_eq!(app.logs_processed, u64::from(u32::MAX) + 2);
    }

    #[test]
    fn test_on_log_buffer_overflow_evicts_oldest() {
        let mut app = App::new();
        for i in 0..MAX_LOG_BUFFER_SIZE + 10 {
            app.on_log(make_entry(&format!("log {}", i), LogLevel::Info));
        }
        assert_eq!(app.logs.len(), MAX_LOG_BUFFER_SIZE);
        assert_eq!(app.logs_processed, (MAX_LOG_BUFFER_SIZE + 10) as u64);
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
        app.rebuild_matcher();
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
        app.rebuild_matcher();
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
        app.rebuild_matcher();
        app.auto_scroll = false;
        app.scroll = 0;
        app.scroll_down();
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_scroll_down_by_empty_filter_no_auto_scroll() {
        let mut app = App::new();
        app.input_buffer = "nonexistent".to_string();
        app.rebuild_matcher();
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
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 1);
    }

    #[test]
    fn test_get_filtered_logs_returns_correct_entries() {
        let mut app = App::new();
        app.on_log(make_entry("error occurred", LogLevel::Error));
        app.on_log(make_entry("info message", LogLevel::Info));
        app.input_buffer = "error".to_string();
        app.rebuild_matcher();
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
        app.rebuild_matcher();
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
        app.rebuild_matcher();
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
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_regex_filter_case_insensitive() {
        let mut app = App::new();
        app.on_log(make_entry("ERROR happened", LogLevel::Error));
        app.on_log(make_entry("error too", LogLevel::Error));
        app.use_regex = true;
        app.input_buffer = "error".to_string();
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 2);
    }

    #[test]
    fn test_regex_invalid_matches_nothing() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = "[invalid".to_string();
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 0);
    }

    #[test]
    fn test_is_regex_invalid_true() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = "[bad".to_string();
        app.rebuild_matcher();
        assert!(app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_valid() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = r"\d+".to_string();
        app.rebuild_matcher();
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_empty() {
        let mut app = App::new();
        app.use_regex = true;
        app.rebuild_matcher();
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_is_regex_invalid_false_when_not_regex_mode() {
        let mut app = App::new();
        app.use_regex = false;
        app.input_buffer = "[bad".to_string();
        app.rebuild_matcher();
        assert!(!app.is_regex_invalid());
    }

    #[test]
    fn test_regex_empty_buffer_shows_all() {
        let mut app = App::new();
        app.on_log(make_entry("a", LogLevel::Info));
        app.on_log(make_entry("b", LogLevel::Info));
        app.use_regex = true;
        app.rebuild_matcher();
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
        app.rebuild_matcher();
        app.min_log_level = Some(LogLevel::Warn);
        // error + warn match level, all 3 match regex, but info is filtered by level
        assert_eq!(app.get_filtered_count(), 2);
    }

    // --- Export and source label tests ---

    /// Creates a unique temporary directory for test isolation.
    fn unique_temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "smartlog_test_{}_{}_{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn test_clear_export_message() {
        let mut app = App::new();
        app.last_export_message = Some("test".to_string());
        app.clear_export_message();
        assert!(app.last_export_message.is_none());
    }

    #[test]
    fn test_clear_export_message_when_already_none() {
        let mut app = App::new();
        app.clear_export_message();
        assert!(app.last_export_message.is_none());
    }

    #[test]
    fn test_export_logs_creates_file() {
        let dir = unique_temp_dir("export");
        let mut app = App::new();
        app.export_dir = dir.clone();
        app.on_log(make_entry("line one", LogLevel::Info));
        app.on_log(make_entry("line two", LogLevel::Warn));

        app.export_logs();

        assert!(app.last_export_message.is_some());
        let msg = app.last_export_message.as_ref().unwrap();
        assert!(msg.contains("Exported 2 logs"));

        // Verify file exists and has content
        let files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("smartlog_export_")
            })
            .collect();
        assert_eq!(files.len(), 1);
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("line one"));
        assert!(content.contains("line two"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_logs_with_filter() {
        let dir = unique_temp_dir("export_filter");
        let mut app = App::new();
        app.export_dir = dir.clone();
        app.on_log(make_entry("keep this", LogLevel::Info));
        app.on_log(make_entry("drop this", LogLevel::Info));
        app.input_buffer = "keep".to_string();
        app.rebuild_matcher();

        app.export_logs();

        let msg = app.last_export_message.as_ref().unwrap();
        assert!(msg.contains("Exported 1 log"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_logs_empty_results() {
        let dir = unique_temp_dir("export_empty");
        let mut app = App::new();
        app.export_dir = dir.clone();
        app.input_buffer = "nomatch".to_string();
        app.rebuild_matcher();

        app.export_logs();

        let msg = app.last_export_message.as_ref().unwrap();
        assert!(msg.contains("Exported 0 logs"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_logs_invalid_dir() {
        // Use a regular file as the "directory" — guaranteed to fail on all platforms
        let file_path =
            std::env::temp_dir().join(format!("smartlog_test_not_a_dir_{}", std::process::id()));
        std::fs::write(&file_path, b"not a directory").unwrap();

        let mut app = App::new();
        app.export_dir = file_path.clone();
        app.on_log(make_entry("test", LogLevel::Info));

        app.export_logs();

        let msg = app.last_export_message.as_ref().unwrap();
        assert!(msg.contains("Export failed"));

        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_export_logs_with_source_prefix() {
        let dir = unique_temp_dir("export_source");
        let mut app = App::new();
        app.export_dir = dir.clone();
        app.on_log(LogEntry {
            raw: "hello".to_string(),
            pretty: "hello".to_string(),
            level: LogLevel::Info,
            timestamp: None,
            source: Some(std::sync::Arc::from("app.log")),
        });
        app.on_log(LogEntry {
            raw: "world".to_string(),
            pretty: "world".to_string(),
            level: LogLevel::Info,
            timestamp: None,
            source: None,
        });

        app.export_logs();

        let files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1);
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("[app.log] hello"));
        assert!(content.contains("world"));
        // "world" should NOT have a source prefix
        assert!(!content.contains("[app.log] world"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Cached matcher tests ---

    #[test]
    fn test_rebuild_matcher_updates_filtering() {
        let mut app = App::new();
        app.on_log(make_entry("alpha", LogLevel::Info));
        app.on_log(make_entry("beta", LogLevel::Info));
        // Without rebuild, cached matcher is None → all match
        assert_eq!(app.get_filtered_count(), 2);
        app.input_buffer = "alpha".to_string();
        // Still 2 because cached matcher hasn't been rebuilt
        assert_eq!(app.get_filtered_count(), 2);
        app.rebuild_matcher();
        // Now only "alpha" matches
        assert_eq!(app.get_filtered_count(), 1);
    }

    #[test]
    fn test_rebuild_matcher_clears_on_empty_buffer() {
        let mut app = App::new();
        app.on_log(make_entry("hello", LogLevel::Info));
        app.input_buffer = "hello".to_string();
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 1);
        app.input_buffer.clear();
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 1); // still 1 entry, all match
    }

    #[test]
    fn test_rebuild_matcher_regex_mode() {
        let mut app = App::new();
        app.on_log(make_entry("error 42", LogLevel::Error));
        app.on_log(make_entry("info ok", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = r"\d+".to_string();
        app.rebuild_matcher();
        assert_eq!(app.get_filtered_count(), 1);
        assert_eq!(app.get_filtered_logs()[0].pretty, "error 42");
    }

    #[test]
    fn test_rebuild_matcher_invalid_regex() {
        let mut app = App::new();
        app.on_log(make_entry("test", LogLevel::Info));
        app.use_regex = true;
        app.input_buffer = "[bad".to_string();
        app.rebuild_matcher();
        assert!(app.is_regex_invalid());
        assert_eq!(app.get_filtered_count(), 0);
    }

    // --- highlight_regex tests ---

    #[test]
    fn test_highlight_regex_returns_some_in_regex_mode() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = r"\d+".to_string();
        app.rebuild_matcher();
        assert!(app.highlight_regex().is_some());
    }

    #[test]
    fn test_highlight_regex_returns_none_in_substring_mode() {
        let mut app = App::new();
        app.input_buffer = "hello".to_string();
        app.rebuild_matcher();
        assert!(app.highlight_regex().is_none());
    }

    #[test]
    fn test_highlight_regex_returns_none_for_invalid() {
        let mut app = App::new();
        app.use_regex = true;
        app.input_buffer = "[bad".to_string();
        app.rebuild_matcher();
        assert!(app.highlight_regex().is_none());
    }

    #[test]
    fn test_highlight_regex_returns_none_when_empty() {
        let app = App::new();
        assert!(app.highlight_regex().is_none());
    }
}
