//! SmartLog - A terminal UI for tailing and filtering JSON logs.
//!
//! This is the main binary entry point that sets up the terminal,
//! spawns the log ingestion task, and runs the main event loop.

use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;

use smartlog::app::App;
use smartlog::config::CHANNEL_BUFFER_SIZE;
use smartlog::event_loop;
use smartlog::sources::spawn_source;
use smartlog::terminal;

/// SmartLog: A TUI for tailing and filtering JSON logs
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to file to tail (optional, reads from stdin if piped, otherwise shows demo)
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Setup Terminal
    let (mut terminal, _guard) = terminal::init()?;

    // 2. Setup App State
    let mut app = App::new();

    // 3. Setup Async Data Channel
    let (tx, mut rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    // 4. Spawn Data Producer (File, Stdin, or Mock)
    let producer = spawn_source(args.file, tx);

    // 5. Main Event Loop
    let res = event_loop::run(&mut terminal, &mut app, &mut rx).await;

    // 6. Stop producer task promptly
    producer.abort();

    // 7. Restore Terminal (Critical step!)
    // TerminalGuard will handle restoration on drop, but we also explicitly restore
    terminal::restore(&mut terminal)?;

    // 8. Propagate errors (terminal is already restored above)
    res?;

    Ok(())
}
