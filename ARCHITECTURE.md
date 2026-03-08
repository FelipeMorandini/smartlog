# SmartLog Architecture

This document provides a deep dive into SmartLog's architecture, design decisions, and implementation details.

## 🏗️ High-Level Architecture

SmartLog follows a producer-consumer pattern with async I/O:

```
┌─────────────────┐
│  Log Sources    │
│  (File/Stdin)   │
└────────┬────────┘
         │ mpsc::channel
         ▼
┌─────────────────┐
│   Event Loop    │◄─── User Input (Keyboard)
│   (tokio)       │◄─── OS Signals (SIGINT/SIGTERM)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   App State     │
│   (logs buffer) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   UI Renderer   │
│   (ratatui)     │
└─────────────────┘
```

## 📦 Module Breakdown

### `main.rs` (bin/smartlog.rs)

**Purpose**: Entry point and orchestration

**Responsibilities**:
- Parse CLI arguments
- Initialize terminal in raw mode
- Create app state
- Spawn log producer task
- Run event loop
- Ensure terminal restoration

**Key Design Decisions**:
- Uses `TerminalGuard` (RAII pattern) to guarantee terminal restoration even on panic
- Aborts producer task explicitly to prevent orphaned tasks
- Handles both explicit restoration and guard-based restoration for redundancy

### `sources.rs`

**Purpose**: Async log ingestion from multiple sources

**Responsibilities**:
- Auto-detect input source (file, stdin, or mock)
- Tail files with rotation detection
- Read from piped stdin
- Generate mock data for demos

**Key Design Decisions**:
- **File Tailing**: Polls at 500ms intervals (configurable in `config.rs`)
  - Starts from EOF for `tail -f` behavior
  - Detects truncation/rotation by comparing file size
  - Resets to beginning on rotation
  - Sends status messages on errors
- **Stdin Reading**: Uses tokio async I/O for non-blocking reads
- **Mock Generator**: Cycles through sample logs for demo mode

**Performance Characteristics**:
- Non-blocking I/O prevents UI freezes
- Buffered reading minimizes syscalls
- Channel-based communication decouples I/O from UI

### `event_loop.rs`

**Purpose**: Main async event loop

**Responsibilities**:
- Multiplex three event sources using `tokio::select!`
- Handle incoming log messages
- Process user keyboard input
- Respond to OS shutdown signals

**Key Design Decisions**:
- **Signal Handling**: Unix systems handle both SIGINT and SIGTERM
- **Event-Driven Input**: Uses crossterm `EventStream` (async, no polling)
- **Graceful Shutdown**: Sets `should_quit` flag rather than panic
- **Channel Closure**: Exits gracefully when producer ends (stdin EOF)

**Concurrency Model**:
- All I/O is async (tokio runtime)
- UI rendering is synchronous but fast (ratatui)
- No mutexes needed (single-threaded event loop)

### `app.rs`

**Purpose**: Application state management

**State Variables**:
- `logs`: `VecDeque` with 2000-entry circular buffer
- `scroll`: Current scroll position
- `auto_scroll`: Follow mode flag
- `input_buffer`: Search query
- `input_mode`: Normal vs Editing

**Key Design Decisions**:
- **Circular Buffer**: `VecDeque` provides O(1) push/pop at both ends
- **Auto-scroll Logic**: 
  - Disabled on manual scroll up
  - Re-enabled when scrolling to bottom
  - Re-enabled on ESC
- **Filtering**: Computed on-the-fly in UI (no separate filtered buffer)

**Memory Management**:
- Bounded buffer prevents OOM on infinite streams
- Old logs discarded automatically
- Pre-allocated capacity reduces reallocations

### `parser.rs`

**Purpose**: Log parsing and syntax highlighting

**Responsibilities**:
- Detect JSON vs plain text
- Parse JSON and extract log level
- Pretty-print JSON
- Color-code by severity
- Highlight search matches

**Key Design Decisions**:
- **JSON Detection**: Try parse, fall back to plain text
- **Level Detection**: Checks multiple field names (`level`, `severity`, `lvl`)
- **Highlighting**: Case-insensitive substring matching with cyan background
- **Color Scheme**:
  - ERROR/FATAL: Red
  - WARN/WARNING: Yellow
  - INFO: Green
  - DEBUG/TRACE: Blue

**Performance**:
- Uses `serde_json` (fastest JSON library in Rust)
- Parsing happens once per log entry
- Styled output cached in `LogEntry`

### `ui.rs`

**Purpose**: Terminal UI rendering

**Responsibilities**:
- Render log area with scrolling
- Render input bar with mode indicator
- Apply filters dynamically
- Show follow status

**Key Design Decisions**:
- **Filtering**: Filters logs on every frame (fast enough for 2000 entries)
- **Scroll Calculation**: Auto-scroll mode snaps to bottom
- **Layout**: 
  - Main area: flexible height
  - Input bar: fixed 3 lines
- **Wrapping**: Preserves JSON indentation

**Rendering Performance**:
- Redraws on every event; ratatui diffs and only updates changed cells

### `inputs.rs`

**Purpose**: Keyboard event handling

**Key Bindings**:
- **Normal Mode**: `q` quit, `/` search, `k/j` scroll, `ESC` clear
- **Editing Mode**: Type to search, `Enter` apply, `ESC` cancel

**Key Design Decisions**:
- **Vim-style Navigation**: `k`/`j` alongside arrow keys
- **ESC Overloading**: Clears filter, exits edit mode, re-enables follow
- **Immediate Feedback**: Filter applies as you type (in normal mode after Enter)

### `terminal.rs`

**Purpose**: Terminal lifecycle management

**Responsibilities**:
- Enable raw mode
- Enter alternate screen
- Restore terminal on exit
- RAII guard for panic safety

**Key Design Decisions**:
- **TerminalGuard**: Implements `Drop` to ensure restoration
- **Redundant Restoration**: Both guard and explicit call for maximum safety
- **Mouse Capture**: Enabled but not currently used (future feature)

### `config.rs`

**Purpose**: Centralized configuration

**Constants**:
- `MAX_LOG_BUFFER_SIZE`: 2000 entries
- `CHANNEL_BUFFER_SIZE`: 100 messages
- `FILE_POLL_INTERVAL_MS`: 500ms

**Benefits**:
- Single source of truth
- Easy tuning without grep
- Documented performance trade-offs

## 🔄 Data Flow

### Log Ingestion Flow

```
File/Stdin → spawn_source()
              ↓
          read_line() (async)
              ↓
          mpsc::send() (async)
              ↓
          event_loop::run()
              ↓
          parse_log()
              ↓
          app.on_log()
              ↓
          logs.push_back()
```

### UI Rendering Flow

```
event_loop::run()
    ↓
terminal.draw()
    ↓
ui::ui()
    ↓
filter logs by input_buffer
    ↓
entry-based slicing + style_log()
    ↓
ratatui diff + render
```

## ⚡ Performance Considerations

### CPU Usage

- **Idle**: ~0% (event loop sleeps until input/logs)
- **Active Tailing**: ~1-2% (periodic file checks)
- **High-Volume Logs**: ~5-10% (parsing + rendering)

### Memory Usage

- **Base**: ~5-10 MB
- **Full Buffer (2000 logs)**: ~20-30 MB (depends on log size)
- **Peak**: Bounded by `MAX_LOG_BUFFER_SIZE`

### Latency

- **Log Ingestion**: <1ms (async, non-blocking)
- **UI Update**: Event-driven (redraws on every log or input event)
- **Input Response**: Near-instant (async event stream, no polling)

## 🛡️ Error Handling

### Strategy

1. **Graceful Degradation**: Continue on non-fatal errors
2. **User Feedback**: Send error messages to log stream
3. **Terminal Safety**: Always restore terminal state
4. **No Panics**: Use `Result` and handle errors explicitly

### Error Types

- **File Errors**: Logged to stream, retry on next poll
- **Parse Errors**: Fall back to plain text
- **Channel Closed**: Graceful shutdown
- **Terminal Errors**: Propagate to main, restore terminal

## 🔮 Future Enhancements

### Planned Features

1. **Regex Filtering**: More powerful search
2. **JSON Path Queries**: Filter on specific fields
3. **Export**: Save filtered logs
4. **Themes**: Customizable color schemes
5. **Config File**: `~/.smartlog/config.toml`
6. **Performance Stats**: Logs/sec, filter hits
7. **Multiple Files**: Tail multiple sources with labels
8. **Horizontal Scroll**: For very long lines

### Performance Optimizations

1. **Incremental Filtering**: Only re-filter on input change
2. **Virtual Scrolling**: Render only visible logs
3. **Mmap Large Files**: Faster initial loading
4. **Parallel Parsing**: Use rayon for JSON parsing

## 🧪 Testing Strategy

### Unit Tests

- Parser: JSON detection, level extraction
- App: Scroll logic, filtering
- Inputs: Keyboard handling, mode switching

### Integration Tests

- File tailing: Rotation, truncation, growth
- Stdin: EOF handling, error recovery
- UI: Rendering, scroll bounds

### Manual Testing

- Performance: Large log files (>1GB)
- Stress: High-volume streams (>1000 logs/sec)
- Edge Cases: Empty files, binary data, malformed JSON

## 📚 References

- [Ratatui Documentation](https://ratatui.rs/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Crossterm Guide](https://docs.rs/crossterm/)
- [The Rust Book](https://doc.rust-lang.org/book/)

---

**Last Updated**: March 2026  
**Maintainer**: Felipe Pires Morandini

