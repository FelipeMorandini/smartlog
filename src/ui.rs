//! Terminal UI rendering.
//!
//! This module handles all the terminal UI rendering using the Ratatui library.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, InputMode};
use crate::parser::style_log;

/// Renders the application UI to the terminal.
///
/// The UI consists of two sections:
/// - Main log area with filtered and styled logs
/// - Input bar showing the current search query and mode
///
/// # Arguments
///
/// * `f` - The Ratatui frame to render to
/// * `app` - The current application state
pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main Log Area
            Constraint::Length(3), // Input Bar
        ])
        .split(f.area());

    // --- 1. Filter Logs ---
    let query = &app.input_buffer;
    let filtered_logs: Vec<_> = app.logs
        .iter()
        .filter(|log| query.is_empty() || log.pretty.to_lowercase().contains(&query.to_lowercase()))
        .collect();

    // --- 2. Calculate Scroll ---
    // If auto_scroll is true, we force scroll to the end of the filtered list
    let scroll_pos = if app.auto_scroll {
        filtered_logs.len().saturating_sub(chunks[0].height as usize) as u16
    } else {
        app.scroll as u16
    };

    // --- 3. Render Logs ---
    let styled_logs: Vec<_> = filtered_logs
        .iter()
        .map(|log| style_log(log, query))
        .collect();

    let logs_block = Paragraph::new(styled_logs)
        .block(Block::default().borders(Borders::ALL).title(" SmartLog "))
        .wrap(Wrap { trim: false }) // Don't trim JSON indentation
        .scroll((scroll_pos, 0));

    f.render_widget(logs_block, chunks[0]);

    // --- 4. Render Input Bar ---
    let (input_style, border_style) = match app.input_mode {
        InputMode::Normal => (Style::default(), Style::default()),
        InputMode::Editing => (Style::default().fg(Color::Yellow), Style::default().fg(Color::Yellow)),
    };

    let status_text = if app.auto_scroll { "FOLLOWING" } else { "PAUSED" };
    let title = format!(" Filter (Press /) | {} ", status_text);

    let input_block = Paragraph::new(app.input_buffer.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );

    f.render_widget(input_block, chunks[1]);
}