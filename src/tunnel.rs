use crate::config::Config;
use crate::error::{CdnError, Result};
use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

pub struct TunnelManager {
    config: Config,
}

impl TunnelManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Start cloudflared tunnel
    pub async fn start_tunnel(&self) -> Result<(Child, String)> {
        if !self.config.tunnel.enabled {
            return Err(CdnError::Tunnel("Tunnel disabled in config".to_string()));
        }

        // Check if cloudflared is installed
        self.check_cloudflared_installed()?;

        let local_url = format!("http://127.0.0.1:{}", self.config.tunnel.local_port);

        log::info!("Starting Cloudflare tunnel to {}", local_url);

        let mut child = tokio::process::Command::new("cloudflared")
            .arg("tunnel")
            .arg("--url")
            .arg(&local_url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CdnError::Tunnel(format!("Failed to spawn cloudflared: {}", e)))?;

        // Extract URL from cloudflared output
        let stdout = child.stdout.take()
            .ok_or_else(|| CdnError::Tunnel("Failed to capture stdout".to_string()))?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let tunnel_url = self.extract_tunnel_url(&mut lines).await?;

        log::info!("Tunnel established at: {}", tunnel_url);

        Ok((child, tunnel_url))
    }

    /// Extract the trycloudflare.com URL from cloudflared output
    async fn extract_tunnel_url(
        &self,
        lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    ) -> Result<String> {
        let timeout = std::time::Duration::from_secs(10);
        let start = std::time::Instant::now();

        while let Some(line) = lines.next_line().await? {
            log::debug!("cloudflared: {}", line);

            if let Some(start_idx) = line.find("https://") {
                if let Some(end_idx) = line[start_idx..].find(" ") {
                    let url = &line[start_idx..start_idx + end_idx];
                    if url.contains("trycloudflare.com") {
                        return Ok(url.to_string());
                    }
                } else if line[start_idx..].contains("trycloudflare.com") {
                    return Ok(line[start_idx..].trim().to_string());
                }
            }

            if start.elapsed() > timeout {
                return Err(CdnError::Tunnel(
                    "Timeout waiting for cloudflared URL".to_string(),
                ));
            }
        }

        Err(CdnError::Tunnel(
            "cloudflared did not produce a tunnel URL".to_string(),
        ))
    }

    /// Check if cloudflared is installed
    pub fn check_cloudflared_installed(&self) -> Result<()> {
        match Command::new("cloudflared")
            .arg("--version")
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    log::info!("Using cloudflared: {}", version.trim());
                    Ok(())
                } else {
                    Err(CdnError::Tunnel(
                        "cloudflared --version returned non-zero exit code".to_string(),
                    ))
                }
            }
            Err(e) => Err(CdnError::Tunnel(format!(
                "cloudflared not found. Install with: sudo apt install cloudflared. Error: {}",
                e
            ))),
        }
    }

    /// Terminate the tunnel
    pub async fn stop_tunnel(mut child: Child) -> Result<()> {
        child.kill().await?;
        child.wait().await?;
        log::info!("Tunnel stopped");
        Ok(())
    }
}
