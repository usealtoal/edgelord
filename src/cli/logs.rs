//! Handler for the `logs` command.

use crate::cli::LogsArgs;
use std::process::Command;

/// Execute the logs command.
pub fn execute(args: &LogsArgs) {
    let mut cmd = Command::new("journalctl");
    cmd.args(["-u", "edgelord", "--output=cat"]);

    if args.follow {
        cmd.arg("-f");
    } else {
        cmd.args(["-n", &args.lines.to_string()]);
    }

    if let Some(ref since) = args.since {
        cmd.args(["--since", since]);
    }

    // Execute and stream output
    let status = cmd.status();

    match status {
        Ok(exit) => {
            if !exit.success() {
                if let Some(code) = exit.code() {
                    if code == 1 {
                        eprintln!("No logs found. Is the edgelord service installed?");
                        eprintln!("Run 'sudo edgelord install' to install the service.");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to execute journalctl: {e}");
            eprintln!("Make sure you're running on a system with systemd.");
        }
    }
}
