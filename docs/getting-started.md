# Getting Started

## Prerequisites

- [Rust toolchain](https://rustup.rs/)
- Polygon USDC + MATIC for live trading

## Installation

```console
$ cargo install edgelord
```

## Configuration

```console
$ edgelord init config.toml
```

The wizard configures network, strategies, and risk limits.

## Secrets

**With [dugout](https://crates.io/crates/dugout)** (recommended—encrypted at rest):

```console
$ cargo install dugout
$ dugout setup && dugout init
$ dugout set WALLET_PRIVATE_KEY
$ dugout run -- edgelord run --config config.toml
```

**With environment variables:**

```console
$ export WALLET_PRIVATE_KEY=<your-key>
$ edgelord run --config config.toml
```

Optional secrets: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, `ANTHROPIC_API_KEY`

## Validation

```console
$ edgelord check config --config config.toml
$ edgelord check health --config config.toml
$ edgelord check live --config config.toml    # Needs wallet secret
```

## Running

```console
$ edgelord run --config config.toml
```

### CLI Flags

| Flag | Description |
|------|-------------|
| `--testnet` | Run on Amoy testnet |
| `--mainnet` | Run on Polygon mainnet |
| `--dry-run` | Detect opportunities without executing |
| `--no-banner` | Suppress startup banner |
| `--json-logs` | Structured JSON logging |
| `--max-exposure N` | Override max total exposure |

### Production Example

```console
$ dugout run -- edgelord run \
    --mainnet \
    --no-banner \
    --json-logs \
    --max-exposure 5000
```

## Monitoring

```console
$ edgelord status --db edgelord.db
$ edgelord statistics today --db edgelord.db
$ edgelord statistics week --db edgelord.db
```

## Mainnet Checklist

1. Run stable dry-run sessions on testnet
2. Set `environment = "mainnet"` and `chain_id = 137`
3. Re-run `check health` and `check live`
4. Start with conservative risk limits
5. Monitor first trades closely

See [Deployment](deployment/README.md) for production operations.
