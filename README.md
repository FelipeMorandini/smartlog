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
- Smooth scrolling with follow mode
- Tails files like `tail -f` (handles truncation/rotation)
- Reads from stdin for easy piping: `tail -f app.log | smartlog`
- Graceful shutdown on Ctrl+C and SIGTERM (Unix), terminal restored every time

## 🚀 Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from the [Releases](https://github.com/felipemorandini/smartlog/releases) page:

**macOS (Apple Silicon):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-macos-silicon -o smartlog
chmod +x smartlog
sudo mv smartlog /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-macos-intel -o smartlog
chmod +x smartlog
sudo mv smartlog /usr/local/bin/
```

**Linux (x86_64):**
```bash
curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-linux-amd64 -o smartlog
chmod +x smartlog
sudo mv smartlog /usr/local/bin/
```

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
| `q` | Quit application |

## 🔎 Filter Mode

1. Press `/` to enter filter mode
2. Type your search query (case-insensitive)
3. Press `Enter` to apply filter and return to normal mode
4. Press `ESC` to clear the filter and return to normal mode

Matching text is highlighted with a cyan background for easy visibility.

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

Keywords: `error`, `warn`, `info` (case-insensitive)

## 🏗️ Architecture

SmartLog is built with modern Rust async patterns:

- **Tokio** — Async runtime for non-blocking I/O
- **Ratatui** — Terminal UI framework
- **Crossterm** — Cross-platform terminal manipulation
- **Serde JSON** — Fast JSON parsing and pretty-printing

### Performance

- **Zero-copy streaming**: Logs are processed as they arrive without blocking
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