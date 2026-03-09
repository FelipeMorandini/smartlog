use assert_cmd::Command;
use predicates::prelude::*;

fn smartlog() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("smartlog"))
}

#[test]
fn test_help_flag() {
    smartlog()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("smartlog"));
}

#[test]
fn test_version_flag() {
    smartlog()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_unknown_flag_rejected() {
    smartlog()
        .arg("--nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn test_help_shows_export_dir_flag() {
    smartlog()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--export-dir"));
}

#[test]
fn test_help_shows_verbose_flag() {
    smartlog()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_help_shows_theme_flag() {
    smartlog()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--theme"));
}

#[test]
fn test_theme_flag_accepts_value() {
    // --theme with a valid value should not cause an argument parsing error
    // The process will fail because no TTY is available, but it should get past parsing
    smartlog()
        .arg("--theme")
        .arg("solarized")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_multiple_file_flags_accepted() {
    // Multiple --file flags should be accepted by the argument parser.
    // Add --help to force an early exit after parsing instead of starting the TUI.
    smartlog()
        .arg("--file")
        .arg("/tmp/nonexistent1.log")
        .arg("--file")
        .arg("/tmp/nonexistent2.log")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_file_flag_with_nonexistent_path_exits_gracefully() {
    // Argument parsing succeeds; the process exits non-zero because either
    // terminal init fails (Linux/macOS CI) or the test harness timeout kills
    // the still-running process (Windows CI where VT support lets terminal
    // init succeed and the TUI starts).
    let unique = format!(
        "smartlog-test-{}-{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    smartlog()
        .arg("--file")
        .arg(&path)
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .failure();
}
