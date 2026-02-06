# Getting Started

This guide covers installation, configuration, and running edgelord.

## Prerequisites

- **Rust 1.75+** — Install via [rustup](https://rustup.rs/)
- **A Polymarket account** — For API access
- **Private key** — For signing transactions (testnet recommended for initial setup)

## Installation

```bash
git clone https://github.com/usealtoal/edgelord.git
cd edgelord
cargo build --release
```

The binary is at `./target/release/edgelord`.

## Provisioning (Recommended)

Provisioning creates an encrypted keystore and updates your config automatically.

```bash
export EDGELORD_KEYSTORE_PASSWORD="..."
./target/release/edgelord provision polymarket --config config.polymarket.toml
```

To import an existing private key into a keystore:

```bash
export EDGELORD_PRIVATE_KEY="0x..."
export EDGELORD_KEYSTORE_PASSWORD="..."
./target/release/edgelord provision polymarket --wallet import --config config.polymarket.toml
```

By default, the keystore is written to:

```
~/.config/edgelord/exchanges/polymarket/keystore.json
```

Use `--keystore-path` to override. For headless setups, you can provide the passphrase via `EDGELORD_KEYSTORE_PASSWORD_FILE`.

## Configuration

Copy the example config:

```bash
cp config.toml.example config.polymarket.toml
```

Key sections:

```toml
# Exchange selection
exchange = "polymarket"

[exchange_config]
environment = "testnet"  # Start with testnet
chain_id = 80002         # Amoy testnet (use 137 for mainnet)

# Which strategies to run
[strategies]
enabled = ["single_condition", "market_rebalancing"]

# Risk limits
[risk]
max_position_per_market = 100   # Start small
max_total_exposure = 500
```

See [Configuration](configuration.md) for all options.

## Environment Variables

If using a keystore (provisioned):

```bash
export EDGELORD_KEYSTORE_PASSWORD="..."              # or EDGELORD_KEYSTORE_PASSWORD_FILE
```

If using a raw private key (manual setup):

```bash
export WALLET_PRIVATE_KEY="0x..."
```

When importing a key into a keystore:

```bash
export EDGELORD_PRIVATE_KEY="0x..."
```

Optional for Telegram alerts (requires `--features telegram`):

```bash
export TELEGRAM_BOT_TOKEN="..."
export TELEGRAM_CHAT_ID="..."
```

## Running

Interactive mode with colored output:

```bash
./target/release/edgelord run --config config.polymarket.toml
```

Production mode with JSON logs:

```bash
./target/release/edgelord run --config config.polymarket.toml --no-banner --json-logs
```

Check status:

```bash
./target/release/edgelord status
```

Check mainnet readiness:

```bash
./target/release/edgelord check live --config config.polymarket.toml
```

## Verifying It Works

On startup, you should see:
1. Connection to WebSocket established
2. Markets being fetched and filtered
3. Order books populating
4. Strategies scanning (opportunities logged when found)

If using testnet, no real funds are at risk. Monitor the logs for a few minutes to confirm detection is working before considering mainnet.

## Next Steps

- Read [Architecture](architecture/overview.md) to understand the system
- Read [Strategies](strategies/overview.md) to understand detection algorithms
- Tune [Configuration](configuration.md) for your risk tolerance
