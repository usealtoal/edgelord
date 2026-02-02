use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

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

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Polymarket SDK error: {0}")]
    Polymarket(#[from] polymarket_client_sdk::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
