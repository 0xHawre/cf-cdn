use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{ProxyError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Local WebSocket proxy bind address
    pub proxy_bind: String,
    
    /// Local WebSocket proxy listen port
    pub proxy_port: u16,
    
    /// WebSocket path (e.g., /ws)
    pub ws_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Port to expose through tunnel (same as proxy_port)
    pub local_port: u16,
    
    /// Cloudflared tunnel URL (will be shown after tunnel starts)
    #[serde(default)]
    pub tunnel_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub tunnel: TunnelConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig {
                proxy_bind: "127.0.0.1".to_string(),
                proxy_port: 8080,
                ws_path: "/ws".to_string(),
            },
            tunnel: TunnelConfig {
                local_port: 8080,
                tunnel_url: None,
            },
        }
    }
}

impl Config {
    /// Load config from TOML file, fall back to defaults
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        match std::fs::read_to_string(path.as_ref()) {
            Ok(content) => {
                toml::from_str(&content)
                    .map_err(|e| ProxyError::Config(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("Config file not found, using defaults");
                Ok(Self::default())
            }
            Err(e) => Err(ProxyError::Io(e)),
        }
    }

    /// Save config to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the full proxy address
    pub fn proxy_addr(&self) -> String {
        format!("{}:{}", self.proxy.proxy_bind, self.proxy.proxy_port)
    }

    /// Get the full WebSocket path
    pub fn ws_full_path(&self) -> String {
        self.proxy.ws_path.clone()
    }
}
