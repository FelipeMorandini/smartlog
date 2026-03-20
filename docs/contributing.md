# Contributing

Thank you for your interest in contributing to SmartLog! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Latest stable Rust (see [rustup.rs](https://rustup.rs/))
- Git

### Setting Up Development Environment

1. Fork and clone the repository:

    ```bash
    git clone https://github.com/YOUR_USERNAME/smartlog.git
    cd smartlog
    ```

2. Build the project:

    ```bash
    cargo build
    ```

3. Run the project:

    ```bash
    cargo run -- --file /path/to/logfile
    ```

4. Run tests:

    ```bash
    cargo test
    ```

## Code Style

- Follow standard Rust formatting: `cargo fmt`
- Ensure no lints: `cargo clippy --all-targets --all-features -- -D warnings`
- Add `///` documentation comments for all public APIs
- Use meaningful variable names
- Keep functions under 40 lines -- extract helpers when exceeding

## Project Structure

```
smartlog/
├── src/
│   ├── bin/
│   │   └── smartlog.rs      # Main binary entry point
│   ├── app.rs               # Application state management
│   ├── config.rs            # Configuration constants
│   ├── event_loop.rs        # Main event loop with signal handling
│   ├── inputs.rs            # Keyboard input handling
│   ├── layout.rs            # Layout measurement helpers
│   ├── parser.rs            # Log parsing and styling
│   ├── sources.rs           # Log sources (file, stdin, mock)
│   ├── terminal.rs          # Terminal setup and restoration
│   ├── theme.rs             # Color theme definitions
│   ├── ui.rs                # UI rendering
│   └── lib.rs               # Library root
├── tests/                   # Integration tests
├── Cargo.toml               # Dependencies and metadata
└── README.md                # User documentation
```

## Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `app.rs` | Application state: log buffer, scroll, input mode |
| `config.rs` | Centralized configuration constants |
| `event_loop.rs` | Async event loop: logs, input, OS signals |
| `inputs.rs` | Keyboard event handling and mode switching |
| `layout.rs` | Visual line counting and layout measurement |
| `parser.rs` | JSON/text parsing, level detection, highlighting |
| `sources.rs` | Async log ingestion from files, stdin, mock |
| `terminal.rs` | Terminal initialization, raw mode, cleanup |
| `theme.rs` | Color theme definitions and cycling |
| `ui.rs` | Ratatui-based rendering of logs and UI |

## Testing Guidelines

When adding features:

1. Add unit tests in the same file using `#[cfg(test)]`
2. Test edge cases (empty input, large buffers, etc.)
3. Ensure all tests pass: `cargo test`

Before submitting, run the full quality gate:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Pull Request Process

1. Create a feature branch from `main`:

    ```bash
    git checkout -b feature/your-feature-name
    ```

2. Make your changes following the code style

3. Commit with clear messages using [Conventional Commits](https://www.conventionalcommits.org/):

    ```
    feat: add new filtering mode
    fix: handle empty log files gracefully
    refactor: extract layout helpers into separate module
    ```

4. Push to your fork and open a Pull Request with:
    - Clear description of changes
    - Reference to related issues
    - Screenshots/examples if UI changes

## Code Review Criteria

Pull requests are reviewed for:

- **Correctness** -- Does it work as intended?
- **Performance** -- Does it handle high-volume logs?
- **Code quality** -- Is it readable and maintainable?
- **Documentation** -- Are public APIs documented?
- **Testing** -- Are edge cases covered?
- **Terminal safety** -- Is terminal state always restored?

## Bug Reports

When reporting bugs, include:

- SmartLog version (`smartlog --version`)
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Sample log file (if applicable)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
