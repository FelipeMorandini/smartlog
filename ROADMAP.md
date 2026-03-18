# SmartLog Roadmap

## Phase 6: Distribution & Polish

### 6.1 Shell Completions
- **Priority:** High
- **Effort:** Small
- Add `clap_complete` dependency
- Add `completions <SHELL>` subcommand (bash, zsh, fish, elvish, powershell)
- Document in README with setup instructions per shell
- Reference: jwt-term already implements this pattern

### 6.2 AUR Package (Arch Linux)
- **Priority:** Medium
- **Effort:** Small
- Create a `PKGBUILD` that downloads the pre-built Linux binary from GitHub Releases
- Submit to the AUR as `smartlog`
- Add AUR install instructions to README
- Consider a `-bin` suffix (`smartlog-bin`) if a source-build variant is also desired

### 6.3 Winget Manifest (Windows)
- **Priority:** Medium
- **Effort:** Small
- Create a manifest pointing to the `smartlog-x86_64-pc-windows-msvc.zip` release asset
- Submit PR to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
- Add `winget install smartlog` instructions to README
- Optionally automate manifest updates via [wingetcreate](https://github.com/microsoft/winget-create) in the release workflow

### 6.4 CHANGELOG.md
- **Priority:** Medium
- **Effort:** Small
- Create a `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com) format
- Backfill entries for v0.1.0 through v0.5.0 from git history / release notes
- Update with each new release going forward

### 6.5 Debian Package (.deb)
- **Priority:** Low
- **Effort:** Medium
- Add `[package.metadata.deb]` section to `Cargo.toml` with description, section, and assets
- Add `cargo-deb` step to the release workflow (build `.deb` for x86_64 and aarch64 Linux targets)
- Attach `.deb` files to GitHub Releases
- Add install instructions to README (`sudo dpkg -i smartlog_*.deb`)

## Suggested Order

| Order | Item | Reason |
|-------|------|--------|
| 1 | 6.1 Shell Completions | Direct UX improvement, parity with jwt-term |
| 2 | 6.4 CHANGELOG.md | Low effort, good practice for all subsequent releases |
| 3 | 6.2 AUR Package | Large Rust-savvy audience on Arch |
| 4 | 6.3 Winget Manifest | Covers Windows users who prefer package managers |
| 5 | 6.5 Debian Package | Broader Linux reach, but more CI complexity |
