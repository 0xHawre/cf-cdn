use cf_cdn::{Config, VlessServer, TunnelManager, Result};
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    env_logger::builder()
        .parse_filters(&log_level)
        .try_init()
        .ok();

    info!("cf-cdn starting");

    // Load or create config
    let config_path = env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
    let mut config = Config::from_file(&config_path).unwrap_or_default();

    // Save default config if not exists
    config.save(&config_path).ok();

    info!("═══════════════════════════════════════");
    info!("VLESS Server Configuration");
    info!("═══════════════════════════════════════");
    info!("Bind Address: {}:{}", config.server.bind, config.server.port);
    info!("WebSocket Path: {}", config.server.ws_path);
    info!("UUID: {}", config.vless.uuid);
    info!("Encryption: {}", config.vless.encryption);
    info!("");

    // Start VLESS server
    let server = VlessServer::new(config.clone()).await?;

    let tunnel_handle = if config.tunnel.enabled {
        // Start tunnel
        let tunnel_manager = TunnelManager::new(config.clone());
        match tunnel_manager.start_tunnel().await {
            Ok((tunnel_child, tunnel_url)) => {
                info!("═══════════════════════════════════════");
                info!("Cloudflare Tunnel");
                info!("═══════════════════════════════════════");
                info!("Public URL: {}", tunnel_url);
                info!("");

                // Save tunnel URL to config
                let mut config_with_url = config.clone();
                config_with_url.tunnel.tunnel_url = Some(tunnel_url.clone());
                config_with_url.save(&config_path).ok();

                // Print sharing links
                info!("═══════════════════════════════════════");
                info!("Client Configuration");
                info!("═══════════════════════════════════════");
                info!("VLESS Link:");
                info!("{}", config_with_url.vless_link(Some(&tunnel_url)));
                info!("");
                info!("VMSS Subscription (base64):");
                info!("{}", config_with_url.vmss_subscription(Some(&tunnel_url)));
                info!("");

                Some(tokio::spawn(async move {
                    let _ = tokio::signal::ctrl_c().await;
                    TunnelManager::stop_tunnel(tunnel_child).await
                }))
            }
            Err(e) => {
                log::warn!("Failed to start tunnel: {}", e);
                info!("═══════════════════════════════════════");
                info!("Local Configuration");
                info!("═══════════════════════════════════════");
                info!("VLESS Link:");
                info!("{}", config.vless_link(None));
                info!("");
                info!("VMSS Subscription (base64):");
                info!("{}", config.vmss_subscription(None));
                info!("");
                None
            }
        }
    } else {
        info!("Cloudflare tunnel is disabled");
        info!("═══════════════════════════════════════");
        info!("Local Configuration");
        info!("═══════════════════════════════════════");
        info!("VLESS Link:");
        info!("{}", config.vless_link(None));
        info!("");
        None
    };

    info!("Server running... Press Ctrl+C to stop");
    info!("═══════════════════════════════════════");
    info!("");

    // Run server until interrupted
    let server_handle = tokio::spawn(async move {
        server.run().await
    });

    // Wait for first to complete
    tokio::select! {
        result = server_handle => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    // Clean up tunnel if exists
    if let Some(tunnel_task) = tunnel_handle {
        tunnel_task.abort();
    }

    info!("cf-cdn stopped");
    Ok(())
}
