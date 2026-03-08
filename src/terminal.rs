//! Terminal setup and restoration utilities.
//!
//! This module handles terminal initialization, raw mode, and ensures
//! proper cleanup even on panics or signals.

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Guard that ensures terminal is restored on drop (even on panic).
pub struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut w = io::stdout();
        let _ = execute!(
            w,
            crossterm::cursor::Show,
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = disable_raw_mode();
    }
}

/// Initializes the terminal for TUI mode.
///
/// Returns a terminal instance and a guard that will restore terminal state on drop.
pub fn init() -> Result<(Terminal<CrosstermBackend<io::Stdout>>, TerminalGuard)> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok((terminal, TerminalGuard))
}

/// Restores the terminal to normal mode.
pub fn restore(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    disable_raw_mode()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_drop_does_not_panic() {
        // TerminalGuard::drop runs cleanup commands that may fail gracefully
        // in non-TTY environments. The key invariant is that it never panics.
        let guard = TerminalGuard;
        drop(guard);
    }

    #[test]
    fn test_terminal_guard_multiple_drops_safe() {
        // Dropping multiple guards sequentially should not cause issues.
        // In production only one guard exists, but this tests idempotency.
        let g1 = TerminalGuard;
        let g2 = TerminalGuard;
        drop(g1);
        drop(g2);
    }
}
