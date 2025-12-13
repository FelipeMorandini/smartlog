use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;

use smartlog::app::App;
use smartlog::inputs::handle_key_event;
use smartlog::parser::parse_log;
use smartlog::ui::ui;

/// SmartLog: A TUI for tailing and filtering JSON logs
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to file to tail (optional, defaults to mock stream if empty)
    #[arg(short, long)]
    file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Setup App State
    let mut app = App::new();

    // 3. Setup Async Data Channel
    let (tx, mut rx) = mpsc::channel(100);

    // 4. Spawn Data Producer (Mock or File)
    tokio::spawn(async move {
        if let Some(_path) = args.file {
            // Real implementation would implement `tail -f` here using `tokio::fs`
            let _ = tx.send(r#"{"level":"INFO", "msg":"Reading from file not implemented in demo"}"#.to_string()).await;
        } else {
            // Mock Log Generator
            let logs = vec![
                r#"{"level": "INFO", "msg": "Service started", "port": 8080, "env": "prod"}"#,
                r#"{"level": "WARN", "msg": "High memory usage", "usage": "85%"}"#,
                "Standard text log line here (nginx style perhaps)",
                r#"{"level": "ERROR", "msg": "DB Connection Failed", "error_code": 500, "details": {"retries": 3}}"#,
                r#"{"level": "DEBUG", "msg": "Payload received", "payload_size": 1024}"#,
            ];

            loop {
                for log in &logs {
                    if tx.send(log.to_string()).await.is_err() {
                        return; // Channel closed
                    }
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                }
            }
        }
    });

    // 5. Main Event Loop
    let res = run_app(&mut terminal, &mut app, &mut rx).await;

    // 6. Restore Terminal (Critical step!)
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::Receiver<String>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        tokio::select! {
            // Handle incoming logs
            Some(line) = rx.recv() => {
                let entry = parse_log(line);
                app.on_log(entry);
            }

            // Handle user input
            // Poll at 100ms interval to keep UI responsive but not CPU heavy
            _ = async {}, if event::poll(Duration::from_millis(100))? => {
                if let Event::Key(key) = event::read()? {
                    handle_key_event(app, key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}