# Usage Guide

## Input Sources

### File Tailing

Tail a log file (similar to `tail -f`):

```bash
smartlog --file /var/log/myapp.log
```

SmartLog starts from the end of the file and follows new lines. If the file is truncated or rotated, it automatically continues from the beginning of the new file.

### Multi-File Tailing

Watch multiple files simultaneously:

```bash
smartlog --file /var/log/app.log --file /var/log/error.log
```

Each log entry is prefixed with the source filename (e.g., `[app.log]`). The status bar shows the number of active files.

### Stdin Piping

Pipe any command output into SmartLog:

```bash
tail -f /var/log/app.log | smartlog
kubectl logs -f my-pod | smartlog
docker logs -f my-container | smartlog
```

### Demo Mode

Running `smartlog` without `--file` and without piped stdin starts a demo stream with sample log entries.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Enter filter mode |
| `ESC` | Exit filter mode / Clear filter / Re-enable auto-scroll |
| `Up` or `k` | Scroll up (pauses auto-scroll) |
| `Down` or `j` | Scroll down |
| `PageUp` | Scroll up by one page |
| `PageDown` | Scroll down by one page |
| `Home` or `g` | Jump to top |
| `End` or `G` | Jump to bottom (re-enables auto-scroll) |
| `Enter` | Apply filter (in filter mode) |
| `w` | Toggle line wrapping on/off |
| `l` | Cycle log level filter (ALL -> ERROR -> WARN -> INFO -> DEBUG -> ALL) |
| `r` | Toggle regex filtering mode |
| `e` | Export filtered logs to file |
| `t` | Toggle relative timestamp display |
| `T` | Cycle color theme |
| Mouse wheel | Scroll up/down |
| `q` | Quit application |

## Filtering

### Text Search

1. Press `/` to enter filter mode
2. Type your search query (case-insensitive)
3. Press `Enter` to apply the filter and return to normal mode
4. Press `ESC` to clear the filter

Matching text is highlighted with a cyan background.

### Regex Filtering

Press `r` to toggle regex mode. When active, the filter input is treated as a regular expression (case-insensitive).

Regex features:

- Patterns like `\d+`, `error|warn`, `connection.*failed`
- Invalid regex patterns are indicated in the status bar
- Uses Rust's `regex` crate (linear-time, ReDoS-safe)

### Log Level Filtering

Press `l` to cycle through minimum log level filters:

**ALL** -> **ERROR** -> **WARN** -> **INFO** -> **DEBUG** -> **ALL**

Only entries at or above the selected severity are shown. This filter combines with the text/regex filter -- both must match for an entry to be visible.

## Relative Timestamps

Press `t` to toggle relative timestamp display. When enabled, each log entry with a detected timestamp shows a prefix like `[3s ago]`, `[5m ago]`, or `[2h ago]`.

The display auto-refreshes every 30 seconds to keep values current.

Timestamps are extracted from:

- **JSON logs**: `timestamp`, `ts`, `time`, `@timestamp`, `datetime`, `date` fields (ISO 8601 strings or Unix epoch seconds/milliseconds/microseconds)
- **Plain text logs**: ISO 8601 or common date/time patterns at the beginning of the line

## Color Themes

Use `--theme <name>` to set the color theme at startup, or press `T` at runtime to cycle:

| Theme | Description |
|-------|-------------|
| `dark` | Default theme for dark terminal backgrounds |
| `light` | Designed for light terminal backgrounds |
| `solarized` | Based on the Solarized color palette |
| `dracula` | Based on the Dracula color palette |

The current theme name is shown in the status bar.

## Export Logs

Press `e` to export the currently filtered logs to a file.

- Default location: current directory
- Custom location: `--export-dir <path>`
- Filename format: `smartlog_export_<timestamp>_<seq>.log`
- The status bar shows confirmation with the file path

## Debug Logging

For troubleshooting SmartLog itself:

```bash
# Enable debug logging (writes to smartlog_debug.log)
smartlog --verbose --file /var/log/app.log

# Custom debug log path
smartlog --verbose --debug-log /tmp/smartlog-debug.log --file /var/log/app.log
```

## CLI Options

```
smartlog [OPTIONS] [COMMAND]

Options:
  --file <FILE>          Log file to tail (can be specified multiple times)
  --theme <THEME>        Color theme (dark, light, solarized, dracula)
  --export-dir <DIR>     Directory for exported log files
  --verbose, -v          Enable debug logging
  --debug-log <PATH>     Custom debug log file path

Commands:
  completions            Generate shell completion scripts
```
