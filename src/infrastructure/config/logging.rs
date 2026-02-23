//! Logging configuration and initialization.
//!
//! Provides configuration for the tracing subscriber used throughout the
//! application. Supports both pretty-printed and JSON output formats.

use serde::Deserialize;
use tracing_subscriber::{fmt, EnvFilter};

/// Logging configuration.
///
/// Controls log level filtering and output format. The `RUST_LOG` environment
/// variable takes precedence over the configured level.
#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    /// Log level filter string.
    ///
    /// Supports standard levels (trace, debug, info, warn, error) and
    /// per-module filters (e.g., "info,edgelord::exchange=debug").
    /// Defaults to "info".
    pub level: String,

    /// Output format.
    ///
    /// Supported values: "pretty" (human-readable) or "json" (structured).
    /// Defaults to "pretty".
    pub format: String,
}

impl LoggingConfig {
    /// Initialize the tracing subscriber with this configuration.
    ///
    /// Configures the global tracing subscriber. Should be called once at
    /// application startup.
    pub fn init(&self) {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&self.level));

        match self.format.as_str() {
            "json" => {
                fmt().json().with_env_filter(filter).init();
            }
            _ => {
                fmt().with_env_filter(filter).init();
            }
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            format: "pretty".into(),
        }
    }
}
