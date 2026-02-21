//! Astral-style CLI output formatting.

use std::fmt::Display;

use owo_colors::OwoColorize;

/// Print the application header.
pub fn header(version: &str) {
    println!("{} {}", "edgelord".bold(), version.dimmed());
    println!();
}

/// Print a labeled value.
pub fn field(label: &str, value: impl Display) {
    println!("  {:<12} {}", label.dimmed(), value);
}

/// Print a success line.
pub fn success(message: &str) {
    println!("  {} {}", "✓".green(), message);
}

/// Print a warning line.
pub fn warning(message: &str) {
    println!("  {} {}", "⚠".yellow(), message);
}

/// Print an error line.
pub fn error(message: &str) {
    eprintln!("  {} {}", "×".red(), message);
}

/// Print a section header.
pub fn section(title: &str) {
    println!();
    println!("{}", title.bold());
}

/// Print an info line (for streaming output).
pub fn info(timestamp: &str, label: &str, message: &str) {
    println!("  {} {} {}", timestamp.dimmed(), label.cyan(), message);
}

/// Print an executed trade line.
pub fn executed(timestamp: &str, message: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "executed".green(),
        message
    );
}

/// Print a rejected opportunity line.
pub fn rejected(timestamp: &str, reason: &str) {
    println!("  {} {} {}", timestamp.dimmed(), "rejected".red(), reason);
}

/// Print an opportunity line.
pub fn opportunity(timestamp: &str, message: &str) {
    println!(
        "  {} {} {}",
        timestamp.dimmed(),
        "opportunity".yellow(),
        message
    );
}

/// Start a progress spinner.
pub fn spinner(message: &str) -> indicatif::ProgressBar {
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
    pb.finish_with_message(format!("{} {}", "✓".green(), message));
}

/// Finish a spinner with failure.
pub fn spinner_fail(pb: &indicatif::ProgressBar, message: &str) {
    pb.finish_with_message(format!("{} {}", "×".red(), message));
}

/// Format a positive value in green.
pub fn positive(value: impl Display) -> String {
    format!("{}", value.to_string().green())
}

/// Format a negative value in red.
pub fn negative(value: impl Display) -> String {
    format!("{}", value.to_string().red())
}

/// Format a highlighted value in cyan.
pub fn highlight(value: impl Display) -> String {
    format!("{}", value.to_string().cyan())
}

/// Format a dimmed/muted value.
pub fn muted(value: impl Display) -> String {
    format!("{}", value.to_string().dimmed())
}

// Aliases for compatibility
/// Alias for `success`.
pub fn ok(message: &str) {
    success(message);
}

/// Alias for `field`.
pub fn key_value(label: &str, value: impl Display) {
    field(label, value);
}

/// Print a note/hint.
pub fn note(message: &str) {
    println!("  {}", message.dimmed());
}

/// Alias for `warning`.
pub fn warn(message: &str) {
    warning(message);
}
