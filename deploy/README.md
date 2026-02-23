# Deployment Guide

## Prerequisites

- VPS with 2+ CPU, 4GB+ RAM (London region recommended for Polymarket)
- GitHub repository secrets configured (see below)
- Dugout identity set up locally

## Secrets Management with Dugout

edgelord uses [dugout](https://crates.io/crates/dugout) for git-native secrets management. Secrets are encrypted at rest and injected at runtime - no plaintext `.env` files on disk.

### Local Setup (One-Time)

```console
$ cargo install dugout
$ dugout setup
$ dugout init
$ dugout set WALLET_PRIVATE_KEY
$ dugout set TELEGRAM_BOT_TOKEN    # Optional
$ dugout set TELEGRAM_CHAT_ID      # Optional
$ git add .dugout.toml && git commit -m "chore: add secrets vault" && git push
```

### VPS Setup (One-Time)

```console
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source ~/.cargo/env
$ cargo install dugout
```

Copy your local identity to VPS:

```console
$ scp ~/.dugout/identity user@vps:~/.dugout/identity
```

Or generate a new identity and add as recipient:

```console
$ dugout setup && dugout whoami    # Copy public key, then locally:
                                   # dugout team add vps <key> && dugout sync && git push
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

```console
$ ssh your-vps
$ cd /opt/edgelord
```

Commands that need secrets:

```console
$ dugout run -- edgelord wallet status --config config/config.toml
$ dugout run -- edgelord check health --config config/config.toml
$ dugout run -- edgelord check live --config config/config.toml
```

Commands that don't need secrets:

```console
$ edgelord status --db data/edgelord.db
$ edgelord statistics today --db data/edgelord.db
$ sudo systemctl status edgelord
$ journalctl -u edgelord -f
```

## Updating Secrets

```console
$ dugout set NEW_SECRET_KEY
$ git add .dugout.toml && git commit -m "chore: update secrets" && git push
```

Then run any deploy—the workflow syncs the vault automatically.
