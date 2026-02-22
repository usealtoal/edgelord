use std::collections::HashSet;

use edgelord::infrastructure::{health_check, Config};

#[test]
fn health_check_reports_critical_services() {
    let config = Config::default();
    let report = health_check(&config);
    let names: HashSet<&'static str> = report.checks().iter().map(|check| check.name()).collect();

    assert!(names.contains("database"), "Expected database check");
    assert!(
        names.contains("exchange_api"),
        "Expected exchange API check"
    );
    assert!(names.contains("exchange_ws"), "Expected exchange WS check");
    assert!(names.contains("strategies"), "Expected strategies check");
}
