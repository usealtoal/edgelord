use thiserror::Error;

use crate::domain::error::DomainError;

/// Configuration-related errors with structured variants.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },

    #[error("failed to read config file: {0}")]
    ReadFile(#[source] std::io::Error),

    #[error("failed to parse config: {0}")]
    Parse(#[source] toml::de::Error),

    #[error("{0}")]
    Other(String),
}

/// Execution-related errors with structured variants.
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("invalid token ID '{token_id}': {reason}")]
    InvalidTokenId { token_id: String, reason: String },

    #[error("order rejected: {0}")]
    OrderRejected(String),

    #[error("failed to build order: {0}")]
    OrderBuildFailed(String),

    #[error("failed to sign order: {0}")]
    SigningFailed(String),

    #[error("failed to submit order: {0}")]
    SubmissionFailed(String),
}

/// Risk management errors.
#[derive(Error, Debug, Clone)]
pub enum RiskError {
    #[error("circuit breaker active: {reason}")]
    CircuitBreakerActive { reason: String },

    #[error("position limit exceeded: {current} >= {limit} for market {market_id}")]
    PositionLimitExceeded {
        market_id: String,
        current: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("exposure limit exceeded: {current} + {additional} > {limit}")]
    ExposureLimitExceeded {
        current: rust_decimal::Decimal,
        additional: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("profit below threshold: {expected} < {threshold}")]
    ProfitBelowThreshold {
        expected: rust_decimal::Decimal,
        threshold: rust_decimal::Decimal,
    },

    #[error("slippage too high: {actual} > {max}")]
    SlippageTooHigh {
        actual: rust_decimal::Decimal,
        max: rust_decimal::Decimal,
    },
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error(transparent)]
    Risk(#[from] RiskError),

    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[cfg(feature = "polymarket")]
    #[error("Polymarket SDK error: {0}")]
    Polymarket(#[from] polymarket_client_sdk::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(Box::new(err))
    }
}

impl From<dialoguer::Error> for Error {
    fn from(err: dialoguer::Error) -> Self {
        // dialoguer::Error wraps an IO error
        Error::Io(std::io::Error::other(err.to_string()))
    }
}
