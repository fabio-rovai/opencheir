use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    Command::cargo_bin("opencheir")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("opencheir"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("serve"));
}

#[test]
fn test_cli_init_subcommand_exists() {
    Command::cargo_bin("opencheir")
        .unwrap()
        .arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--data-dir"));
}

#[test]
fn test_cli_serve_subcommand_exists() {
    Command::cargo_bin("opencheir")
        .unwrap()
        .arg("serve")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));
}
