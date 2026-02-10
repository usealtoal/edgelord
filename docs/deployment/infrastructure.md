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

# Install dugout for secrets management
cargo install dugout
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

## Secrets Setup

Set up dugout identity on the VPS:

```bash
# Option A: Copy your local identity
scp ~/.dugout/identity user@vps:~/.dugout/identity

# Option B: Generate new identity and add as recipient
dugout setup
dugout whoami  # Share this key with team to be added as recipient
```

Clone the repo to access the encrypted vault:

```bash
cd /opt/edgelord
git clone https://github.com/usealtoal/edgelord.git repo
```

## Systemd Service

Install from CLI with dugout integration:

```bash
sudo ./target/release/edgelord service install \
  --config /opt/edgelord/config.toml \
  --user edgelord \
  --working-dir /opt/edgelord \
  --dugout
```

This generates a systemd unit that runs:
```
ExecStart=dugout run -- /opt/edgelord/current/edgelord run --config ...
```

For legacy `.env` file approach (not recommended):

```bash
sudo ./target/release/edgelord service install \
  --config /opt/edgelord/config.toml \
  --user edgelord \
  --working-dir /opt/edgelord
```

Uninstall:

```bash
sudo ./target/release/edgelord service uninstall
```

## Deployment Validation

Before marking a host live:

1. `check config` passes
2. `check connection` passes
3. `check live` passes for intended mode
4. Logs and status commands behave as expected
