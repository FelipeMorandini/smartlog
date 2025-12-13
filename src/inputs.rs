//! Keyboard input handling for the application.

use crossterm::event::{KeyCode, KeyEvent};
use crate::app::{App, InputMode};

/// Handles keyboard input events and updates the application state.
///
/// # Arguments
///
/// * `app` - The application state
/// * `key` - The keyboard event to handle
///
/// # Behavior
///
/// **Normal mode**: `q` quits, `/` enters editing, `k`/`j` or arrows scroll, `Esc` clears search
///
/// **Editing mode**: `Enter`/`Esc` exits editing, characters are added to input buffer
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('/') => {
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
            KeyCode::Esc => {
                // Reset search
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
            }
            _ => {}
        },
    }
}