# Phase 3: Execution Implementation Plan

> **Status:** ✅ COMPLETE

**Goal:** Execute arbitrage trades on Polymarket Amoy testnet when opportunities are detected.

**Architecture:** PolymarketExecutor implements OrderExecutor trait, receives Opportunity from detector, builds signed orders using polymarket-client-sdk, submits to CLOB API, and tracks positions in domain. Wire execution into app orchestration.

**Tech Stack:** polymarket-client-sdk v0.4 with `clob` feature, alloy-signer-local for Ethereum signing, rust_decimal for prices.

**Final Structure:**
```
src/
├── app.rs                   # Application orchestration (handles detection → execution flow)
├── domain/
│   └── position.rs          # Position, PositionLeg, PositionTracker (exchange-agnostic)
├── exchange/
│   └── traits.rs            # OrderExecutor trait, ExecutionResult enum
└── polymarket/
    └── executor.rs          # PolymarketExecutor (implements OrderExecutor)
```

---

## Prerequisites

- Phases 1 & 2 complete (WebSocket connection, detection working)
- Amoy testnet wallet with test USDC (get from faucet)
- Private key for signing transactions

---

## Task 1: Add polymarket-client-sdk Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependency to Cargo.toml**

Add under `[dependencies]`:

```toml
# Polymarket CLOB client
polymarket-client-sdk = { version = "0.2", features = ["clob"] }
```

**Step 2: Verify dependency resolves**

Run: `cargo check`
Expected: Compiles without errors (may download new crates)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add polymarket-client-sdk for CLOB trading"
```

---

## Task 2: Add Wallet Configuration

**Files:**
- Modify: `src/config.rs`
- Modify: `config.toml`

**Step 1: Read current config.rs to understand structure**

Use Read tool on `src/config.rs`.

**Step 2: Add wallet section to Config struct**

Add to `Config` struct:

```rust
#[derive(Debug, Deserialize)]
pub struct WalletConfig {
    /// Private key loaded from WALLET_PRIVATE_KEY env var
    #[serde(default)]
    pub private_key: Option<String>,
}
```

And add field to main Config:

```rust
pub wallet: WalletConfig,
```

**Step 3: Update config loading to read private key from env**

The private key should come from `WALLET_PRIVATE_KEY` environment variable, not the config file. Add logic to load it:

```rust
impl WalletConfig {
    pub fn load() -> Self {
        Self {
            private_key: std::env::var("WALLET_PRIVATE_KEY").ok(),
        }
    }
}
```

Modify Config::load to populate wallet config.

**Step 4: Add placeholder wallet section to config.toml**

```toml
[wallet]
# Private key loaded from WALLET_PRIVATE_KEY env var (never commit!)
```

**Step 5: Add chain_id to NetworkConfig**

Add to NetworkConfig:

```rust
pub chain_id: u64,  // 80002 for Amoy testnet, 137 for Polygon mainnet
```

Update config.toml:

```toml
[network]
chain_id = 80002  # Amoy testnet
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add src/config.rs config.toml
git commit -m "config: add wallet and chain_id configuration"
```

---

## Task 3: Create Executor Module Structure

**Files:**
- Create: `src/executor/mod.rs`
- Create: `src/executor/orders.rs`
- Create: `src/executor/positions.rs`
- Modify: `src/main.rs` (add mod declaration)

**Step 1: Create executor/mod.rs**

```rust
//! Trade execution module.

mod orders;
mod positions;

pub use orders::OrderExecutor;
pub use positions::{Position, PositionStatus, PositionTracker};
```

**Step 2: Create executor/orders.rs with stub**

```rust
//! Order building and submission.

use crate::domain::Opportunity;
use crate::error::Result;

/// Executes trades on Polymarket CLOB.
pub struct OrderExecutor {
    // Will hold authenticated client
}

impl OrderExecutor {
    /// Create new executor (unauthenticated placeholder).
    pub fn new() -> Self {
        Self {}
    }

    /// Execute an arbitrage opportunity.
    pub async fn execute(&self, _opportunity: &Opportunity) -> Result<()> {
        // Placeholder - will implement in next task
        Ok(())
    }
}
```

**Step 3: Create executor/positions.rs with types**

```rust
//! Position tracking.

use crate::domain::{MarketId, Price, TokenId, Volume};
use chrono::{DateTime, Utc};

/// Unique position identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PositionId(pub u64);

/// Status of a position.
#[derive(Debug, Clone)]
pub enum PositionStatus {
    /// All legs filled successfully.
    Open,
    /// Some legs filled, exposure exists.
    PartialFill {
        filled: Vec<TokenId>,
        missing: Vec<TokenId>,
    },
    /// Position closed (market settled or sold).
    Closed { pnl: Price },
}

/// A single leg of a position.
#[derive(Debug, Clone)]
pub struct PositionLeg {
    pub token_id: TokenId,
    pub size: Volume,
    pub entry_price: Price,
}

/// An arbitrage position (YES + NO tokens held).
#[derive(Debug, Clone)]
pub struct Position {
    pub id: PositionId,
    pub market_id: MarketId,
    pub legs: Vec<PositionLeg>,
    pub entry_cost: Price,
    pub guaranteed_payout: Price,
    pub opened_at: DateTime<Utc>,
    pub status: PositionStatus,
}

/// Tracks all open positions.
pub struct PositionTracker {
    positions: Vec<Position>,
    next_id: u64,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            next_id: 1,
        }
    }

    /// Record a new position.
    pub fn add(&mut self, position: Position) {
        self.positions.push(position);
    }

    /// Get all open positions.
    pub fn open_positions(&self) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| matches!(p.status, PositionStatus::Open))
            .collect()
    }

    /// Total exposure (sum of entry costs for open positions).
    pub fn total_exposure(&self) -> Price {
        self.open_positions()
            .iter()
            .map(|p| p.entry_cost)
            .sum()
    }

    /// Generate next position ID.
    pub fn next_id(&mut self) -> PositionId {
        let id = PositionId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Add mod executor to main.rs**

Add after other mod declarations:

```rust
mod executor;
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors (warnings OK for unused)

**Step 6: Commit**

```bash
git add src/executor/ src/main.rs
git commit -m "feat: add executor module structure with position tracking"
```

---

## Task 4: Implement Authenticated CLOB Client

**Files:**
- Modify: `src/executor/orders.rs`
- Modify: `src/config.rs`

**Step 1: Read polymarket-client-sdk API**

The SDK provides:
- `Client::new(host, config)` - create unauthenticated client
- `client.authentication_builder(&signer).authenticate().await` - authenticate
- `client.limit_order().token_id().side().price().size().build().await` - build order
- `client.sign(&signer, order).await` - sign order
- `client.post_order(signed_order).await` - submit order

**Step 2: Update OrderExecutor with authenticated client**

```rust
//! Order building and submission.

use crate::config::Config;
use crate::domain::{Opportunity, Price, TokenId, Volume};
use crate::error::{Error, Result};

use alloy::signers::local::LocalSigner;
use polymarket_client_sdk::clob::{Client, Config as ClobConfig, Side};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};

/// Executes trades on Polymarket CLOB.
pub struct OrderExecutor {
    client: Client,
    signer: LocalSigner,
}

impl OrderExecutor {
    /// Create new executor with authentication.
    pub async fn new(config: &Config) -> Result<Self> {
        let private_key = config
            .wallet
            .private_key
            .as_ref()
            .ok_or_else(|| Error::Config("WALLET_PRIVATE_KEY not set".into()))?;

        let signer = LocalSigner::from_str(private_key)
            .map_err(|e| Error::Config(format!("Invalid private key: {}", e)))?
            .with_chain_id(Some(config.network.chain_id));

        let clob_config = ClobConfig::default();
        let client = Client::new(&config.network.api_url, clob_config)
            .map_err(|e| Error::Api(format!("Failed to create CLOB client: {}", e)))?
            .authentication_builder(&signer)
            .authenticate()
            .await
            .map_err(|e| Error::Api(format!("Authentication failed: {}", e)))?;

        info!("CLOB client authenticated");

        Ok(Self { client, signer })
    }

    /// Execute an arbitrage opportunity by buying YES and NO tokens.
    pub async fn execute(&self, opportunity: &Opportunity) -> Result<ExecutionResult> {
        info!(
            market = %opportunity.market_id,
            edge = %opportunity.edge,
            volume = %opportunity.volume,
            "Executing arbitrage"
        );

        // Build orders for both legs
        let yes_order = self.build_order(
            &opportunity.yes_token,
            Side::Buy,
            opportunity.yes_ask,
            opportunity.volume,
        ).await?;

        let no_order = self.build_order(
            &opportunity.no_token,
            Side::Buy,
            opportunity.no_ask,
            opportunity.volume,
        ).await?;

        // Sign orders
        let signed_yes = self.client.sign(&self.signer, yes_order).await
            .map_err(|e| Error::Execution(format!("Failed to sign YES order: {}", e)))?;

        let signed_no = self.client.sign(&self.signer, no_order).await
            .map_err(|e| Error::Execution(format!("Failed to sign NO order: {}", e)))?;

        // Submit both orders (in parallel for speed)
        let (yes_result, no_result) = tokio::join!(
            self.client.post_order(signed_yes),
            self.client.post_order(signed_no)
        );

        // Handle results
        match (yes_result, no_result) {
            (Ok(yes_resp), Ok(no_resp)) => {
                info!(
                    yes_order_id = ?yes_resp,
                    no_order_id = ?no_resp,
                    "Both orders submitted successfully"
                );
                Ok(ExecutionResult::Success {
                    yes_order_id: format!("{:?}", yes_resp),
                    no_order_id: format!("{:?}", no_resp),
                })
            }
            (Ok(_), Err(e)) => {
                warn!(error = %e, "NO order failed, YES order succeeded - EXPOSURE!");
                Ok(ExecutionResult::PartialFill {
                    filled_leg: opportunity.yes_token.clone(),
                    failed_leg: opportunity.no_token.clone(),
                    error: e.to_string(),
                })
            }
            (Err(e), Ok(_)) => {
                warn!(error = %e, "YES order failed, NO order succeeded - EXPOSURE!");
                Ok(ExecutionResult::PartialFill {
                    filled_leg: opportunity.no_token.clone(),
                    failed_leg: opportunity.yes_token.clone(),
                    error: e.to_string(),
                })
            }
            (Err(yes_err), Err(no_err)) => {
                warn!(yes_error = %yes_err, no_error = %no_err, "Both orders failed");
                Ok(ExecutionResult::Failed {
                    reason: format!("YES: {}, NO: {}", yes_err, no_err),
                })
            }
        }
    }

    async fn build_order(
        &self,
        token_id: &TokenId,
        side: Side,
        price: Price,
        size: Volume,
    ) -> Result<polymarket_client_sdk::clob::Order> {
        use alloy::primitives::U256;

        let token_u256 = U256::from_str(&token_id.0)
            .map_err(|e| Error::Execution(format!("Invalid token ID: {}", e)))?;

        self.client
            .limit_order()
            .token_id(token_u256)
            .side(side)
            .price(price)
            .size(size)
            .build()
            .await
            .map_err(|e| Error::Execution(format!("Failed to build order: {}", e)))
    }
}

/// Result of an execution attempt.
#[derive(Debug)]
pub enum ExecutionResult {
    Success {
        yes_order_id: String,
        no_order_id: String,
    },
    PartialFill {
        filled_leg: TokenId,
        failed_leg: TokenId,
        error: String,
    },
    Failed {
        reason: String,
    },
}
```

**Step 3: Add Execution error variant to error.rs**

Read `src/error.rs` first, then add:

```rust
#[error("Execution error: {0}")]
Execution(String),
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles (may have warnings)

**Step 5: Commit**

```bash
git add src/executor/orders.rs src/error.rs
git commit -m "feat: implement authenticated CLOB client and order execution"
```

---

## Task 5: Wire Executor to Detection Flow

**Files:**
- Modify: `src/main.rs`

**Step 1: Read current main.rs structure**

Use Read tool.

**Step 2: Add executor initialization in run()**

After loading config and before websocket, add:

```rust
// Initialize executor (authenticated CLOB client)
let executor = if config.wallet.private_key.is_some() {
    match OrderExecutor::new(&config).await {
        Ok(exec) => {
            info!("Executor initialized - trading ENABLED");
            Some(Arc::new(exec))
        }
        Err(e) => {
            warn!(error = %e, "Failed to initialize executor - trading DISABLED");
            None
        }
    }
} else {
    info!("No wallet configured - trading DISABLED (detection only)");
    None
};
```

**Step 3: Update handle_message to execute trades**

Change signature to accept optional executor:

```rust
async fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    config: &DetectorConfig,
    executor: Option<&OrderExecutor>,
) {
    match msg {
        WsMessage::Book(book) => {
            cache.update_from_ws(&book);

            let token_id = TokenId::from(book.asset_id.clone());
            if let Some(pair) = registry.get_market_for_token(&token_id) {
                if let Some(opp) = detect_single_condition(pair, cache, config) {
                    info!(
                        market = %opp.market_id,
                        question = %opp.question,
                        edge = %opp.edge,
                        volume = %opp.volume,
                        expected_profit = %opp.expected_profit,
                        "ARBITRAGE DETECTED"
                    );

                    // Execute if executor is available
                    if let Some(exec) = executor {
                        match exec.execute(&opp).await {
                            Ok(result) => info!(?result, "Execution complete"),
                            Err(e) => warn!(error = %e, "Execution failed"),
                        }
                    }
                }
            }
        }
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}
```

**Step 4: Update websocket callback to pass executor**

The callback needs to handle async execution. Update the handler:

```rust
let executor_clone = executor.clone();
handler
    .run(token_ids, move |msg| {
        let cache = cache_clone.clone();
        let registry = registry_clone.clone();
        let detector_config = detector_config_clone.clone();
        let exec = executor_clone.clone();

        tokio::spawn(async move {
            handle_message(msg, &cache, &registry, &detector_config, exec.as_deref()).await;
        });
    })
    .await?;
```

**Step 5: Add executor imports to main.rs**

```rust
use executor::OrderExecutor;
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire executor to detection flow"
```

---

## Task 6: Add Position Tracking to Executor

**Files:**
- Modify: `src/executor/orders.rs`
- Modify: `src/executor/positions.rs`

**Step 1: Add PositionTracker to OrderExecutor**

Update OrderExecutor to hold position tracker:

```rust
use parking_lot::Mutex;
use crate::executor::positions::{Position, PositionId, PositionLeg, PositionStatus, PositionTracker};

pub struct OrderExecutor {
    client: Client,
    signer: LocalSigner,
    positions: Mutex<PositionTracker>,
}
```

Initialize in `new()`:

```rust
Ok(Self {
    client,
    signer,
    positions: Mutex::new(PositionTracker::new()),
})
```

**Step 2: Record positions on successful execution**

In `execute()`, after successful submission:

```rust
ExecutionResult::Success { .. } => {
    // Record position
    let mut tracker = self.positions.lock();
    let position = Position {
        id: tracker.next_id(),
        market_id: opportunity.market_id.clone(),
        legs: vec![
            PositionLeg {
                token_id: opportunity.yes_token.clone(),
                size: opportunity.volume,
                entry_price: opportunity.yes_ask,
            },
            PositionLeg {
                token_id: opportunity.no_token.clone(),
                size: opportunity.volume,
                entry_price: opportunity.no_ask,
            },
        ],
        entry_cost: opportunity.total_cost * opportunity.volume,
        guaranteed_payout: opportunity.volume,  // $1 per share pair
        opened_at: chrono::Utc::now(),
        status: PositionStatus::Open,
    };

    info!(
        position_id = position.id.0,
        entry_cost = %position.entry_cost,
        guaranteed_profit = %(position.guaranteed_payout - position.entry_cost),
        "Position opened"
    );

    tracker.add(position);

    Ok(ExecutionResult::Success { yes_order_id, no_order_id })
}
```

**Step 3: Add method to get current exposure**

```rust
impl OrderExecutor {
    /// Get total exposure (capital at risk in open positions).
    pub fn total_exposure(&self) -> Price {
        self.positions.lock().total_exposure()
    }

    /// Get number of open positions.
    pub fn open_position_count(&self) -> usize {
        self.positions.lock().open_positions().len()
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/executor/
git commit -m "feat: track positions after successful execution"
```

---

## Task 7: Add Basic Execution Tests

**Files:**
- Create: `tests/executor_tests.rs`

**Step 1: Create test file with unit tests**

```rust
//! Executor unit tests.

use edgelord::domain::{MarketId, Opportunity, Price, TokenId, Volume};
use edgelord::executor::positions::{Position, PositionLeg, PositionStatus, PositionTracker};
use rust_decimal_macros::dec;

#[test]
fn test_position_tracker_new() {
    let tracker = PositionTracker::new();
    assert_eq!(tracker.open_positions().len(), 0);
    assert_eq!(tracker.total_exposure(), dec!(0));
}

#[test]
fn test_position_tracker_add_position() {
    let mut tracker = PositionTracker::new();

    let position = Position {
        id: tracker.next_id(),
        market_id: MarketId::from("market-1".to_string()),
        legs: vec![
            PositionLeg {
                token_id: TokenId::from("yes-token"),
                size: dec!(100),
                entry_price: dec!(0.45),
            },
            PositionLeg {
                token_id: TokenId::from("no-token"),
                size: dec!(100),
                entry_price: dec!(0.50),
            },
        ],
        entry_cost: dec!(95),  // 100 * (0.45 + 0.50)
        guaranteed_payout: dec!(100),
        opened_at: chrono::Utc::now(),
        status: PositionStatus::Open,
    };

    tracker.add(position);

    assert_eq!(tracker.open_positions().len(), 1);
    assert_eq!(tracker.total_exposure(), dec!(95));
}

#[test]
fn test_position_tracker_multiple_positions() {
    let mut tracker = PositionTracker::new();

    // Add first position
    let pos1 = Position {
        id: tracker.next_id(),
        market_id: MarketId::from("market-1".to_string()),
        legs: vec![],
        entry_cost: dec!(50),
        guaranteed_payout: dec!(55),
        opened_at: chrono::Utc::now(),
        status: PositionStatus::Open,
    };
    tracker.add(pos1);

    // Add second position
    let pos2 = Position {
        id: tracker.next_id(),
        market_id: MarketId::from("market-2".to_string()),
        legs: vec![],
        entry_cost: dec!(75),
        guaranteed_payout: dec!(80),
        opened_at: chrono::Utc::now(),
        status: PositionStatus::Open,
    };
    tracker.add(pos2);

    assert_eq!(tracker.open_positions().len(), 2);
    assert_eq!(tracker.total_exposure(), dec!(125));  // 50 + 75
}

#[test]
fn test_position_id_increments() {
    let mut tracker = PositionTracker::new();

    let id1 = tracker.next_id();
    let id2 = tracker.next_id();
    let id3 = tracker.next_id();

    assert_eq!(id1.0, 1);
    assert_eq!(id2.0, 2);
    assert_eq!(id3.0, 3);
}
```

**Step 2: Update lib exports if needed**

If the project doesn't have a lib.rs, the tests may need to import from the binary. Check structure and adjust imports accordingly.

**Step 3: Run tests**

Run: `cargo test executor`
Expected: All tests pass

**Step 4: Commit**

```bash
git add tests/executor_tests.rs
git commit -m "test: add position tracker unit tests"
```

---

## Task 8: Test on Amoy Testnet

**Files:**
- Modify: `config.toml`
- Create: `.env` (local only, gitignored)

**Step 1: Configure for Amoy testnet**

Update config.toml:

```toml
[network]
ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/"
api_url = "https://clob.polymarket.com"  # Same URL, uses Amoy credentials
chain_id = 80002  # Amoy testnet
```

**Step 2: Set up test wallet**

Create `.env` with test wallet:

```bash
WALLET_PRIVATE_KEY=<your-testnet-private-key>
```

**Step 3: Get test USDC**

Instructions for user:
1. Go to Polygon Amoy faucet
2. Get test MATIC for gas
3. Get test USDC from Polymarket testnet faucet (if available)

**Step 4: Run bot in detection mode first**

Run: `cargo run`
Expected: Bot connects, detects opportunities, logs them

**Step 5: Run bot with wallet to test execution**

With WALLET_PRIVATE_KEY set, run: `cargo run`
Expected: Bot attempts to execute when opportunity found

**Step 6: Verify execution logs**

Check for:
- "CLOB client authenticated" on startup
- "Executing arbitrage" when opportunity found
- "Both orders submitted successfully" or error messages

**Step 7: Document any issues**

Create notes on testnet behavior for future reference.

---

## Verification Checklist

Phase 3 complete:

- [x] `cargo check` passes
- [x] `cargo test` passes (all existing + new tests)
- [x] `cargo clippy` passes without errors
- [x] Bot starts with "trading DISABLED" when no wallet configured
- [x] Bot starts with "trading ENABLED" when wallet configured
- [x] Detection still works (opportunities logged)
- [x] Execution attempted when wallet configured and opportunity found
- [x] Position tracking records successful trades

**Note:** This plan was superseded by the comprehensive restructure which moved executor to `polymarket/executor.rs` and position tracking to `domain/position.rs`.

---

## Notes

**SDK Version Compatibility:**
- polymarket-client-sdk requires Rust 1.88.0+
- If build fails, run `rustup update`

**Testnet Limitations:**
- Testnet may have limited markets
- Arbitrage opportunities may be rare/nonexistent on testnet
- Can manually test order submission with known token IDs

**Security:**
- Never commit private keys
- Use separate testnet wallet
- Start with small amounts on mainnet

**Error Handling:**
- Current implementation logs errors but doesn't implement recovery
- Partial fill handling is logged but not automatically resolved
- Phase 4 (Risk) will add proper circuit breakers
