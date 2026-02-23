//! Astral-style CLI output formatting.
//!
//! Provides consistent, visually appealing terminal output with support for
//! JSON mode (for scripting), quiet mode, and verbosity levels. Output
//! styling follows the Astral tools aesthetic with colored symbols and
//! structured formatting.

use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

use owo_colors::OwoColorize;
use serde_json::json;

/// Runtime output configuration shared by CLI handlers.
///
/// Controls output formatting behavior including JSON mode for scripting,
/// quiet mode for reduced output, and verbosity levels for debugging.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputConfig {
    /// Emit machine-readable JSON output instead of human-readable text.
    pub json: bool,
    /// Suppress non-essential output.
    pub quiet: bool,
    /// Verbosity level (0 = normal, 1+ = increasingly verbose).
    pub verbose: u8,
}

impl OutputConfig {
    /// Create a new output configuration.
    #[must_use]
    pub const fn new(json: bool, quiet: bool, verbose: u8) -> Self {
        Self {
            json,
            quiet,
            verbose,
        }
    }
}

/// Global output configuration singleton.
static OUTPUT_CONFIG: OnceLock<RwLock<OutputConfig>> = OnceLock::new();

/// Return a reference to the global configuration cell.
fn config_cell() -> &'static RwLock<OutputConfig> {
    OUTPUT_CONFIG.get_or_init(|| RwLock::new(OutputConfig::default()))
}

/// Read the current output configuration.
fn read_config() -> OutputConfig {
    match config_cell().read() {
        Ok(config) => *config,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Update the global output configuration.
fn write_config(config: OutputConfig) {
    match config_cell().write() {
        Ok(mut current) => *current = config,
        Err(poisoned) => *poisoned.into_inner() = config,
    }
}

/// Check if regular (non-JSON) output should be suppressed.
fn regular_output_suppressed(config: OutputConfig) -> bool {
    !config.json && config.quiet
}

/// Emit a JSON line with type and payload structure.
fn emit_json_line(kind: &str, payload: serde_json::Value) {
    println!(
        "{}",
        json!({
            "type": kind,
            "payload": payload,
        })
    );
}

/// Apply output settings from global CLI flags.
///
/// Call this early in the CLI entry point to configure output behavior
/// based on parsed command-line arguments.
pub fn configure(config: OutputConfig) {
    write_config(config);
}

/// Return whether machine-readable JSON output is enabled.
#[must_use]
pub fn is_json() -> bool {
    read_config().json
}

/// Return whether quiet mode is enabled.
#[must_use]
pub fn is_quiet() -> bool {
    read_config().quiet
}

/// Return the global verbosity level from `-v` flags.
#[must_use]
pub fn verbosity() -> u8 {
    read_config().verbose
}

/// Print the application header with name and version.
pub fn header(version: &str) {
    let config = read_config();
    if config.json {
        emit_json_line(
            "header",
            json!({
                "app": "edgelord",
                "version": version,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("{} {}", "edgelord".bold(), version.dimmed());
    println!();
}

/// Print a labeled value.
pub fn field(label: &str, value: impl Display) {
    let config = read_config();
    let value = value.to_string();

    if config.json {
        emit_json_line(
            "field",
            json!({
                "label": label,
                "value": value,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {:<12} {}", label.dimmed(), value);
}

/// Print a success line.
pub fn success(message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("success", json!({ "message": message }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {} {}", "✓".green(), message);
}

/// Print a warning line.
pub fn warning(message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("warning", json!({ "message": message }));
        return;
    }

    println!("  {} {}", "⚠".yellow(), message);
}

/// Print an error line.
pub fn error(message: &str) {
    let config = read_config();

    if config.json {
        eprintln!(
            "{}",
            json!({
                "type": "error",
                "payload": { "message": message },
            })
        );
        return;
    }

    eprintln!("  {} {}", "×".red(), message);
}

/// Print a section header.
pub fn section(title: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("section", json!({ "title": title }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!();
    println!("{}", title.bold());
}

/// Print an info line (for streaming output).
pub fn info(timestamp: &str, label: &str, message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "info",
            json!({
                "timestamp": timestamp,
                "label": label,
                "message": message,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {} {} {}", timestamp.dimmed(), label.cyan(), message);
}

/// Print an executed trade line.
pub fn executed(timestamp: &str, message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "executed",
            json!({
                "timestamp": timestamp,
                "message": message,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "executed".green(),
        message
    );
}

/// Print a rejected opportunity line.
pub fn rejected(timestamp: &str, reason: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "rejected",
            json!({
                "timestamp": timestamp,
                "reason": reason,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {} {} {}", timestamp.dimmed(), "rejected".red(), reason);
}

/// Print an opportunity line.
pub fn opportunity(timestamp: &str, message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "opportunity",
            json!({
                "timestamp": timestamp,
                "message": message,
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "opportunity".yellow(),
        message
    );
}

/// Braille spinner animation frames (Astral-style).
const BRAILLE_SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Create and start a progress spinner with Astral-style braille animation.
///
/// Returns a hidden progress bar in JSON or quiet mode.
pub fn spinner(message: &str) -> indicatif::ProgressBar {
    let config = read_config();
    if config.json || config.quiet {
        let pb = indicatif::ProgressBar::hidden();
        pb.set_message(message.to_string());
        return pb;
    }

    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .tick_strings(BRAILLE_SPINNER)
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Finish a spinner with a success checkmark.
pub fn spinner_success(pb: &indicatif::ProgressBar, message: &str) {
    let config = read_config();
    if config.json {
        emit_json_line("spinner_success", json!({ "message": message }));
        pb.finish_and_clear();
        return;
    }
    if config.quiet {
        pb.finish_and_clear();
        return;
    }

    pb.finish_with_message(format!("{} {}", "✓".green(), message));
}

/// Finish a spinner with a failure mark.
pub fn spinner_fail(pb: &indicatif::ProgressBar, message: &str) {
    let config = read_config();
    if config.json {
        emit_json_line("spinner_fail", json!({ "message": message }));
        pb.finish_and_clear();
        return;
    }

    pb.finish_with_message(format!("{} {}", "×".red(), message));
}

/// Format a positive value in green.
pub fn positive(value: impl Display) -> String {
    let value = value.to_string();
    if is_json() {
        return value;
    }
    format!("{}", value.green())
}

/// Format a negative value in red.
pub fn negative(value: impl Display) -> String {
    let value = value.to_string();
    if is_json() {
        return value;
    }
    format!("{}", value.red())
}

/// Format a highlighted value in cyan.
pub fn highlight(value: impl Display) -> String {
    let value = value.to_string();
    if is_json() {
        return value;
    }
    format!("{}", value.cyan())
}

/// Format a dimmed/muted value.
pub fn muted(value: impl Display) -> String {
    let value = value.to_string();
    if is_json() {
        return value;
    }
    format!("{}", value.dimmed())
}

/// Print a note/hint.
pub fn note(message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("note", json!({ "message": message }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {}", message.dimmed());
}

/// Print a hint with "hint:" prefix (Astral-style).
pub fn hint(message: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("hint", json!({ "message": message }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {}: {}", "hint".cyan().dimmed(), message.dimmed());
}

/// Print an action in progress (Astral-style "Building...", "Checking...").
pub fn action(verb: &str, target: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "action",
            json!({
                "verb": verb,
                "target": target,
                "status": "in_progress",
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {} {}...", verb.bold().cyan(), target);
}

/// Print a completed action (Astral-style "✓ Built...", "✓ Checked...").
pub fn action_done(verb: &str, target: &str) {
    let config = read_config();

    if config.json {
        emit_json_line(
            "action",
            json!({
                "verb": verb,
                "target": target,
                "status": "done",
            }),
        );
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    println!("  {} {} {}", "✓".green(), verb.bold().green(), target);
}

/// Print multiple lines of content, each indented.
pub fn lines(content: &str) {
    let config = read_config();

    if config.json {
        emit_json_line("lines", json!({ "content": content }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    for line in content.lines() {
        println!("  {}", line);
    }
}

/// Emit a JSON value directly (for commands that need custom JSON output).
pub fn json_output(value: serde_json::Value) {
    println!("{}", value);
}

/// Print a table header row.
pub fn table_header(columns: &[(&str, usize)]) {
    let config = read_config();

    if config.json {
        let cols: Vec<&str> = columns.iter().map(|(name, _)| *name).collect();
        emit_json_line("table_header", json!({ "columns": cols }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    let mut line = String::from("  ");
    for (name, width) in columns {
        line.push_str(&format!("{:>width$} ", name, width = width));
    }
    println!("{}", line.dimmed());
}

/// Print a table separator line.
pub fn table_separator(widths: &[usize]) {
    let config = read_config();

    if config.json {
        return; // No separator in JSON mode
    }
    if regular_output_suppressed(config) {
        return;
    }

    let mut line = String::from("  ");
    for width in widths {
        line.push_str(&"─".repeat(*width));
        line.push(' ');
    }
    println!("{}", line.dimmed());
}

/// Print a table data row.
pub fn table_row(cells: &[String], widths: &[usize]) {
    let config = read_config();

    if config.json {
        emit_json_line("table_row", json!({ "cells": cells }));
        return;
    }
    if regular_output_suppressed(config) {
        return;
    }

    let mut line = String::from("  ");
    for (cell, width) in cells.iter().zip(widths.iter()) {
        line.push_str(&format!("{:>width$} ", cell, width = width));
    }
    println!("{}", line);
}
