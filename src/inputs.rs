//! Keyboard input handling for the application.

use crate::app::{App, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

/// Handles keyboard input events and updates the application state.
///
/// # Arguments
///
/// * `app` - The application state
/// * `key` - The keyboard event to handle
///
/// # Behavior
///
/// **Normal mode**: `q` quits, `/` enters editing, `k`/`j` or arrows scroll,
/// `PageUp`/`PageDown` scroll by page, `Home`/`g` jump to top, `End`/`G` jump to bottom,
/// `Esc` clears search, `w` toggles line wrap, `l` cycles log level filter,
/// `r` toggles regex mode, `e` exports filtered logs
///
/// **Editing mode**: `Enter` applies filter, `Esc` cancels and clears filter,
/// characters are added to input buffer
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Clear transient export feedback on any key press
    app.clear_export_message();

    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('/') => {
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
            KeyCode::PageUp => app.scroll_up_by(app.visible_height as usize),
            KeyCode::PageDown => app.scroll_down_by(app.visible_height as usize),
            KeyCode::Home | KeyCode::Char('g') => app.scroll_to_top(),
            KeyCode::End | KeyCode::Char('G') => app.scroll_to_bottom(),
            KeyCode::Char('w') => app.line_wrap = !app.line_wrap,
            KeyCode::Char('l') => {
                app.cycle_log_level();
                app.clamp_scroll();
            }
            KeyCode::Char('r') => {
                app.use_regex = !app.use_regex;
                app.clamp_scroll();
            }
            KeyCode::Char('e') => app.export_logs(),
            KeyCode::Esc => {
                app.input_buffer.clear();
                app.clamp_scroll();
                app.auto_scroll = true;
            }
            _ => {}
        },
        InputMode::Editing => match key.code {
            KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
                app.clamp_scroll();
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
                app.clamp_scroll();
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
                app.clamp_scroll();
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input_buffer.clear();
                app.clamp_scroll();
                app.auto_scroll = true;
            }
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{LogEntry, LogLevel};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn app_with_logs(n: usize) -> App {
        let mut app = App::new();
        for i in 0..n {
            app.on_log(LogEntry {
                raw: format!("log {}", i),
                pretty: format!("log {}", i),
                level: LogLevel::Info,
            });
        }
        app
    }

    // --- Normal mode tests ---

    #[test]
    fn test_q_quits() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_slash_enters_editing() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn test_up_scrolls_up() {
        let mut app = app_with_logs(10);
        app.scroll = 5;
        handle_key_event(&mut app, key(KeyCode::Up));
        assert_eq!(app.scroll, 4);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_k_scrolls_up() {
        let mut app = app_with_logs(10);
        app.scroll = 5;
        handle_key_event(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.scroll, 4);
    }

    #[test]
    fn test_down_scrolls_down() {
        let mut app = app_with_logs(10);
        app.auto_scroll = false;
        app.scroll = 0;
        handle_key_event(&mut app, key(KeyCode::Down));
        assert_eq!(app.scroll, 1);
    }

    #[test]
    fn test_j_scrolls_down() {
        let mut app = app_with_logs(10);
        app.auto_scroll = false;
        app.scroll = 0;
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.scroll, 1);
    }

    #[test]
    fn test_esc_normal_clears_filter() {
        let mut app = App::new();
        app.input_buffer = "test".to_string();
        app.auto_scroll = false;
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert!(app.input_buffer.is_empty());
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_home_scrolls_to_top() {
        let mut app = app_with_logs(20);
        app.scroll = 15;
        handle_key_event(&mut app, key(KeyCode::Home));
        assert_eq!(app.scroll, 0);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_end_scrolls_to_bottom() {
        let mut app = app_with_logs(20);
        app.auto_scroll = false;
        app.scroll = 0;
        handle_key_event(&mut app, key(KeyCode::End));
        assert_eq!(app.scroll, 19);
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_big_g_scrolls_to_bottom() {
        let mut app = app_with_logs(20);
        app.auto_scroll = false;
        app.scroll = 0;
        handle_key_event(&mut app, key(KeyCode::Char('G')));
        assert_eq!(app.scroll, 19);
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_small_g_scrolls_to_top() {
        let mut app = app_with_logs(20);
        app.scroll = 15;
        handle_key_event(&mut app, key(KeyCode::Char('g')));
        assert_eq!(app.scroll, 0);
        assert!(!app.auto_scroll);
    }

    #[test]
    fn test_page_up() {
        let mut app = app_with_logs(50);
        app.scroll = 30;
        app.visible_height = 20;
        handle_key_event(&mut app, key(KeyCode::PageUp));
        assert_eq!(app.scroll, 10);
    }

    #[test]
    fn test_page_down() {
        let mut app = app_with_logs(50);
        app.auto_scroll = false;
        app.scroll = 10;
        app.visible_height = 20;
        handle_key_event(&mut app, key(KeyCode::PageDown));
        assert_eq!(app.scroll, 30);
    }

    // --- Scroll clamping on filter change tests ---

    #[test]
    fn test_editing_char_clamps_scroll() {
        let mut app = app_with_logs(10);
        app.input_mode = InputMode::Editing;
        app.scroll = 9;
        // Typing a filter that matches fewer entries should clamp scroll
        app.input_buffer = "log ".to_string();
        handle_key_event(&mut app, key(KeyCode::Char('0')));
        // "log 0" matches only 1 entry, scroll should be clamped to 0
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_editing_backspace_clamps_scroll() {
        let mut app = app_with_logs(10);
        app.input_mode = InputMode::Editing;
        app.input_buffer = "log 0".to_string();
        app.scroll = 0;
        // Backspace broadens filter to "log " which matches all 10
        handle_key_event(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.input_buffer, "log ");
    }

    #[test]
    fn test_enter_clamps_scroll() {
        let mut app = app_with_logs(10);
        app.input_mode = InputMode::Editing;
        app.input_buffer = "log 0".to_string();
        app.scroll = 5; // Beyond the 1 match
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert_eq!(app.scroll, 0);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    // --- Editing mode tests ---

    #[test]
    fn test_editing_enter_applies_filter() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        app.input_buffer = "test".to_string();
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.input_buffer, "test");
    }

    #[test]
    fn test_editing_esc_clears_and_goes_to_normal() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        app.input_buffer = "test".to_string();
        app.auto_scroll = false;
        handle_key_event(&mut app, key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_editing_char_appends() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        handle_key_event(&mut app, key(KeyCode::Char('a')));
        handle_key_event(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.input_buffer, "ab");
    }

    #[test]
    fn test_editing_backspace_removes() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        app.input_buffer = "abc".to_string();
        handle_key_event(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.input_buffer, "ab");
    }

    #[test]
    fn test_editing_backspace_on_empty() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        handle_key_event(&mut app, key(KeyCode::Backspace));
        assert!(app.input_buffer.is_empty());
    }

    // --- KeyEventKind guard tests ---

    #[test]
    fn test_release_event_ignored() {
        let mut app = App::new();
        let release = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        handle_key_event(&mut app, release);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_repeat_event_ignored() {
        let mut app = App::new();
        let repeat = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Repeat,
            state: KeyEventState::NONE,
        };
        handle_key_event(&mut app, repeat);
        assert!(!app.should_quit);
    }

    // --- New feature key binding tests ---

    #[test]
    fn test_w_toggles_line_wrap() {
        let mut app = App::new();
        assert!(app.line_wrap);
        handle_key_event(&mut app, key(KeyCode::Char('w')));
        assert!(!app.line_wrap);
        handle_key_event(&mut app, key(KeyCode::Char('w')));
        assert!(app.line_wrap);
    }

    #[test]
    fn test_l_cycles_log_level() {
        let mut app = app_with_logs(5);
        assert!(app.min_log_level.is_none());
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.min_log_level, Some(LogLevel::Error));
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.min_log_level, Some(LogLevel::Warn));
    }

    #[test]
    fn test_l_clamps_scroll() {
        let mut app = App::new();
        app.on_log(LogEntry {
            raw: "err".to_string(),
            pretty: "err".to_string(),
            level: LogLevel::Error,
        });
        app.on_log(LogEntry {
            raw: "info".to_string(),
            pretty: "info".to_string(),
            level: LogLevel::Info,
        });
        app.scroll = 1;
        // Cycle to Error-only -> 1 match -> scroll clamps to 0
        handle_key_event(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_r_toggles_regex() {
        let mut app = App::new();
        assert!(!app.use_regex);
        handle_key_event(&mut app, key(KeyCode::Char('r')));
        assert!(app.use_regex);
        handle_key_event(&mut app, key(KeyCode::Char('r')));
        assert!(!app.use_regex);
    }

    #[test]
    fn test_r_clamps_scroll() {
        let mut app = App::new();
        app.on_log(LogEntry {
            raw: "hello 123".to_string(),
            pretty: "hello 123".to_string(),
            level: LogLevel::Info,
        });
        app.input_buffer = "[invalid".to_string();
        app.scroll = 0;
        // Toggle regex on -> invalid regex -> 0 matches -> scroll clamps
        handle_key_event(&mut app, key(KeyCode::Char('r')));
        assert_eq!(app.scroll, 0);
    }

    // --- Export key binding tests ---

    #[test]
    fn test_e_triggers_export() {
        let dir = std::env::temp_dir().join(format!(
            "smartlog_test_e_key_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let mut app = app_with_logs(3);
        app.export_dir = dir.clone();

        handle_key_event(&mut app, key(KeyCode::Char('e')));
        assert!(app.last_export_message.is_some());
        assert!(app
            .last_export_message
            .as_ref()
            .unwrap()
            .contains("Exported 3 logs"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_key_press_clears_export_message() {
        let mut app = App::new();
        app.last_export_message = Some("previous export".to_string());
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert!(app.last_export_message.is_none());
    }

    #[test]
    fn test_editing_key_clears_export_message() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        app.last_export_message = Some("previous export".to_string());
        handle_key_event(&mut app, key(KeyCode::Char('a')));
        assert!(app.last_export_message.is_none());
    }
}
