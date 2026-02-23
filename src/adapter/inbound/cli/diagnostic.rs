//! Miette-based error diagnostics for CLI error presentation.
//!
//! Provides rich error types with source code context, labels, and help
//! suggestions for improved user experience when errors occur.
//!
//! The struct fields are used by miette's derive macros at runtime to
//! render formatted error output with code snippets and annotations.
#![allow(unused)]

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Configuration error with source location context.
///
/// Displays the configuration file content with a labeled span pointing
/// to the problematic location, along with an optional help message.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::config))]
pub struct ConfigError {
    /// Human-readable error message.
    pub message: String,

    /// Source content (typically the configuration file).
    #[source_code]
    pub src: String,

    /// Byte offset and length of the problematic region.
    #[label("here")]
    pub span: SourceSpan,

    /// Optional help text with suggestions for fixing the error.
    #[help]
    pub help: Option<String>,
}

impl ConfigError {
    /// Create a new configuration error with source location.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable error description
    /// * `src` - Source content (e.g., configuration file text)
    /// * `offset` - Byte offset of the error location
    /// * `len` - Length of the problematic span in bytes
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        src: impl Into<String>,
        offset: usize,
        len: usize,
    ) -> Self {
        Self {
            message: message.into(),
            src: src.into(),
            span: (offset, len).into(),
            help: None,
        }
    }

    /// Add a help suggestion to the error.
    ///
    /// Returns the modified error for method chaining.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Strategy-related error.
///
/// Reports errors in arbitrage strategy configuration or execution,
/// with optional help suggestions for resolution.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::strategy))]
pub struct StrategyError {
    /// Human-readable error message.
    pub message: String,

    /// Optional help text with suggestions for fixing the error.
    #[help]
    pub help: Option<String>,
}

impl StrategyError {
    /// Create a new strategy error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
        }
    }

    /// Add a help suggestion to the error.
    ///
    /// Returns the modified error for method chaining.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Network connection error.
///
/// Reports failures in WebSocket or HTTP connections to external services,
/// with a default help suggestion to check network and exchange status.
#[derive(Debug, Error, Diagnostic)]
#[error("connection failed: {message}")]
#[diagnostic(
    code(edgelord::connection),
    help("check your network connection and exchange status")
)]
pub struct ConnectionError {
    /// Detailed error message from the connection failure.
    pub message: String,
}

impl ConnectionError {
    /// Create a new connection error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Wallet operation error.
///
/// Reports errors in wallet operations such as signing, approvals,
/// or balance queries, with optional help suggestions.
#[derive(Debug, Error, Diagnostic)]
#[error("wallet error: {message}")]
#[diagnostic(code(edgelord::wallet))]
pub struct WalletError {
    /// Human-readable error message.
    pub message: String,

    /// Optional help text with suggestions for fixing the error.
    #[help]
    pub help: Option<String>,
}

impl WalletError {
    /// Create a new wallet error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
        }
    }

    /// Add a help suggestion to the error.
    ///
    /// Returns the modified error for method chaining.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    // Tests for ConfigError

    #[test]
    fn test_config_error_new_basic() {
        let error = ConfigError::new("Invalid syntax", "key = value", 0, 3);

        assert_eq!(error.message, "Invalid syntax");
        assert_eq!(error.src, "key = value");
        assert!(error.help.is_none());
    }

    #[test]
    fn test_config_error_with_help() {
        let error =
            ConfigError::new("Missing value", "key = ", 4, 2).with_help("Provide a valid value");

        assert_eq!(error.message, "Missing value");
        assert_eq!(error.help, Some("Provide a valid value".to_string()));
    }

    #[test]
    fn test_config_error_span_offset_and_length() {
        let error = ConfigError::new("Error", "0123456789", 5, 3);

        // SourceSpan should be (5, 3) - offset 5, length 3
        let span = error.span;
        assert_eq!(span.offset(), 5);
        assert_eq!(span.len(), 3);
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::new("Test error message", "source", 0, 1);
        let display = format!("{}", error);
        assert_eq!(display, "Test error message");
    }

    #[test]
    fn test_config_error_from_string_types() {
        // Test with String
        let error1 = ConfigError::new(String::from("message"), String::from("src"), 0, 1);
        assert_eq!(error1.message, "message");
        assert_eq!(error1.src, "src");

        // Test with &str
        let error2 = ConfigError::new("message", "src", 0, 1);
        assert_eq!(error2.message, "message");
        assert_eq!(error2.src, "src");
    }

    #[test]
    fn test_config_error_method_chaining() {
        let error = ConfigError::new("error", "src", 0, 1)
            .with_help("first help")
            .with_help("second help");

        // Last help should win
        assert_eq!(error.help, Some("second help".to_string()));
    }

    #[test]
    fn test_config_error_debug() {
        let error = ConfigError::new("Test", "source", 0, 1);
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ConfigError"));
        assert!(debug_str.contains("message"));
    }

    // Tests for StrategyError

    #[test]
    fn test_strategy_error_new() {
        let error = StrategyError::new("Strategy failed");

        assert_eq!(error.message, "Strategy failed");
        assert!(error.help.is_none());
    }

    #[test]
    fn test_strategy_error_with_help() {
        let error = StrategyError::new("No markets found").with_help("Check market configuration");

        assert_eq!(error.message, "No markets found");
        assert_eq!(error.help, Some("Check market configuration".to_string()));
    }

    #[test]
    fn test_strategy_error_display() {
        let error = StrategyError::new("Strategy calculation error");
        let display = format!("{}", error);
        assert_eq!(display, "Strategy calculation error");
    }

    #[test]
    fn test_strategy_error_from_string() {
        let error = StrategyError::new(String::from("dynamic message"));
        assert_eq!(error.message, "dynamic message");
    }

    #[test]
    fn test_strategy_error_method_chaining() {
        let error = StrategyError::new("error")
            .with_help("help1")
            .with_help("help2");
        assert_eq!(error.help, Some("help2".to_string()));
    }

    // Tests for ConnectionError

    #[test]
    fn test_connection_error_new() {
        let error = ConnectionError::new("Connection refused");

        assert_eq!(error.message, "Connection refused");
    }

    #[test]
    fn test_connection_error_display() {
        let error = ConnectionError::new("timeout");
        let display = format!("{}", error);
        assert_eq!(display, "connection failed: timeout");
    }

    #[test]
    fn test_connection_error_from_string() {
        let error = ConnectionError::new(String::from("network unreachable"));
        assert_eq!(error.message, "network unreachable");
    }

    #[test]
    fn test_connection_error_debug() {
        let error = ConnectionError::new("test");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ConnectionError"));
        assert!(debug_str.contains("message"));
    }

    // Tests for WalletError

    #[test]
    fn test_wallet_error_new() {
        let error = WalletError::new("Insufficient balance");

        assert_eq!(error.message, "Insufficient balance");
        assert!(error.help.is_none());
    }

    #[test]
    fn test_wallet_error_with_help() {
        let error = WalletError::new("Approval failed").with_help("Ensure sufficient gas");

        assert_eq!(error.message, "Approval failed");
        assert_eq!(error.help, Some("Ensure sufficient gas".to_string()));
    }

    #[test]
    fn test_wallet_error_display() {
        let error = WalletError::new("signing failed");
        let display = format!("{}", error);
        assert_eq!(display, "wallet error: signing failed");
    }

    #[test]
    fn test_wallet_error_from_string() {
        let error = WalletError::new(String::from("key not found"));
        assert_eq!(error.message, "key not found");
    }

    #[test]
    fn test_wallet_error_method_chaining() {
        let error = WalletError::new("error")
            .with_help("help1")
            .with_help("help2");
        assert_eq!(error.help, Some("help2".to_string()));
    }

    // Tests for Error trait implementation

    #[test]
    fn test_config_error_is_error() {
        let error: Box<dyn Error> = Box::new(ConfigError::new("test", "src", 0, 1));
        assert!(error.source().is_none());
    }

    #[test]
    fn test_strategy_error_is_error() {
        let error: Box<dyn Error> = Box::new(StrategyError::new("test"));
        assert!(error.source().is_none());
    }

    #[test]
    fn test_connection_error_is_error() {
        let error: Box<dyn Error> = Box::new(ConnectionError::new("test"));
        assert!(error.source().is_none());
    }

    #[test]
    fn test_wallet_error_is_error() {
        let error: Box<dyn Error> = Box::new(WalletError::new("test"));
        assert!(error.source().is_none());
    }

    // Edge case tests

    #[test]
    fn test_config_error_empty_strings() {
        let error = ConfigError::new("", "", 0, 0);
        assert_eq!(error.message, "");
        assert_eq!(error.src, "");
    }

    #[test]
    fn test_strategy_error_empty_message() {
        let error = StrategyError::new("");
        assert_eq!(error.message, "");
    }

    #[test]
    fn test_connection_error_empty_message() {
        let error = ConnectionError::new("");
        assert_eq!(error.message, "");
        assert_eq!(format!("{}", error), "connection failed: ");
    }

    #[test]
    fn test_wallet_error_empty_message() {
        let error = WalletError::new("");
        assert_eq!(error.message, "");
    }

    #[test]
    fn test_config_error_unicode_message() {
        let error = ConfigError::new("Unicode: ", "src: ", 0, 4);
        assert!(error.message.contains(""));
        assert!(error.src.contains(""));
    }

    #[test]
    fn test_help_with_empty_string() {
        let error = StrategyError::new("test").with_help("");
        assert_eq!(error.help, Some("".to_string()));
    }

    #[test]
    fn test_config_error_large_span() {
        let large_src = "x".repeat(10000);
        let error = ConfigError::new("error", large_src.clone(), 5000, 1000);

        assert_eq!(error.span.offset(), 5000);
        assert_eq!(error.span.len(), 1000);
        assert_eq!(error.src.len(), 10000);
    }
}
