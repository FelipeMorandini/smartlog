# SmartLog Roadmap

## Phase 6: Distribution & Polish

### 6.1 Shell Completions — DONE
- [x] Add `clap_complete` dependency
- [x] Add `completions <SHELL>` subcommand (bash, zsh, fish, elvish, powershell)
- [x] Document in README with setup instructions per shell

### 6.2 AUR Package (Arch Linux) — DONE
- [x] Create a `PKGBUILD` that downloads the pre-built Linux binary from GitHub Releases
- [x] Add AUR install instructions to README
- [x] Use `-bin` suffix (`smartlog-bin`)
- [x] Register `smartlog-bin` on the AUR and push initial PKGBUILD
- [x] Automate AUR updates in the release workflow (`update-aur` job in `release.yml`)

### 6.3 Winget Manifest (Windows) — DONE
- [x] Create a manifest (v1.6.0 schema) pointing to the GitHub Release `.zip` assets
- [x] Add `winget install` instructions to README
- [x] Compute real SHA256 hashes for current release assets
- [x] Submit initial PR to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
- [x] Automate manifest updates via `wingetcreate` in the release workflow (`update-winget` job)

### 6.4 CHANGELOG.md — DONE
- [x] Create a `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com) format
- [x] Backfill entries for v0.1.0 through v0.6.0 from git history / release notes
- [x] Update with each new release going forward

### 6.5 Debian Package (.deb) — DONE
- [x] Add `[package.metadata.deb]` section to `Cargo.toml`
- [x] Add `cargo-deb` step to the release workflow (build `.deb` for x86_64 and aarch64 Linux targets)
- [x] Attach `.deb` files to GitHub Releases automatically
- [x] Add install instructions to README

## Remaining Work

All distribution channels are complete. No remaining work.

## Completed Distribution Channels

| Channel | Automated? | Notes |
|---------|-----------|-------|
| GitHub Releases (6 targets) | Yes | Triggered by version tag |
| Homebrew (macOS & Linux) | Yes | Formula auto-updated on release |
| crates.io | Yes | Published on release |
| Debian .deb (x86_64 + ARM64) | Yes | Built and attached to release |
| AUR (Arch Linux) | Yes | `update-aur` job in release.yml, `AUR_SSH_KEY` secret configured |
| Winget (Windows) | Yes | `update-winget` job in release.yml, `WINGET_PAT` secret configured |
