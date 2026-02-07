//! Shared CLI output helpers for consistent operator-facing text.

use std::fmt::Display;
use std::io::{self, Write};

const RULE_WIDTH: usize = 56;

/// Print a section header and separator.
pub fn section(title: &str) {
    println!();
    println!("{title}");
    println!("{}", "─".repeat(RULE_WIDTH));
}

/// Print a simple key/value line.
pub fn key_value(label: &str, value: impl Display) {
    println!("{label:<14} {value}");
}

/// Print a successful status line.
pub fn ok(message: &str) {
    println!("✓ {message}");
}

/// Print a warning status line.
pub fn warn(message: &str) {
    println!("⚠ {message}");
}

/// Print an error status line.
pub fn error(message: &str) {
    eprintln!("✗ {message}");
}

/// Print a single-line note.
pub fn note(message: &str) {
    println!("{message}");
}

/// Start a progress line in the format `Label... `.
pub fn progress(label: &str) {
    print!("{label}... ");
    let _ = io::stdout().flush();
}

/// Finish a progress line.
pub fn progress_done(success: bool) {
    println!("{}", if success { "ok" } else { "failed" });
}
