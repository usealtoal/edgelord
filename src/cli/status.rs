//! Handler for the `status` command.

use std::process::Command;

/// Execute the status command.
pub fn execute() {
    // Check if systemd service is running
    let output = Command::new("systemctl")
        .args(["is-active", "edgelord"])
        .output();

    let status = match output {
        Ok(out) => {
            let status_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if status_str == "active" {
                "● running"
            } else if status_str == "inactive" {
                "○ stopped"
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

    let version = env!("CARGO_PKG_VERSION");

    println!();
    println!("edgelord v{version}");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Some(ref p) = pid {
        println!("Status:      {status} (pid {p})");
    } else {
        println!("Status:      {status}");
    }

    // Get uptime if running
    if pid.is_some() {
        if let Ok(output) = Command::new("systemctl")
            .args(["show", "edgelord", "--property=ActiveEnterTimestamp", "--value"])
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
