# Operations

This runbook focuses on operating edgelord safely in production.

## Routine Commands

```bash
systemctl status edgelord
journalctl -u edgelord -f
./target/release/edgelord status --db edgelord.db
./target/release/edgelord statistics today --db edgelord.db
```

## Incident Triage Order

1. Confirm process health and restart state.
2. Confirm exchange/API connectivity.
3. Confirm wallet availability and gas/capital balances.
4. Confirm risk gates are not intentionally blocking execution.

## Hardening Checklist

- Service runs as non-root user
- Secret files are restricted (`chmod 600`)
- SSH is key-based and hardened
- Host firewall is enabled
- Dependency and OS patch cadence is defined
- Backups exist for non-secret operational config

## Change Management

Before each deployment:

1. `cargo fmt --all -- --check`
2. `cargo test`
3. `check config` and `check live` against production config
4. Restart service and validate logs/status

## Recommended Rollout Pattern

- Stage in `dry_run` first.
- Observe behavior and metrics.
- Enable execution with low exposure caps.
- Increase limits only after stable operation windows.
