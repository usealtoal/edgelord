# Getting Started

This guide gets edgelord running in a safe baseline configuration.

## 1. Prerequisites

- Rust 1.75+
- [dugout](https://crates.io/crates/dugout) for secrets management (recommended)
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

## 3. Initialize Config

Recommended (interactive wizard):

```bash
./target/release/edgelord init config.toml
```

The wizard configures:
- Network (`testnet`/`mainnet`)
- Enabled strategies (`single_condition`, `market_rebalancing`, `combinatorial`)
- Risk limits

Non-interactive (template only):

```bash
./target/release/edgelord config init config.toml
```

If you need to overwrite, add `--force`.

## 4. Set Up Secrets with Dugout

edgelord works best with dugout so secrets are encrypted at rest and injected at runtime.

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

# Optional: Add LLM credentials for combinatorial inference
dugout set ANTHROPIC_API_KEY
dugout set OPENAI_API_KEY

# Commit the encrypted vault
git add .dugout.toml
git commit -m "feat: add encrypted secrets vault"
```

## 5. Provision Wallet (Keystore Alternative)

If you prefer keystore-based wallet handling instead of `WALLET_PRIVATE_KEY`:

Generate new keystore:

```bash
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --wallet generate --config config.toml
```

Import an existing private key:

```bash
export EDGELORD_PRIVATE_KEY="0x..."
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --wallet import --config config.toml
```

## 6. Validate Configuration and Readiness

```bash
dugout run -- ./target/release/edgelord check config --config config.toml
dugout run -- ./target/release/edgelord check health --config config.toml
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
# Run on testnet
dugout run -- ./target/release/edgelord run --testnet

# Run on mainnet (shortcut for chain_id=137)
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

## 8. Observe and Inspect

```bash
./target/release/edgelord status --db edgelord.db --config config.toml
./target/release/edgelord statistics today --db edgelord.db
./target/release/edgelord statistics week --db edgelord.db
```

If running under systemd, use:

```bash
journalctl -u edgelord -f
```

## Moving to Mainnet

Before switching from testnet to mainnet:

1. Keep `dry_run = true` until repeated stable dry-run sessions are clean.
2. Switch `[exchange_config] environment = "mainnet"` and `chain_id = 137`.
3. Re-run `check health` and `check live` and confirm no blockers.
4. Start with conservative `risk` limits.

For infrastructure and operations details, continue to [Deployment](deployment/README.md).
