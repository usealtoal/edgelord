# Infrastructure Setup

VPS selection, server configuration, and running edgelord as a service.

## VPS Selection

### Requirements

- **Location**: Europe (Frankfurt, Amsterdam) for low latency to Polymarket
- **Provider**: Must not block crypto/trading traffic
- **IP**: Non-US IP address (Polymarket geo-blocks US)

### Recommended Providers

| Provider | Location | Notes |
|----------|----------|-------|
| [Hetzner](https://www.hetzner.com/cloud) | Germany, Finland | Best price/performance, crypto-friendly |
| [Vultr](https://www.vultr.com/) | Amsterdam, Frankfurt | Good API, easy setup |
| [OVH](https://www.ovhcloud.com/) | France, Germany | Cheap, reliable |
| [DigitalOcean](https://www.digitalocean.com/) | Amsterdam, Frankfurt | Developer-friendly |

**Avoid**: AWS, GCP, Azure in US regions.

### Recommended Specs

| Use Case | CPU | RAM | Storage | Cost |
|----------|-----|-----|---------|------|
| Testing | 2 vCPU | 2 GB | 20 GB SSD | ~$5/mo |
| Production | 2 vCPU | 4 GB | 40 GB SSD | ~$8/mo |
| Multi-strategy | 4 vCPU | 8 GB | 40 GB SSD | ~$15/mo |

## Server Setup

### Example: Hetzner

1. Create account at [hetzner.com](https://www.hetzner.com/cloud)
2. New project → Add server
3. Select:
   - Location: Falkenstein (fsn1) or Helsinki (hel1)
   - Image: Ubuntu 24.04
   - Type: CPX21 (3 vCPU, 4GB RAM)
4. Add your SSH key
5. Create server

### Initial Setup

```bash
# SSH in
ssh root@<server-ip>

# Create non-root user
adduser edgelord
usermod -aG sudo edgelord

# Switch to new user
su - edgelord
```

### Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build essentials
sudo apt install -y build-essential pkg-config libssl-dev curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify
rustc --version
```

### Clone and Build

```bash
# Clone repository
git clone https://github.com/usealtoal/edgelord.git
cd edgelord

# Build release binary
cargo build --release

# Verify
./target/release/edgelord --help
```

## Configuration

### Config File

```bash
# Create config directory
mkdir -p ~/.config/edgelord

# Copy example config
cp config.toml.example ~/.config/edgelord/config.toml

# Edit
nano ~/.config/edgelord/config.toml
```

Key settings for production:

```toml
[exchange]
name = "polymarket"
testnet = false  # IMPORTANT: false for real trading

[strategies]
enabled = ["single_condition", "market_rebalancing"]

[risk]
max_position_size = 100.0      # Start small
max_total_exposure = 500.0     # Total across all positions
min_profit_threshold = 0.50    # Minimum $0.50 profit
max_slippage = 0.02            # 2% slippage tolerance

[reconnection]
max_retries = 10
initial_delay_ms = 1000
max_delay_ms = 60000
```

### Environment File

```bash
nano ~/.config/edgelord/.env
```

```bash
# Wallet private key (without 0x prefix)
POLYMARKET_PRIVATE_KEY=your_private_key_here

# Optional: Telegram alerts (see telegram.md)
TELEGRAM_BOT_TOKEN=123456:ABC-DEF...
TELEGRAM_CHAT_ID=your_chat_id
```

Secure permissions:

```bash
chmod 600 ~/.config/edgelord/.env
chmod 600 ~/.config/edgelord/config.toml
```

## Systemd Service

Run edgelord as a background service that starts on boot.

### Create Service File

```bash
sudo nano /etc/systemd/system/edgelord.service
```

```ini
[Unit]
Description=Edgelord Arbitrage Bot
After=network.target

[Service]
Type=simple
User=edgelord
WorkingDirectory=/home/edgelord/edgelord
EnvironmentFile=/home/edgelord/.config/edgelord/.env
ExecStart=/home/edgelord/edgelord/target/release/edgelord run --config /home/edgelord/.config/edgelord/config.toml
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/edgelord/.config/edgelord

[Install]
WantedBy=multi-user.target
```

### Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable edgelord
sudo systemctl start edgelord
```

### Manage Service

```bash
# Check status
sudo systemctl status edgelord

# View logs
journalctl -u edgelord -f

# Restart
sudo systemctl restart edgelord

# Stop
sudo systemctl stop edgelord
```

## Verification

### Test Connectivity

```bash
# Check WebSocket connection
./target/release/edgelord check connection

# Verify non-US IP
curl ifconfig.me

# Test Polymarket API
curl -I https://clob.polymarket.com
```

### Dry Run

```bash
# Run without executing trades
./target/release/edgelord run --dry-run
```

### Expected Startup

1. Connects to Polymarket WebSocket
2. Fetches active markets
3. Subscribes to order book updates
4. Begins scanning for opportunities

Watch logs:
```bash
journalctl -u edgelord -f
```

## Updates

```bash
# Pull latest
cd ~/edgelord
git pull

# Rebuild
cargo build --release

# Restart
sudo systemctl restart edgelord
```
