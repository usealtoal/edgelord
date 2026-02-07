# Wallet Setup

Edgelord requires an EVM wallet for Polygon-based execution paths.

## Recommended Wallet Model

- Use a dedicated trading wallet.
- Keep only operational capital in that wallet.
- Keep long-term holdings in separate cold/warm custody.

## Funding Requirements

- USDC for position capital
- MATIC for gas

## Provisioning Flow (Preferred)

```bash
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --config config.toml
```

Import existing private key into keystore:

```bash
export EDGELORD_PRIVATE_KEY="0x..."
export EDGELORD_KEYSTORE_PASSWORD="change-me"
./target/release/edgelord provision polymarket --wallet import --config config.toml
```

## Verification

```bash
./target/release/edgelord wallet address --config config.toml
./target/release/edgelord wallet status --config config.toml
```

## Capital Controls

Keep risk limits conservative during rollout:

```toml
[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
```

Increase gradually only after stable runs and operational confidence.
