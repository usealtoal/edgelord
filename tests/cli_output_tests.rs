//! CLI output integration tests.

use assert_cmd::Command;
use predicates::prelude::*;

fn edgelord() -> Command {
    Command::cargo_bin("edgelord").unwrap()
}

#[test]
fn test_help() {
    edgelord()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("edgelord"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_version() {
    edgelord()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("edgelord"));
}

#[test]
fn test_strategies_list() {
    edgelord()
        .args(["strategies", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("single-condition"))
        .stdout(predicate::str::contains("market-rebalancing"))
        .stdout(predicate::str::contains("combinatorial"));
}

#[test]
fn test_strategies_explain() {
    edgelord()
        .args(["strategies", "explain", "single-condition"])
        .assert()
        .success()
        .stdout(predicate::str::contains("YES/NO"));
}

#[test]
fn test_color_never_flag() {
    edgelord()
        .args(["--color", "never", "--help"])
        .assert()
        .success();
}

#[test]
fn test_init_help() {
    edgelord()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config"));
}
