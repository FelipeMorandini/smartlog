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
/// `Esc` clears search
///
/// **Editing mode**: `Enter` applies filter, `Esc` cancels and clears filter,
/// characters are added to input buffer
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

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
            KeyCode::Esc => {
                app.input_buffer.clear();
                app.auto_scroll = true;
            }
            _ => {}
        },
        InputMode::Editing => match key.code {
            KeyCode::Enter => app.input_mode = InputMode::Normal,
            KeyCode::Char(c) => app.input_buffer.push(c),
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input_buffer.clear();
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
}
