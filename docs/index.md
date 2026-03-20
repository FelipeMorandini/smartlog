# SmartLog

A fast, ergonomic terminal UI for tailing and filtering JSON and plain-text logs in real time.

Built with **Rust** | Async I/O with **Tokio** | Beautiful TUI with **Ratatui**

---

## Features

- **Auto-detect log format** -- Automatically identifies JSON vs. plain text
- **Pretty-print JSON** -- Colorized output with severity-based coloring
- **Live filtering** -- Instant substring and regex search with match highlighting
- **Log level filtering** -- Cycle through ERROR, WARN, INFO, DEBUG levels
- **Line wrapping** -- Toggle wrapping on/off
- **Mouse support** -- Scroll with the mouse wheel
- **Follow mode** -- Smooth auto-scroll that tracks new log entries
- **File tailing** -- `tail -f` style with truncation and rotation detection
- **Stdin piping** -- Pipe any command output: `tail -f app.log | smartlog`
- **Multi-file tailing** -- Watch multiple files simultaneously with source labels
- **Color themes** -- Dark, light, solarized, and dracula presets
- **Relative timestamps** -- See how long ago each log entry was written
- **Log export** -- Save filtered logs to files
- **Debug logging** -- Built-in diagnostics for troubleshooting SmartLog itself
- **Shell completions** -- Tab completion for bash, zsh, fish, elvish, powershell
- **Graceful shutdown** -- Terminal state always restored, even on panic

## Quick Start

Tail a specific log file:

```bash
smartlog --file /var/log/myapp.log
```

Tail multiple files at once:

```bash
smartlog --file /var/log/app.log --file /var/log/error.log
```

Pipe logs directly:

```bash
tail -f /var/log/app.log | smartlog
```

Run `smartlog --help` to see all available options.

## Log Format Support

### JSON Logs

SmartLog automatically detects and pretty-prints JSON with level detection:

```json
{"level": "ERROR", "msg": "Database connection failed", "error_code": 500}
```

Supported level fields: `level`, `severity`, `lvl`

| Level | Color |
|-------|-------|
| ERROR / ERR / FATAL | Red |
| WARN / WARNING | Yellow |
| INFO / INFORMATION | Green |
| DEBUG / TRACE | Blue |

### Plain Text Logs

For non-JSON logs, SmartLog scans for keywords:

```
2024-12-13 10:30:45 ERROR Database connection timeout
```

Keywords: `error`, `fatal`, `warn`, `info`, `debug`, `trace` (case-insensitive)

## Performance

- **Async non-blocking streaming** -- Logs are processed as they arrive
- **Bounded memory** -- Circular buffer of last 2000 entries (configurable)
- **Concurrent I/O** -- File tailing and terminal rendering run concurrently
- **Fast JSON parsing** -- Powered by serde_json

## Reliability

- **Graceful shutdown** -- Handles SIGINT (Ctrl+C) and SIGTERM cleanly
- **Terminal restoration** -- RAII guard ensures terminal state is always restored
- **File rotation handling** -- Automatically detects and recovers from log rotation
- **Error recovery** -- Continues running on temporary errors

## Troubleshooting

- If nothing appears when running `smartlog` without `--file`, make sure you're piping input (e.g., `... | smartlog`). Without piped stdin and no file, SmartLog shows a demo stream.
- Ensure `smartlog` has read access to any files you tail.
- On Windows, make sure your terminal supports VT sequences (Windows 10+).

## License

MIT License. See [LICENSE](https://github.com/felipemorandini/smartlog/blob/main/LICENSE) for details.

## Author

**Felipe Pires Morandini**

- GitHub: [@felipemorandini](https://github.com/felipemorandini)
