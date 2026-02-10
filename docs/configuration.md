# Configuration Reference

Primary configuration file: `config.toml`.

Start from `config.toml.example` and override only what is required for your environment.

## Configuration Priority

1. Built-in defaults
2. `config.toml`
3. CLI flags
4. Environment variables (typically for secrets)

## Core Top-Level Settings

```toml
profile = "local"      # local | production | custom
dry_run = true
database = "edgelord.db"
exchange = "polymarket"
```

## Exchange Configuration

```toml
[exchange_config]
type = "polymarket"
environment = "testnet"      # testnet | mainnet
chain_id = 80002              # 80002 testnet, 137 mainnet
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"
```

Key nested groups:

- `[exchange_config.http]`: REST timeout + retry policy
- `[exchange_config.connections]`: connection lifecycle and fanout
- `[exchange_config.market_filter]`: market universe and quality thresholds
- `[exchange_config.scoring.*]`: subscription prioritization heuristics
- `[exchange_config.dedup]`: market-event dedup strategy

## Strategies

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing"]

[strategies.single_condition]
min_edge = 0.05
min_profit = 0.50

[strategies.market_rebalancing]
min_edge = 0.03
min_profit = 1.00
max_outcomes = 10

[strategies.combinatorial]
enabled = false
max_iterations = 20
tolerance = 0.0001
gap_threshold = 0.02
```

## Risk Management

```toml
[risk]
max_position_per_market = 100.0
max_total_exposure = 500.0
min_profit_threshold = 0.50
max_slippage = 0.02
```

## Telegram Integration (Optional)

Telegram support is optional and requires building with the `telegram` feature.

```bash
cargo build --release --features telegram
```

```toml
[telegram]
enabled = true
notify_opportunities = false
notify_executions = true
notify_risk_rejections = true
```

Runtime bot commands are accepted only from `TELEGRAM_CHAT_ID` and include:

- `/status`, `/health`, `/positions`
- `/pause`, `/resume`
- `/set_risk <field> <value>` where `field` is `min_profit`, `max_slippage`, `max_position`, or `max_exposure`

## Optional Inference and Cluster Detection

Enable when using combinatorial relation-based detection.

```toml
[inference]
enabled = false
min_confidence = 0.7
ttl_seconds = 3600
batch_size = 30

[cluster_detection]
enabled = false
debounce_ms = 100
min_gap = 0.02
```

## Secrets and Environment Variables

Do not commit secrets to `config.toml`. Use [dugout](https://crates.io/crates/dugout) for secrets management.

### Dugout Setup (Recommended)

```bash
# Initialize dugout in the project
dugout init

# Add secrets interactively
dugout set WALLET_PRIVATE_KEY
dugout set TELEGRAM_BOT_TOKEN
dugout set TELEGRAM_CHAT_ID

# Commit encrypted vault
git add .dugout.toml
git commit -m "feat: add encrypted secrets vault"
```

Run commands with secrets injected:

```bash
dugout run -- edgelord run --config config.toml
```

Or spawn a shell with secrets loaded:

```bash
dugout env
edgelord run --config config.toml
```

### Required Secrets

| Variable | Description | Required |
|----------|-------------|----------|
| `WALLET_PRIVATE_KEY` | Trading wallet private key | Yes |
| `TELEGRAM_BOT_TOKEN` | Telegram bot token | If telegram enabled |
| `TELEGRAM_CHAT_ID` | Telegram chat ID | If telegram enabled |
| `ANTHROPIC_API_KEY` | Anthropic API key | If using LLM inference |
| `OPENAI_API_KEY` | OpenAI API key | If using LLM inference |

### Legacy Variables

These are used by the provisioning system:

- `EDGELORD_PRIVATE_KEY`
- `EDGELORD_KEYSTORE_PASSWORD`
- `EDGELORD_KEYSTORE_PASSWORD_FILE`

## Validation Workflow

```bash
./target/release/edgelord check config --config config.toml
./target/release/edgelord check live --config config.toml
```

For full command options, see [CLI Reference](cli-reference.md).
