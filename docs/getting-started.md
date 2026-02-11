# Getting Started

This guide gets edgelord running in a safe baseline configuration.

## 1. Prerequisites

- Rust 1.75+
- [dugout](https://crates.io/crates/dugout) for secrets management
- Access to a Polymarket-compatible wallet setup
- Polygon USDC + MATIC if you intend to trade live

## 2. Build

```bash
git clone https://github.com/usealtoal/edgelord.git
cd edgelord
cargo build --release
```

Binary location:

```text
./target/release/edgelord
```

## 3. Create Config

```bash
cp config.toml.example config.toml
```

Recommended first-run posture in `config.toml`:

```toml
profile = "local"
dry_run = true

[exchange_config]
environment = "testnet"
chain_id = 80002

[strategies]
enabled = ["single_condition", "market_rebalancing"]
```

## 4. Set Up Secrets with Dugout

edgelord uses dugout for secure secrets management. Secrets are encrypted at rest and injected at runtime.

```bash
# Install dugout (if not already installed)
cargo install dugout

# Initialize your identity (first time only)
dugout setup

# Initialize dugout in the project
dugout init

# Add your wallet private key
dugout set WALLET_PRIVATE_KEY

# Optional: Add Telegram credentials
dugout set TELEGRAM_BOT_TOKEN
dugout set TELEGRAM_CHAT_ID

# Commit the encrypted vault
git add .dugout.toml
git commit -m "feat: add encrypted secrets vault"
```

## 5. Provision Wallet (Alternative)

If you prefer the keystore-based approach instead of dugout:

```bash
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --config config.toml
```

Import existing key:

```bash
export EDGELORD_PRIVATE_KEY="0x..."
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --wallet import --config config.toml
```

## 6. Validate Configuration and Connectivity

```bash
dugout run -- ./target/release/edgelord check config --config config.toml
dugout run -- ./target/release/edgelord check connection --config config.toml
dugout run -- ./target/release/edgelord check live --config config.toml
```

## 7. Run

Using dugout (recommended):

```bash
dugout run -- ./target/release/edgelord run --config config.toml
```

Or spawn a shell with secrets loaded:

```bash
dugout env
./target/release/edgelord run --config config.toml
```

### Environment Shortcuts

Quickly switch between testnet and mainnet without editing config:

```bash
# Run on testnet (default)
dugout run -- ./target/release/edgelord run --testnet

# Run on mainnet (sets chain_id=137, environment=mainnet)
dugout run -- ./target/release/edgelord run --mainnet --dry-run
```

### Typical Production Flags

```bash
dugout run -- ./target/release/edgelord run \
  --mainnet \
  --no-banner \
  --json-logs \
  --max-exposure 5000 \
  --max-slippage 0.02
```

### CLI Overrides

Any config setting can be overridden via CLI flags:

```bash
# Custom market filters
dugout run -- ./target/release/edgelord run --max-markets 100 --min-volume 5000

# Connection tuning
dugout run -- ./target/release/edgelord run --max-connections 5 --connection-ttl 60

# Risk limits
dugout run -- ./target/release/edgelord run --max-exposure 5000 --max-position 500
```

See `edgelord run --help` for all available flags.

## 8. Observe and Inspect

```bash
./target/release/edgelord status --db edgelord.db
./target/release/edgelord statistics today --db edgelord.db
./target/release/edgelord logs --follow
```

## Moving to Mainnet

Before switching from testnet to mainnet:

1. Set `dry_run = false` only after repeated stable dry-run sessions.
2. Switch `[exchange_config] environment = "mainnet"` and `chain_id = 137`.
3. Re-run `check live` and confirm no blockers.
4. Start with conservative `risk` limits.

For infrastructure and operations details, continue to [Deployment](deployment/README.md).
