# GitHub Copilot Instructions

## Code Review Guidelines

When performing pull request code reviews, follow these rules:

- **Review only the changed lines.** Limit comments to lines that are new or modified in the current diff. Do not raise issues on code that was not changed in this pull request or review cycle.
- **Be thorough on the first review pass.** Identify all potential problems, style violations, logic errors, and improvement opportunities in the changed code during the initial review so that the author can address everything at once.
- **Do not repeat previous comments.** If a subsequent review is requested after the author has addressed earlier feedback, only comment on lines that are part of the new or updated changes. Do not re-raise issues that were already commented on in prior review rounds unless they are present in newly changed lines.
- **Respect the language and toolchain.** This project is written in Rust. Apply Rust-specific best practices: prefer idiomatic Rust (iterators, `?` operator, pattern matching), flag any `unwrap()`/`expect()` calls without justification in production paths, and ensure `cargo fmt` and `cargo clippy` rules are respected.
- **Flag security and safety issues first.** Highlight unsafe code blocks, potential panics, or anything that could compromise terminal safety (e.g., missing `TerminalGuard` cleanup) before stylistic issues.
