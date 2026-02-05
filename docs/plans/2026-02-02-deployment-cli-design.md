# Deployment & CLI Design

> Status: Historical
> Superseded by: N/A
> Summary:
> - Scope: Global Flags
> Planned Outcomes:
> - Global Flags
> - `run` Subcommand Flags


## Overview

Operational infrastructure for running edgelord locally and on a European VPS, with a polished CLI interface and GitHub Actions deployment pipeline.

## CLI Structure

```
edgelord
├── run        # Foreground mode (interactive, ASCII art, live logs)
├── status     # Quick health check (running? exposure? today's stats)
├── logs       # Tail logs (wrapper around journalctl)
├── install    # Install systemd unit file
└── uninstall  # Remove systemd unit file
```

### Global Flags

```
--config <PATH>      Config file path [default: config.toml]
--chain-id <ID>      Override chain ID (80002=testnet, 137=mainnet)
--log-level <LEVEL>  Override log level (debug, info, warn, error)
--dry-run            Detect opportunities but don't execute
```

### `run` Subcommand Flags

```
edgelord run [FLAGS]
  --no-banner        Skip ASCII art (for systemd/non-interactive)
  --json-logs        JSON format instead of pretty (for production)

  # Strategy overrides
  --strategies <LIST>      e.g., "single_condition,market_rebalancing"
  --min-edge <DECIMAL>     Override minimum edge
  --min-profit <DECIMAL>   Override minimum profit

  # Risk overrides
  --max-exposure <DECIMAL>     Max total exposure
  --max-position <DECIMAL>     Max per-market position

  # Notifications
  --telegram-enabled           Enable Telegram notifications
```

## Configuration Layering

```
CLI flags  →  override  →  Config file  →  override  →  Defaults
    ↑
Env vars for secrets (WALLET_PRIVATE_KEY, TELEGRAM_BOT_TOKEN)
```

## Project Structure

```
edgelord/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── cli/
│   │   ├── mod.rs           # CLI arg definitions
│   │   ├── banner.rs        # ASCII art banner
│   │   ├── run.rs           # `run` command handler
│   │   ├── status.rs        # `status` command handler
│   │   ├── logs.rs          # `logs` command handler
│   │   └── service.rs       # `install`/`uninstall` handlers
│   └── ...
│
├── deploy/
│   ├── edgelord.service.tmpl  # systemd unit template
│   └── config.prod.toml       # Production config example
│
├── .github/
│   └── workflows/
│       └── deploy.yml       # Build → Test → Deploy to VPS
│
├── config.toml              # Local dev config (testnet)
└── Cargo.toml
```

## Interactive Mode (`edgelord run`)

Displays colored ASCII banner on startup (terminal-aware), streams live logs, graceful Ctrl+C shutdown.

```
     ▄▄▄▄▄▄▄▄▄
   ▄█▒█▒█▒█▒█▒█▄            __________  ______________    ____  ____  ____
  █▒█▒█▒█▒█▒█▒█▒█          / ____/ __ \/ ____/ ____/ /   / __ \/ __ \/ __ \
  █▄▄▄▄▄▄▄▄▄▄▄▄▄█         / __/ / / / / / __/ __/ / /   / / / / /_/ / / / /
  █░░░●░░░░░●░░░█        / /___/ /_/ / /_/ / /___/ /___/ /_/ / _, _/ /_/ /
  █░░░░░░░░░░░░░█       /_____/_____/\____/_____/_____/\____/_/ |_/_____/
   █░░░░▄▄░░░░░█
    ▀█▄▄▄▄▄▄▄█▀          "This aggression will not stand, man."
     ▀█▀   ▀█▀

[2024-02-03 12:34:56] INFO  Connected to Polymarket WebSocket
[2024-02-03 12:34:57] INFO  Subscribed to 142 tokens (71 markets)
```

## `status` Command

```
$ edgelord status

edgelord v0.1.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status:      ● running (pid 12345)
Uptime:      3d 14h 22m
Network:     mainnet (chain 137)
Strategies:  single_condition, market_rebalancing

Positions:   2 open
Exposure:    $847.50 / $10,000 max
Today:       5 opportunities, 3 executed, $12.40 profit
```

## `logs` Command

Wrapper around journalctl:

```bash
$ edgelord logs              # tail -f style
$ edgelord logs --lines 100  # last 100 lines
$ edgelord logs --since "1 hour ago"
```

## `install` / `uninstall` Commands

**`edgelord install`:**
```bash
$ sudo edgelord install --config /opt/edgelord/config.toml

✓ Created /etc/systemd/system/edgelord.service
✓ Reloaded systemd daemon
✓ Enabled edgelord service (starts on boot)

Start with: sudo systemctl start edgelord
```

**Generated systemd service file:**
```ini
[Unit]
Description=Edgelord Arbitrage Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=edgelord
Group=edgelord
WorkingDirectory=/opt/edgelord
ExecStart=/opt/edgelord/edgelord run --no-banner --json-logs --config /opt/edgelord/config.toml
Restart=on-failure
RestartSec=5
EnvironmentFile=/opt/edgelord/.env

[Install]
WantedBy=multi-user.target
```

**`edgelord uninstall`:**
```bash
$ sudo edgelord uninstall

✓ Stopped edgelord service
✓ Disabled edgelord service
✓ Removed /etc/systemd/system/edgelord.service
✓ Reloaded systemd daemon
```

## GitHub Actions Deployment

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]
    paths-ignore: ['docs/**', '*.md']
  workflow_dispatch:

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - Build release binary
      - Run test suite
      - Upload binary as artifact

  deploy:
    needs: build-and-test
    runs-on: ubuntu-latest
    steps:
      - Download binary artifact
      - SCP binary to VPS
      - SSH: stop service, replace binary, start service
```

**GitHub Secrets:**
- `VPS_HOST` — VPS IP address
- `VPS_USER` — SSH user (e.g., edgelord)
- `VPS_SSH_KEY` — Private key for SSH
- `WALLET_PRIVATE_KEY`
- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

**VPS directory structure:**
```
/opt/edgelord/
├── edgelord              # Binary
├── config.toml           # Production config
└── .env                  # Secrets
```

## New Dependencies

```toml
clap = { version = "4", features = ["derive"] }
```
