# Telegram Alerts

Telegram alerts provide lightweight execution and risk visibility.

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

## Recommended Alert Policy

- Keep opportunity alerts disabled initially to reduce noise.
- Keep execution and risk rejection alerts enabled in production.
- Route critical alerts to a shared on-call channel if operating as a team.
