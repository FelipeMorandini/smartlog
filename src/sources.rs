//! Async log input sources: stdin, file tailing, and mock generator.

use crate::config::FILE_POLL_INTERVAL_MS;
use std::io::IsTerminal;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;

use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Spawns the most appropriate log source based on CLI args and whether stdin is a TTY.
///
/// Priority:
/// - If `file` is Some, tail that file.
/// - Else if stdin is piped (not a TTY), read from stdin.
/// - Else spawn a mock generator.
pub fn spawn_source(file: Option<String>, tx: mpsc::Sender<String>) -> JoinHandle<()> {
    if let Some(path) = file {
        return spawn_tail_file(PathBuf::from(path), tx);
    }

    // Detect if stdin is piped; if so, read from it.
    if !std::io::stdin().is_terminal() {
        return spawn_stdin_reader(tx);
    }

    // Fallback: mock generator
    spawn_mock(tx)
}

/// Tail a file like `tail -F` (basic):
/// - Start from end of file
/// - Periodically check for growth and read new lines
/// - If truncated/rotated, start from beginning
fn spawn_tail_file(path: PathBuf, tx: mpsc::Sender<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut offset: u64;

        // Initialize offset to current file length if it exists
        match fs::metadata(&path).await {
            Ok(meta) => offset = meta.len(),
            Err(e) => {
                // Send error message but continue trying
                let _ = tx.send(format!("⚠️  Waiting for file to exist: {} ({})", path.display(), e)).await;

                // If file doesn't exist yet, wait until it appears
                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    match fs::metadata(&path).await {
                        Ok(meta) => {
                            offset = meta.len();
                            let _ = tx.send(format!("✓ File found: {}", path.display())).await;
                            break;
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        let mut buf = String::new();

        loop {
            // Periodically check for new content or truncation
            match fs::metadata(&path).await {
                Ok(meta) => {
                    let len = meta.len();
                    if len < offset {
                        // Truncated or rotated
                        let _ = tx.send(format!("⚠️  File truncated or rotated: {}", path.display())).await;
                        offset = 0;
                    }

                    if len > offset {
                        // Read new data from offset to EOF
                        match fs::File::open(&path).await {
                            Ok(file) => {
                                let mut reader = BufReader::new(file);
                                if reader.seek(SeekFrom::Start(offset)).await.is_ok() {
                                    loop {
                                        buf.clear();
                                        match reader.read_line(&mut buf).await {
                                            Ok(0) => break, // EOF reached
                                            Ok(_) => {
                                                // Strip trailing newline to normalize
                                                if buf.ends_with('\n') {
                                                    while buf.ends_with(['\n', '\r']) {
                                                        buf.pop();
                                                    }
                                                }
                                                if tx.send(buf.clone()).await.is_err() {
                                                    return; // Receiver dropped
                                                }
                                            }
                                            Err(e) => {
                                                let _ = tx.send(format!("⚠️  Error reading line: {}", e)).await;
                                                break;
                                            }
                                        }
                                    }
                                    // Update offset to actual reader position after reading
                                    offset = reader.stream_position().await.unwrap_or(len);
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!("⚠️  Error opening file: {}", e)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    // File temporarily unavailable; reset offset and keep trying
                    let _ = tx.send(format!("⚠️  File unavailable: {} ({})", path.display(), e)).await;
                    offset = 0;
                }
            }

            tokio::time::sleep(Duration::from_millis(FILE_POLL_INTERVAL_MS)).await;
        }
    })
}

/// Spawn a task that reads newline-delimited logs from stdin (piped input).
fn spawn_stdin_reader(tx: mpsc::Sender<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut buf = String::new();

        loop {
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) => {
                    // EOF reached - normal end of piped input
                    let _ = tx.send("ℹ️  End of input stream reached".to_string()).await;
                    return;
                }
                Ok(_) => {
                    if buf.ends_with('\n') {
                        while buf.ends_with(['\n', '\r']) {
                            buf.pop();
                        }
                    }
                    if tx.send(buf.clone()).await.is_err() {
                        return; // Receiver dropped
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("⚠️  Error reading from stdin: {}", e)).await;
                    return;
                }
            }
        }
    })
}

/// Spawn a mock generator for demo purposes.
fn spawn_mock(tx: mpsc::Sender<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
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
    })
}
