# Operations

This runbook focuses on operating edgelord safely in production.

## Routine Commands

Commands that don't need secrets:

```bash
systemctl status edgelord
journalctl -u edgelord -f
edgelord status --db /opt/edgelord/data/edgelord.db --config /opt/edgelord/config/config.toml
edgelord statistics today --db /opt/edgelord/data/edgelord.db
```

Commands that need secrets (use dugout):

```bash
# Option A: Spawn shell with secrets loaded
dugout env
edgelord wallet status
edgelord check health --config /opt/edgelord/config/config.toml
edgelord check live --config /opt/edgelord/config/config.toml

# Option B: Run individual commands
dugout run -- edgelord wallet status
dugout run -- edgelord check health --config /opt/edgelord/config/config.toml
dugout run -- edgelord check connection --config /opt/edgelord/config/config.toml
```

## Incident Triage Order

1. Confirm process health and restart state.
2. Confirm exchange/API connectivity.
3. Confirm wallet availability and gas/capital balances.
4. Confirm risk gates are not intentionally blocking execution.

## Hardening Checklist

- Service runs as non-root user
- Secrets managed via dugout (no plaintext `.env` files)
- Dugout identity file restricted (`chmod 600 ~/.dugout/identity`)
- SSH is key-based and hardened
- Host firewall is enabled
- Dependency and OS patch cadence is defined
- Backups exist for non-secret operational config

## Change Management

Before each deployment:

1. `cargo fmt --all -- --check`
2. `cargo test`
3. `check config`, `check health`, and `check live` against production config
4. Restart service and validate logs/status

## Recommended Rollout Pattern

- Stage in `dry_run` first.
- Observe behavior and metrics.
- Enable execution with low exposure caps.
- Increase limits only after stable operation windows.
