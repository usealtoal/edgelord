# Infrastructure

This guide covers VPS setup for edgelord deployment.

## Recommended Baseline

- 2-4 vCPU
- 4-8 GB RAM
- SSD storage
- Stable outbound network performance
- Region: London for Polymarket (lowest latency)

## Automated Deployment (Recommended)

Use GitHub Actions for deployment. See [Deployment Guide](../../deploy/README.md).

VPS only needs:
1. Rust + dugout installed
2. Dugout identity configured

The workflow handles binary, config, and vault syncing automatically.

## VPS Bootstrap

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install dugout
cargo install dugout
```

## Dugout Identity Setup

```bash
# Option A: Copy your local identity (simpler)
# From your local machine:
scp ~/.dugout/identity user@vps:~/.dugout/identity

# Option B: Generate new identity on VPS
dugout setup
dugout whoami  # Add this key as recipient locally
```

## Runtime Layout

After deployment:

```text
/opt/edgelord/
  current/                # symlink -> releases/<sha>
  releases/<git-sha>/     # deployed releases
  config/config.toml      # production config
  data/edgelord.db        # sqlite database
  .dugout.toml            # encrypted vault
```

## Systemd Service

The deploy workflow installs the service automatically. Manual install:

```bash
sudo edgelord service install \
  --config /opt/edgelord/config/config.toml \
  --user root \
  --working-dir /opt/edgelord \
  --dugout
```

Uninstall:

```bash
sudo edgelord service uninstall
```

## Validation

Before going live:

```bash
cd /opt/edgelord
edgelord check config --config config/config.toml
dugout run -- edgelord check connection --config config/config.toml
dugout run -- edgelord check live --config config/config.toml
```
