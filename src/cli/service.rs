//! Handlers for `install` and `uninstall` commands.

use crate::cli::InstallArgs;
use std::fs;
use std::process::Command;

const SERVICE_PATH: &str = "/etc/systemd/system/edgelord.service";

/// Generate the systemd service file content.
fn generate_service_file(args: &InstallArgs, binary_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Edgelord Arbitrage Service
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
pub fn execute_install(args: &InstallArgs) {
    // Get the path to the current binary
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/opt/edgelord/edgelord".to_string());

    let service_content = generate_service_file(args, &binary_path);

    // Check if running as root
    if !is_root() {
        eprintln!("Error: This command must be run as root (use sudo)");
        std::process::exit(1);
    }

    // Write service file
    match fs::write(SERVICE_PATH, &service_content) {
        Ok(()) => println!("✓ Created {SERVICE_PATH}"),
        Err(e) => {
            eprintln!("Failed to create service file: {e}");
            std::process::exit(1);
        }
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        println!("✓ Reloaded systemd daemon");
    } else {
        eprintln!("Failed to reload systemd daemon");
        std::process::exit(1);
    }

    // Enable service
    if run_systemctl(&["enable", "edgelord"]) {
        println!("✓ Enabled edgelord service (starts on boot)");
    } else {
        eprintln!("Failed to enable service");
        std::process::exit(1);
    }

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
}

/// Execute the uninstall command.
pub fn execute_uninstall() {
    // Check if running as root
    if !is_root() {
        eprintln!("Error: This command must be run as root (use sudo)");
        std::process::exit(1);
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
        match fs::remove_file(SERVICE_PATH) {
            Ok(()) => println!("✓ Removed {SERVICE_PATH}"),
            Err(e) => {
                eprintln!("Failed to remove service file: {e}");
                std::process::exit(1);
            }
        }
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        println!("✓ Reloaded systemd daemon");
    }

    println!();
    println!("Edgelord service has been uninstalled.");
    println!();
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
