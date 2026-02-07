# Getting Started

This guide gets edgelord running in a safe baseline configuration.

## 1. Prerequisites

- Rust 1.75+
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

## 4. Provision Wallet (Recommended)

Provisioning creates or imports an encrypted keystore and updates config paths.

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

## 5. Validate Configuration and Connectivity

```bash
./target/release/edgelord check config --config config.toml
./target/release/edgelord check connection --config config.toml
./target/release/edgelord check live --config config.toml
```

## 6. Run

```bash
./target/release/edgelord run --config config.toml
```

Typical production flags:

```bash
./target/release/edgelord run --config config.toml --no-banner --json-logs
```

## 7. Observe and Inspect

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
