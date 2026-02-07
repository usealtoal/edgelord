# Infrastructure

This guide covers VPS setup and service deployment.

## Recommended Baseline

- 2-4 vCPU
- 4-8 GB RAM
- SSD storage
- Stable outbound network performance
- Region aligned with your latency and compliance requirements

## Host Setup

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev curl git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Build and Configure

```bash
git clone https://github.com/usealtoal/edgelord.git
cd edgelord
cargo build --release
cp config.toml.example config.toml
```

Validate:

```bash
./target/release/edgelord check config --config config.toml
./target/release/edgelord check connection --config config.toml
```

## Systemd Service

Install from CLI:

```bash
./target/release/edgelord service install \
  --config /opt/edgelord/config.toml \
  --user edgelord \
  --working-dir /opt/edgelord
```

Uninstall:

```bash
./target/release/edgelord service uninstall
```

## Deployment Validation

Before marking a host live:

1. `check config` passes
2. `check connection` passes
3. `check live` passes for intended mode
4. Logs and status commands behave as expected
