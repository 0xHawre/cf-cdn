use crate::config::Config;
use crate::error::{CdnError, Result};
use crate::vless::{VlessRequest, validate_uuid};
use crate::socks5::{Socks5Client, relay_traffic, format_connection};
use bytes::BytesMut;
use futures::stream::StreamExt;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_async;

pub struct VlessServer {
    config: Config,
    listener: TcpListener,
}

impl VlessServer {
    pub async fn new(config: Config) -> Result<Self> {
        let addr = format!("{}:{}", config.server.bind, config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| CdnError::Connection(e.to_string()))?;

        let listener = TcpListener::bind(addr).await?;
        log::info!("VLESS server listening on {}", addr);

        Ok(Self { config, listener })
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    log::debug!("New connection from {}", addr);
                    let config = self.config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, addr, &config).await {
                            log::error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                    return Err(CdnError::Io(e));
                }
            }
        }
    }

    async fn handle_client(stream: TcpStream, addr: SocketAddr, config: &Config) -> Result<()> {
        // Upgrade to WebSocket
        let ws_stream = accept_async(stream).await
            .map_err(|e| CdnError::WebSocket(e))?;

        log::info!("WebSocket connection established from {}", addr);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let mut client_buf = BytesMut::with_capacity(1024);

        // Read VLESS request header from first WebSocket message
        let vless_req = loop {
            match ws_receiver.next().await {
                Some(Ok(Message::Binary(data))) => {
                    client_buf.extend_from_slice(&data);

                    // Try to parse VLESS request
                    match VlessRequest::parse(client_buf.clone()) {
                        Ok((req, remaining)) => {
                            client_buf = remaining;
                            break req;
                        }
                        Err(e) => {
                            // Need more data
                            if client_buf.len() > 1024 {
                                return Err(CdnError::VlessProtocol("Header too large".to_string()));
                            }
                            continue;
                        }
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    log::info!("Client {} closed connection", addr);
                    return Ok(());
                }
                Some(Ok(_)) => {
                    return Err(CdnError::VlessProtocol(
                        "Expected binary message".to_string(),
                    ));
                }
                Some(Err(e)) => return Err(CdnError::WebSocket(e)),
                None => {
                    return Err(CdnError::VlessProtocol("Unexpected end of stream".to_string()));
                }
            }
        };

        // Validate UUID
        validate_uuid(&vless_req.uuid, &config.vless.uuid)?;

        let conn_info = format_connection(vless_req.command, &vless_req.address, vless_req.port);
        log::info!("VLESS request from {}: {}", addr, conn_info);

        // Connect to destination
        let mut dest_client = match Socks5Client::connect(vless_req.address.clone(), vless_req.port).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to connect to {}: {}", vless_req.address, e);
                // Send close message to client
                ws_sender.send(Message::Close(None)).await.ok();
                return Err(e);
            }
        };

        // Split destination connection
        let dest_stream = dest_client.stream_mut().split();
        let (mut dest_read, mut dest_write) = dest_stream;

        // Send any buffered data to destination
        if !client_buf.is_empty() {
            dest_write.write_all(&client_buf).await?;
        }

        // Relay loop
        use tokio::io::AsyncReadExt;
        loop {
            let mut buf = BytesMut::with_capacity(8192);
            tokio::select! {
                // WebSocket to destination
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            dest_write.write_all(&data).await
                                .map_err(|e| CdnError::Connection(e.to_string()))?;
                        }
                        Some(Ok(Message::Close(_))) => {
                            log::debug!("Client {} closed WebSocket", addr);
                            break;
                        }
                        Some(Ok(_)) => {
                            log::warn!("Unexpected message type from client");
                        }
                        Some(Err(e)) => return Err(CdnError::WebSocket(e)),
                        None => {
                            log::debug!("WebSocket receiver ended");
                            break;
                        }
                    }
                }

                // Destination to WebSocket
                read_result = dest_read.read_buf(&mut buf) => {
                    match read_result {
                        Ok(0) => {
                            log::debug!("Destination closed connection");
                            break;
                        }
                        Ok(_n) => {
                            let msg = Message::Binary(buf.to_vec().into());
                            if let Err(e) = ws_sender.send(msg).await {
                                log::error!("Failed to send to WebSocket: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Error reading from destination: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        log::info!("Connection closed: {}", conn_info);
        Ok(())
    }
}
