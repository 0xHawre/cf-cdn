# cf-tunnel-proxy

A WebSocket proxy that integrates with Cloudflare Tunnels.

# Architecture
```
mindmap
  root((mrmine))
    eClient
      phone
      laptop
      HTTPS / WebSocket :443
    Cloudflare Edge
      TLS terminate
      hostname → tunnel
    Outbound tunnel
      cloudflared
    Your VPS / Laptop
      localhost:8080
    WebSocket Proxy
      this project
    Internet
      upstream
```
## Key Concepts

- **WebSocket Proxy**: Listens on `127.0.0.1:8080`, accepts WebSocket connections
- **Cloudflare Tunnel**: `cloudflared` binary creates an encrypted outbound tunnel to Cloudflare's edge
- **URL Masking**: Clients connect to `https://random-name.trycloudflare.com` instead of your VPS IP
- **No Inbound Ports**: Origin server stays bound to `127.0.0.1` only; firewall doesn't need to open ports

## Prerequisites

### Install cloudflared

**Debian/Ubuntu:**
```bash
sudo mkdir -p --mode=0755 /usr/share/keyrings
curl -fsSL https://pkg.cloudflare.com/cloudflare-public-v2.gpg \
  | sudo tee /usr/share/keyrings/cloudflare-public-v2.gpg >/dev/null

echo "deb [signed-by=/usr/share/keyrings/cloudflare-public-v2.gpg] https://pkg.cloudflare.com/cloudflared any main" \
  | sudo tee /etc/apt/sources.list.d/cloudflared.list

sudo apt update && sudo apt install cloudflared
```

**Verify:**
```bash
cloudflared --version
```

### Rust (for building)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Building

```bash
git clone https://github.com/0xHawre/cf-cdn.git
cd cf-cdn
cargo build --release
```

Binary: `target/release/proxy` and `target/release/tunnel-cli`.

## Usage

### Start the Proxy with Tunnel

```bash
RUST_LOG=info ./target/release/proxy
```

This will:
1. Start WebSocket proxy on `127.0.0.1:8080`
2. Launch `cloudflared tunnel` to Cloudflare edge
3. Print your public tunnel URL (e.g., `https://quiet-marble-otter.trycloudflare.com`)
4. Display client configuration

**Output:**
```
cf-tunnel-proxy starting
Configuration loaded: bind=127.0.0.1:8080
✓ Tunnel running at: https://quiet-marble-otter.trycloudflare.com
Client configuration:
  Address: https://quiet-marble-otter.trycloudflare.com
  Port: 443
  TLS: enabled
  Transport: WebSocket
  Path: /ws

WebSocket proxy listening on: 127.0.0.1:8080
Press Ctrl+C to stop
```

### Verify Installation

```bash
./target/release/tunnel-cli check
```

### Start Tunnel Only (for debugging)

```bash
./target/release/tunnel-cli start
```

## Configuration

Copy and edit `config.example.toml`:

```bash
cp config.example.toml config.toml
```

Edit as needed:
```toml
[proxy]
proxy_bind = "127.0.0.1"  # Keep localhost for security
proxy_port = 8080         # Match tunnel local_port
ws_path = "/ws"           # WebSocket path clients use

[tunnel]
local_port = 8080         # Must match proxy_port
tunnel_url = ""           # Auto-filled on first run
```

## Logging

Control log level:

```bash
RUST_LOG=debug ./target/release/proxy
RUST_LOG=trace ./target/release/proxy
```

## Client Configuration

After starting the proxy, use this in your client (v2ray, Nekoray, etc.):

```
Protocol: VLESS or similar WebSocket-based
Address: quiet-marble-otter.trycloudflare.com  (from output)
Port: 443
TLS: enabled
Transport: WebSocket
Path: /ws
SNI: quiet-marble-otter.trycloudflare.com
Host: quiet-marble-otter.trycloudflare.com
```

The client will appear to be exiting from your VPS IP, not your actual home network.

## Security Notes

- **Quick Tunnels**: Ephemeral (URL changes on restart), no uptime guarantee (~200 req/s limit)
- **Best for**: Testing, development, temporary use
- **NOT for**: Production services, high-traffic apps

For persistent use, upgrade to a named tunnel with your own domain:
```bash
cloudflared tunnel create my-tunnel
cloudflared tunnel route dns my-tunnel vpn.yourdomain.com
# Point cloudflared at this proxy's localhost:8080
```
## Troubleshooting

### "cloudflared not found"
```bash
cloudflared --version  # Check if installed
which cloudflared      # Verify PATH
```

### Tunnel URL takes a long time
Normal; cloudflared needs 5-10 seconds to establish the connection.

### WebSocket connection refused
- Check proxy is running: `RUST_LOG=debug ./target/release/proxy`
- Verify localhost:8080 is listening: `ss -tlnp | grep 8080`
- Check firewall isn't blocking outbound to Cloudflare

### Client connects but no internet
- Verify proxy is actually forwarding (stub implementation doesn't proxy yet)
- Check client config (path, SNI, etc.)
- Look at logs: `RUST_LOG=debug ./target/release/proxy`

## Next Steps
## License

