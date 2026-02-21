use std::fs;

use crate::cli::output;
use crate::error::{Error, Result};

use super::common::{is_root, run_systemctl, SERVICE_PATH};

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
        output::success("Stopped edgelord service");
    }

    // Disable service
    if run_systemctl(&["disable", "edgelord"]) {
        output::success("Disabled edgelord service");
    }

    // Remove service file
    if std::path::Path::new(SERVICE_PATH).exists() {
        fs::remove_file(SERVICE_PATH)?;
        output::success(&format!("Removed {SERVICE_PATH}"));
    }

    // Reload systemd
    if run_systemctl(&["daemon-reload"]) {
        output::success("Reloaded systemd daemon");
    }

    output::section("Service Removed");
    output::field("Status", "edgelord service uninstalled");
    Ok(())
}
