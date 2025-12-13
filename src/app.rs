//! Application state management and core logic.

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
    /// Rolling buffer of log entries (max 2000)
    pub logs: VecDeque<LogEntry>,
    /// Current scroll position
    pub scroll: usize,
    /// Whether to automatically scroll to the latest logs
    pub auto_scroll: bool,
    /// Flag to signal the application should quit
    pub should_quit: bool,
    /// Total number of logs processed (for debugging)
    pub logs_processed: usize,
}

impl App {
    /// Creates a new `App` instance with default values.
    pub fn new() -> App {
        App {
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            logs: VecDeque::with_capacity(2000), // Keep last 2000 logs in memory
            scroll: 0,
            auto_scroll: true,
            should_quit: false,
            logs_processed: 0,
        }
    }

    /// Adds a new log entry to the buffer.
    ///
    /// If the buffer exceeds 2000 entries, the oldest entry is removed.
    pub fn on_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= 2000 {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
        self.logs_processed += 1;

        // If auto-scroll is on, we don't manually increment scroll.
        // The UI layer will handle snapping to bottom if auto_scroll is true.
    }

    /// Scrolls the view up by one line and disables auto-scroll.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    /// Scrolls the view down by one line.
    ///
    /// Re-enables auto-scroll if the user reaches the bottom.
    pub fn scroll_down(&mut self) {
        // If we are at the bottom, engage auto-scroll
        let max_scroll = self.get_filtered_count().saturating_sub(1);

        if self.scroll < max_scroll {
            self.scroll += 1;
        }

        // Re-enable auto-scroll if we hit the bottom
        if self.scroll >= max_scroll {
            self.auto_scroll = true;
        }
    }

    /// Returns the number of log entries matching the current filter.
    ///
    /// If the input buffer is empty, returns the total number of logs.
    pub fn get_filtered_count(&self) -> usize {
        if self.input_buffer.is_empty() {
            self.logs.len()
        } else {
            let q = self.input_buffer.to_lowercase();
            self.logs.iter().filter(|l| l.pretty.to_lowercase().contains(&q)).count()
        }
    }
}