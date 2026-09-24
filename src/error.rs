use thiserror::Error;

#[derive(Error, Debug)]
pub enum CdnError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("VLESS protocol error: {0}")]
    VlessProtocol(String),

    #[error("SOCKS5 error: {0}")]
    Socks5(String),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("Encoding error: {0}")]
    Encoding(String),
}

pub type Result<T> = std::result::Result<T, CdnError>;
