//! Async log input sources: stdin, file tailing, and mock generator.

use crate::config::{FILE_POLL_INTERVAL_MS, MAX_LOG_LINE_SIZE};
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
/// - If truncated/rotated, reopen from beginning
/// - Keeps file handle open across poll cycles for efficiency
fn spawn_tail_file(path: PathBuf, tx: mpsc::Sender<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut offset: u64 = wait_for_file(&path, &tx).await;
        let mut reader: Option<BufReader<fs::File>> = None;
        let mut buf = String::new();

        loop {
            match fs::metadata(&path).await {
                Ok(meta) => {
                    let len = meta.len();
                    if len < offset {
                        let _ = tx
                            .send(format!("⚠️  File truncated or rotated: {}", path.display()))
                            .await;
                        offset = 0;
                        reader = None; // Force reopen
                    }

                    if len > offset {
                        if reader.is_none() {
                            reader = open_and_seek(&path, offset).await;
                        }
                        if let Some(ref mut r) = reader {
                            if read_new_lines(r, &tx, &mut buf).await.is_err() {
                                return; // Receiver dropped
                            }
                            offset = r.stream_position().await.unwrap_or(len);
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(format!("⚠️  File unavailable: {} ({})", path.display(), e))
                        .await;
                    offset = 0;
                    reader = None; // Close handle, file may have been deleted
                }
            }

            tokio::time::sleep(Duration::from_millis(FILE_POLL_INTERVAL_MS)).await;
        }
    })
}

/// Waits for a file to appear and returns its initial length.
async fn wait_for_file(path: &PathBuf, tx: &mpsc::Sender<String>) -> u64 {
    match fs::metadata(path).await {
        Ok(meta) => meta.len(),
        Err(e) => {
            let _ = tx
                .send(format!(
                    "⚠️  Waiting for file to exist: {} ({})",
                    path.display(),
                    e
                ))
                .await;
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Ok(meta) = fs::metadata(path).await {
                    let _ = tx.send(format!("✓ File found: {}", path.display())).await;
                    return meta.len();
                }
            }
        }
    }
}

/// Opens a file and seeks to the given offset, returning a buffered reader.
async fn open_and_seek(path: &PathBuf, offset: u64) -> Option<BufReader<fs::File>> {
    let file = fs::File::open(path).await.ok()?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset)).await.ok()?;
    Some(reader)
}

/// Reads all new lines from the reader and sends them to the channel.
///
/// Returns `Ok(())` on success or EOF, `Err(())` if the channel receiver was dropped.
async fn read_new_lines(
    reader: &mut BufReader<fs::File>,
    tx: &mpsc::Sender<String>,
    buf: &mut String,
) -> Result<(), ()> {
    loop {
        buf.clear();
        match reader.read_line(buf).await {
            Ok(0) => return Ok(()), // EOF reached
            Ok(_) => {
                while buf.ends_with(['\n', '\r']) {
                    buf.pop();
                }
                let was_oversized = buf.len() > MAX_LOG_LINE_SIZE;
                truncate_line(buf);
                if tx.send(buf.clone()).await.is_err() {
                    return Err(()); // Receiver dropped
                }
                if was_oversized {
                    buf.shrink_to(MAX_LOG_LINE_SIZE);
                }
            }
            Err(e) => {
                let _ = tx.send(format!("⚠️  Error reading line: {}", e)).await;
                return Ok(());
            }
        }
    }
}

/// Suffix appended to truncated lines.
const TRUNCATION_SUFFIX: &str = " ... [truncated]";

/// Truncates a line in-place if it exceeds [`MAX_LOG_LINE_SIZE`] bytes.
///
/// The resulting string (including the truncation suffix) is guaranteed to
/// be at most `MAX_LOG_LINE_SIZE` bytes. Truncation respects UTF-8 char
/// boundaries so we never produce invalid strings.
fn truncate_line(line: &mut String) {
    if line.len() > MAX_LOG_LINE_SIZE {
        if TRUNCATION_SUFFIX.len() >= MAX_LOG_LINE_SIZE {
            // Suffix alone would exceed the limit — just hard-truncate.
            let mut end = MAX_LOG_LINE_SIZE;
            while end > 0 && !line.is_char_boundary(end) {
                end -= 1;
            }
            line.truncate(end);
            return;
        }

        let mut end = MAX_LOG_LINE_SIZE - TRUNCATION_SUFFIX.len();
        while end > 0 && !line.is_char_boundary(end) {
            end -= 1;
        }
        line.truncate(end);
        line.push_str(TRUNCATION_SUFFIX);
    }
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
                    while buf.ends_with(['\n', '\r']) {
                        buf.pop();
                    }
                    let was_oversized = buf.len() > MAX_LOG_LINE_SIZE;
                    truncate_line(&mut buf);
                    if tx.send(buf.clone()).await.is_err() {
                        return; // Receiver dropped
                    }
                    if was_oversized {
                        buf.shrink_to(MAX_LOG_LINE_SIZE);
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(format!("⚠️  Error reading from stdin: {}", e))
                        .await;
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
                let mut line = log.to_string();
                truncate_line(&mut line);
                if tx.send(line).await.is_err() {
                    return; // Channel closed
                }
                tokio::time::sleep(Duration::from_millis(1500)).await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_line_short_unchanged() {
        let mut line = "short line".to_string();
        truncate_line(&mut line);
        assert_eq!(line, "short line");
    }

    #[test]
    fn test_truncate_line_exact_limit_unchanged() {
        let mut line = "a".repeat(MAX_LOG_LINE_SIZE);
        truncate_line(&mut line);
        assert_eq!(line.len(), MAX_LOG_LINE_SIZE);
    }

    #[test]
    fn test_truncate_line_over_limit() {
        let mut line = "a".repeat(MAX_LOG_LINE_SIZE + 100);
        truncate_line(&mut line);
        assert!(line.ends_with(TRUNCATION_SUFFIX));
        assert!(line.len() <= MAX_LOG_LINE_SIZE);
    }

    #[test]
    fn test_truncate_line_respects_utf8_boundary() {
        // Multi-byte char: each is 3 bytes in UTF-8. Create a string that just
        // exceeds MAX_LOG_LINE_SIZE bytes to trigger truncation without over-allocating.
        let repeat_count = MAX_LOG_LINE_SIZE / 3 + 1;
        let mut line = "\u{4e16}".repeat(repeat_count);
        truncate_line(&mut line);
        assert!(line.ends_with(TRUNCATION_SUFFIX));
        assert!(line.len() <= MAX_LOG_LINE_SIZE);
        // Must still be valid UTF-8
        let _ = line.as_str();
    }

    #[test]
    fn test_truncate_line_empty() {
        let mut line = String::new();
        truncate_line(&mut line);
        assert!(line.is_empty());
    }
}
