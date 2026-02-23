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
