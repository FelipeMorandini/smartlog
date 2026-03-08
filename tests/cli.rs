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
fn test_file_flag_with_nonexistent_path_exits_gracefully() {
    // Argument parsing succeeds; the process exits non-zero because either
    // terminal init fails (Linux/macOS CI) or the TUI times out (Windows CI
    // where VT support lets terminal init succeed).
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
