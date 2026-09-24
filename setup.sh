#!/bin/bash

# cf-cdn Setup Script
# Automates installation for Linux systems
# Usage: ./setup.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Banner
echo -e "${BLUE}"
echo "╔════════════════════════════════════════╗"
echo "║         cf-cdn Setup Script            ║"
echo "║     VLESS Proxy with Cloudflare        ║"
echo "╚════════════════════════════════════════╝"
echo -e "${NC}"

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo -e "${RED}✗ Error: This script only supports Linux${NC}"
    echo "Your OS: $OSTYPE"
    echo ""
    echo "For other systems, see manual installation in README.md"
    exit 1
fi

echo -e "${GREEN}✓ Linux detected${NC}"
echo ""

# System Information
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}System Information${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    VER=$VERSION_ID
    echo -e "${GREEN}✓${NC} OS: $OS $VER"
else
    echo -e "${YELLOW}?${NC} OS: Unknown"
fi

# Get architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "x86_64" ]]; then
    echo -e "${GREEN}✓${NC} Architecture: x86_64 (supported)"
elif [[ "$ARCH" == "aarch64" ]]; then
    echo -e "${GREEN}✓${NC} Architecture: ARM64 (supported)"
else
    echo -e "${YELLOW}!${NC} Architecture: $ARCH (may not be supported)"
fi

# CPU count
CPUS=$(nproc)
echo -e "${GREEN}✓${NC} CPUs: $CPUS"

# Memory
if command -v free &> /dev/null; then
    MEM=$(free -h | awk 'NR==2 {print $2}')
    echo -e "${GREEN}✓${NC} Memory: $MEM"
fi

# Kernel
KERNEL=$(uname -r)
echo -e "${GREEN}✓${NC} Kernel: $KERNEL"

echo ""

# Check for supported distributions
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Distribution Check${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [[ "$OS" == *"Ubuntu"* ]] || [[ "$OS" == *"Debian"* ]]; then
    echo -e "${GREEN}✓${NC} Supported distribution: $OS"
    PKG_MANAGER="apt"
elif [[ "$OS" == *"CentOS"* ]] || [[ "$OS" == *"Red Hat"* ]]; then
    echo -e "${GREEN}✓${NC} Supported distribution: $OS"
    PKG_MANAGER="yum"
elif [[ "$OS" == *"Alpine"* ]]; then
    echo -e "${GREEN}✓${NC} Supported distribution: $OS"
    PKG_MANAGER="apk"
else
    echo -e "${YELLOW}!${NC} Untested distribution: $OS"
    echo "Attempting with apt (assuming Debian-based)..."
    PKG_MANAGER="apt"
fi

echo ""

# Dependency Check
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Dependency Check${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# Check git
if command -v git &> /dev/null; then
    GIT_VER=$(git --version | awk '{print $3}')
    echo -e "${GREEN}✓${NC} Git: $GIT_VER"
else
    echo -e "${YELLOW}✗${NC} Git: NOT INSTALLED"
    echo "  Installing..."
    if [[ "$PKG_MANAGER" == "apt" ]]; then
        sudo apt update && sudo apt install -y git
    elif [[ "$PKG_MANAGER" == "yum" ]]; then
        sudo yum install -y git
    elif [[ "$PKG_MANAGER" == "apk" ]]; then
        sudo apk add git
    fi
    echo -e "${GREEN}✓${NC} Git installed"
fi

# Check Rust
if command -v rustc &> /dev/null; then
    RUST_VER=$(rustc --version | awk '{print $2}')
    echo -e "${GREEN}✓${NC} Rust: $RUST_VER"
else
    echo -e "${YELLOW}✗${NC} Rust: NOT INSTALLED"
    echo "  Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓${NC} Rust installed"
fi

# Check cargo
if command -v cargo &> /dev/null; then
    CARGO_VER=$(cargo --version | awk '{print $2}')
    echo -e "${GREEN}✓${NC} Cargo: $CARGO_VER"
else
    echo -e "${RED}✗${NC} Cargo not found (should be with Rust)"
    exit 1
fi

# Check cloudflared
if command -v cloudflared &> /dev/null; then
    CF_VER=$(cloudflared --version | head -1)
    echo -e "${GREEN}✓${NC} Cloudflared: installed"
else
    echo -e "${YELLOW}✗${NC} Cloudflared: NOT INSTALLED"
    echo "  Installing..."
    if [[ "$PKG_MANAGER" == "apt" ]]; then
        sudo mkdir -p --mode=0755 /usr/share/keyrings
        curl -fsSL https://pkg.cloudflare.com/cloudflare-public-v2.gpg | sudo tee /usr/share/keyrings/cloudflare-public-v2.gpg >/dev/null
        echo "deb [signed-by=/usr/share/keyrings/cloudflare-public-v2.gpg] https://pkg.cloudflare.com/cloudflared any main" | sudo tee /etc/apt/sources.list.d/cloudflared.list
        sudo apt update && sudo apt install -y cloudflared
    elif [[ "$PKG_MANAGER" == "yum" ]]; then
        sudo yum install -y cloudflared
    elif [[ "$PKG_MANAGER" == "apk" ]]; then
        sudo apk add cloudflared
    fi
    echo -e "${GREEN}✓${NC} Cloudflared installed"
fi

# Check build-essential
if [[ "$PKG_MANAGER" == "apt" ]]; then
    if dpkg -l | grep -q build-essential; then
        echo -e "${GREEN}✓${NC} Build tools: installed"
    else
        echo -e "${YELLOW}✗${NC} Build tools: NOT INSTALLED"
        echo "  Installing..."
        sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
        echo -e "${GREEN}✓${NC} Build tools installed"
    fi
fi

echo ""

# Build Project
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Building cf-cdn${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ ! -f Cargo.toml ]; then
    echo -e "${RED}✗${NC} Cargo.toml not found. Are you in the cf-cdn directory?"
    exit 1
fi

echo "Running: cargo build --release"
echo "(This may take 2-5 minutes on first build)"
echo ""

if cargo build --release; then
    echo ""
    echo -e "${GREEN}✓${NC} Build successful!"
else
    echo -e "${RED}✗${NC} Build failed!"
    exit 1
fi

echo ""

# Configuration
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Configuration${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ ! -f config.toml ]; then
    echo "Creating config.toml..."
    cp config.example.toml config.toml
    echo -e "${GREEN}✓${NC} Config created: config.toml"
    echo ""
    echo "Edit your configuration:"
    echo "  nano config.toml"
else
    echo -e "${GREEN}✓${NC} Config already exists: config.toml"
fi

echo ""

# Summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Installation Complete!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo ""
echo -e "${GREEN}Next steps:${NC}"
echo ""
echo "1. Configure (optional):"
echo "   nano config.toml"
echo ""
echo "2. Run the server:"
echo "   RUST_LOG=info ./target/release/cf-cdn"
echo ""
echo "3. In another terminal, check system info:"
echo "   ./target/release/info"
echo ""
echo "4. Check cloudflared:"
echo "   ./target/release/tunnel-cli check"
echo ""
echo "5. Enable Cloudflare tunnel (edit config.toml):"
echo "   [tunnel]"
echo "   enabled = true"
echo ""

echo -e "${BLUE}Documentation:${NC}"
echo "  README.md       - Full documentation"
echo "  ARCHITECTURE.md - Design details"
echo "  config.toml     - Your configuration"
echo ""

echo -e "${GREEN}Ready to go! 🚀${NC}"
