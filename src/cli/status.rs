//! Handler for the `status` command.

use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Duration, Utc};

use crate::app::status::StatusFile;

/// Default path for the status file.
const DEFAULT_STATUS_PATH: &str = "/var/run/edgelord/status.json";

/// Execute the status command.
pub fn execute() {
    let version = env!("CARGO_PKG_VERSION");

    // Try to read and display rich status from status file
    if let Some(status) = try_read_status_file(DEFAULT_STATUS_PATH) {
        display_rich_status(&status, version);
    } else {
        // Fall back to basic systemd status
        display_basic_status(version);
    }
}

/// Try to read the status file and return parsed status if valid.
fn try_read_status_file(path: &str) -> Option<StatusFile> {
    let path = Path::new(path);

    // Check if file exists and is readable
    let content = fs::read_to_string(path).ok()?;

    // Parse JSON
    let status: StatusFile = serde_json::from_str(&content).ok()?;

    // Check if PID is still alive (detect stale status files)
    if !is_pid_alive(status.pid) {
        return None;
    }

    // Check if status file is stale (not updated in 5 minutes)
    if status.updated_at < Utc::now() - Duration::minutes(5) {
        return None;
    }

    Some(status)
}

/// Check if a process with the given PID is still running.
fn is_pid_alive(pid: u32) -> bool {
    // Use kill with signal 0 to check if process exists
    // This doesn't actually send a signal, just checks existence
    let result = unsafe { libc::kill(pid as i32, 0) };
    if result == 0 {
        return true;
    }
    // EPERM means process exists but we can't signal it
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Format uptime as "3d 14h 22m" style.
fn format_uptime(started_at: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(started_at);

    let total_minutes = duration.num_minutes();
    if total_minutes < 0 {
        return "just started".to_string();
    }

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = total_minutes % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

/// Display rich status output from the status file.
fn display_rich_status(status: &StatusFile, version: &str) {
    let uptime = format_uptime(status.started_at);
    let strategies = status.config.strategies.join(", ");

    // Format exchange/environment info
    let exchange = &status.config.exchange;
    let environment = &status.config.environment;
    let chain_info = status
        .config
        .chain_id
        .map(|id| format!(" (chain {id})"))
        .unwrap_or_default();

    println!();
    println!("edgelord v{version}");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    println!("Status:      \u{25cf} running (pid {})", status.pid);
    println!("Uptime:      {uptime}");
    println!("Exchange:    {exchange} ({environment}){chain_info}");
    println!("Strategies:  {strategies}");
    println!();
    println!("Positions:   {} open", status.runtime.positions_open);
    println!(
        "Exposure:    ${} / ${} max",
        status.runtime.exposure_current, status.runtime.exposure_max
    );
    println!(
        "Today:       {} opportunities, {} executed, ${} profit",
        status.today.opportunities_detected,
        status.today.trades_executed,
        status.today.profit_realized
    );
    println!();
}

/// Display basic systemd status (fallback).
fn display_basic_status(version: &str) {
    // Check if systemd service is running
    let output = Command::new("systemctl")
        .args(["is-active", "edgelord"])
        .output();

    let status = match output {
        Ok(out) => {
            let status_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if status_str == "active" {
                "\u{25cf} running"
            } else if status_str == "inactive" {
                "\u{25cb} stopped"
            } else {
                "? unknown"
            }
        }
        Err(_) => "? systemd not available",
    };

    // Get PID if running
    let pid = Command::new("systemctl")
        .args(["show", "edgelord", "--property=MainPID", "--value"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|p| p != "0" && !p.is_empty());

    println!();
    println!("edgelord v{version}");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");

    if let Some(ref p) = pid {
        println!("Status:      {status} (pid {p})");
    } else {
        println!("Status:      {status}");
    }

    // Get uptime if running
    if pid.is_some() {
        if let Ok(output) = Command::new("systemctl")
            .args([
                "show",
                "edgelord",
                "--property=ActiveEnterTimestamp",
                "--value",
            ])
            .output()
        {
            let timestamp = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !timestamp.is_empty() {
                println!("Since:       {timestamp}");
            }
        }
    }

    println!();
    println!("Use 'edgelord logs' to view logs");
    println!("Use 'sudo systemctl start edgelord' to start");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_uptime_minutes_only() {
        let now = Utc::now();
        let started_at = now - Duration::minutes(45);
        assert_eq!(format_uptime(started_at), "45m");
    }

    #[test]
    fn test_format_uptime_hours_and_minutes() {
        let now = Utc::now();
        let started_at = now - Duration::hours(3) - Duration::minutes(22);
        assert_eq!(format_uptime(started_at), "3h 22m");
    }

    #[test]
    fn test_format_uptime_days_hours_minutes() {
        let now = Utc::now();
        let started_at = now - Duration::days(2) - Duration::hours(5) - Duration::minutes(10);
        assert_eq!(format_uptime(started_at), "2d 5h 10m");
    }

    #[test]
    fn test_format_uptime_just_started() {
        // Future time should return "just started"
        let now = Utc::now();
        let started_at = now + Duration::minutes(5);
        assert_eq!(format_uptime(started_at), "just started");
    }

    #[test]
    fn test_format_uptime_zero_minutes() {
        let now = Utc::now();
        let started_at = now - Duration::seconds(30);
        assert_eq!(format_uptime(started_at), "0m");
    }

    #[test]
    fn test_is_pid_alive_current_process() {
        // Current process should be alive
        let pid = std::process::id();
        assert!(is_pid_alive(pid));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
        // Very high PID should not exist
        assert!(!is_pid_alive(999_999_999));
    }
}
