# Wallet Setup

Edgelord requires an EVM wallet for Polygon-based execution paths.

## Recommended Wallet Model

- Use a dedicated trading wallet.
- Keep only operational capital in that wallet.
- Keep long-term holdings in separate cold/warm custody.

## Funding Requirements

- USDC for position capital
- MATIC for gas

## Secrets with Dugout (Recommended)

Store your wallet private key securely with dugout:

```bash
dugout init                      # Initialize if not already done
dugout set WALLET_PRIVATE_KEY    # Enter your private key securely
git add .dugout.toml && git commit -m "chore: add wallet secret"
```

## Provisioning Flow (Alternative)

If using keystore-based approach:

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
# With dugout
dugout run -- edgelord wallet address --config config.toml
dugout run -- edgelord wallet status --config config.toml

# Or in a dugout shell
dugout env
edgelord wallet address --config config.toml
edgelord wallet status --config config.toml
```

## Capital Controls

Keep risk limits conservative during rollout:

```toml
[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
```

Increase gradually only after stable runs and operational confidence.
