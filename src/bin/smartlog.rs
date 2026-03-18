//! SmartLog - A terminal UI for tailing and filtering JSON logs.
//!
//! This is the main binary entry point that sets up the terminal,
//! spawns the log ingestion task, and runs the main event loop.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;
use tokio::sync::mpsc;

use smartlog::app::App;
use smartlog::config::CHANNEL_BUFFER_SIZE;
use smartlog::event_loop;
use smartlog::sources::spawn_sources;
use smartlog::terminal;
use smartlog::theme::Theme;

/// SmartLog: A TUI for tailing and filtering JSON logs
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path(s) to file(s) to tail (can specify multiple with repeated --file flags)
    #[arg(short, long, value_name = "FILE")]
    file: Vec<String>,

    /// Directory for exported log files (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    export_dir: Option<PathBuf>,

    /// Enable verbose debug logging to smartlog_debug.log
    #[arg(short, long)]
    verbose: bool,

    /// Path for the debug log file (requires --verbose)
    #[arg(long, value_name = "PATH")]
    debug_log: Option<PathBuf>,

    /// Color theme: dark, light, solarized, dracula (default: dark)
    #[arg(long, value_name = "THEME", default_value = "dark")]
    theme: String,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
enum Command {
    /// Generate shell completions for the specified shell
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

/// Derives a human-readable source label from CLI args and stdin state.
fn source_label(files: &[String]) -> String {
    match files.len() {
        0 => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                "stdin".to_string()
            } else {
                "demo".to_string()
            }
        }
        1 => {
            let name = std::path::Path::new(&files[0])
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| files[0].clone());
            format!("file: {name}")
        }
        n => format!("{n} files"),
    }
}

/// Initializes the tracing subscriber for debug logging.
///
/// When `verbose` is true, logs to the given path (defaulting to
/// `smartlog_debug.log` in the current directory).
/// When false, no subscriber is installed (zero overhead).
fn init_tracing(verbose: bool, debug_log: Option<&std::path::Path>) -> Result<()> {
    if verbose {
        let default_path = PathBuf::from("smartlog_debug.log");
        let path = debug_log.unwrap_or(&default_path);
        let file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create debug log: {}", path.display()))?;
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .init();
        tracing::info!(path = %path.display(), "SmartLog debug logging enabled");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle completions subcommand (no TUI needed)
    if let Some(Command::Completions { shell }) = &args.command {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(*shell, &mut cmd, name, &mut std::io::stdout());
        return Ok(());
    }

    // 0. Initialize tracing (before anything else)
    init_tracing(args.verbose, args.debug_log.as_deref())?;

    // 1. Setup Terminal
    let (mut terminal, _guard) = terminal::init()?;

    // 2. Setup App State
    let mut app = App::new();
    app.source_label = source_label(&args.file);
    app.theme = Theme::by_name(&args.theme);
    if let Some(dir) = args.export_dir {
        app.export_dir = dir;
    }
    tracing::debug!(source = %app.source_label, theme = app.theme.name, "App initialized");

    // 3. Setup Async Data Channel
    let (tx, mut rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    // 4. Spawn Data Producer(s) (File(s), Stdin, or Mock)
    let producers = spawn_sources(&args.file, tx);
    tracing::debug!(count = producers.len(), "Log source(s) spawned");

    // 5. Main Event Loop
    let res = event_loop::run(&mut terminal, &mut app, &mut rx).await;

    // 6. Stop producer tasks promptly
    for producer in &producers {
        producer.abort();
    }

    // 7. Restore Terminal (Critical step!)
    // TerminalGuard will handle restoration on drop, but we also explicitly restore
    terminal::restore(&mut terminal)?;

    tracing::debug!("Shutdown complete");

    // 8. Propagate errors (terminal is already restored above)
    res?;

    Ok(())
}
