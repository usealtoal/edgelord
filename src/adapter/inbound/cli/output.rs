//! Astral-style CLI output formatting.

use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

use owo_colors::OwoColorize;
use serde_json::json;

/// Runtime output configuration shared by CLI handlers.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputConfig {
    pub json: bool,
    pub quiet: bool,
    pub verbose: u8,
}

impl OutputConfig {
    #[must_use]
    pub const fn new(json: bool, quiet: bool, verbose: u8) -> Self {
        Self {
            json,
            quiet,
            verbose,
        }
    }
}

static OUTPUT_CONFIG: OnceLock<RwLock<OutputConfig>> = OnceLock::new();

fn config_cell() -> &'static RwLock<OutputConfig> {
    OUTPUT_CONFIG.get_or_init(|| RwLock::new(OutputConfig::default()))
}

fn read_config() -> OutputConfig {
    match config_cell().read() {
        Ok(config) => *config,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

fn write_config(config: OutputConfig) {
    match config_cell().write() {
        Ok(mut current) => *current = config,
        Err(poisoned) => *poisoned.into_inner() = config,
    }
}

fn regular_output_suppressed(config: OutputConfig) -> bool {
    !config.json && config.quiet
}

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
pub fn configure(config: OutputConfig) {
    write_config(config);
}

/// Whether machine-readable JSON output is enabled.
#[must_use]
pub fn is_json() -> bool {
    read_config().json
}

/// Whether quiet mode is enabled.
#[must_use]
pub fn is_quiet() -> bool {
    read_config().quiet
}

/// Global verbosity level from `-v` flags.
#[must_use]
pub fn verbosity() -> u8 {
    read_config().verbose
}

/// Print the application header.
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

/// Start a progress spinner.
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
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Finish a spinner with success.
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

/// Finish a spinner with failure.
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
