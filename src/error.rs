use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Polymarket SDK error: {0}")]
    Polymarket(#[from] polymarket_client_sdk::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
