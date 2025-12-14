//! Main application event loop.
//!
//! This module handles the core event loop that processes:
//! - Incoming log messages
//! - User keyboard input
//! - OS shutdown signals

use std::{io, time::Duration, future::Future};
use crossterm::event::{self, Event};
use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::app::App;
use crate::config::UI_POLL_INTERVAL_MS;
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
/// - User keyboard input
/// - OS shutdown signals (SIGINT/SIGTERM)
///
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

    loop {
        terminal.draw(|f| ui(f, app))?;

        tokio::select! {
            // Handle incoming logs
            maybe_line = rx.recv() => {
                match maybe_line {
                    Some(line) => {
                        let entry = parse_log(line);
                        app.on_log(entry);
                    }
                    None => {
                        // Producer ended; exit gracefully
                        app.should_quit = true;
                    }
                }
            }

            // Handle user input
            // Poll at UI_POLL_INTERVAL_MS to keep UI responsive but not CPU heavy
            _ = async {}, if event::poll(Duration::from_millis(UI_POLL_INTERVAL_MS))? => {
                if let Event::Key(key) = event::read()? {
                    handle_key_event(app, key);
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

