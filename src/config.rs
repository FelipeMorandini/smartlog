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

/// Polling interval for file changes in milliseconds.
///
/// This determines how frequently we check for new content in tailed files.
/// Lower values = more responsive but higher CPU usage.
pub const FILE_POLL_INTERVAL_MS: u64 = 500;

/// UI event polling interval in milliseconds.
///
/// This determines how frequently we check for user input.
/// Affects UI responsiveness vs CPU usage trade-off.
pub const UI_POLL_INTERVAL_MS: u64 = 100;

