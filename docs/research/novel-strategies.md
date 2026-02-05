# Novel Strategy Research

> Prepared for Edgelord — February 2026
> Goal: Identify high-value strategies that fit the existing architecture

---

## Executive Summary

Your current strategies capture the **low-hanging fruit** (single-condition + market rebalancing = ~99.7% of historical $40M). That's good—simple works. But competition will compress these edges over time.

This document proposes **6 novel strategies** ordered by implementation complexity and expected value:

| Strategy | Complexity | Expected Edge | Competition |
|----------|------------|---------------|-------------|
| Deep Book Arbitrage | Low | Medium | Low |
| Cross-Exchange Arbitrage | Medium | High | Medium |
| Temporal Patterns | Medium | Medium | Low |
| Information Edge | High | Very High | High |
| Market Making | High | Medium-High | Medium |
| Dependency Discovery | High | Low-Medium | Very Low |

---

## 1. Deep Book Arbitrage

### The Problem

Current strategies only look at **best bid/ask**. Real order books have depth:

```
Ask side:           |    Bid side:
$0.52 x 50 shares   |    $0.48 x 30 shares
$0.54 x 100 shares  |    $0.46 x 80 shares
$0.55 x 200 shares  |    $0.45 x 150 shares
```

If you need 150 shares, VWAP ≠ $0.52. It's higher.

### The Opportunity

Sometimes the **aggregate** of multiple price levels creates arbitrage that best-ask alone misses:

```
YES side: $0.40 x 20, $0.42 x 50, $0.44 x 100
NO side:  $0.50 x 20, $0.52 x 50, $0.54 x 100

Best asks: $0.40 + $0.50 = $0.90 ✓ (detected)

But if you need 100 shares:
YES VWAP: (20×0.40 + 50×0.42 + 30×0.44) / 100 = $0.422
NO VWAP:  (20×0.50 + 50×0.52 + 30×0.54) / 100 = $0.522
Total: $0.944 — still profitable!
```

### Implementation

```rust
/// Deep book analysis for larger positions
pub struct DeepBookStrategy {
    /// Target position sizes to analyze
    target_volumes: Vec<Decimal>,  // e.g., [50, 100, 200, 500]
    min_edge: Decimal,
}

impl Strategy for DeepBookStrategy {
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        let mut opportunities = vec![];
        
        for target_vol in &self.target_volumes {
            // Calculate VWAP for each outcome at this volume
            let vwaps = ctx.market.outcomes()
                .iter()
                .filter_map(|o| {
                    let book = ctx.cache.get(o.token_id())?;
                    calculate_vwap_ask(&book, *target_vol)
                })
                .collect::<Vec<_>>();
            
            // Check if sum of VWAPs < payout
            let total_vwap: Decimal = vwaps.iter().sum();
            if total_vwap < ctx.market.payout() - self.min_edge {
                opportunities.push(build_deep_opportunity(...));
            }
        }
        opportunities
    }
}
```

### Fits Architecture

- New strategy module: `src/core/strategy/deep_book/`
- Uses existing `OrderBookCache` (already has full depth)
- Config: target volumes, min edge per volume tier

### Expected Value

- **Implementation effort:** 1-2 days
- **Edge:** Captures opportunities others miss by only looking at top-of-book
- **Risk:** Execution slippage if book moves during multi-level fill

---

## 2. Cross-Exchange Arbitrage

### The Problem

Same event, different prices across platforms:

| Platform | "Trump wins 2028" YES |
|----------|----------------------|
| Polymarket | $0.42 |
| Kalshi | $0.45 |
| PredictIt | $0.40 |

### The Opportunity

Buy low on one exchange, sell high on another. Classic arbitrage but with prediction market complications:
- Different settlement rules
- Fee structures vary
- Withdrawal delays
- Position limits (PredictIt: $850)

### Implementation

```rust
pub trait CrossExchangeArbitrage {
    /// Find same events across exchanges
    fn match_events(&self, markets: &[Market]) -> Vec<EventCluster>;
    
    /// Compare prices accounting for fees
    fn find_spreads(&self, cluster: &EventCluster) -> Option<CrossExchangeOpportunity>;
}

pub struct CrossExchangeOpportunity {
    event_id: String,
    buy_exchange: Exchange,
    sell_exchange: Exchange,
    buy_price: Decimal,
    sell_price: Decimal,
    net_edge: Decimal,  // After fees
    max_volume: Decimal, // Limited by smaller side
}
```

### Challenges

1. **Event matching:** "Trump wins" vs "Donald J. Trump elected" — need fuzzy/LLM matching
2. **Fee normalization:** Polymarket 0%, Kalshi 7% on profit, PredictIt 10%
3. **Capital efficiency:** Money locked on multiple exchanges
4. **Execution timing:** Need to hit both sides fast

### Architecture Extension

New exchange implementations:
- `src/core/exchange/kalshi/`
- `src/core/exchange/predictit/`

Cross-exchange coordinator in `app/`:
```rust
pub struct CrossExchangeOrchestrator {
    exchanges: Vec<Box<dyn ExchangeClient>>,
    event_matcher: EventMatcher,
    fee_calculator: FeeCalculator,
}
```

### Expected Value

- **Implementation effort:** 1-2 weeks (need Kalshi/PredictIt clients)
- **Edge:** Significant — these markets often diverge by 3-5%
- **Risk:** Execution risk if you only fill one side

---

## 3. Temporal Patterns

### The Problem

Prediction markets have predictable inefficiencies at certain times:
- **Market open/close:** Less liquidity, wider spreads
- **Weekend effect:** Retail traders more active
- **News cycles:** 9am EST news drops cause overreactions
- **Settlement windows:** Price converges to 0 or 1 near expiry

### The Opportunity

Different strategies work better at different times:

```
Strategy          | Best Time
------------------|------------------
Rebalancing       | High liquidity periods
Single-condition  | After news events
Mean reversion    | Weekend overreactions
```

### Implementation

```rust
pub struct TemporalStrategy {
    /// Time-based strategy selection
    schedule: StrategySchedule,
    
    /// Historical pattern database
    patterns: PatternDatabase,
}

impl TemporalStrategy {
    fn current_regime(&self) -> MarketRegime {
        let now = Utc::now();
        let hour = now.hour();
        let day = now.weekday();
        
        match (day, hour) {
            (Sat | Sun, _) => MarketRegime::WeekendRetail,
            (_, 13..=14) => MarketRegime::USMarketOpen,  // 9-10am EST
            (_, 20..=21) => MarketRegime::USMarketClose,
            _ => MarketRegime::Normal,
        }
    }
    
    fn adjusted_thresholds(&self, regime: MarketRegime) -> Thresholds {
        match regime {
            MarketRegime::WeekendRetail => Thresholds {
                min_edge: dec!(0.02),  // Lower bar, more noise
                max_position: dec!(50), // Smaller size
            },
            MarketRegime::USMarketOpen => Thresholds {
                min_edge: dec!(0.03),
                max_position: dec!(200), // More liquidity
            },
            // ...
        }
    }
}
```

### Data Requirements

Need to build pattern database from historical data:
- Price movements by hour/day
- Spread patterns
- Liquidity cycles

### Expected Value

- **Implementation effort:** 3-5 days
- **Edge:** 10-20% improvement in existing strategy performance
- **Risk:** Patterns may not persist

---

## 4. Information Edge (News-Based)

### The Problem

Markets are slow. News breaks → humans read it → humans trade. This takes 30-60 seconds minimum.

### The Opportunity

Automated news ingestion + LLM interpretation + instant trading:

```
10:00:00.000 — Reuters: "FDA approves Pfizer drug"
10:00:00.050 — Your system: Parse headline
10:00:00.200 — Your system: Identify affected markets
10:00:00.300 — Your system: Place trades
10:00:00.500 — Humans: "Wait, what happened?"
10:00:30.000 — Humans: Start trading
```

### Implementation

```rust
pub struct NewsStrategy {
    /// News feed connections
    feeds: Vec<Box<dyn NewsFeed>>,
    
    /// LLM for interpretation
    llm: LlmClient,
    
    /// Market matcher
    matcher: MarketMatcher,
}

impl NewsStrategy {
    async fn on_news(&self, headline: &str, content: &str) -> Vec<Trade> {
        // 1. Quick relevance check (fast, local)
        if !self.is_market_relevant(headline) {
            return vec![];
        }
        
        // 2. LLM interpretation (50-200ms with good infra)
        let interpretation = self.llm.interpret_news(headline, content).await;
        
        // 3. Match to markets
        let affected_markets = self.matcher.find_markets(&interpretation);
        
        // 4. Determine direction and confidence
        let trades = affected_markets.iter()
            .filter_map(|m| self.calculate_trade(m, &interpretation))
            .collect();
        
        trades
    }
}
```

### News Sources

1. **Twitter/X API** — Fast but noisy
2. **Reuters/Bloomberg** — Authoritative but expensive
3. **Official sources** — Government announcements, SEC filings
4. **Polymarket API** — Watch for new market creation (signals upcoming events)

### Risk Management

This is **not arbitrage** — it's directional betting with information edge. Requires:
- Position limits
- Confidence thresholds
- Correlation management (don't bet same direction on 10 related markets)

### Expected Value

- **Implementation effort:** 2-3 weeks
- **Edge:** Potentially huge (10-50% on individual events)
- **Risk:** Wrong interpretation = loss. LLM hallucinations are dangerous here.

---

## 5. Market Making

### The Problem

Current strategies are **reactive** — wait for mispricing, then trade. You're competing with everyone else watching the same prices.

### The Opportunity

Be the one **creating** the prices. Provide liquidity on both sides, capture spread:

```
Your quotes:
  YES: Bid $0.48, Ask $0.52  (4% spread)
  NO:  Bid $0.46, Ask $0.50

If both sides trade:
  You bought YES at $0.48, sold NO at $0.50
  Net position: +YES, -NO (delta neutral if sizes match)
  Captured: $0.02 per share on each side = $0.04 total
```

### Implementation

```rust
pub struct MarketMaker {
    /// Target spread
    target_spread: Decimal,
    
    /// Inventory limits
    max_inventory: Decimal,
    
    /// Quote update frequency
    quote_interval: Duration,
}

impl MarketMaker {
    fn calculate_quotes(&self, market: &Market, inventory: &Inventory) -> Quotes {
        let mid_price = self.estimate_fair_value(market);
        
        // Skew quotes based on inventory
        let skew = self.inventory_skew(inventory);
        
        Quotes {
            bid: mid_price - self.target_spread/2 + skew,
            ask: mid_price + self.target_spread/2 + skew,
        }
    }
    
    fn inventory_skew(&self, inventory: &Inventory) -> Decimal {
        // If long, lower bid (discourage buying), raise ask (encourage selling)
        // If short, opposite
        let ratio = inventory.position / self.max_inventory;
        ratio * self.skew_factor
    }
}
```

### Challenges

1. **Adverse selection:** Informed traders hit your quotes when they know price is moving
2. **Inventory risk:** Can get stuck with large positions
3. **Competition:** Other market makers with better systems
4. **Capital requirements:** Need funds on both sides

### Architecture

New service: `src/core/service/market_maker/`
- Quote manager
- Inventory tracker
- Risk controls

### Expected Value

- **Implementation effort:** 2-3 weeks
- **Edge:** Consistent small profits, scales with volume
- **Risk:** Significant — can lose money if market moves against inventory

---

## 6. Dependency Discovery (Complete Combinatorial)

### The Problem

Your combinatorial strategy infrastructure is built but needs dependency data. Which markets are logically connected?

### The Opportunity

Use LLMs to discover relationships between markets:

```
Markets:
1. "Trump wins Pennsylvania"
2. "Trump wins nationally"
3. "Republicans control Senate"

Dependencies (LLM discovered):
- 1 IMPLIES contribution to 2
- 1 CORRELATES with 3 (same voter base)
```

### Implementation

The infrastructure exists in `src/core/strategy/combinatorial/`. What's needed:

```rust
// New module: src/core/strategy/combinatorial/discovery.rs

pub struct DependencyDiscovery {
    llm: LlmClient,
    cache: DependencyCache,
}

impl DependencyDiscovery {
    pub async fn discover_dependencies(&self, markets: &[Market]) -> Vec<Dependency> {
        let prompt = self.build_prompt(markets);
        let response = self.llm.complete(&prompt).await?;
        self.parse_dependencies(&response)
    }
    
    fn build_prompt(&self, markets: &[Market]) -> String {
        format!(r#"
Analyze these prediction markets for logical dependencies.

Markets:
{}

For each dependency found, specify:
- Type: implies, mutually_exclusive, or linear_constraint
- Markets involved (by index)
- Confidence (0-1)
- Brief reasoning

Respond in JSON format.
"#, self.format_markets(markets))
    }
}
```

### Integration

Wire into existing Frank-Wolfe:

```rust
impl CombinatorialStrategy {
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // Get dependencies for this market's cluster
        let deps = self.dependency_cache.get_cluster(ctx.market.id())?;
        
        // Build constraint matrix
        let constraints = deps.to_ilp_constraints();
        
        // Run Frank-Wolfe with constraints
        let result = frank_wolfe(&ctx.prices(), &constraints, &self.config);
        
        if result.gap > self.config.gap_threshold {
            vec![self.build_opportunity(result)]
        } else {
            vec![]
        }
    }
}
```

### Expected Value

- **Implementation effort:** 1-2 weeks
- **Edge:** Small (0.24% of historical profits), but interesting
- **Risk:** LLM hallucinations create false dependencies → bad trades

---

## Prioritized Roadmap

### Phase 1: Quick Wins (This Week)
1. **Deep Book Arbitrage** — Extends existing strategies, low risk
2. **Temporal adjustments** — Config-only changes, test different thresholds by time

### Phase 2: Medium Effort (Next 2 Weeks)
3. **Cross-Exchange setup** — Add Kalshi client, start with monitoring only
4. **Dependency Discovery** — Complete combinatorial with LLM integration

### Phase 3: Advanced (Month+)
5. **Information Edge** — Requires news feeds, LLM infra, careful risk management
6. **Market Making** — Significant capital requirements, different risk profile

---

## Architecture Recommendations

### Keep
- ✅ Strategy trait is clean, extensible
- ✅ Domain separation works well
- ✅ Config-driven thresholds

### Add
- **Multi-exchange abstraction** — Factor out Polymarket-specific stuff
- **LLM client interface** — For dependency discovery + news interpretation
- **Time-aware context** — Pass current time/regime to strategies
- **Deep book access** — Expose full order book depth in `DetectionContext`

### Consider
- **Event sourcing** — Log all opportunities (taken or not) for backtesting
- **Paper trading mode** — Execute strategies without real money for validation
- **Metrics/telemetry** — Prometheus/Grafana for monitoring edge capture

---

## Questions for You

1. **Risk appetite:** How much are you willing to lose on directional bets (news strategy)?
2. **Capital allocation:** How much can you deploy per exchange for cross-exchange?
3. **LLM budget:** What's acceptable latency/cost for LLM calls?
4. **Exchange priority:** Kalshi vs PredictIt vs others — which first?
5. **Backtesting:** Do you have historical data, or should we start collecting?

---

## References

- [Polymarket Arbitrage Paper](https://arxiv.org/abs/2508.03474)
- [Combinatorial Market Making](https://arxiv.org/abs/1606.02825)
- [Market Making in Prediction Markets](https://arxiv.org/abs/1911.05883)
- [High-Frequency Trading Strategies](https://www.sciencedirect.com/science/article/pii/S0304405X13002663)
