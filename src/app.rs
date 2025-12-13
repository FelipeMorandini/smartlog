use crate::parser::{LogEntry, LogLevel};
use std::collections::VecDeque;

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub logs: VecDeque<LogEntry>,
    pub scroll: usize,
    pub auto_scroll: bool,
    pub should_quit: bool,
    // Metrics for debugging or status bar
    pub logs_processed: usize,
}

impl App {
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

    pub fn on_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= 2000 {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
        self.logs_processed += 1;

        // If auto-scroll is on, we don't manually increment scroll.
        // The UI layer will handle snapping to bottom if auto_scroll is true.
    }

    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

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

    // Helper to calculate how many items match filter (for scrolling logic)
    pub fn get_filtered_count(&self) -> usize {
        if self.input_buffer.is_empty() {
            self.logs.len()
        } else {
            let q = self.input_buffer.to_lowercase();
            self.logs.iter().filter(|l| l.pretty.to_lowercase().contains(&q)).count()
        }
    }
}