# Infrastructure

This guide covers VPS setup for edgelord deployment.

## Recommended Baseline

- 2-4 vCPU
- 4-8 GB RAM
- SSD storage
- Stable outbound network performance
- Region: London for Polymarket (lowest latency)

## Automated Deployment (Recommended)

Use GitHub Actions for deployment. See [DigitalOcean Runbook](digitalocean-manual.md) for the full workflow.

VPS only needs Rust and dugout installed. The workflow handles binary, config, and vault syncing.

## VPS Bootstrap

```console
$ sudo apt update && sudo apt upgrade -y
$ sudo apt install -y build-essential pkg-config libssl-dev curl
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source ~/.cargo/env
$ cargo install dugout
```

## Dugout Identity Setup

Copy your local identity to the VPS:

```console
$ scp ~/.dugout/identity user@vps:~/.dugout/identity
```

Or generate a new identity on the VPS:

```console
$ dugout setup
$ dugout whoami    # Add this key as recipient locally
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

The deploy workflow installs and updates the `edgelord` systemd unit.

```console
$ sudo systemctl status edgelord
$ sudo systemctl restart edgelord
$ journalctl -u edgelord -f
```

## Validation

Before going live:

```console
$ cd /opt/edgelord
$ edgelord check config --config config/config.toml
$ edgelord check health --config config/config.toml
$ dugout run -- edgelord check connection --config config/config.toml
$ dugout run -- edgelord check live --config config/config.toml
```
