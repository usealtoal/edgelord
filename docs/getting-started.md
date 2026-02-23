# Getting Started

## Prerequisites

- Rust 1.75+
- [dugout](https://crates.io/crates/dugout) for secrets management
- Polygon USDC + MATIC for live trading

## Installation

```console
$ git clone https://github.com/usealtoal/edgelord.git
$ cd edgelord
$ cargo build --release
```

## Configuration

```console
$ ./target/release/edgelord init config.toml
```

The wizard configures network, strategies, and risk limits. Use `--force` to overwrite.

## Secrets

```console
$ cargo install dugout
$ dugout setup              # First-time identity setup
$ dugout init               # Initialize vault in project

$ dugout set WALLET_PRIVATE_KEY
$ dugout set TELEGRAM_BOT_TOKEN      # Optional
$ dugout set TELEGRAM_CHAT_ID        # Optional
$ dugout set ANTHROPIC_API_KEY       # Optional (combinatorial)
```

## Validation

```console
$ dugout run -- ./target/release/edgelord check config --config config.toml
$ dugout run -- ./target/release/edgelord check health --config config.toml
$ dugout run -- ./target/release/edgelord check live --config config.toml
```

## Running

```console
$ dugout run -- ./target/release/edgelord run --config config.toml
```

### CLI Flags

| Flag | Description |
|------|-------------|
| `--testnet` | Run on Mumbai testnet |
| `--mainnet` | Run on Polygon mainnet |
| `--dry-run` | Detect opportunities without executing |
| `--no-banner` | Suppress startup banner |
| `--json-logs` | Structured JSON logging |
| `--max-exposure N` | Override max total exposure |

### Production Example

```console
$ dugout run -- ./target/release/edgelord run \
    --mainnet \
    --no-banner \
    --json-logs \
    --max-exposure 5000
```

## Monitoring

```console
$ ./target/release/edgelord status --db edgelord.db
$ ./target/release/edgelord statistics today --db edgelord.db
$ ./target/release/edgelord statistics week --db edgelord.db
```

## Mainnet Checklist

1. Run stable dry-run sessions on testnet
2. Set `environment = "mainnet"` and `chain_id = 137`
3. Re-run `check health` and `check live`
4. Start with conservative risk limits
5. Monitor first trades closely

See [Deployment](deployment/README.md) for production operations.
