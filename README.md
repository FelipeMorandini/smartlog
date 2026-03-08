<div align="center">

# SmartLog

A fast, ergonomic terminal UI for tailing and filtering JSON and plain-text logs in real time.

Built with Rust • Async I/O with Tokio • Beautiful TUI with Ratatui.

</div>

---

## ✨ Features

- Auto-detects JSON vs. plain text
- Pretty-prints JSON with level coloring
- Live filtering with instant highlight (`/` to search)
- Regex filtering mode (`r` to toggle)
- Log level filtering — cycle through ERROR, WARN, INFO, DEBUG (`l` key)
- Line wrapping toggle (`w` key)
- Mouse wheel scrolling
- Smooth scrolling with follow mode
- Tails files like `tail -f` (handles truncation/rotation)
- Reads from stdin for easy piping: `tail -f app.log | smartlog`
- Graceful shutdown on Ctrl+C and SIGTERM (Unix), terminal restored every time

## 🚀 Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from the [Releases](https://github.com/felipemorandini/smartlog/releases) page:

**macOS (Apple Silicon):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-apple-darwin.tar.gz | tar xz
sudo mv smartlog /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-apple-darwin.tar.gz | tar xz
sudo mv smartlog /usr/local/bin/
```

**Linux (x86_64):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo mv smartlog /usr/local/bin/
```

**Linux (ARM64):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-unknown-linux-musl.tar.gz | tar xz
sudo mv smartlog /usr/local/bin/
```

**Windows (x86_64):**

Download [`smartlog-x86_64-pc-windows-msvc.zip`](https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-pc-windows-msvc.zip), extract, and add `smartlog.exe` to your PATH.

**Windows (ARM64):**

Download [`smartlog-aarch64-pc-windows-msvc.zip`](https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-pc-windows-msvc.zip), extract, and add `smartlog.exe` to your PATH.

### Using Cargo

If you have Rust installed:

```bash
cargo install --git https://github.com/felipemorandini/smartlog
```

### Building from Source

```bash
git clone https://github.com/felipemorandini/smartlog
cd smartlog
cargo build --release
# Binary will be at: target/release/smartlog
```

## 🔧 Quick Start

Tail a specific log file:

```
smartlog --file /var/log/myapp.log
```

Tail multiple files at once:

```
smartlog --file /var/log/app.log --file /var/log/error.log
```

Or pipe logs directly:

```
tail -f /var/log/app.log | smartlog
```

### Input Sources and Behavior

- When using `--file`, SmartLog tails from the end of the file and follows new lines (similar to `tail -f`). If the file is truncated/rotated, it continues from the beginning of the new file.
- When no `--file` is provided, SmartLog automatically reads from stdin if it is piped; otherwise it starts a demo stream.
- Press Ctrl+C to exit gracefully. On Unix, receiving SIGTERM also exits gracefully and restores the terminal state.

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Enter filter mode |
| `ESC` | Exit filter mode / Clear filter / Re-enable auto-scroll |
| `↑` or `k` | Scroll up (pauses auto-scroll) |
| `↓` or `j` | Scroll down |
| `PageUp` | Scroll up by one page |
| `PageDown` | Scroll down by one page |
| `Home` or `g` | Jump to top |
| `End` or `G` | Jump to bottom (re-enables auto-scroll) |
| `Enter` | Apply filter (in filter mode) |
| `w` | Toggle line wrapping on/off |
| `l` | Cycle log level filter (ALL → ERROR → WARN → INFO → DEBUG → ALL) |
| `r` | Toggle regex filtering mode |
| `e` | Export filtered logs to file |
| `t` | Toggle relative timestamp display |
| `T` | Cycle color theme (dark → light → solarized → dracula) |
| Mouse wheel | Scroll up/down |
| `q` | Quit application |

## 🔎 Filter Mode

1. Press `/` to enter filter mode
2. Type your search query (case-insensitive)
3. Press `Enter` to apply filter and return to normal mode
4. Press `ESC` to clear the filter and return to normal mode

Matching text is highlighted with a cyan background for easy visibility.

### Regex Filtering

Press `r` to toggle regex mode. When active, the filter input is treated as a regular expression (case-insensitive). Invalid regex patterns are indicated in the status bar.

### Log Level Filtering

Press `l` to cycle through minimum log level filters: ALL → ERROR → WARN → INFO → DEBUG → ALL. Only entries at or above the selected severity are shown. This filter combines with the text/regex filter (both must match).

### Export Logs

Press `e` to export the currently filtered logs to a file. The file is saved as `smartlog_export_<timestamp>_<seq>.log` in the current directory (or the directory specified by `--export-dir`). The status bar shows confirmation with the file path.

### Debug Logging

Use `--verbose` (or `-v`) to enable debug logging. Diagnostic output is written to `smartlog_debug.log` in the current directory. This is useful for troubleshooting SmartLog itself.

### Relative Timestamps

Press `t` to toggle relative timestamp display. When enabled, each log entry with a detected timestamp shows a prefix like `[3s ago]`, `[5m ago]`, or `[2h ago]`. Timestamps are extracted from:
- **JSON logs**: `timestamp`, `ts`, `time`, `@timestamp`, `datetime` fields (ISO 8601 strings or epoch seconds/milliseconds)
- **Plain text logs**: ISO 8601 patterns at the beginning of the line (e.g., `2024-01-15T10:30:45Z`, `2024-01-15 10:30:45`)

### Multi-File Tailing

Tail multiple files simultaneously by specifying `--file` multiple times:

```bash
smartlog --file /var/log/app.log --file /var/log/error.log
```

Each log entry is prefixed with the source filename (e.g., `[app.log]`) so you can tell which file it came from. The status bar shows the number of active files.

### Color Themes

Use `--theme <name>` to set the color theme, or press `T` at runtime to cycle through themes:

- **dark** (default): Designed for dark terminal backgrounds
- **light**: Designed for light terminal backgrounds
- **solarized**: Based on the Solarized color palette
- **dracula**: Based on the Dracula color palette

The current theme name is shown in the status bar.

## 📊 Log Format Support

SmartLog intelligently handles various log formats:

### JSON Logs

Automatically detects and pretty-prints JSON with level detection:

```json
{"level": "ERROR", "msg": "Database connection failed", "error_code": 500}
```

Supported level fields: `level`, `severity`, `lvl`

Supported level values:
- **ERROR** / **ERR** / **FATAL** → Red
- **WARN** / **WARNING** → Yellow  
- **INFO** / **INFORMATION** → Green
- **DEBUG** / **TRACE** → Blue

### Plain Text Logs

For non-JSON logs, SmartLog scans for keywords:

```
2024-12-13 10:30:45 ERROR Database connection timeout
```

Keywords: `error`, `fatal`, `warn`, `info`, `debug`, `trace` (case-insensitive)

## 🏗️ Architecture

SmartLog is built with modern Rust async patterns:

- **Tokio** — Async runtime for non-blocking I/O
- **Ratatui** — Terminal UI framework
- **Crossterm** — Cross-platform terminal manipulation
- **Serde JSON** — Fast JSON parsing and pretty-printing

### Performance

- **Async non-blocking streaming**: Logs are processed as they arrive without blocking
- **Efficient buffering**: Maintains last 2000 logs in memory (configurable in code)
- **Async I/O**: File tailing and terminal rendering happen concurrently
- **Fast JSON parsing**: Uses serde_json for high-performance parsing

### Reliability

- **Graceful shutdown**: Handles SIGINT (Ctrl+C) and SIGTERM cleanly
- **Terminal restoration**: Always restores terminal state, even on panic
- **File rotation handling**: Automatically detects and recovers from log rotation
- **Error recovery**: Continues running even if temporary errors occur

## 🧰 Troubleshooting

- If nothing appears when you run `smartlog` without `--file`, make sure you're piping input (e.g., `... | smartlog`). If stdin is a TTY and no file is provided, SmartLog shows a demo stream.
- Permissions: ensure `smartlog` has read access to any files you tail.
- Windows: make sure your terminal supports the necessary VT sequences (Windows 10+ typically works).

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🐛 Bug Reports & Feature Requests

Found a bug or have a feature request? Please [open an issue](https://github.com/felipemorandini/smartlog/issues/new) on GitHub.

## 👤 Author

**Felipe Pires Morandini**
- GitHub: [@felipemorandini](https://github.com/felipemorandini)
- Email: felipepiresmorandini@gmail.com

## 🙏 Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) — An amazing TUI framework
- Inspired by tools like `tail`, `less`, and `jq`

---

<div align="center">
Made with ❤️ and Rust 🦀
</div>