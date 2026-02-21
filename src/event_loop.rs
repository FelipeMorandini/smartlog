//! Main application event loop.
//!
//! This module handles the core event loop that processes:
//! - Incoming log messages
//! - User keyboard input
//! - OS shutdown signals

use std::{io, future::Future};
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use ratatui::backend::Backend;
use ratatui::Terminal;
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
        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Runs the main application event loop.
///
/// This function handles three types of events:
/// - Incoming log messages from the channel
/// - User keyboard input (async via crossterm EventStream)
/// - OS shutdown signals (SIGINT/SIGTERM)
///
/// When the log producer channel closes (e.g., stdin EOF), the app continues
/// running so the user can still scroll, filter, and review buffered logs.
/// The loop terminates when `app.should_quit` is set to `true`.
///
/// # Arguments
///
/// * `terminal` - The terminal instance to draw to
/// * `app` - The application state
/// * `rx` - Channel receiver for incoming log lines
///
/// # Returns
///
/// `Ok(())` on successful exit, or an IO error if terminal operations fail.
pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::Receiver<String>,
) -> io::Result<()> {
    let mut shutdown_fut = std::pin::Pin::from(
        Box::new(shutdown_signal()) as Box<dyn Future<Output = ()> + Send>
    );

    let mut event_stream = EventStream::new();
    let mut channel_open = true;

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Update visible height for page scroll calculations
        if let Ok(size) = crossterm::terminal::size() {
            app.visible_height = size.1.saturating_sub(5);
        }

        tokio::select! {
            // Handle incoming logs (only if channel is still open)
            maybe_line = rx.recv(), if channel_open => {
                match maybe_line {
                    Some(line) => {
                        let entry = parse_log(line);
                        app.on_log(entry);
                    }
                    None => {
                        // Producer ended; stop listening but don't quit.
                        // User can still scroll/filter existing logs and quit with 'q'.
                        channel_open = false;
                    }
                }
            }

            // Handle user input (async via EventStream)
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        handle_key_event(app, key);
                    }
                    Some(Ok(_)) => {
                        // Mouse events, resize events, etc. -- ignore for now
                    }
                    Some(Err(_)) => {
                        // Event read error -- ignore and continue
                    }
                    None => {
                        // EventStream ended (shouldn't happen normally)
                        app.should_quit = true;
                    }
                }
            }

            // Handle OS shutdown signals
            _ = &mut shutdown_fut => {
                app.should_quit = true;
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
