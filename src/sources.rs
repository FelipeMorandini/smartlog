//! Async log input sources: stdin, file tailing, and mock generator.

use crate::config::{FILE_POLL_INTERVAL_MS, MAX_LOG_LINE_SIZE};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// A raw log line with optional source metadata.
///
/// Used as the channel message type between log sources and the event loop.
pub struct RawLogMessage {
    /// The raw log line text.
    pub line: String,
    /// Optional source identifier (e.g., filename for multi-file tailing).
    /// Uses `Arc<str>` so cloning per message is a cheap pointer copy.
    pub source: Option<Arc<str>>,
}

/// Spawns log source(s) based on CLI args and whether stdin is a TTY.
///
/// When multiple files are provided, each gets its own tailing task with
/// source metadata. All tasks share the same channel sender (multi-producer).
///
/// Priority:
/// - If `files` is non-empty, tail those files.
/// - Else if stdin is piped (not a TTY), read from stdin.
/// - Else spawn a mock generator.
pub fn spawn_sources(files: &[String], tx: mpsc::Sender<RawLogMessage>) -> Vec<JoinHandle<()>> {
    if !files.is_empty() {
        let multi = files.len() > 1;
        let basename_counts = if multi {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for path in files {
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                *counts.entry(name).or_insert(0) += 1;
            }
            counts
        } else {
            HashMap::new()
        };
        return files
            .iter()
            .map(|path| {
                let source: Option<Arc<str>> = if multi {
                    let basename = Path::new(path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned());
                    let label = match basename {
                        Some(ref name) if basename_counts.get(name).copied().unwrap_or(0) > 1 => {
                            path.clone()
                        }
                        Some(name) => name,
                        None => path.clone(),
                    };
                    Some(Arc::from(label))
                } else {
                    None
                };
                tracing::info!(path = %path, multi, "Spawning file tail source");
                spawn_tail_file(PathBuf::from(path), source, tx.clone())
            })
            .collect();
    }

    if !std::io::stdin().is_terminal() {
        tracing::info!("Spawning stdin source");
        return vec![spawn_stdin_reader(tx)];
    }

    tracing::info!("Spawning mock demo source");
    vec![spawn_mock(tx)]
}

/// Tail a file like `tail -F` (basic):
/// - Start from end of file
/// - Periodically check for growth and read new lines
/// - If truncated/rotated, reopen from beginning
/// - Keeps file handle open across poll cycles for efficiency
fn spawn_tail_file(
    path: PathBuf,
    source: Option<Arc<str>>,
    tx: mpsc::Sender<RawLogMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut offset: u64 = wait_for_file(&path, &tx, &source).await;
        let mut reader: Option<BufReader<fs::File>> = None;
        let mut buf = String::new();
        let mut raw = Vec::new();

        loop {
            match fs::metadata(&path).await {
                Ok(meta) => {
                    let len = meta.len();
                    if len < offset {
                        let _ = tx
                            .send(RawLogMessage {
                                line: format!("⚠️  File truncated or rotated: {}", path.display()),
                                source: source.clone(),
                            })
                            .await;
                        offset = 0;
                        reader = None; // Force reopen
                    }

                    if len > offset {
                        if reader.is_none() {
                            reader = open_and_seek(&path, offset).await;
                        }
                        if let Some(ref mut r) = reader {
                            if read_new_lines(r, &tx, &source, &mut buf, &mut raw)
                                .await
                                .is_err()
                            {
                                return; // Receiver dropped
                            }
                            offset = r.stream_position().await.unwrap_or(len);
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(RawLogMessage {
                            line: format!("⚠️  File unavailable: {} ({})", path.display(), e),
                            source: source.clone(),
                        })
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
async fn wait_for_file(
    path: &Path,
    tx: &mpsc::Sender<RawLogMessage>,
    source: &Option<Arc<str>>,
) -> u64 {
    match fs::metadata(path).await {
        Ok(meta) => meta.len(),
        Err(e) => {
            let _ = tx
                .send(RawLogMessage {
                    line: format!("⚠️  Waiting for file to exist: {} ({})", path.display(), e),
                    source: source.clone(),
                })
                .await;
            loop {
                tokio::time::sleep(Duration::from_millis(FILE_POLL_INTERVAL_MS)).await;
                if let Ok(meta) = fs::metadata(path).await {
                    let _ = tx
                        .send(RawLogMessage {
                            line: format!("✓ File found: {}", path.display()),
                            source: source.clone(),
                        })
                        .await;
                    return meta.len();
                }
            }
        }
    }
}

/// Opens a file and seeks to the given offset, returning a buffered reader.
async fn open_and_seek(path: &Path, offset: u64) -> Option<BufReader<fs::File>> {
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
    tx: &mpsc::Sender<RawLogMessage>,
    source: &Option<Arc<str>>,
    buf: &mut String,
    raw: &mut Vec<u8>,
) -> Result<(), ()> {
    loop {
        match read_line_bounded(reader, buf, raw).await {
            Ok(0) => return Ok(()), // EOF reached
            Ok(_) => {
                truncate_line(buf);
                if tx
                    .send(RawLogMessage {
                        line: buf.clone(),
                        source: source.clone(),
                    })
                    .await
                    .is_err()
                {
                    return Err(()); // Receiver dropped
                }
            }
            Err(e) => {
                let _ = tx
                    .send(RawLogMessage {
                        line: format!("⚠️  Error reading line: {}", e),
                        source: source.clone(),
                    })
                    .await;
                return Ok(());
            }
        }
    }
}

/// Drains bytes until a newline (`\n`) or EOF is reached.
///
/// Discards all bytes without allocating. Used to skip the remainder of an
/// oversized line after the retained portion has been collected. Reads in
/// chunks via `fill_buf`/`consume` for efficiency. Returns the number of
/// bytes consumed during the drain (including the newline, if found).
async fn drain_until_newline<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> std::io::Result<usize> {
    let mut drained: usize = 0;
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(drained); // EOF
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            reader.consume(pos + 1);
            return Ok(drained + pos + 1);
        }
        let len = available.len();
        drained += len;
        reader.consume(len);
    }
}

/// Reads a single line while bounding the retained *raw byte* prefix.
///
/// Collects up to `MAX_LOG_LINE_SIZE` raw bytes into the caller-provided
/// `raw` buffer, then drains any remainder of the line without allocation.
/// Decodes to UTF-8 once at the end (invalid sequences become the
/// replacement character via `String::from_utf8_lossy`). Returns `0` at EOF.
/// Strips trailing `\r\n` / `\n`. Reuses the caller-provided `buf` and `raw`
/// buffers to avoid per-line heap allocations on the hot path.
///
/// Note: because `from_utf8_lossy` may expand invalid byte sequences into
/// the U+FFFD replacement character (up to 3 bytes in UTF-8), the resulting
/// `buf` may temporarily exceed `MAX_LOG_LINE_SIZE` bytes even though the
/// raw byte prefix is bounded. Callers that require a strict byte limit on
/// the final retained line must apply `truncate_line` after calling this.
async fn read_line_bounded<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    buf: &mut String,
    raw: &mut Vec<u8>,
) -> std::io::Result<usize> {
    buf.clear();
    raw.clear();
    let limit = MAX_LOG_LINE_SIZE;
    let mut total_consumed: usize = 0;
    let mut found_newline = false;

    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            break; // EOF
        }

        let newline_pos = available.iter().position(|&b| b == b'\n');
        let chunk_end = newline_pos.unwrap_or(available.len());
        let remaining = limit.saturating_sub(raw.len());
        let to_keep = chunk_end.min(remaining);

        if to_keep > 0 {
            raw.extend_from_slice(&available[..to_keep]);
        }

        let consume_len = if newline_pos.is_some() {
            chunk_end + 1
        } else {
            chunk_end
        };
        total_consumed += consume_len;
        reader.consume(consume_len);

        if newline_pos.is_some() {
            found_newline = true;
            break;
        }
        if raw.len() >= limit {
            break;
        }
    }

    if !found_newline && raw.len() >= limit {
        total_consumed += drain_until_newline(reader).await?;
    }

    // Strip trailing \r for Windows-style line endings
    if raw.last() == Some(&b'\r') {
        raw.pop();
    }

    buf.push_str(&String::from_utf8_lossy(raw));

    Ok(total_consumed)
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
fn spawn_stdin_reader(tx: mpsc::Sender<RawLogMessage>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut buf = String::new();
        let mut raw = Vec::new();

        loop {
            match read_line_bounded(&mut reader, &mut buf, &mut raw).await {
                Ok(0) => {
                    let _ = tx
                        .send(RawLogMessage {
                            line: "ℹ️  End of input stream reached".to_string(),
                            source: None,
                        })
                        .await;
                    return;
                }
                Ok(_) => {
                    truncate_line(&mut buf);
                    if tx
                        .send(RawLogMessage {
                            line: buf.clone(),
                            source: None,
                        })
                        .await
                        .is_err()
                    {
                        return; // Receiver dropped
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(RawLogMessage {
                            line: format!("⚠️  Error reading from stdin: {}", e),
                            source: None,
                        })
                        .await;
                    return;
                }
            }
        }
    })
}

/// Spawn a mock generator for demo purposes.
fn spawn_mock(tx: mpsc::Sender<RawLogMessage>) -> JoinHandle<()> {
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
                if tx.send(RawLogMessage { line, source: None }).await.is_err() {
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
    use tokio::io::BufReader as TokioBufReader;

    // --- read_line_bounded tests ---

    #[tokio::test]
    async fn test_read_line_bounded_normal_line() {
        let data = b"hello world\n";
        let mut reader = TokioBufReader::new(&data[..]);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let n = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "hello world");
        assert_eq!(n, 12); // 11 chars + newline
    }

    #[tokio::test]
    async fn test_read_line_bounded_eof_without_newline() {
        let data = b"no newline";
        let mut reader = TokioBufReader::new(&data[..]);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let n = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "no newline");
        assert_eq!(n, 10);
    }

    #[tokio::test]
    async fn test_read_line_bounded_empty_eof() {
        let data = b"";
        let mut reader = TokioBufReader::new(&data[..]);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let n = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(n, 0);
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_read_line_bounded_strips_cr() {
        let data = b"windows line\r\n";
        let mut reader = TokioBufReader::new(&data[..]);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "windows line");
    }

    #[tokio::test]
    async fn test_read_line_bounded_oversized_line_capped() {
        let oversized = "x".repeat(MAX_LOG_LINE_SIZE * 2);
        let data = format!("{}\n", oversized);
        let mut reader = TokioBufReader::new(data.as_bytes());
        let mut buf = String::new();
        let mut raw = Vec::new();
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert!(buf.len() <= MAX_LOG_LINE_SIZE);
    }

    #[tokio::test]
    async fn test_read_line_bounded_oversized_drains_remainder() {
        let oversized = "x".repeat(MAX_LOG_LINE_SIZE * 2);
        let data = format!("{}\nnext line\n", oversized);
        let mut reader = TokioBufReader::new(data.as_bytes());
        let mut buf = String::new();
        let mut raw = Vec::new();
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert!(buf.len() <= MAX_LOG_LINE_SIZE);
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "next line");
    }

    #[tokio::test]
    async fn test_read_line_bounded_multiple_lines() {
        let data = b"line1\nline2\nline3\n";
        let mut reader = TokioBufReader::new(&data[..]);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "line1");
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "line2");
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert_eq!(buf, "line3");
    }

    #[tokio::test]
    async fn test_read_line_bounded_invalid_utf8_replacement() {
        let data: &[u8] = b"a\xF0\x28\x8C\x28b\n";
        let mut reader = TokioBufReader::new(data);
        let mut buf = String::new();
        let mut raw = Vec::new();
        let _ = read_line_bounded(&mut reader, &mut buf, &mut raw)
            .await
            .unwrap();
        assert!(buf.contains('\u{FFFD}'));
        assert!(buf.starts_with('a'));
        assert!(buf.ends_with('b'));
    }

    // --- truncate_line tests ---

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
        let repeat_count = MAX_LOG_LINE_SIZE / 3 + 1;
        let mut line = "\u{4e16}".repeat(repeat_count);
        truncate_line(&mut line);
        assert!(line.ends_with(TRUNCATION_SUFFIX));
        assert!(line.len() <= MAX_LOG_LINE_SIZE);
        let _ = line.as_str();
    }

    #[test]
    fn test_truncate_line_empty() {
        let mut line = String::new();
        truncate_line(&mut line);
        assert!(line.is_empty());
    }
}
