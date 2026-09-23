use thiserror::Error;

//Custom error handling with thiserror
//
//
//@
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("Connection error: {0}")]
    Connection(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;
