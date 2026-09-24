use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::error::{CdnError, Result};
use crate::vless::Command;

/// SOCKS5 request builder
pub struct Socks5Client {
    stream: TcpStream,
    address: String,
    port: u16,
}

impl Socks5Client {
    /// Connect to destination via direct TCP (for learning; can extend to actual SOCKS5)
    pub async fn connect(address: String, port: u16) -> Result<Self> {
        log::debug!("Connecting to {}:{}", address, port);

        // Try direct TCP connection
        let stream = TcpStream::connect(format!("{}:{}", address, port))
            .await
            .map_err(|e| CdnError::Connection(
                format!("Failed to connect to {}:{}: {}", address, port, e)
            ))?;

        log::info!("Connected to {}:{}", address, port);

        Ok(Self {
            stream,
            address,
            port,
        })
    }

    /// Get mutable reference to stream for IO operations
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Get immutable reference to stream
    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    /// Get destination address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get destination port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Write data to destination
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.write_all(buf).await?;
        Ok(())
    }

    /// Read data from destination
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.stream.read(buf).await?;
        Ok(n)
    }
}

/// Handle bidirectional traffic between client and destination
pub async fn relay_traffic(
    client_read: &mut tokio::io::ReadHalf<tokio::net::TcpStream>,
    client_write: &mut tokio::io::WriteHalf<tokio::net::TcpStream>,
    dest_read: &mut tokio::io::ReadHalf<TcpStream>,
    dest_write: &mut tokio::io::WriteHalf<TcpStream>,
) -> Result<()> {
    use tokio::io::AsyncReadExt as _;

    let mut client_buf = vec![0u8; 8192];
    let mut dest_buf = vec![0u8; 8192];

    loop {
        tokio::select! {
            // Client to destination
            result = client_read.read(&mut client_buf) => {
                let n = result?;
                if n == 0 {
                    log::debug!("Client disconnected");
                    break;
                }
                dest_write.write_all(&client_buf[..n]).await?;
            }

            // Destination to client
            result = dest_read.read(&mut dest_buf) => {
                let n = result?;
                if n == 0 {
                    log::debug!("Destination disconnected");
                    break;
                }
                client_write.write_all(&dest_buf[..n]).await?;
            }
        }
    }

    Ok(())
}

/// Format connection info for logging
pub fn format_connection(command: Command, address: &str, port: u16) -> String {
    let cmd_str = match command {
        Command::TCP => "TCP",
        Command::UDP => "UDP",
    };
    format!("{} to {}:{}", cmd_str, address, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_connection() {
        assert_eq!(
            format_connection(Command::TCP, "example.com", 443),
            "TCP to example.com:443"
        );
    }

}
