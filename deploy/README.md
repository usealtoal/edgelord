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
# Install Rust and dugout
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo install dugout

# Option A: Copy your local identity to VPS (simpler)
# Run from your local machine:
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

> **Note:** The deploy workflow automatically syncs the dugout vault and config to the VPS. No need to clone the repo on the VPS.

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

### What Gets Deployed

Each deploy syncs:
- **Binary** - The compiled `edgelord` binary
- **Dugout vault** - `.dugout.toml` (encrypted secrets)
- **Config** - `deploy/config.prod.toml` (only on first deploy, preserves customizations)

### Deploy

1. Go to Actions → Manual Deploy
2. Select branch/tag
3. Configure strategy toggles and overrides
4. Type `deploy-production` to confirm
5. Run workflow

The workflow will:
- Build and test the binary
- Upload binary, vault, and config to VPS
- Install/update the systemd service
- Restart and verify health

## Management Commands

After deployment, SSH to VPS. Change to the deploy directory first:

```bash
cd /opt/edgelord
```

Commands that need secrets (use dugout):

```bash
# Start a shell with secrets loaded
dugout env
edgelord wallet status --config /opt/edgelord/config/config.toml
edgelord check health --config /opt/edgelord/config/config.toml
edgelord check live --config /opt/edgelord/config/config.toml

# Or run individual commands
dugout run -- edgelord wallet status --config /opt/edgelord/config/config.toml
```

Commands that don't need secrets:

```bash
edgelord status --db /opt/edgelord/data/edgelord.db --config /opt/edgelord/config/config.toml
edgelord statistics today --db /opt/edgelord/data/edgelord.db
sudo systemctl status edgelord
sudo systemctl restart edgelord
journalctl -u edgelord -f
```

## Updating Secrets

When you update secrets locally:

```bash
dugout set NEW_SECRET_KEY
git add .dugout.toml && git commit -m "chore: update secrets" && git push
```

Then run a deploy (any mode) - the workflow syncs the vault automatically.

## Legacy: Manual .env Setup

If not using dugout (not recommended), create a `.env` file on the VPS:

```bash
sudo tee /opt/edgelord/.env << 'EOF'
WALLET_PRIVATE_KEY=0x...
TELEGRAM_BOT_TOKEN=...
TELEGRAM_CHAT_ID=...
EOF
sudo chmod 600 /opt/edgelord/.env
```

Then deploy with `dugout: false` in the workflow inputs.
