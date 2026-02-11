use std::fs;

use crate::cli::output;
use crate::cli::InstallArgs;
use crate::error::{Error, Result};

use super::common::{is_root, run_systemctl, SERVICE_PATH};

/// Generate the systemd service file content.
fn generate_service_file(args: &InstallArgs, binary_path: &str) -> String {
    // Build extra arguments from runtime overrides
    let mut extra_args = Vec::new();

    if let Some(ref strategies) = args.strategies {
        extra_args.push(format!("--strategies {}", strategies));
    }
    if let Some(min_edge) = args.min_edge {
        extra_args.push(format!("--min-edge {}", min_edge));
    }
    if let Some(min_profit) = args.min_profit {
        extra_args.push(format!("--min-profit {}", min_profit));
    }
    if let Some(max_exposure) = args.max_exposure {
        extra_args.push(format!("--max-exposure {}", max_exposure));
    }
    if let Some(max_position) = args.max_position {
        extra_args.push(format!("--max-position {}", max_position));
    }
    if args.dry_run {
        extra_args.push("--dry-run".to_string());
    }
    if args.telegram_enabled {
        extra_args.push("--telegram-enabled".to_string());
    }
    if let Some(max_slippage) = args.max_slippage {
        extra_args.push(format!("--max-slippage {}", max_slippage));
    }
    if let Some(max_markets) = args.max_markets {
        extra_args.push(format!("--max-markets {}", max_markets));
    }
    if let Some(max_connections) = args.max_connections {
        extra_args.push(format!("--max-connections {}", max_connections));
    }
    if let Some(execution_timeout) = args.execution_timeout {
        extra_args.push(format!("--execution-timeout {}", execution_timeout));
    }

    let extra_args_str = if extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", extra_args.join(" "))
    };

    let edgelord_cmd = format!(
        "{binary} run --no-banner --json-logs --config {config}{extra_args}",
        binary = binary_path,
        config = args.config.display(),
        extra_args = extra_args_str,
    );

    let (exec_start, extra_env) = if args.dugout {
        // Use dugout to inject secrets at runtime
        // Use full path to dugout since systemd doesn't inherit user PATH
        let home_dir = if args.user == "root" {
            "/root".to_string()
        } else {
            format!("/home/{}", args.user)
        };
        let dugout_path = format!("{}/.cargo/bin/dugout", home_dir);
        (
            format!("{} run -- {}", dugout_path, edgelord_cmd),
            String::new(),
        )
    } else {
        // Use traditional .env file
        (
            edgelord_cmd,
            format!(
                "EnvironmentFile=-{working_dir}/.env\n",
                working_dir = args.working_dir.display()
            ),
        )
    };

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
{extra_env}ExecStart={exec_start}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
        user = args.user,
        working_dir = args.working_dir.display(),
        extra_env = extra_env,
        exec_start = exec_start,
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
    output::ok(&format!("Created {SERVICE_PATH}"));

    // Reload systemd
    if !run_systemctl(&["daemon-reload"]) {
        return Err(Error::Connection(
            "failed to reload systemd daemon".to_string(),
        ));
    }
    output::ok("Reloaded systemd daemon");

    // Enable service
    if !run_systemctl(&["enable", "edgelord"]) {
        return Err(Error::Connection("failed to enable service".to_string()));
    }
    output::ok("Enabled edgelord service (starts on boot)");

    // Create status directory with correct ownership
    let status_dir = "/var/run/edgelord";
    if !std::path::Path::new(status_dir).exists() {
        if let Err(e) = fs::create_dir_all(status_dir) {
            output::warn(&format!("Failed to create {status_dir}: {e}"));
        } else {
            // chown to service user
            let user = &args.user;
            let _ = std::process::Command::new("chown")
                .args(["-R", user, status_dir])
                .status();
            output::ok(&format!("Created {status_dir}"));
        }
    }

    output::section("Service Ready");
    output::key_value("Start", "sudo systemctl start edgelord");
    output::key_value("Logs", "edgelord logs -f");
    Ok(())
}
