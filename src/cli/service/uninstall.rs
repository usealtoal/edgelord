use std::fs;

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
