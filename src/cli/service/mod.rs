//! Handlers for `install` and `uninstall` commands.

use crate::cli::InstallArgs;
use crate::error::{Error, Result};
use std::fs;
use std::process::Command;

const SERVICE_PATH: &str = "/etc/systemd/system/edgelord.service";

/// Generate the systemd service file content.
fn generate_service_file(args: &InstallArgs, binary_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Multi-strategy arbitrage detection and execution system for prediction markets
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User={user}
Group={user}
WorkingDirectory={working_dir}
ExecStart={binary} run --no-banner --json-logs --config {config}
Restart=on-failure
RestartSec=5
EnvironmentFile=-{working_dir}/.env

[Install]
WantedBy=multi-user.target
"#,
        user = args.user,
        working_dir = args.working_dir.display(),
        binary = binary_path,
        config = args.config.display(),
    )
}

/// Execute the install command.
pub fn execute_install(args: &InstallArgs) -> Result<()> {
    // Get the path to the current binary
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/opt/edgelord/edgelord".to_string());

    let service_content = generate_service_file(args, &binary_path);

    // Check if running as root
    if !is_root() {
        return Err(Error::Connection(
            "this command must be run as root (use sudo)".to_string(),
        ));
    }

    // Write service file
    fs::write(SERVICE_PATH, &service_content)?;
    println!("✓ Created {SERVICE_PATH}");

    // Reload systemd
    if !run_systemctl(&["daemon-reload"]) {
        return Err(Error::Connection("failed to reload systemd daemon".to_string()));
    }
    println!("✓ Reloaded systemd daemon");

    // Enable service
    if !run_systemctl(&["enable", "edgelord"]) {
        return Err(Error::Connection("failed to enable service".to_string()));
    }
    println!("✓ Enabled edgelord service (starts on boot)");

    // Create status directory with correct ownership
    let status_dir = "/var/run/edgelord";
    if !std::path::Path::new(status_dir).exists() {
        if let Err(e) = fs::create_dir_all(status_dir) {
            eprintln!("Warning: Failed to create {status_dir}: {e}");
        } else {
            // chown to service user
            let user = &args.user;
            let _ = std::process::Command::new("chown")
                .args(["-R", user, status_dir])
                .status();
            println!("✓ Created {status_dir}");
        }
    }

    println!();
    println!("Start with: sudo systemctl start edgelord");
    println!("View logs:  edgelord logs -f");
    println!();
    Ok(())
}

/// Execute the uninstall command.
pub fn execute_uninstall() -> Result<()> {
    // Check if running as root
    if !is_root() {
        return Err(Error::Connection(
            "this command must be run as root (use sudo)".to_string(),
        ));
    }

    // Stop service if running
    if run_systemctl(&["stop", "edgelord"]) {
        println!("✓ Stopped edgelord service");
    }

    // Disable service
    if run_systemctl(&["disable", "edgelord"]) {
        println!("✓ Disabled edgelord service");
    }

    // Remove service file
    if std::path::Path::new(SERVICE_PATH).exists() {
        fs::remove_file(SERVICE_PATH)?;
        println!("✓ Removed {SERVICE_PATH}");
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        println!("✓ Reloaded systemd daemon");
    }

    println!();
    println!("Edgelord service has been uninstalled.");
    println!();
    Ok(())
}

fn run_systemctl(args: &[&str]) -> bool {
    Command::new("systemctl")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_root() -> bool {
    // On Unix, check if effective UID is 0
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
