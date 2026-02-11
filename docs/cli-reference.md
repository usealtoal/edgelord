# CLI Reference

Use `edgelord --help` for full generated help.

## Running Commands with Secrets

Commands that require secrets (wallet key, API keys) should be run via dugout:

```bash
# Run a single command with secrets
dugout run -- edgelord <command>

# Or spawn a shell with secrets loaded
dugout env
edgelord <command>
```

Commands that **need secrets**: `run`, `check connection`, `check live`, `check telegram`, `wallet *`, `provision *`

Commands that **don't need secrets**: `status`, `statistics *`, `logs`, `config *`, `service *`

## Core Commands

### `run`

Run the detector/executor in foreground mode.

```bash
dugout run -- edgelord run --config config.toml
```

#### Environment Shortcuts

Quick environment switching without editing config:

```bash
# Use mainnet (sets chain_id=137, environment=mainnet)
dugout run -- edgelord run --mainnet

# Use testnet (sets chain_id=80002, environment=testnet)
dugout run -- edgelord run --testnet
```

#### All Run Flags

| Flag | Description | Example |
|------|-------------|---------|
| `-c, --config` | Path to config file | `--config config.toml` |
| `--chain-id` | Override chain ID | `--chain-id 137` |
| `--mainnet` | Shortcut for mainnet (chain_id=137) | `--mainnet` |
| `--testnet` | Shortcut for testnet (chain_id=80002) | `--testnet` |
| `--log-level` | Override log level | `--log-level debug` |
| `--dry-run` | Detect but don't execute | `--dry-run` |
| `--no-banner` | Skip ASCII art banner | `--no-banner` |
| `--json-logs` | Use JSON log format | `--json-logs` |
| `--strategies` | Comma-separated strategies | `--strategies "single_condition"` |
| `--min-edge` | Override minimum edge | `--min-edge 0.05` |
| `--min-profit` | Override minimum profit | `--min-profit 0.50` |
| `--max-exposure` | Override max total exposure | `--max-exposure 5000` |
| `--max-position` | Override max position per market | `--max-position 500` |
| `--max-slippage` | Override max slippage (0.02=2%) | `--max-slippage 0.03` |
| `--telegram-enabled` | Enable Telegram notifications | `--telegram-enabled` |
| `--max-markets` | Max markets to track | `--max-markets 100` |
| `--min-volume` | Min 24h volume filter (USD) | `--min-volume 5000` |
| `--min-liquidity` | Min liquidity filter (USD) | `--min-liquidity 1000` |
| `--max-connections` | Max WebSocket connections | `--max-connections 5` |
| `--subs-per-connection` | Subscriptions per connection | `--subs-per-connection 250` |
| `--connection-ttl` | Connection TTL in seconds | `--connection-ttl 60` |
| `--execution-timeout` | Execution timeout in seconds | `--execution-timeout 60` |
| `--stats-interval` | Stats update interval in seconds | `--stats-interval 60` |
| `--database` | Path to SQLite database | `--database /var/lib/edgelord/data.db` |

#### Example Invocations

```bash
# Quick mainnet dry-run with custom filters
dugout run -- edgelord run --mainnet --dry-run --max-markets 100 --min-volume 5000

# Production with conservative settings
dugout run -- edgelord run \
  --mainnet \
  --no-banner \
  --json-logs \
  --max-exposure 5000 \
  --max-position 500 \
  --max-slippage 0.02

# Development with verbose logging
dugout run -- edgelord run --testnet --log-level debug --dry-run

# Custom connection pool settings
dugout run -- edgelord run --max-connections 5 --subs-per-connection 250 --connection-ttl 60
```

### `status`

Show current status from database-backed state.

```bash
edgelord status --db edgelord.db
```

### `statistics`

Query and export historical stats.

```bash
edgelord statistics today --db edgelord.db
edgelord statistics week --db edgelord.db
edgelord statistics history 30 --db edgelord.db
edgelord statistics export --days 30 --output stats.csv --db edgelord.db
edgelord statistics prune --days 30 --db edgelord.db
```

## Configuration Commands

```bash
edgelord config init config.toml
edgelord config show --config config.toml
edgelord config validate --config config.toml
```

## Diagnostics

```bash
# Config validation (no secrets needed)
edgelord check config --config config.toml

# These require secrets
dugout run -- edgelord check live --config config.toml
dugout run -- edgelord check connection --config config.toml
dugout run -- edgelord check telegram --config config.toml
```

`check telegram` validates delivery only. Interactive bot commands are documented in `docs/deployment/telegram.md`.

## Provisioning

Provision exchange-specific wallet/config defaults (requires secrets):

```bash
dugout run -- edgelord provision polymarket --config config.toml
dugout run -- edgelord provision polymarket --wallet import --config config.toml
```

## Wallet Commands

All wallet commands require secrets:

```bash
dugout run -- edgelord wallet address --config config.toml
dugout run -- edgelord wallet status --config config.toml
dugout run -- edgelord wallet approve --config config.toml --amount 1000 --yes
dugout run -- edgelord wallet sweep --config config.toml --to 0x... --asset usdc --network polygon --yes
```

## Service Management

Install with dugout for secrets injection (recommended):

```bash
sudo edgelord service install \
  --config /opt/edgelord/config.toml \
  --user edgelord \
  --working-dir /opt/edgelord \
  --dugout
```

### Service Install Flags

| Flag | Description | Example |
|------|-------------|---------|
| `--config` | Path to config file | `--config /opt/edgelord/config.toml` |
| `--user` | User to run service as | `--user edgelord` |
| `--working-dir` | Working directory | `--working-dir /opt/edgelord` |
| `--dugout` | Use dugout for secrets | `--dugout` |
| `--strategies` | Comma-separated strategies | `--strategies "single_condition"` |
| `--min-edge` | Minimum edge threshold | `--min-edge 0.05` |
| `--min-profit` | Minimum profit threshold | `--min-profit 0.50` |
| `--max-exposure` | Maximum total exposure | `--max-exposure 5000` |
| `--max-position` | Maximum position per market | `--max-position 500` |
| `--max-slippage` | Maximum slippage tolerance | `--max-slippage 0.02` |
| `--max-markets` | Maximum markets to track | `--max-markets 100` |
| `--max-connections` | Maximum WebSocket connections | `--max-connections 5` |
| `--execution-timeout` | Execution timeout in seconds | `--execution-timeout 60` |
| `--dry-run` | Enable dry run mode | `--dry-run` |
| `--telegram-enabled` | Enable Telegram | `--telegram-enabled` |

Example with overrides baked into systemd unit:

```bash
sudo edgelord service install \
  --config /opt/edgelord/config.toml \
  --user edgelord \
  --dugout \
  --max-exposure 5000 \
  --max-slippage 0.02 \
  --telegram-enabled
```

Uninstall:

```bash
sudo edgelord service uninstall
```

## Logs

```bash
edgelord logs --lines 100
edgelord logs --follow
edgelord logs --since "1 hour ago"
```

## Flag Priority

When the same setting is specified in multiple places, CLI flags take precedence:

1. Built-in defaults (lowest)
2. Config file (`config.toml`)
3. CLI flags (highest)

For example, if `config.toml` sets `max_slippage = 0.02` but you run with `--max-slippage 0.05`, the effective value is `0.05`.
