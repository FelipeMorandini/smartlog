//! Configuration constants for SmartLog.
//!
//! This module contains all configurable constants used throughout the application.

/// Maximum number of log entries to keep in memory.
///
/// When this limit is reached, the oldest entries are discarded.
/// This prevents unbounded memory growth when tailing long-running logs.
pub const MAX_LOG_BUFFER_SIZE: usize = 2000;

/// Channel buffer size for log transmission.
///
/// This determines how many log lines can be buffered between the
/// producer (file/stdin reader) and consumer (UI) before backpressure occurs.
pub const CHANNEL_BUFFER_SIZE: usize = 100;

/// Maximum retained length of a single log line in bytes.
///
/// The bounded reader collects at most this many raw bytes into its per-line
/// buffer. Any remaining bytes beyond this limit are drained without further
/// growth of that buffer, and lines exceeding this limit are truncated with a
/// suffix for display. This constant therefore bounds the retained raw bytes
/// per line; overall peak memory for log reading also includes the
/// `BufReader`'s internal buffer and any temporary UTF-8 decoding overhead.
pub const MAX_LOG_LINE_SIZE: usize = 65_536; // 64 KB

/// Maximum length of the search input buffer in characters.
///
/// Prevents unbounded memory growth from user input in the filter bar.
pub const MAX_INPUT_BUFFER_SIZE: usize = 256;

/// Polling interval for file changes in milliseconds.
///
/// This determines how frequently we check for new content in tailed files.
/// Lower values = more responsive but higher CPU usage.
pub const FILE_POLL_INTERVAL_MS: u64 = 500;
