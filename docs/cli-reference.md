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

Useful flags:

- `--dry-run`
- `--chain-id <id>`
- `--strategies "single_condition,market_rebalancing"`
- `--max-exposure <decimal>`
- `--no-banner`
- `--json-logs`

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

Additional runtime options for `service install`:

- `--strategies <list>` - Comma-separated strategies
- `--min-edge <decimal>` - Minimum edge threshold
- `--min-profit <decimal>` - Minimum profit threshold
- `--max-exposure <decimal>` - Maximum total exposure
- `--max-position <decimal>` - Maximum position per market
- `--dry-run` - Enable dry run mode
- `--telegram-enabled` - Enable Telegram notifications

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
