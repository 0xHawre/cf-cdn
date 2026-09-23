use cf_tunnel_proxy::{Config, WebSocketProxy, TunnelManager};
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> cf_tunnel_proxy::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    info!("cf-tunnel-proxy starting");

    // Load or create config
    let config = Config::from_file("config.toml").unwrap_or_default();
    info!("Configuration loaded: bind={}:{}", config.proxy.proxy_bind, config.proxy.proxy_port);

    // Create proxy
    let proxy = WebSocketProxy::new(config.clone()).await?;
    let tunnel_manager = TunnelManager::new(config.clone());

    // Start tunnel in background
    let (tunnel_child, tunnel_url) = tunnel_manager.start_tunnel().await?;
    info!("✓ Tunnel running at: {}", tunnel_url);
    
    // Save tunnel URL to config
    let mut config_with_url = config;
    config_with_url.tunnel.tunnel_url = Some(tunnel_url.clone());
    config_with_url.save("config.toml").ok();

    info!("Client configuration:");
    info!("  Address: {}", tunnel_url);
    info!("  Port: 443");
    info!("  TLS: enabled");
    info!("  Transport: WebSocket");
    info!("  Path: {}", config_with_url.proxy.ws_path);
    info!("");
    info!("WebSocket proxy listening on: {}", config_with_url.proxy_addr());
    info!("Press Ctrl+C to stop");

    // Run proxy until interrupted
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });

    // Wait for interrupt signal
    tokio::select! {
        result = proxy_handle => {
            if let Err(e) = result {
                eprintln!("Proxy error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    // Clean up tunnel
    TunnelManager::stop_tunnel(tunnel_child).await?;
    info!("Proxy stopped");

    Ok(())
}
