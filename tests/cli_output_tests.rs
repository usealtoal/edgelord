//! CLI output integration tests.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn edgelord() -> Command {
    cargo_bin_cmd!("edgelord")
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
        .stdout(predicate::str::contains("single_condition"))
        .stdout(predicate::str::contains("market_rebalancing"))
        .stdout(predicate::str::contains("combinatorial"));
}

#[test]
fn test_strategies_explain() {
    edgelord()
        .args(["strategies", "explain", "single_condition"])
        .assert()
        .success()
        .stdout(predicate::str::contains("YES/NO"));
}

#[test]
fn test_strategies_explain_accepts_hyphen_alias() {
    edgelord()
        .args(["strategies", "explain", "single-condition"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[strategies.single_condition]"));
}

#[test]
fn test_check_help_lists_health() {
    edgelord()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("health"));
}

#[test]
fn test_init_json_mode_is_rejected_with_guidance() {
    edgelord()
        .args(["--json", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("config init"));
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
