use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
use crate::error::{CdnError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlessConfig {
    /// VLESS user UUID
    pub uuid: String,
    
    /// Encryption level (none, auto)
    #[serde(default = "default_encryption")]
    pub encryption: String,
    
    /// Enable network (tcp, udp, both)
    #[serde(default = "default_network")]
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address for VLESS server
    #[serde(default = "default_bind")]
    pub bind: String,
    
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// WebSocket path
    #[serde(default = "default_ws_path")]
    pub ws_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Enable Cloudflare tunnel
    #[serde(default)]
    pub enabled: bool,
    
    /// Local port to tunnel
    pub local_port: u16,
    
    /// Cloudflare tunnel URL (auto-filled)
    #[serde(default)]
    pub tunnel_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub vless: VlessConfig,
    pub tunnel: TunnelConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

// Default values
fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8888
}

fn default_ws_path() -> String {
    "/vless".to_string()
}

fn default_encryption() -> String {
    "none".to_string()
}

fn default_network() -> String {
    "tcp".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: default_bind(),
                port: default_port(),
                ws_path: default_ws_path(),
            },
            vless: VlessConfig {
                uuid: Uuid::new_v4().to_string(),
                encryption: default_encryption(),
                network: default_network(),
            },
            tunnel: TunnelConfig {
                enabled: false,
                local_port: default_port(),
                tunnel_url: None,
            },
            logging: LoggingConfig {
                level: default_log_level(),
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
                    .map_err(|e| CdnError::Config(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("Config file not found, using defaults");
                Ok(Self::default())
            }
            Err(e) => Err(CdnError::Io(e)),
        }
    }

    /// Save config to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| CdnError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get VLESS link (share format)
    pub fn vless_link(&self, tunnel_url: Option<&str>) -> String {
        let host = tunnel_url
            .unwrap_or(&format!("{}:{}", self.server.bind, self.server.port));
        
        format!(
            "vless://{}@{}?encryption={}&type=ws&path={}",
            self.vless.uuid,
            host,
            self.vless.encryption,
            urlencoding::encode(&self.server.ws_path)
        )
    }

    /// Get VMSS subscription format
    pub fn vmss_subscription(&self, tunnel_url: Option<&str>) -> String {
        let vless_link = self.vless_link(tunnel_url);
        base64::encode(format!("{}\n", vless_link))
    }
}

// For URL encoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}
