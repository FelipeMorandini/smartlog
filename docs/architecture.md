# Architecture

This document provides a deep dive into SmartLog's architecture, design decisions, and implementation details.

## High-Level Architecture

SmartLog follows a producer-consumer pattern with async I/O:

```
┌─────────────────┐
│  Log Sources    │
│  (File/Stdin)   │
└────────┬────────┘
         │ mpsc::channel
         v
┌─────────────────┐
│   Event Loop    │<--- User Input (Keyboard)
│   (tokio)       │<--- OS Signals (SIGINT/SIGTERM)
└────────┬────────┘
         │
         v
┌─────────────────┐
│   App State     │
│   (logs buffer) │
└────────┬────────┘
         │
         v
┌─────────────────┐
│   UI Renderer   │
│   (ratatui)     │
└─────────────────┘
```

## Module Breakdown

### `main.rs` (bin/smartlog.rs) -- Entry Point

- Parses CLI arguments
- Initializes terminal in raw mode
- Creates app state
- Spawns log producer tasks
- Runs event loop
- Ensures terminal restoration

Uses `TerminalGuard` (RAII pattern) to guarantee terminal restoration even on panic. Aborts producer tasks explicitly to prevent orphaned tasks.

### `sources.rs` -- Async Log Ingestion

Handles auto-detection of input source (file, stdin, or mock) and supports:

- **File Tailing**: Polls at configurable intervals, starts from EOF, detects truncation/rotation
- **Stdin Reading**: Tokio async I/O for non-blocking reads
- **Mock Generator**: Cycles through sample logs for demo mode

Uses `RawLogMessage` struct for channel protocol, carrying both the raw text and an optional `Arc<str>` source label.

### `event_loop.rs` -- Event Multiplexing

Multiplexes three event sources using `tokio::select!`:

- Incoming log messages
- User keyboard input (via crossterm `EventStream`)
- OS shutdown signals (SIGINT/SIGTERM)

Single-threaded event loop with no mutexes needed.

### `app.rs` -- Application State

Manages:

- `logs`: `VecDeque` circular buffer with 2000-entry cap
- `scroll`: Entry-based scroll position
- `auto_scroll`: Follow mode flag
- `input_buffer`: Search query
- `input_mode`: Normal vs Editing
- `cached_matcher`: `TextMatcher` rebuilt on input/regex changes

Filtering is computed on-the-fly in the UI renderer (no separate filtered buffer).

### `parser.rs` -- Log Parsing & Highlighting

- Detects JSON vs plain text (try parse, fall back)
- Extracts log level from multiple field names (`level`, `severity`, `lvl`)
- Pretty-prints JSON with severity-based coloring
- Highlights search matches (substring and regex)
- Uses `Highlight` enum (None/Substring/Regex) for `style_log`

### `ui.rs` -- Terminal UI Rendering

- Renders log area with entry-based scrolling
- Renders input bar with mode indicator
- Applies filters dynamically per frame
- Shows follow status, theme name, and source info in status bar
- Prepends metadata (timestamps, source labels) to entries

### `inputs.rs` -- Keyboard Event Handling

- **Normal Mode**: `q` quit, `/` search, `k`/`j` scroll, `ESC` clear, `w` wrap, `l` level, `r` regex, `e` export, `t` timestamps, `T` theme
- **Editing Mode**: Type to search, `Enter` apply, `ESC` cancel
- Vim-style navigation (`k`/`j` alongside arrow keys)

### `terminal.rs` -- Terminal Lifecycle

`TerminalGuard` implements `Drop` to ensure raw mode is disabled and alternate screen is exited. Both guard-based and explicit restoration for maximum safety.

### `layout.rs` -- Layout Measurement

Shared helpers that eliminate duplication between `ui.rs` and `inputs.rs`:

- `compute_visual_lines()` -- Wrapped line count for a text span
- `compute_raw_lines()` -- Unwrapped line count
- `metadata_prefix_display_width()` -- Width of timestamp/source prefixes
- `entry_visual_lines()` -- Full visual height of a single entry

### `config.rs` -- Configuration Constants

- `MAX_LOG_BUFFER_SIZE`: 2000 entries
- `CHANNEL_BUFFER_SIZE`: 100 messages
- `FILE_POLL_INTERVAL_MS`: 500ms
- `TIMESTAMP_REFRESH_INTERVAL_SECS`: 30s

### `theme.rs` -- Color Themes

Defines `Theme` struct with 4 const presets (dark, light, solarized, dracula). Supports runtime cycling and lookup by name.

## Data Flow

### Log Ingestion

```
File/Stdin -> spawn_sources()
               |
           read_line() (async)
               |
           mpsc::send() (async)
               |
           event_loop::run()
               |
           parse_log()
               |
           app.on_log()
               |
           logs.push_back()
```

### UI Rendering

```
event_loop::run()
    |
terminal.draw()
    |
ui::ui()
    |
filter logs by input_buffer + level
    |
entry-based slicing + style_log()
    |
ratatui diff + render
```

## Performance Characteristics

### CPU Usage

| State | Usage |
|-------|-------|
| Idle | ~0% (event loop sleeps) |
| Active tailing | ~1-2% (periodic file checks) |
| High-volume logs | ~5-10% (parsing + rendering) |

### Memory Usage

| State | Usage |
|-------|-------|
| Base | ~5-10 MB |
| Full buffer (2000 logs) | ~20-30 MB |
| Peak | Bounded by `MAX_LOG_BUFFER_SIZE` |

### Latency

| Operation | Latency |
|-----------|---------|
| Log ingestion | <1ms (async, non-blocking) |
| UI update | Event-driven (redraws per event) |
| Input response | Near-instant (async event stream) |

## Error Handling

1. **Graceful Degradation**: Continue on non-fatal errors
2. **User Feedback**: Send error messages to log stream
3. **Terminal Safety**: Always restore terminal state
4. **No Panics**: Use `Result` and handle errors explicitly

| Error Type | Handling |
|-----------|----------|
| File errors | Logged to stream, retry on next poll |
| Parse errors | Fall back to plain text |
| Channel closed | Graceful shutdown |
| Terminal errors | Propagate to main, restore terminal |

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (2021 edition, MSRV 1.74.0) |
| Async Runtime | Tokio |
| TUI Framework | Ratatui |
| Terminal Backend | Crossterm |
| CLI Framework | Clap v4 |
| JSON Parsing | serde_json |
| Regex Engine | regex crate (linear-time, ReDoS-safe) |
| Timestamps | Chrono |
| Error Handling | Anyhow |
| Debug Logging | Tracing |
