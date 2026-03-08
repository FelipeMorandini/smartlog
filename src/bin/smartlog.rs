//! SmartLog - A terminal UI for tailing and filtering JSON logs.
//!
//! This is the main binary entry point that sets up the terminal,
//! spawns the log ingestion task, and runs the main event loop.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
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

    /// Directory for exported log files (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    export_dir: Option<PathBuf>,

    /// Enable verbose debug logging to smartlog_debug.log
    #[arg(short, long)]
    verbose: bool,
}

/// Derives a human-readable source label from CLI args and stdin state.
fn source_label(file: Option<&str>) -> String {
    if let Some(path) = file {
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        format!("file: {name}")
    } else if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        "stdin".to_string()
    } else {
        "demo".to_string()
    }
}

/// Initializes the tracing subscriber for debug logging.
///
/// When `verbose` is true, logs to `smartlog_debug.log` in the current directory.
/// When false, no subscriber is installed (zero overhead).
fn init_tracing(verbose: bool) -> Result<()> {
    if verbose {
        let file = std::fs::File::create("smartlog_debug.log")
            .context("Failed to create smartlog_debug.log")?;
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .init();
        tracing::info!("SmartLog debug logging enabled");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 0. Initialize tracing (before anything else)
    init_tracing(args.verbose)?;

    // 1. Setup Terminal
    let (mut terminal, _guard) = terminal::init()?;

    // 2. Setup App State
    let mut app = App::new();
    app.source_label = source_label(args.file.as_deref());
    if let Some(dir) = args.export_dir {
        app.export_dir = dir;
    }
    tracing::debug!(source = %app.source_label, "App initialized");

    // 3. Setup Async Data Channel
    let (tx, mut rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    // 4. Spawn Data Producer (File, Stdin, or Mock)
    let producer = spawn_source(args.file, tx);
    tracing::debug!("Log source spawned");

    // 5. Main Event Loop
    let res = event_loop::run(&mut terminal, &mut app, &mut rx).await;

    // 6. Stop producer task promptly
    producer.abort();

    // 7. Restore Terminal (Critical step!)
    // TerminalGuard will handle restoration on drop, but we also explicitly restore
    terminal::restore(&mut terminal)?;

    tracing::debug!("Shutdown complete");

    // 8. Propagate errors (terminal is already restored above)
    res?;

    Ok(())
}
