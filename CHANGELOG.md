# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CHANGELOG.md following Keep a Changelog format
- Shell completions subcommand via `clap_complete` (bash, zsh, fish, elvish, powershell)
- AUR package (PKGBUILD) for Arch Linux installation
- Winget manifest for Windows package manager installation
- Debian package (.deb) support via cargo-deb in release workflow

## [0.5.0] - 2026-02-28

### Changed

- Refactored layout handling by extracting visual line calculations to a new module
- Skip zero-length regex matches in log styling
- Updated timestamp tick behavior

### Added

- Regex highlighting for log entries
- Enhanced timestamp refresh functionality

## [0.4.2] - 2026-02-25

### Changed

- Optimized log filtering and export functionality with cached matchers

## [0.4.1] - 2026-02-22

### Fixed

- Return 0 for page entry calculations when viewport is empty or has no entries
- Handle empty filtered logs gracefully in pagination functions
- Enhanced log area dimensions and scrolling functionality

## [0.4.0] - 2026-02-18

### Added

- Relative timestamp display toggle (`t` key)
- Color theme support with 4 themes: dark, light, solarized, dracula (`T` key to cycle)
- Enhanced timestamp parsing (ISO 8601, Unix epoch, slash-separated dates)
- Multi-file basename counting in spawn_sources
- Support for `--help` flag in argument parser

### Changed

- Updated timestamp label to 'REL TIME'
- Improved JSON timestamp extraction

## [0.3.0] - 2026-02-12

### Added

- Log export functionality (`e` key) with sequential file naming
- Source label in exported files
- Export feedback message in status bar

### Security

- Redact sensitive user input in debug logs for key events

## [0.2.1] - 2026-02-08

### Fixed

- Bounded allocation for log reading to prevent memory exhaustion
- Improved line reading logic with raw byte handling
- Enhanced documentation for log line size limits

## [0.2.0] - 2026-02-05

### Added

- Log level filtering (`l` key) — cycle through ALL, ERROR, WARN, INFO, DEBUG
- Regex filtering mode (`r` key)
- Filter display shows total logs consistently

## [0.1.9] - 2026-02-02

### Fixed

- Version bump to align Cargo.toml with release tag

## [0.1.8] - 2026-01-30

### Fixed

- Log line truncation logic for UTF-8 boundary handling
- Scrolling logic and scroll clamping for filtered entries
- Missing newline at end of README.md

## [0.1.7] - 2026-01-27

### Fixed

- Auto-scroll logic in parser
- Terminal input handling
- Updated CI configuration

## [0.1.6] - 2026-01-24

### Fixed

- Event loop to use async EventStream
- Scrolling and parsing improvements

## [0.1.5] - 2026-01-21

### Added

- Configuration constants module
- Improved terminal management

## [0.1.4] - 2026-01-18

### Added

- Application state management
- Core logic for SmartLog TUI

## [0.1.3] - 2026-01-15

### Fixed

- Permissions configuration in release workflow

## [0.1.2] - 2026-01-12

### Fixed

- Release workflow to use dynamic artifact names and support merging multiple artifacts

## [0.1.1] - 2026-01-09

### Fixed

- Updated GitHub Actions to use upload and download artifact actions v4

## [0.1.0] - 2026-01-06

### Added

- Initial release
- Terminal UI for tailing and filtering JSON and plain-text logs
- Auto-detection of JSON vs plain text
- Pretty-printing with level coloring
- Live filtering with instant highlight
- Keyboard shortcuts for navigation
- File tailing with truncation/rotation handling
- Stdin support for piping
- Graceful shutdown on Ctrl+C and SIGTERM
- CI/CD with cross-platform builds (6 targets)
- Homebrew formula auto-update in release workflow

[Unreleased]: https://github.com/felipemorandini/smartlog/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/felipemorandini/smartlog/compare/v0.4.2...v0.5.0
[0.4.2]: https://github.com/felipemorandini/smartlog/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/felipemorandini/smartlog/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/felipemorandini/smartlog/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/felipemorandini/smartlog/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/felipemorandini/smartlog/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/felipemorandini/smartlog/compare/v0.1.9...v0.2.0
[0.1.9]: https://github.com/felipemorandini/smartlog/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/felipemorandini/smartlog/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/felipemorandini/smartlog/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/felipemorandini/smartlog/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/felipemorandini/smartlog/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/felipemorandini/smartlog/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/felipemorandini/smartlog/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/felipemorandini/smartlog/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/felipemorandini/smartlog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/felipemorandini/smartlog/releases/tag/v0.1.0
