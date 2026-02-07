# CLI Reference

Use `edgelord --help` for full generated help.

## Core Commands

### `run`

Run the detector/executor in foreground mode.

```bash
edgelord run --config config.toml
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
edgelord check config --config config.toml
edgelord check live --config config.toml
edgelord check connection --config config.toml
edgelord check telegram --config config.toml
```

## Provisioning

Provision exchange-specific wallet/config defaults.

```bash
edgelord provision polymarket --config config.toml
edgelord provision polymarket --wallet import --config config.toml
```

## Wallet Commands

```bash
edgelord wallet address --config config.toml
edgelord wallet status --config config.toml
edgelord wallet approve --config config.toml --amount 1000 --yes
edgelord wallet sweep --config config.toml --to 0x... --asset usdc --network polygon --yes
```

## Service Management

```bash
edgelord service install --config /opt/edgelord/config.toml --user edgelord --working-dir /opt/edgelord
edgelord service uninstall
```

## Logs

```bash
edgelord logs --lines 100
edgelord logs --follow
edgelord logs --since "1 hour ago"
```
