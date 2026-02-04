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

[exchange_config]
environment = "testnet"        # "testnet" or "mainnet"
chain_id = 80002               # 80002 (Amoy testnet) or 137 (Polygon mainnet)
```

## Strategies

```toml
[strategies]
enabled = ["single_condition", "market_rebalancing"]

[strategies.single_condition]
min_edge = 0.05      # Minimum profit margin (5%)
min_profit = 0.50    # Minimum dollar profit per trade

[strategies.market_rebalancing]
min_edge = 0.03      # Minimum profit margin (3%)
min_profit = 1.00    # Minimum dollar profit per trade
max_outcomes = 10    # Skip markets with more outcomes

[strategies.combinatorial]
enabled = false           # Requires dependency configuration
max_iterations = 20       # Frank-Wolfe iterations
tolerance = 0.0001        # Convergence threshold
gap_threshold = 0.02      # Minimum gap to act on
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

## Telegram Notifications

Requires building with `--features telegram`:

```toml
[telegram]
enabled = true
notify_opportunities = false     # Alert on detection
notify_executions = true         # Alert on trades
notify_risk_rejections = true    # Alert on rejected opportunities
```

Environment variables (never put in config file):

```bash
export TELEGRAM_BOT_TOKEN="..."
export TELEGRAM_CHAT_ID="..."
```

## CLI Overrides

Common flags:

```bash
edgelord run --chain-id 137           # Override chain
edgelord run --max-exposure 5000      # Override risk limit
edgelord run --no-banner --json-logs  # Production mode
```

Run `edgelord run --help` for all options.
