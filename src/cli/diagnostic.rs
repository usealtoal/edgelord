//! Miette-based error diagnostics for beautiful CLI errors.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Configuration error with source location.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::config))]
pub struct ConfigError {
    pub message: String,

    #[source_code]
    pub src: String,

    #[label("here")]
    pub span: SourceSpan,

    #[help]
    pub help: Option<String>,
}

impl ConfigError {
    pub fn new(message: impl Into<String>, src: impl Into<String>, offset: usize, len: usize) -> Self {
        Self {
            message: message.into(),
            src: src.into(),
            span: (offset, len).into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Strategy error.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(edgelord::strategy))]
pub struct StrategyError {
    pub message: String,

    #[help]
    pub help: Option<String>,
}

impl StrategyError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Connection error.
#[derive(Debug, Error, Diagnostic)]
#[error("connection failed: {message}")]
#[diagnostic(
    code(edgelord::connection),
    help("check your network connection and exchange status")
)]
pub struct ConnectionError {
    pub message: String,
}

impl ConnectionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Wallet error.
#[derive(Debug, Error, Diagnostic)]
#[error("wallet error: {message}")]
#[diagnostic(code(edgelord::wallet))]
pub struct WalletError {
    pub message: String,

    #[help]
    pub help: Option<String>,
}

impl WalletError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
