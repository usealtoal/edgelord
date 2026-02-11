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

### Market Filter

Controls which markets are tracked based on volume and liquidity thresholds.

```toml
[exchange_config.market_filter]
max_markets = 500              # Maximum markets to track
max_subscriptions = 2000       # Maximum token subscriptions
min_volume_24h = 1000.0        # Minimum 24h volume (USD)
min_liquidity = 500.0          # Minimum liquidity (USD)
max_spread_pct = 0.10          # Maximum bid-ask spread (10%)
include_binary = true          # Include 2-outcome markets
include_multi_outcome = true   # Include multi-outcome markets
max_outcomes = 20              # Maximum outcomes per market
```

CLI overrides: `--max-markets`, `--min-volume`, `--min-liquidity`

### Deduplication

Controls how duplicate WebSocket messages are filtered.

```toml
[exchange_config.dedup]
enabled = true
strategy = "hash"              # Primary: hash | timestamp | content
fallback = "timestamp"         # Fallback if primary fails
cache_ttl_secs = 5             # How long to remember seen messages
max_cache_entries = 100000     # Maximum cache entries
```

## Connection Pool

WebSocket connection management for high-volume subscriptions.

```toml
[connection_pool]
max_connections = 10           # Maximum WebSocket connections
subscriptions_per_connection = 500  # Tokens per connection before rotation
connection_ttl_secs = 120      # Connection lifetime before refresh
preemptive_reconnect_secs = 30 # Start reconnect before TTL expires
health_check_interval_secs = 30  # Check connection health interval
max_silent_secs = 60           # Max seconds without data before reconnect
channel_capacity = 10000       # Event buffer size
```

CLI overrides: `--max-connections`, `--subs-per-connection`, `--connection-ttl`

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

CLI overrides: `--strategies`, `--min-edge`, `--min-profit`

## Risk Management

```toml
[risk]
max_position_per_market = 100.0   # Maximum position size per market (USD)
max_total_exposure = 500.0        # Maximum total exposure (USD)
min_profit_threshold = 0.50       # Minimum profit to execute (USD)
max_slippage = 0.02               # Maximum slippage (0.02 = 2%)
execution_timeout_secs = 30       # Trade execution timeout (seconds)
```

CLI overrides: `--max-position`, `--max-exposure`, `--min-profit`, `--max-slippage`, `--execution-timeout`

## Telegram Integration (Optional)

Telegram support is optional and requires building with the `telegram` feature.

```bash
cargo build --release --features telegram
```

```toml
[telegram]
enabled = true
notify_opportunities = false      # Send opportunity alerts (noisy)
notify_executions = true          # Send execution alerts
notify_risk_rejections = true     # Send risk rejection alerts
stats_interval_secs = 30          # Stats polling interval
position_display_limit = 10       # Max positions shown in /positions
```

CLI overrides: `--telegram-enabled`, `--stats-interval`

Runtime bot commands are accepted only from `TELEGRAM_CHAT_ID` and include:

- `/status`, `/health`, `/positions`, `/stats`, `/pool`, `/markets`, `/version`
- `/pause`, `/resume`
- `/set_risk <field> <value>` where `field` is `min_profit`, `max_slippage`, `max_position`, or `max_exposure`

## Governor (Adaptive Scaling)

Controls adaptive subscription management based on latency metrics.

```toml
[governor]
enabled = true

[governor.latency]
target_p50_ms = 10             # Target median latency
target_p95_ms = 50             # Target 95th percentile
target_p99_ms = 100            # Target 99th percentile
max_p99_ms = 200               # Maximum acceptable p99

[governor.scaling]
check_interval_secs = 10       # How often to check metrics
expand_threshold = 0.70        # Add subs when utilization below this
contract_threshold = 1.20      # Remove subs when utilization above this
expand_step = 50               # Subscriptions to add per cycle
contract_step = 100            # Subscriptions to remove per cycle
cooldown_secs = 60             # Minimum time between scaling actions
```

## Reconnection

WebSocket reconnection behavior with exponential backoff.

```toml
[reconnection]
initial_delay_ms = 1000        # Initial delay (1 second)
max_delay_ms = 60000           # Maximum delay (60 seconds)
backoff_multiplier = 2.0       # Double delay each failure
max_consecutive_failures = 10  # Trip circuit breaker after failures
circuit_breaker_cooldown_ms = 300000  # 5 minute cooldown
```

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
max_clusters_per_cycle = 10
channel_capacity = 1000
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

## CLI Override Summary

All config values can be overridden via CLI flags. Here's a quick reference:

| Config Setting | CLI Flag |
|----------------|----------|
| `exchange_config.chain_id` | `--chain-id`, `--mainnet`, `--testnet` |
| `exchange_config.market_filter.max_markets` | `--max-markets` |
| `exchange_config.market_filter.min_volume_24h` | `--min-volume` |
| `exchange_config.market_filter.min_liquidity` | `--min-liquidity` |
| `connection_pool.max_connections` | `--max-connections` |
| `connection_pool.subscriptions_per_connection` | `--subs-per-connection` |
| `connection_pool.connection_ttl_secs` | `--connection-ttl` |
| `risk.max_slippage` | `--max-slippage` |
| `risk.execution_timeout_secs` | `--execution-timeout` |
| `risk.max_position_per_market` | `--max-position` |
| `risk.max_total_exposure` | `--max-exposure` |
| `telegram.stats_interval_secs` | `--stats-interval` |
| `database` | `--database` |

## Validation Workflow

```bash
./target/release/edgelord check config --config config.toml
./target/release/edgelord config show --config config.toml
./target/release/edgelord check live --config config.toml
```

For full command options, see [CLI Reference](cli-reference.md).
