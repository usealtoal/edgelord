# Deployment Guide

Production deployment for running edgelord against live Polymarket.

## Prerequisites

### Regulatory Note

Polymarket restricts access from the United States. The web interface geo-blocks US IPs, though the underlying smart contracts on Polygon are permissionless. US persons should consult legal counsel regarding regulatory compliance before trading. This guide is technical documentation, not legal advice.

### What You Need

1. **Crypto wallet** — Ethereum-compatible (MetaMask, hardware wallet, or raw private key)
2. **USDC on Polygon** — Polymarket uses USDC on Polygon network
3. **VPS outside US** — For API access and low latency
4. **Domain (optional)** — For monitoring dashboards

## Infrastructure

### VPS Selection

Key factors:
- **Location**: Europe (Frankfurt, Amsterdam) for low latency to Polymarket
- **Provider**: Must not block crypto/trading traffic
- **Specs**: 2 vCPU, 4GB RAM minimum; more for multiple strategies

Recommended providers:

| Provider | Location | Notes |
|----------|----------|-------|
| [Hetzner](https://www.hetzner.com/cloud) | Germany, Finland | Best price/performance, crypto-friendly |
| [Vultr](https://www.vultr.com/) | Amsterdam, Frankfurt | Good API, easy setup |
| [OVH](https://www.ovhcloud.com/) | France, Germany | Cheap, reliable |
| [DigitalOcean](https://www.digitalocean.com/) | Amsterdam, Frankfurt | Developer-friendly |

**Avoid**: AWS, GCP, Azure in US regions. Some have strict ToS around trading.

### Recommended Specs

| Use Case | CPU | RAM | Storage |
|----------|-----|-----|---------|
| Testing | 2 vCPU | 2 GB | 20 GB SSD |
| Production (single strategy) | 2 vCPU | 4 GB | 40 GB SSD |
| Production (all strategies) | 4 vCPU | 8 GB | 40 GB SSD |

### Example: Hetzner Setup

```bash
# Create server via Hetzner Cloud Console or CLI
# - Location: Falkenstein (fsn1) or Helsinki (hel1)
# - Image: Ubuntu 24.04
# - Type: CPX21 (3 vCPU, 4GB RAM, €7.50/mo)

# SSH in
ssh root@<server-ip>

# Create non-root user
adduser edgelord
usermod -aG sudo edgelord
su - edgelord
```

## Server Setup

### 1. System Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build essentials
sudo apt install -y build-essential pkg-config libssl-dev curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Clone and Build

```bash
# Clone repository
git clone https://github.com/usealtoal/edgelord.git
cd edgelord

# Build release binary
cargo build --release

# Verify
./target/release/edgelord --help
```

### 3. Configuration

```bash
# Create config directory
mkdir -p ~/.config/edgelord

# Copy example config
cp config.toml.example ~/.config/edgelord/config.toml

# Edit configuration
nano ~/.config/edgelord/config.toml
```

Key configuration for production:

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

### 4. Environment Variables

Create environment file:

```bash
nano ~/.config/edgelord/.env
```

```bash
# Wallet private key (without 0x prefix)
POLYMARKET_PRIVATE_KEY=your_private_key_here

# Optional: Telegram alerts
TELEGRAM_BOT_TOKEN=123456:ABC-DEF...
TELEGRAM_CHAT_ID=your_chat_id
```

Secure permissions:

```bash
chmod 600 ~/.config/edgelord/.env
chmod 600 ~/.config/edgelord/config.toml
```

### 5. Systemd Service

Create service file:

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

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable edgelord
sudo systemctl start edgelord

# Check status
sudo systemctl status edgelord

# View logs
journalctl -u edgelord -f
```

## Wallet Setup

### Creating a Wallet

For production, use a dedicated wallet — not your main holdings.

**Option A: Generate new wallet**
```bash
# Using cast (from foundry)
cast wallet new

# Save the private key securely
```

**Option B: Use existing wallet**
- Export private key from MetaMask or hardware wallet
- Use a fresh address for trading operations

### Funding the Wallet

1. **Get MATIC for gas** — Small amount (~5 MATIC) for transaction fees
2. **Get USDC on Polygon** — Your trading capital

Funding paths:
- Bridge from Ethereum mainnet via [Polygon Bridge](https://wallet.polygon.technology/bridge)
- Buy directly on Polygon via exchange that supports Polygon withdrawals
- Use [Jumper](https://jumper.exchange/) or [Bungee](https://bungee.exchange/) for cross-chain

### Polymarket Approval

Before trading, approve Polymarket contracts to spend your USDC:

```bash
# The bot handles this automatically on first run, or manually:
./target/release/edgelord approve --amount 1000
```

## Verification

### Test Connectivity

```bash
# Check WebSocket connection
./target/release/edgelord check-connection

# Dry run (no actual trades)
./target/release/edgelord run --dry-run
```

### Monitor Initial Operation

```bash
# Watch logs
journalctl -u edgelord -f

# Check status file
./target/release/edgelord status
```

Expected startup:
1. Connects to Polymarket WebSocket
2. Fetches active markets
3. Subscribes to order book updates
4. Begins scanning for opportunities

## Monitoring

### Log Management

```bash
# Configure log rotation
sudo nano /etc/logrotate.d/edgelord
```

```
/var/log/edgelord/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
}
```

### Health Checks

Simple health check script:

```bash
#!/bin/bash
# /home/edgelord/health-check.sh

if ! systemctl is-active --quiet edgelord; then
    echo "Edgelord is down!" | telegram-send --stdin
    sudo systemctl restart edgelord
fi
```

Add to crontab:
```bash
*/5 * * * * /home/edgelord/health-check.sh
```

### Telegram Alerts

The bot sends alerts for opportunities detected, trades executed, and errors.

#### Step 1: Create a Telegram Bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot`
3. Choose a name: `Edgelord Alerts` (display name, can have spaces)
4. Choose a username: `edgelord_alerts_bot` (must end in `bot`, be unique)
5. BotFather replies with your **bot token**:
   ```
   Use this token to access the HTTP API:
   7123456789:AAHxxxxxxxxxxxxxxxxxxxxxxxxxxx
   ```
   Save this — it's your `TELEGRAM_BOT_TOKEN`

#### Step 2: Set Bot Profile Picture

1. Still in BotFather chat, send `/setuserpic`
2. Select your bot from the list
3. Send an image (square recommended, 512x512px works well)
4. BotFather confirms: "Success! Userpic for 'Edgelord Alerts' has been updated."

Optional bot customization:
```
/setdescription  — Short description shown when someone opens the bot
/setabouttext    — Longer about text
/setcommands     — Set command menu (not needed for alert-only bot)
```

#### Step 3: Get Your Chat ID

The bot needs to know where to send messages. Get your personal chat ID:

1. Start a chat with your new bot (search for it, click Start)
2. Send any message to the bot (e.g., "hello")
3. Open this URL in your browser (replace TOKEN with your bot token):
   ```
   https://api.telegram.org/bot<TOKEN>/getUpdates
   ```
4. Find `"chat":{"id":` in the response — that number is your chat ID:
   ```json
   "chat": {
     "id": 123456789,
     "first_name": "Your Name",
     "type": "private"
   }
   ```

**For a group chat**: Add the bot to the group, send a message in the group, then check `getUpdates`. Group chat IDs are negative numbers (e.g., `-987654321`).

#### Step 4: Configure Environment

Add to your `.env` file:

```bash
TELEGRAM_BOT_TOKEN=7123456789:AAHxxxxxxxxxxxxxxxxxxxxxxxxxxx
TELEGRAM_CHAT_ID=123456789
```

#### Step 5: Configure Alerts

In `config.toml`:

```toml
[notifications.telegram]
enabled = true
min_profit_alert = 1.00  # Only alert for profits > $1
alert_on_error = true    # Alert on errors/restarts
alert_on_opportunity = true  # Alert when opportunity detected
alert_on_execution = true    # Alert when trade executes
```

#### Step 6: Test

```bash
# Test that alerts work
./target/release/edgelord test-telegram
```

You should receive a test message in Telegram.

#### Example Alert Messages

**Opportunity detected:**
```
🔍 Opportunity Found
Market: Will Bitcoin hit $100k?
Strategy: single_condition
Edge: 3.2% ($4.80)
Volume: 150 shares
```

**Trade executed:**
```
✅ Trade Executed
Market: Will Bitcoin hit $100k?
Profit: $4.52
Legs: 2/2 filled
```

**Error:**
```
⚠️ Error
WebSocket disconnected, reconnecting...
```

## Security Checklist

- [ ] Non-root user for running the bot
- [ ] Private key in environment file with `600` permissions
- [ ] Firewall enabled (UFW), only SSH open
- [ ] SSH key authentication only (disable password)
- [ ] Fail2ban installed
- [ ] Separate wallet for trading (not your main holdings)
- [ ] Start with small position limits

```bash
# Basic firewall setup
sudo ufw allow OpenSSH
sudo ufw enable

# Disable password authentication
sudo nano /etc/ssh/sshd_config
# Set: PasswordAuthentication no
sudo systemctl restart sshd
```

## Troubleshooting

### Connection Issues

```
Error: WebSocket connection failed
```
- Check if VPS can reach Polymarket: `curl -I https://clob.polymarket.com`
- Verify you're not on a US IP: `curl ifconfig.me`
- Check firewall isn't blocking outbound

### Wallet Issues

```
Error: Insufficient balance
```
- Check USDC balance on Polygon
- Check MATIC balance for gas
- Verify correct network (Polygon mainnet, not Mumbai testnet)

### Performance Issues

```
Warning: Detection latency > 100ms
```
- Check server load: `htop`
- Consider upgrading VPS specs
- Check network latency: `ping clob.polymarket.com`

## Updates

```bash
# Pull latest code
cd ~/edgelord
git pull

# Rebuild
cargo build --release

# Restart service
sudo systemctl restart edgelord
```

## Quick Reference

| Command | Description |
|---------|-------------|
| `systemctl status edgelord` | Check service status |
| `journalctl -u edgelord -f` | Follow logs |
| `./target/release/edgelord status` | Show bot status |
| `systemctl restart edgelord` | Restart bot |
| `systemctl stop edgelord` | Stop bot |
