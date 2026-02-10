# Deployment Guide

## Prerequisites

- VPS with 2+ CPU, 4GB+ RAM (London region recommended for Polymarket)
- GitHub repository secrets configured (see below)
- Dugout identity set up locally

## Secrets Management with Dugout

edgelord uses [dugout](https://crates.io/crates/dugout) for git-native secrets management. Secrets are encrypted at rest and injected at runtime - no plaintext `.env` files on disk.

### Local Setup (One-Time)

```bash
# Install dugout
cargo install dugout

# Initialize your identity (if you haven't already)
dugout setup

# Initialize dugout in the project
dugout init

# Add your secrets
dugout set WALLET_PRIVATE_KEY      # Your trading wallet private key
dugout set TELEGRAM_BOT_TOKEN      # Telegram bot token (optional)
dugout set TELEGRAM_CHAT_ID        # Telegram chat ID (optional)
dugout set ANTHROPIC_API_KEY       # For LLM inference (optional)
dugout set OPENAI_API_KEY          # For LLM inference (optional)

# Commit the encrypted vault
git add .dugout.toml
git commit -m "feat: add encrypted secrets vault"
git push
```

### VPS Setup (One-Time)

```bash
# Install dugout on VPS
cargo install dugout

# Option A: Copy your local identity to VPS
scp ~/.dugout/identity user@vps:~/.dugout/identity

# Option B: Generate new identity on VPS and add as recipient
# (on VPS)
dugout setup
dugout whoami  # Copy this public key

# (locally)
dugout team add vps <vps-public-key>
dugout sync
git add .dugout.toml && git commit -m "chore: add vps as recipient" && git push
```

## VPS Initial Setup

1. Create edgelord user:
   ```bash
   sudo useradd -r -s /bin/false edgelord
   sudo mkdir -p /opt/edgelord/{config,releases}
   sudo chown -R edgelord:edgelord /opt/edgelord
   ```

2. Copy production config:
   ```bash
   sudo cp deploy/config.prod.toml /opt/edgelord/config/config.toml
   sudo chown edgelord:edgelord /opt/edgelord/config/config.toml
   ```

3. Clone repo for dugout vault access:
   ```bash
   cd /opt/edgelord
   sudo -u edgelord git clone https://github.com/usealtoal/edgelord.git repo
   ```

## GitHub Actions Deployment

### Required Secrets

Configure these in your repository settings (Settings → Secrets → Actions):

| Secret | Description |
|--------|-------------|
| `DEPLOY_HOST` | VPS hostname or IP |
| `DEPLOY_SSH_PORT` | SSH port (usually 22) |
| `DEPLOY_USER` | SSH user with sudo access |
| `DEPLOY_SSH_KEY` | Private SSH key (ed25519 recommended) |
| `DEPLOY_KNOWN_HOSTS` | Output of `ssh-keyscan -p PORT HOST` |
| `DEPLOY_PATH` | Deployment path (e.g., `/opt/edgelord`) |

### Workflow Inputs

The deploy workflow supports runtime configuration:

**Strategy Toggles:**
- `strategy_single_condition` - Enable single-condition strategy (default: true)
- `strategy_market_rebalancing` - Enable market-rebalancing strategy (default: true)
- `strategy_combinatorial` - Enable combinatorial strategy (default: false)

**Runtime Overrides:**
- `min_edge` - Minimum edge threshold
- `min_profit` - Minimum profit threshold
- `max_exposure` - Maximum total exposure
- `max_position` - Maximum position per market
- `dry_run` - Detect but don't trade
- `telegram_enabled` - Enable Telegram notifications
- `dugout` - Use dugout for secrets (default: true)

### Deploy

1. Go to Actions → Manual Deploy
2. Select branch/tag
3. Configure strategy toggles and overrides
4. Type `deploy-production` to confirm
5. Run workflow

## Management Commands

After deployment, SSH to VPS and use dugout for commands that need secrets:

```bash
# Start a shell with secrets loaded
dugout env
# Then run commands normally:
edgelord status
edgelord wallet status
edgelord check live --config /opt/edgelord/config/config.toml

# Or run individual commands:
dugout run -- edgelord wallet status --config /opt/edgelord/config/config.toml
```

Commands that don't need secrets work directly:

```bash
# View logs
edgelord logs -f

# View statistics
edgelord statistics today

# Service management
sudo systemctl status edgelord
sudo systemctl restart edgelord
sudo systemctl stop edgelord
```

## Legacy: Manual .env Setup

If not using dugout (not recommended), create a `.env` file:

```bash
sudo tee /opt/edgelord/.env << 'EOF'
WALLET_PRIVATE_KEY=0x...
TELEGRAM_BOT_TOKEN=...
TELEGRAM_CHAT_ID=...
EOF
sudo chmod 600 /opt/edgelord/.env
sudo chown edgelord:edgelord /opt/edgelord/.env
```

Then deploy with `dugout: false` in the workflow inputs.
