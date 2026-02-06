# Exchange Provisioning CLI (Polymarket-first)

> Status: Proposed
> Summary:
> - Add a `provision` CLI flow that makes Polymarket mainnet setup fast and safe.
> - Use encrypted keystores with headless passphrase support (env/file).
> - Scale to other exchanges via per-exchange provisioners and configs.

**Date:** 2026-02-06
**Status:** Proposed
**Author:** Claude + Ryan

## Overview

Mainnet validation is the next milestone. To remove setup friction and reduce operator error, add a provisioning flow that handles wallet setup, config wiring, and exchange-specific funding instructions. The flow must be headless-friendly (VPS automation) while keeping keys secure by default. The design is Polymarket-first but structured to support account-based exchanges (Kalshi) without rewriting the CLI.

## Goals

1. One command to provision a Polymarket VPS environment end-to-end.
2. Secure by default: no plaintext private keys on disk.
3. Headless automation: non-interactive, repeatable, idempotent.
4. Exchange-specific behavior with a shared CLI contract.
5. Clear post-provision instructions for funding and verification.

## Non-goals

- Multi-cloud/IaC automation (handled outside the CLI).
- Managed custody or hardware wallets.
- A GUI or web-based provisioning flow.

## User Experience

Primary command:

```
edgelord provision polymarket --config config.polymarket.toml
```

Wallet modes:

- `--wallet generate` creates a new encrypted keystore.
- `--wallet import` imports a private key into an encrypted keystore.

Headless secrets:

- `EDGELORD_KEYSTORE_PASSWORD` or `EDGELORD_KEYSTORE_PASSWORD_FILE`.
- `EDGELORD_PRIVATE_KEY` only used with `--wallet import`.

Outputs:

- Keystore path
- Wallet address
- Funding instructions (token + network)
- Next steps (run, check live)
- Optional machine-readable summary via `--json`

## Configuration Strategy

Use **separate config files per exchange** to avoid cross-contamination:

- `config.polymarket.toml`
- `config.kalshi.toml`

Provisioning updates only the config passed in, plus exchange-scoped secret paths:

- `~/.config/edgelord/exchanges/polymarket/keystore.json`
- `~/.config/edgelord/exchanges/kalshi/credentials.json`

This allows switching by config path and supports parallel operation if desired.

## Architecture

### High-level flow

```
preflight -> wallet setup -> config update -> funding instructions -> optional service install
```

### Components

- `ProvisionerRegistry` routes `edgelord provision <exchange>` to the correct adapter.
- `ExchangeProvisioner` trait defines a common contract:
  - `preflight()`
  - `configure()`
  - `funding_instructions()`
  - `install_service()`
- `SecretsStore` abstraction handles keystore creation and secure import.

### Polymarket provisioning

- Keystore: Ethereum JSON keystore (V3) encrypted with passphrase.
- Passphrase supplied via env or file.
- Config is updated with:
  - `exchange = "polymarket"`
  - chain id / environment
  - keystore path (new config field)
- Funding instructions explicitly call out:
  - Token: USDC
  - Network: Polygon
  - Warning about wrong-chain transfers

### Kalshi provisioning (future)

- No wallet step; account credentials only.
- Funding instructions point to Kalshi deposit methods.

## CLI Additions

### Provisioning

```
edgelord provision polymarket [--wallet generate|import] [--keystore-path PATH] [--config PATH]
```

### Wallet support

```
edgelord wallet address --config PATH
edgelord wallet sweep --to <address> --asset usdc --network polygon --config PATH
```

### Live readiness

```
edgelord check live --config PATH
```

This is **exchange-specific**:
- Polymarket: mainnet chain id, wallet configured, approvals set, dry_run off.
- Kalshi: production credentials present and demo env not selected.

## Error Handling

- Missing keystore passphrase: hard fail with actionable message.
- Attempt to overwrite existing keystore: require `--force`.
- Invalid chain/network mismatch: fail fast before writing config.
- Missing funding info: warn and print manual steps.

## Testing

- Unit tests for provisioning config writes.
- Unit tests for keystore create/import with passphrase env.
- CLI tests for `provision polymarket --json` and `check live`.
- Integration test (mock): provision -> config validate -> run dry-run.

## Open Questions

- Should keystore passphrase be stored in an OS keychain if available?
- Do we want `edgelord provision polymarket --install-service` in v1, or follow-up?

## Success Criteria

- A new VPS can be fully provisioned for Polymarket in under 5 minutes.
- No plaintext private keys are written to disk.
- `edgelord check live` surfaces all blockers for a mainnet run.
