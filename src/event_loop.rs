//! Main application event loop.
//!
//! This module handles the core event loop that processes:
//! - Incoming log messages
//! - User keyboard input
//! - OS shutdown signals

use crossterm::event::{Event, EventStream, MouseEventKind};
use futures_util::StreamExt;
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::{future::Future, io};
use tokio::sync::mpsc;

use crate::app::App;
use crate::inputs::handle_key_event;
use crate::parser::parse_log;
use crate::ui::ui;

/// Creates a future that completes when a shutdown signal is received.
///
/// On Unix: handles both SIGINT (Ctrl+C) and SIGTERM
/// On other platforms: handles only SIGINT (Ctrl+C)
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {},
                    _ = sigterm.recv() => {},
                }
            }
            Err(_) => {
                // SIGTERM not available (sandboxed/containerized env); Ctrl+C only
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Maximum consecutive event stream errors before force-quitting.
const MAX_CONSECUTIVE_EVENT_ERRORS: u32 = 50;

/// Processes an incoming log channel message.
///
/// Returns `true` if the channel is still open, `false` when the producer
/// has ended (channel yields `None`).
fn handle_log_message(app: &mut App, maybe_line: Option<String>) -> bool {
    match maybe_line {
        Some(line) => {
            tracing::trace!(len = line.len(), "Log line received");
            let entry = parse_log(line);
            app.on_log(entry);
            true
        }
        None => {
            tracing::debug!("Log channel closed");
            false
        }
    }
}

/// Returns a display-safe representation of a key code for tracing.
///
/// Character keys are redacted to `Char(*)` to avoid logging sensitive
/// user input (e.g., filter queries) into debug log files.
fn redact_key(code: crossterm::event::KeyCode) -> String {
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Char(_) => "Char(*)".to_string(),
        other => format!("{:?}", other),
    }
}

/// Processes a terminal event (keyboard, mouse, or error).
///
/// Returns the updated consecutive error count. Sets `app.should_quit` when
/// the error threshold is reached or the event stream ends.
fn handle_terminal_event(
    app: &mut App,
    maybe_event: Option<Result<Event, io::Error>>,
    consecutive_errors: u32,
) -> u32 {
    match maybe_event {
        Some(Ok(Event::Key(key))) => {
            tracing::trace!(code = %redact_key(key.code), "Key event");
            handle_key_event(app, key);
            0
        }
        Some(Ok(Event::Mouse(mouse))) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => app.scroll_up(),
                MouseEventKind::ScrollDown => app.scroll_down(),
                _ => {}
            }
            0
        }
        Some(Ok(_)) => 0,
        Some(Err(_)) => {
            let new_count = consecutive_errors + 1;
            if new_count >= MAX_CONSECUTIVE_EVENT_ERRORS {
                app.should_quit = true;
            }
            new_count
        }
        None => {
            app.should_quit = true;
            consecutive_errors
        }
    }
}

/// Updates the visible height from the current terminal size.
fn update_visible_height(app: &mut App) {
    if let Ok(size) = crossterm::terminal::size() {
        app.visible_height = size.1.saturating_sub(5);
    }
}

/// Runs the main application event loop.
///
/// Multiplexes log messages, user input, and OS signals via `tokio::select!`.
/// When the log producer closes (e.g., stdin EOF), the app continues running
/// so the user can scroll and filter buffered logs. Exits when `should_quit`
/// is set.
pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::Receiver<String>,
) -> io::Result<()> {
    let mut shutdown_fut =
        std::pin::Pin::from(Box::new(shutdown_signal()) as Box<dyn Future<Output = ()> + Send>);

    let mut event_stream = EventStream::new();
    let mut channel_open = true;
    let mut consecutive_event_errors: u32 = 0;

    loop {
        terminal.draw(|f| ui(f, app))?;
        update_visible_height(app);

        tokio::select! {
            maybe_line = rx.recv(), if channel_open => {
                channel_open = handle_log_message(app, maybe_line);
            }
            maybe_event = event_stream.next() => {
                consecutive_event_errors = handle_terminal_event(app, maybe_event, consecutive_event_errors);
            }
            _ = &mut shutdown_fut => {
                tracing::info!("Shutdown signal received");
                app.should_quit = true;
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    };

    fn press_key(code: KeyCode) -> Option<Result<Event, io::Error>> {
        Some(Ok(Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })))
    }

    fn mouse_scroll(kind: MouseEventKind) -> Option<Result<Event, io::Error>> {
        Some(Ok(Event::Mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })))
    }

    // --- handle_log_message tests ---

    #[test]
    fn test_handle_log_message_adds_entry() {
        let mut app = App::new();
        let open = handle_log_message(&mut app, Some("hello".to_string()));
        assert!(open);
        assert_eq!(app.logs.len(), 1);
        assert_eq!(app.logs[0].raw, "hello");
    }

    #[test]
    fn test_handle_log_message_none_closes_channel() {
        let mut app = App::new();
        let open = handle_log_message(&mut app, None);
        assert!(!open);
        assert!(!app.should_quit); // App should NOT quit on channel close
    }

    // --- handle_terminal_event tests ---

    #[test]
    fn test_handle_terminal_event_key_q_quits() {
        let mut app = App::new();
        let errors = handle_terminal_event(&mut app, press_key(KeyCode::Char('q')), 0);
        assert_eq!(errors, 0);
        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_terminal_event_key_resets_error_count() {
        let mut app = App::new();
        app.on_log(crate::parser::parse_log("test".to_string()));
        let errors = handle_terminal_event(&mut app, press_key(KeyCode::Down), 10);
        assert_eq!(errors, 0);
    }

    #[test]
    fn test_handle_terminal_event_mouse_scroll_up() {
        let mut app = App::new();
        app.on_log(crate::parser::parse_log("a".to_string()));
        app.on_log(crate::parser::parse_log("b".to_string()));
        app.scroll = 1;
        let errors = handle_terminal_event(&mut app, mouse_scroll(MouseEventKind::ScrollUp), 0);
        assert_eq!(errors, 0);
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn test_handle_terminal_event_mouse_scroll_down() {
        let mut app = App::new();
        app.on_log(crate::parser::parse_log("a".to_string()));
        app.on_log(crate::parser::parse_log("b".to_string()));
        app.auto_scroll = false;
        app.scroll = 0;
        let errors = handle_terminal_event(&mut app, mouse_scroll(MouseEventKind::ScrollDown), 0);
        assert_eq!(errors, 0);
        assert_eq!(app.scroll, 1);
    }

    #[test]
    fn test_handle_terminal_event_mouse_other_ignored() {
        let mut app = App::new();
        let errors = handle_terminal_event(
            &mut app,
            mouse_scroll(MouseEventKind::Down(MouseButton::Left)),
            5,
        );
        assert_eq!(errors, 0); // resets on any Ok event
        assert!(!app.should_quit);
    }

    #[test]
    fn test_handle_terminal_event_other_event_resets_errors() {
        let mut app = App::new();
        let errors = handle_terminal_event(&mut app, Some(Ok(Event::FocusGained)), 15);
        assert_eq!(errors, 0);
    }

    #[test]
    fn test_handle_terminal_event_error_increments() {
        let mut app = App::new();
        let err = io::Error::other("test");
        let errors = handle_terminal_event(&mut app, Some(Err(err)), 0);
        assert_eq!(errors, 1);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_handle_terminal_event_error_threshold_quits() {
        let mut app = App::new();
        let err = io::Error::other("test");
        let errors = handle_terminal_event(&mut app, Some(Err(err)), 49);
        assert_eq!(errors, 50);
        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_terminal_event_none_quits() {
        let mut app = App::new();
        let errors = handle_terminal_event(&mut app, None, 0);
        assert_eq!(errors, 0);
        assert!(app.should_quit);
    }
}
