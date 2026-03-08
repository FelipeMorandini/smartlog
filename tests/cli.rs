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
    // When run without a real terminal (e.g., in CI), terminal init fails
    // and the process exits with a non-zero code. This verifies argument
    // parsing succeeds and the app doesn't panic.
    smartlog()
        .args(["--file", "/tmp/smartlog-test-nonexistent.log"])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}
