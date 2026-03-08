# Contributing to SmartLog

Thank you for your interest in contributing to SmartLog! This document provides guidelines and information for contributors.

## 🚀 Getting Started

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

4. Run tests (when available):
```bash
cargo test
```

## 🏗️ Project Structure

```
smartlog/
├── src/
│   ├── bin/
│   │   └── smartlog.rs      # Main binary entry point
│   ├── app.rs               # Application state management
│   ├── config.rs            # Configuration constants
│   ├── event_loop.rs        # Main event loop with signal handling
│   ├── inputs.rs            # Keyboard input handling
│   ├── parser.rs            # Log parsing and styling
│   ├── sources.rs           # Log sources (file, stdin, mock)
│   ├── terminal.rs          # Terminal setup and restoration
│   ├── ui.rs                # UI rendering
│   └── lib.rs               # Library root
├── Cargo.toml               # Dependencies and metadata
└── README.md                # User documentation
```

## 🎯 Module Responsibilities

- **app.rs**: Manages application state including logs buffer, scroll position, and input mode
- **config.rs**: Centralized configuration constants (buffer sizes, intervals)
- **event_loop.rs**: Async event loop handling logs, input, and OS signals
- **inputs.rs**: Keyboard event handling and mode switching
- **parser.rs**: JSON/text log parsing, level detection, and syntax highlighting
- **sources.rs**: Async log ingestion from files, stdin, or mock data
- **terminal.rs**: Terminal initialization, raw mode, and cleanup
- **ui.rs**: Ratatui-based rendering of logs and search interface

## 📝 Code Style

- Follow standard Rust formatting: `cargo fmt`
- Ensure no lints: `cargo clippy`
- Add documentation comments for public APIs
- Use meaningful variable names
- Keep functions focused and modular

## 🧪 Testing Guidelines

When adding features:

1. Add unit tests in the same file using `#[cfg(test)]`
2. Test edge cases (empty input, large buffers, etc.)
3. Ensure tests pass: `cargo test`

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

## 🔧 Adding New Features

### Feature Request Process

1. Check existing issues to avoid duplicates
2. Open an issue describing the feature
3. Wait for discussion and approval
4. Implement the feature in a branch
5. Submit a pull request

### Implementation Guidelines

1. **Keep it modular**: Add new functionality in appropriate modules
2. **Use config.rs**: Add configurable constants instead of magic numbers
3. **Document thoroughly**: Add doc comments explaining the feature
4. **Handle errors gracefully**: Don't crash on recoverable errors
5. **Consider performance**: SmartLog handles high-volume log streams

## 📦 Pull Request Process

1. Create a feature branch:
```bash
git checkout -b feature/your-feature-name
```

2. Make your changes following the code style

3. Commit with clear messages:
```bash
git commit -m "Add feature: description of what you added"
```

4. Push to your fork:
```bash
git push origin feature/your-feature-name
```

5. Open a Pull Request with:
   - Clear description of changes
   - Reference to related issues
   - Screenshots/examples if UI changes

## 🐛 Bug Reports

When reporting bugs, include:

- SmartLog version (`smartlog --version`)
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Sample log file (if applicable)

## 💡 Feature Suggestions

We welcome feature suggestions! Priority areas:

- **Performance**: Optimizations for high-volume logs
- **Filtering**: Advanced filter syntax (regex, JSON path)
- **UI**: New keyboard shortcuts, themes
- **Export**: Save filtered logs to files
- **Usability**: Better error messages, help screens

## 🔍 Code Review Criteria

Pull requests are reviewed for:

- **Correctness**: Does it work as intended?
- **Performance**: Does it handle large log volumes?
- **Code quality**: Is it readable and maintainable?
- **Documentation**: Are public APIs documented?
- **Testing**: Are edge cases covered?
- **Consistency**: Does it match existing code style?

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Questions?

Feel free to:
- Open an issue for questions
- Tag maintainers in discussions
- Reach out via email: felipepiresmorandini@gmail.com

Thank you for contributing to SmartLog! 🎉

