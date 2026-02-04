# Configuration

All configuration lives in `config.toml`. Settings can be overridden via CLI flags or environment variables.

## Priority Order

1. Built-in defaults
2. Config file (`config.toml`)
3. CLI flags
4. Environment variables (secrets only)

## Exchange

```toml
exchange = "polymarket"

[polymarket]
environment = "testnet"        # "testnet" or "mainnet"
chain_id = 80002               # 80002 (Amoy testnet) or 137 (Polygon mainnet)
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
api_url = "https://clob.polymarket.com"
```

## Strategies

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing", "combinatorial"]

[strategies.single_condition]
min_edge = 0.05      # Minimum profit margin (5%)
min_profit = 0.50    # Minimum dollar profit per trade

[strategies.market_rebalancing]
min_edge = 0.03      # Minimum profit margin (3%)
min_profit = 1.00    # Minimum dollar profit per trade
max_outcomes = 10    # Skip markets with more outcomes

[strategies.combinatorial]
enabled = true            # Must explicitly enable
max_iterations = 20       # Frank-Wolfe iterations
tolerance = 0.0001        # Convergence threshold
gap_threshold = 0.02      # Minimum gap to act on (2%)
```

## LLM Provider

Configure the LLM for relation inference:

```toml
[llm]
provider = "anthropic"    # "anthropic" or "openai"

[llm.anthropic]
model = "claude-3-5-sonnet-20241022"
temperature = 0.2         # Low for consistent JSON
max_tokens = 4096

[llm.openai]
model = "gpt-4-turbo"
temperature = 0.2
max_tokens = 4096
```

**Environment variables (required):**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
export OPENAI_API_KEY="sk-..."
```

## Relation Inference

Control how the LLM discovers market relations:

```toml
[inference]
enabled = true            # Enable LLM inference at startup
min_confidence = 0.7      # Filter relations below this confidence
ttl_seconds = 3600        # How long relations are valid (1 hour)
price_change_threshold = 0.05  # Re-infer on 5% price change
scan_interval_seconds = 3600   # Full re-scan interval
batch_size = 30           # Markets per LLM call
```

## Cluster Detection

Real-time detection on related market clusters:

```toml
[cluster_detection]
enabled = true            # Enable cluster detection service
debounce_ms = 100         # Minimum interval between detection runs
min_gap = 0.02            # Minimum arbitrage gap to report (2%)
max_clusters_per_cycle = 50    # Max clusters per detection cycle
channel_capacity = 1024   # Order book update channel size
```

## Risk Management

```toml
[risk]
max_position_per_market = 1000   # Max exposure per market ($)
max_total_exposure = 10000       # Max total portfolio exposure ($)
min_profit_threshold = 0.05      # Skip opportunities below this ($)
max_slippage = 0.02              # Reject if slippage exceeds 2%
```

## Reconnection

```toml
[reconnection]
initial_delay_ms = 1000          # First retry delay
max_delay_ms = 60000             # Cap on exponential backoff
backoff_multiplier = 2.0         # Delay doubles each failure
max_consecutive_failures = 10    # Trips circuit breaker
circuit_breaker_cooldown_ms = 300000  # 5 min cooldown
```

## Governor (Adaptive Subscription)

```toml
[governor]
enabled = true

[governor.latency]
target_p50_ms = 10
target_p95_ms = 50
target_p99_ms = 100
max_p99_ms = 200

[governor.scaling]
check_interval_secs = 10
expand_threshold = 0.70
contract_threshold = 1.20
expand_step = 50
contract_step = 100
cooldown_secs = 60
```

## Telegram Notifications

Requires building with `--features telegram`:

```toml
[telegram]
enabled = true
notify_opportunities = false     # Alert on detection (noisy!)
notify_executions = true         # Alert on trades
notify_risk_rejections = true    # Alert on rejected opportunities
```

**Environment variables:**

```bash
export TELEGRAM_BOT_TOKEN="..."
export TELEGRAM_CHAT_ID="..."
```

## Logging

```toml
[logging]
level = "info"      # "debug", "info", "warn", "error"
format = "pretty"   # "pretty" or "json"
```

## Operational

```toml
dry_run = false                   # Detect but don't execute
status_file = "/var/run/edgelord/status.json"  # Optional status file
```

## CLI Overrides

Common flags:

```bash
edgelord run --chain-id 137           # Override chain
edgelord run --max-exposure 5000      # Override risk limit
edgelord run --no-banner --json-logs  # Production mode
edgelord run --dry-run                # Detection only
```

Run `edgelord run --help` for all options.

## Complete Example

See `config.toml.example` for a complete annotated configuration file.
