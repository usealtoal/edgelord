# Telegram Integration

Telegram alerts provide lightweight execution and risk visibility.

Build with Telegram support enabled:

```bash
cargo build --release --features telegram
```

## Required Environment Variables

- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

## Config Toggle

```toml
[telegram]
enabled = true
notify_opportunities = false
notify_executions = true
notify_risk_rejections = true
```

## Connectivity Check

```bash
./target/release/edgelord check telegram --config config.toml
```

## Bot Commands

When Telegram is enabled in config and `TELEGRAM_BOT_TOKEN`/`TELEGRAM_CHAT_ID` are set, the bot also accepts commands from the configured `TELEGRAM_CHAT_ID`:

- `/start` and `/help`
- `/status`
- `/health`
- `/positions`
- `/pause`
- `/resume`
- `/set_risk <field> <value>`

Supported `set_risk` fields:

- `min_profit`
- `max_slippage` (0 to 1)
- `max_position`
- `max_exposure`

Runtime risk updates apply immediately and are process-local (they do not rewrite `config.toml`).

## Recommended Alert Policy

- Keep opportunity alerts disabled initially to reduce noise.
- Keep execution and risk rejection alerts enabled in production.
- Route critical alerts to a shared on-call channel if operating as a team.
