# Installation

## Homebrew (macOS & Linux)

```bash
brew install felipemorandini/tap/smartlog
```

## AUR (Arch Linux)

```bash
# Using an AUR helper (e.g., yay, paru)
yay -S smartlog-bin
```

## Winget (Windows)

```powershell
winget install FelipeMorandini.smartlog
```

## Debian/Ubuntu (.deb)

Download the `.deb` package for your architecture from [GitHub Releases](https://github.com/felipemorandini/smartlog/releases):

```bash
# x86_64
sudo dpkg -i smartlog_<version>-1_amd64.deb

# ARM64
sudo dpkg -i smartlog_<version>-1_arm64.deb
```

## Cargo (crates.io)

```bash
cargo install smartlog
```

## Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/felipemorandini/smartlog/releases) page.

=== "macOS (Apple Silicon)"

    ```bash
    curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-apple-darwin.tar.gz | tar xz
    sudo mv smartlog /usr/local/bin/
    ```

=== "macOS (Intel)"

    ```bash
    curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-apple-darwin.tar.gz | tar xz
    sudo mv smartlog /usr/local/bin/
    ```

=== "Linux (x86_64)"

    ```bash
    curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-unknown-linux-musl.tar.gz | tar xz
    sudo mv smartlog /usr/local/bin/
    ```

=== "Linux (ARM64)"

    ```bash
    curl -L https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-unknown-linux-musl.tar.gz | tar xz
    sudo mv smartlog /usr/local/bin/
    ```

=== "Windows (x86_64)"

    Download [`smartlog-x86_64-pc-windows-msvc.zip`](https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-x86_64-pc-windows-msvc.zip), extract, and add `smartlog.exe` to your PATH.

=== "Windows (ARM64)"

    Download [`smartlog-aarch64-pc-windows-msvc.zip`](https://github.com/felipemorandini/smartlog/releases/latest/download/smartlog-aarch64-pc-windows-msvc.zip), extract, and add `smartlog.exe` to your PATH.

## Building from Source

Requires Rust 1.74.0 or later.

```bash
git clone https://github.com/felipemorandini/smartlog
cd smartlog
cargo build --release
# Binary will be at: target/release/smartlog
```

## Shell Completions

Generate tab-completion scripts for your shell:

```bash
smartlog completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

=== "Bash"

    ```bash
    smartlog completions bash > /etc/bash_completion.d/smartlog
    ```

=== "Zsh"

    ```bash
    smartlog completions zsh > ~/.zfunc/_smartlog
    ```

=== "Fish"

    ```bash
    smartlog completions fish > ~/.config/fish/completions/smartlog.fish
    ```
