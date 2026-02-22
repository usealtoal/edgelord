# CLI Reference

Use `edgelord --help` for full generated help.

## Global Flags

Available on every command:

| Flag | Description |
|------|-------------|
| `--color <auto|always|never>` | Force color behavior |
| `--json` | Structured machine-readable output |
| `-q, --quiet` | Suppress regular human output |
| `-v, --verbose` | Increase verbosity (`-v`, `-vv`, `-vvv`) |

## Running Commands with Secrets

Commands that need wallet/API secrets should be run via dugout:

```bash
# Single command with secrets
dugout run -- edgelord <command>

# Or open a shell with secrets loaded
dugout env
edgelord <command>
```

Commands that typically need secrets:
- `run`
- `check connection`
- `check live`
- `check telegram`
- `wallet *`
- `provision polymarket`

Commands that typically do not need secrets:
- `init`
- `config *`
- `check config`
- `check health`
- `status`
- `statistics *`
- `strategies *`

## Core Commands

### `init`

Interactive setup wizard:

```bash
edgelord init config.toml
```

For scripted setup use:

```bash
edgelord config init config.toml
```

### `run`

Run the detector/executor in foreground mode:

```bash
dugout run -- edgelord run --config config.toml
```

#### Important run flags

| Flag | Description | Example |
|------|-------------|---------|
| `-c, --config` | Path to config file | `--config config.toml` |
| `--mainnet` | Shortcut for chain_id=137 | `--mainnet` |
| `--testnet` | Shortcut for chain_id=80002 | `--testnet` |
| `--dry-run` | Detect but do not execute | `--dry-run` |
| `--json-logs` | Use JSON runtime logs | `--json-logs` |
| `--strategies` | Comma-separated strategy keys | `--strategies "single_condition,market_rebalancing"` |
| `--max-exposure` | Override risk max exposure | `--max-exposure 5000` |
| `--max-position` | Override max position per market | `--max-position 500` |
| `--max-slippage` | Override slippage tolerance | `--max-slippage 0.02` |
| `--max-markets` | Override tracked market count | `--max-markets 100` |
| `--max-connections` | Override WS connection cap | `--max-connections 5` |
| `--subs-per-connection` | Override fanout per connection | `--subs-per-connection 250` |
| `--connection-ttl` | Override connection lifetime seconds | `--connection-ttl 60` |
| `--database` | Override sqlite file path | `--database /var/lib/edgelord/edgelord.db` |

### `status`

Show current status from database-backed state:

```bash
edgelord status --db edgelord.db --config config.toml
```

### `statistics`

Query and export historical stats:

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

## Diagnostics (`check`)

```bash
edgelord check config --config config.toml
edgelord check health --config config.toml

dugout run -- edgelord check connection --config config.toml
dugout run -- edgelord check live --config config.toml
dugout run -- edgelord check telegram --config config.toml
```

`check telegram` validates delivery only. Interactive bot commands are documented in `docs/deployment/telegram.md`.

## Strategy Discovery

```bash
edgelord strategies list
edgelord strategies explain single_condition
```

`strategies explain` also accepts hyphen aliases (for example `single-condition`) but canonical keys are snake_case.

## Provisioning

Provision Polymarket wallet/config defaults:

```bash
dugout run -- edgelord provision polymarket --config config.toml
dugout run -- edgelord provision polymarket --wallet import --config config.toml
```

## Wallet Commands

```bash
dugout run -- edgelord wallet address --config config.toml
dugout run -- edgelord wallet status --config config.toml
dugout run -- edgelord wallet approve --config config.toml --amount 1000 --yes
dugout run -- edgelord wallet sweep --config config.toml --to 0x... --asset usdc --network polygon --yes
```

## Output Modes

Human-readable default:

```bash
edgelord status
```

Machine-readable JSON:

```bash
edgelord --json status
edgelord --json statistics today
edgelord --json check health -c config.toml
```

Quiet mode:

```bash
edgelord --quiet run -c config.toml
```
