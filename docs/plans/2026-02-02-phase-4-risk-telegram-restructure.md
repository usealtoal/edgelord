# Phase 4: Risk Management + Telegram + Repository Restructure

> Status: Historical
> Superseded by: N/A
> Summary:
> - Goal: Restructure src/ for clean layering, add risk management with position limits and circuit breakers, and implement Telegram notifications for alerts and daily summaries.
> - Scope: Task 1: Create adapter/polymarket directory structure
> Planned Outcomes:
> - Task 1: Create adapter/polymarket directory structure
> - Task 2: Create app module from app.rs


> **Status:** ✅ COMPLETE — Merged in PR #7 on 2026-02-02

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure src/ for clean layering, add risk management with position limits and circuit breakers, and implement Telegram notifications for alerts and daily summaries.

**Architecture:** Three-part change: (1) Restructure src/ to separate adapters, services, and app layers; (2) Add RiskManager service that wraps execution with pre-trade checks; (3) Add Notifier trait with Telegram implementation for real-time alerts. Shared AppState holds positions and risk state accessible to all services.

**Tech Stack:** teloxide (Telegram bot), tokio channels for events, parking_lot for shared state

---

## Part 1: Repository Restructure

### Task 1: Create adapter/polymarket directory structure

**Files:**
- Create: `src/adapter/mod.rs`
- Create: `src/adapter/polymarket/mod.rs`
- Move: `src/polymarket/*.rs` → `src/adapter/polymarket/`

**Step 1: Create adapter directory and move files**

```bash
mkdir -p src/adapter/polymarket
mv src/polymarket/client.rs src/adapter/polymarket/
mv src/polymarket/executor.rs src/adapter/polymarket/
mv src/polymarket/messages.rs src/adapter/polymarket/
mv src/polymarket/registry.rs src/adapter/polymarket/
mv src/polymarket/types.rs src/adapter/polymarket/
mv src/polymarket/websocket.rs src/adapter/polymarket/
mv src/polymarket/mod.rs src/adapter/polymarket/
rmdir src/polymarket
```

**Step 2: Create adapter/mod.rs**

```rust
//! Exchange adapter implementations.

#[cfg(feature = "polymarket")]
pub mod polymarket;
```

**Step 3: Update src/lib.rs imports**

Change:
```rust
#[cfg(feature = "polymarket")]
pub mod polymarket;
```

To:
```rust
pub mod adapter;
```

**Step 4: Update all imports throughout codebase**

In `src/app.rs`, change:
```rust
use crate::polymarket::{...};
```
To:
```rust
use crate::adapter::polymarket::{...};
```

**Step 5: Run tests**

```bash
cargo test
```
Expected: All 96 tests pass

**Step 6: Commit**

```bash
git add -A && git commit -m "refactor: move polymarket to adapter/polymarket"
```

---

### Task 2: Create app module from app.rs

**Files:**
- Create: `src/app/mod.rs`
- Create: `src/app/orchestrator.rs`
- Move: `src/app.rs` content → `src/app/orchestrator.rs`
- Move: `src/config.rs` → `src/app/config.rs`

**Step 1: Create app directory structure**

```bash
mkdir -p src/app
mv src/app.rs src/app/orchestrator.rs
mv src/config.rs src/app/config.rs
```

**Step 2: Create src/app/mod.rs**

```rust
//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;

pub use config::{Config, LoggingConfig, NetworkConfig, StrategiesConfig, WalletConfig};
pub use orchestrator::App;
```

**Step 3: Update src/app/orchestrator.rs imports**

Change the module path for config:
```rust
use crate::app::config::Config;
```

And update polymarket import:
```rust
use crate::adapter::polymarket::{...};
```

**Step 4: Update src/app/config.rs imports**

Change:
```rust
use crate::error::{ConfigError, Result};
```
(This stays the same, just verify it works)

**Step 5: Update src/lib.rs**

Change:
```rust
pub mod config;

#[cfg(feature = "polymarket")]
pub mod app;
```

To:
```rust
#[cfg(feature = "polymarket")]
pub mod app;
```

And update the app module to not be feature-gated for config:
```rust
pub mod app;
```

Actually, simpler approach - keep config accessible:
```rust
pub mod app;
```

**Step 6: Update src/main.rs**

Change:
```rust
use edgelord::config::Config;
use edgelord::app::App;
```

To:
```rust
use edgelord::app::{App, Config};
```

**Step 7: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 8: Commit**

```bash
git add -A && git commit -m "refactor: create app module with config and orchestrator"
```

---

### Task 3: Create service module skeleton

**Files:**
- Create: `src/service/mod.rs`

**Step 1: Create service directory and mod.rs**

```bash
mkdir -p src/service
```

```rust
//! Cross-cutting services - risk management, notifications, etc.

// Services will be added in subsequent tasks
```

**Step 2: Update src/lib.rs**

Add:
```rust
pub mod service;
```

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: add empty service module"
```

---

### Task 4: Create shared AppState

**Files:**
- Create: `src/app/state.rs`
- Modify: `src/app/mod.rs`

**Step 1: Create src/app/state.rs**

```rust
//! Shared application state.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;
use rust_decimal::Decimal;

use crate::domain::{PositionTracker, Price};

/// Risk limits configuration.
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Maximum position size per market in dollars.
    pub max_position_per_market: Decimal,
    /// Maximum total exposure across all positions.
    pub max_total_exposure: Decimal,
    /// Minimum profit threshold to execute.
    pub min_profit_threshold: Decimal,
    /// Maximum slippage tolerance (e.g., 0.02 = 2%).
    pub max_slippage: Decimal,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_per_market: Decimal::from(1000),
            max_total_exposure: Decimal::from(10000),
            min_profit_threshold: Decimal::new(5, 2), // $0.05
            max_slippage: Decimal::new(2, 2),         // 2%
        }
    }
}

/// Shared application state accessible by all services.
pub struct AppState {
    /// Position tracker for all open/closed positions.
    positions: RwLock<PositionTracker>,
    /// Risk limits configuration.
    risk_limits: RiskLimits,
    /// Circuit breaker - when true, no new trades.
    circuit_breaker: AtomicBool,
    /// Reason for circuit breaker activation.
    circuit_breaker_reason: RwLock<Option<String>>,
}

impl AppState {
    /// Create new app state with given risk limits.
    pub fn new(risk_limits: RiskLimits) -> Self {
        Self {
            positions: RwLock::new(PositionTracker::new()),
            risk_limits,
            circuit_breaker: AtomicBool::new(false),
            circuit_breaker_reason: RwLock::new(None),
        }
    }

    /// Get read access to positions.
    pub fn positions(&self) -> parking_lot::RwLockReadGuard<'_, PositionTracker> {
        self.positions.read()
    }

    /// Get write access to positions.
    pub fn positions_mut(&self) -> parking_lot::RwLockWriteGuard<'_, PositionTracker> {
        self.positions.write()
    }

    /// Get risk limits.
    pub fn risk_limits(&self) -> &RiskLimits {
        &self.risk_limits
    }

    /// Check if circuit breaker is active.
    pub fn is_circuit_breaker_active(&self) -> bool {
        self.circuit_breaker.load(Ordering::SeqCst)
    }

    /// Activate circuit breaker with reason.
    pub fn activate_circuit_breaker(&self, reason: impl Into<String>) {
        self.circuit_breaker.store(true, Ordering::SeqCst);
        *self.circuit_breaker_reason.write() = Some(reason.into());
    }

    /// Reset circuit breaker.
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.store(false, Ordering::SeqCst);
        *self.circuit_breaker_reason.write() = None;
    }

    /// Get circuit breaker reason if active.
    pub fn circuit_breaker_reason(&self) -> Option<String> {
        self.circuit_breaker_reason.read().clone()
    }

    /// Get current total exposure.
    pub fn total_exposure(&self) -> Price {
        self.positions.read().total_exposure()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(RiskLimits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_circuit_breaker_active());
        assert!(state.circuit_breaker_reason().is_none());
    }

    #[test]
    fn test_circuit_breaker() {
        let state = AppState::default();

        state.activate_circuit_breaker("test reason");
        assert!(state.is_circuit_breaker_active());
        assert_eq!(state.circuit_breaker_reason(), Some("test reason".to_string()));

        state.reset_circuit_breaker();
        assert!(!state.is_circuit_breaker_active());
        assert!(state.circuit_breaker_reason().is_none());
    }

    #[test]
    fn test_risk_limits_default() {
        let limits = RiskLimits::default();
        assert_eq!(limits.max_position_per_market, Decimal::from(1000));
        assert_eq!(limits.max_total_exposure, Decimal::from(10000));
    }
}
```

**Step 2: Update src/app/mod.rs**

```rust
//! Application layer - orchestration, configuration, and shared state.

mod config;
mod orchestrator;
mod state;

pub use config::{Config, LoggingConfig, NetworkConfig, StrategiesConfig, WalletConfig};
pub use orchestrator::App;
pub use state::{AppState, RiskLimits};
```

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass (including new state tests)

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add AppState with RiskLimits and circuit breaker"
```

---

### Task 5: Update lib.rs exports and verify structure

**Files:**
- Modify: `src/lib.rs`

**Step 1: Update src/lib.rs with clean exports**

```rust
//! Edgelord - Multi-strategy arbitrage detection and execution.
//!
//! # Architecture
//!
//! ```text
//! src/
//! ├── domain/          # Exchange-agnostic business logic
//! │   ├── strategy/    # Pluggable detection strategies
//! │   └── solver/      # LP/ILP solver abstraction
//! ├── exchange/        # Exchange trait definitions
//! ├── adapter/         # Exchange implementations
//! │   └── polymarket/  # Polymarket CLOB integration
//! ├── service/         # Cross-cutting services (risk, notifications)
//! └── app/             # Application layer (config, state, orchestration)
//! ```
//!
//! # Features
//!
//! - `polymarket` - Enable Polymarket exchange support (default)
//! - `telegram` - Enable Telegram notifications (coming soon)

pub mod domain;
pub mod error;
pub mod exchange;
pub mod adapter;
pub mod service;
pub mod app;
```

**Step 2: Run full test suite**

```bash
cargo test
```
Expected: All tests pass

**Step 3: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1 | head -50
```
Expected: No errors (warnings OK for now)

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: update lib.rs exports for new structure"
```

---

## Part 2: Risk Management

### Task 6: Add RiskError variants

**Files:**
- Modify: `src/error.rs`

**Step 1: Add RiskError enum**

Add after `ExecutionError`:

```rust
/// Risk management errors.
#[derive(Error, Debug, Clone)]
pub enum RiskError {
    #[error("circuit breaker active: {reason}")]
    CircuitBreakerActive { reason: String },

    #[error("position limit exceeded: {current} >= {limit} for market {market_id}")]
    PositionLimitExceeded {
        market_id: String,
        current: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("exposure limit exceeded: {current} + {additional} > {limit}")]
    ExposureLimitExceeded {
        current: rust_decimal::Decimal,
        additional: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("profit below threshold: {expected} < {threshold}")]
    ProfitBelowThreshold {
        expected: rust_decimal::Decimal,
        threshold: rust_decimal::Decimal,
    },

    #[error("slippage too high: {actual} > {max}")]
    SlippageTooHigh {
        actual: rust_decimal::Decimal,
        max: rust_decimal::Decimal,
    },
}
```

**Step 2: Add RiskError to main Error enum**

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error(transparent)]
    Risk(#[from] RiskError),

    // ... rest unchanged
}
```

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add RiskError variants"
```

---

### Task 7: Implement RiskManager service

**Files:**
- Create: `src/service/risk.rs`
- Modify: `src/service/mod.rs`

**Step 1: Create src/service/risk.rs**

```rust
//! Risk management service.
//!
//! Provides pre-execution checks for position limits, exposure caps,
//! and circuit breaker status.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::app::AppState;
use crate::domain::Opportunity;
use crate::error::RiskError;

/// Result of a risk check.
#[derive(Debug, Clone)]
pub enum RiskCheckResult {
    /// Trade is allowed to proceed.
    Approved,
    /// Trade is rejected with reason.
    Rejected(RiskError),
}

impl RiskCheckResult {
    /// Check if approved.
    pub fn is_approved(&self) -> bool {
        matches!(self, RiskCheckResult::Approved)
    }

    /// Get rejection error if rejected.
    pub fn rejection_error(&self) -> Option<&RiskError> {
        match self {
            RiskCheckResult::Rejected(e) => Some(e),
            RiskCheckResult::Approved => None,
        }
    }
}

/// Risk manager that validates trades before execution.
pub struct RiskManager {
    state: Arc<AppState>,
}

impl RiskManager {
    /// Create a new risk manager with shared state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Check if an opportunity passes all risk checks.
    pub fn check(&self, opportunity: &Opportunity) -> RiskCheckResult {
        // Check circuit breaker first
        if let Err(e) = self.check_circuit_breaker() {
            return RiskCheckResult::Rejected(e);
        }

        // Check profit threshold
        if let Err(e) = self.check_profit_threshold(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        // Check exposure limits
        if let Err(e) = self.check_exposure_limit(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        // Check position limit for this market
        if let Err(e) = self.check_position_limit(opportunity) {
            return RiskCheckResult::Rejected(e);
        }

        RiskCheckResult::Approved
    }

    /// Check if circuit breaker is active.
    fn check_circuit_breaker(&self) -> Result<(), RiskError> {
        if self.state.is_circuit_breaker_active() {
            let reason = self
                .state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string());
            warn!(reason = %reason, "Circuit breaker active");
            return Err(RiskError::CircuitBreakerActive { reason });
        }
        Ok(())
    }

    /// Check if expected profit meets threshold.
    fn check_profit_threshold(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let threshold = self.state.risk_limits().min_profit_threshold;
        let expected = opportunity.expected_profit();

        if expected < threshold {
            return Err(RiskError::ProfitBelowThreshold { expected, threshold });
        }
        Ok(())
    }

    /// Check if total exposure would exceed limit.
    fn check_exposure_limit(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let current = self.state.total_exposure();
        let additional = opportunity.total_cost() * opportunity.volume();
        let limit = self.state.risk_limits().max_total_exposure;

        if current + additional > limit {
            warn!(
                current = %current,
                additional = %additional,
                limit = %limit,
                "Exposure limit would be exceeded"
            );
            return Err(RiskError::ExposureLimitExceeded {
                current,
                additional,
                limit,
            });
        }
        Ok(())
    }

    /// Check if position in this market would exceed limit.
    fn check_position_limit(&self, opportunity: &Opportunity) -> Result<(), RiskError> {
        let market_id = opportunity.market_id();
        let limit = self.state.risk_limits().max_position_per_market;

        // Calculate current position in this market
        let current = self
            .state
            .positions()
            .iter()
            .filter(|p| p.market_id() == market_id && p.status().is_open())
            .map(|p| p.entry_cost())
            .sum::<Decimal>();

        let additional = opportunity.total_cost() * opportunity.volume();

        if current + additional > limit {
            warn!(
                market_id = %market_id,
                current = %current,
                additional = %additional,
                limit = %limit,
                "Position limit would be exceeded"
            );
            return Err(RiskError::PositionLimitExceeded {
                market_id: market_id.to_string(),
                current,
                limit,
            });
        }
        Ok(())
    }

    /// Record a successful execution (updates state).
    pub fn record_execution(&self, opportunity: &Opportunity) {
        info!(
            market_id = %opportunity.market_id(),
            volume = %opportunity.volume(),
            profit = %opportunity.expected_profit(),
            "Execution recorded"
        );
        // Position is added by executor, we just log here
    }

    /// Trigger circuit breaker.
    pub fn trigger_circuit_breaker(&self, reason: impl Into<String>) {
        let reason = reason.into();
        warn!(reason = %reason, "Triggering circuit breaker");
        self.state.activate_circuit_breaker(reason);
    }

    /// Reset circuit breaker.
    pub fn reset_circuit_breaker(&self) {
        info!("Resetting circuit breaker");
        self.state.reset_circuit_breaker();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RiskLimits;
    use crate::domain::{MarketId, TokenId};
    use rust_decimal_macros::dec;

    fn make_opportunity(volume: Decimal, yes_ask: Decimal, no_ask: Decimal) -> Opportunity {
        use crate::domain::Opportunity;
        Opportunity::builder()
            .market_id(MarketId::from("test-market"))
            .question("Test?")
            .yes_token(TokenId::from("yes"), yes_ask)
            .no_token(TokenId::from("no"), no_ask)
            .volume(volume)
            .build()
            .unwrap()
    }

    #[test]
    fn test_check_approved() {
        let state = Arc::new(AppState::default());
        let risk = RiskManager::new(state);

        // Good opportunity: 10% edge, $10 volume = $1 profit
        let opp = make_opportunity(dec!(10), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(result.is_approved());
    }

    #[test]
    fn test_check_circuit_breaker() {
        let state = Arc::new(AppState::default());
        state.activate_circuit_breaker("test");
        let risk = RiskManager::new(state);

        let opp = make_opportunity(dec!(10), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::CircuitBreakerActive { .. })
        ));
    }

    #[test]
    fn test_check_profit_below_threshold() {
        let limits = RiskLimits {
            min_profit_threshold: dec!(1.0), // Require $1 minimum
            ..Default::default()
        };
        let state = Arc::new(AppState::new(limits));
        let risk = RiskManager::new(state);

        // Only $0.10 edge * $1 volume = $0.10 profit (below $1 threshold)
        let opp = make_opportunity(dec!(1), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::ProfitBelowThreshold { .. })
        ));
    }

    #[test]
    fn test_check_exposure_limit() {
        let limits = RiskLimits {
            max_total_exposure: dec!(100), // Only $100 max
            min_profit_threshold: dec!(0),
            ..Default::default()
        };
        let state = Arc::new(AppState::new(limits));
        let risk = RiskManager::new(state);

        // $90 cost * $200 volume = $18,000 exposure (way over $100)
        let opp = make_opportunity(dec!(200), dec!(0.45), dec!(0.45));
        let result = risk.check(&opp);

        assert!(!result.is_approved());
        assert!(matches!(
            result.rejection_error(),
            Some(RiskError::ExposureLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_trigger_and_reset_circuit_breaker() {
        let state = Arc::new(AppState::default());
        let risk = RiskManager::new(state.clone());

        assert!(!state.is_circuit_breaker_active());

        risk.trigger_circuit_breaker("test reason");
        assert!(state.is_circuit_breaker_active());

        risk.reset_circuit_breaker();
        assert!(!state.is_circuit_breaker_active());
    }
}
```

**Step 2: Update src/service/mod.rs**

```rust
//! Cross-cutting services - risk management, notifications, etc.

mod risk;

pub use risk::{RiskCheckResult, RiskManager};
```

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass including new risk tests

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement RiskManager service"
```

---

### Task 8: Add risk config to app config

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add RiskConfig struct**

Add after `WalletConfig`:

```rust
use crate::app::RiskLimits;
use rust_decimal::Decimal;

/// Risk management configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size per market in dollars.
    #[serde(default = "default_max_position")]
    pub max_position_per_market: Decimal,

    /// Maximum total exposure across all positions.
    #[serde(default = "default_max_exposure")]
    pub max_total_exposure: Decimal,

    /// Minimum profit threshold to execute.
    #[serde(default = "default_min_profit")]
    pub min_profit_threshold: Decimal,

    /// Maximum slippage tolerance.
    #[serde(default = "default_max_slippage")]
    pub max_slippage: Decimal,
}

fn default_max_position() -> Decimal {
    Decimal::from(1000)
}

fn default_max_exposure() -> Decimal {
    Decimal::from(10000)
}

fn default_min_profit() -> Decimal {
    Decimal::new(5, 2) // $0.05
}

fn default_max_slippage() -> Decimal {
    Decimal::new(2, 2) // 2%
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_per_market: default_max_position(),
            max_total_exposure: default_max_exposure(),
            min_profit_threshold: default_min_profit(),
            max_slippage: default_max_slippage(),
        }
    }
}

impl From<RiskConfig> for RiskLimits {
    fn from(config: RiskConfig) -> Self {
        RiskLimits {
            max_position_per_market: config.max_position_per_market,
            max_total_exposure: config.max_total_exposure,
            min_profit_threshold: config.min_profit_threshold,
            max_slippage: config.max_slippage,
        }
    }
}
```

**Step 2: Add risk field to Config**

```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub risk: RiskConfig,
}
```

**Step 3: Update Config::default()**

Add to Default impl:
```rust
risk: RiskConfig::default(),
```

**Step 4: Update mod.rs exports**

```rust
pub use config::{Config, LoggingConfig, NetworkConfig, RiskConfig, StrategiesConfig, WalletConfig};
```

**Step 5: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add RiskConfig to app configuration"
```

---

## Part 3: Notification System

### Task 9: Add teloxide dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add telegram feature and teloxide dependency**

```toml
[features]
default = ["polymarket"]
polymarket = ["dep:polymarket-client-sdk", "dep:alloy-signer-local"]
telegram = ["dep:teloxide"]

[dependencies]
# ... existing deps ...

# Telegram bot (optional)
teloxide = { version = "0.13", features = ["macros"], optional = true }
```

**Step 2: Run cargo check**

```bash
cargo check --features telegram
```
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock && git commit -m "deps: add teloxide for telegram notifications"
```

---

### Task 10: Create Notifier trait

**Files:**
- Create: `src/service/notifier.rs`
- Modify: `src/service/mod.rs`

**Step 1: Create src/service/notifier.rs**

```rust
//! Notification system for alerts and events.
//!
//! The `Notifier` trait defines the interface for notification handlers.
//! Multiple notifiers can be registered with the `NotifierRegistry`.

use crate::adapter::polymarket::ArbitrageExecutionResult;
use crate::domain::Opportunity;
use crate::error::RiskError;

/// Events that can trigger notifications.
#[derive(Debug, Clone)]
pub enum Event {
    /// Arbitrage opportunity detected.
    OpportunityDetected(OpportunityEvent),
    /// Execution completed (success or failure).
    ExecutionCompleted(ExecutionEvent),
    /// Risk check rejected a trade.
    RiskRejected(RiskEvent),
    /// Circuit breaker activated.
    CircuitBreakerActivated { reason: String },
    /// Circuit breaker reset.
    CircuitBreakerReset,
    /// Daily summary.
    DailySummary(SummaryEvent),
}

/// Opportunity detection event.
#[derive(Debug, Clone)]
pub struct OpportunityEvent {
    pub market_id: String,
    pub question: String,
    pub edge: rust_decimal::Decimal,
    pub volume: rust_decimal::Decimal,
    pub expected_profit: rust_decimal::Decimal,
}

impl From<&Opportunity> for OpportunityEvent {
    fn from(opp: &Opportunity) -> Self {
        Self {
            market_id: opp.market_id().to_string(),
            question: opp.question().to_string(),
            edge: opp.edge(),
            volume: opp.volume(),
            expected_profit: opp.expected_profit(),
        }
    }
}

/// Execution result event.
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    pub market_id: String,
    pub success: bool,
    pub details: String,
}

impl ExecutionEvent {
    pub fn from_result(market_id: &str, result: &ArbitrageExecutionResult) -> Self {
        match result {
            ArbitrageExecutionResult::Success { yes_order_id, no_order_id } => Self {
                market_id: market_id.to_string(),
                success: true,
                details: format!("YES: {}, NO: {}", yes_order_id, no_order_id),
            },
            ArbitrageExecutionResult::PartialFill { filled_leg, error, .. } => Self {
                market_id: market_id.to_string(),
                success: false,
                details: format!("Partial fill ({}): {}", filled_leg, error),
            },
            ArbitrageExecutionResult::Failed { reason } => Self {
                market_id: market_id.to_string(),
                success: false,
                details: format!("Failed: {}", reason),
            },
        }
    }
}

/// Risk rejection event.
#[derive(Debug, Clone)]
pub struct RiskEvent {
    pub market_id: String,
    pub reason: String,
}

impl RiskEvent {
    pub fn new(market_id: &str, error: &RiskError) -> Self {
        Self {
            market_id: market_id.to_string(),
            reason: error.to_string(),
        }
    }
}

/// Daily summary event.
#[derive(Debug, Clone)]
pub struct SummaryEvent {
    pub date: chrono::NaiveDate,
    pub opportunities_detected: u64,
    pub trades_executed: u64,
    pub trades_successful: u64,
    pub total_profit: rust_decimal::Decimal,
    pub current_exposure: rust_decimal::Decimal,
}

/// Trait for notification handlers.
///
/// Implement this trait to receive events from the system.
/// Notifications are fire-and-forget (async but not awaited).
pub trait Notifier: Send + Sync {
    /// Handle an event.
    fn notify(&self, event: Event);
}

/// Registry of notifiers.
pub struct NotifierRegistry {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotifierRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { notifiers: vec![] }
    }

    /// Register a notifier.
    pub fn register(&mut self, notifier: Box<dyn Notifier>) {
        self.notifiers.push(notifier);
    }

    /// Notify all registered notifiers.
    pub fn notify_all(&self, event: Event) {
        for notifier in &self.notifiers {
            notifier.notify(event.clone());
        }
    }

    /// Number of registered notifiers.
    pub fn len(&self) -> usize {
        self.notifiers.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.notifiers.is_empty()
    }
}

impl Default for NotifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A no-op notifier for testing or when notifications are disabled.
pub struct NullNotifier;

impl Notifier for NullNotifier {
    fn notify(&self, _event: Event) {
        // Do nothing
    }
}

/// A logging notifier that logs events via tracing.
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, event: Event) {
        use tracing::info;
        match event {
            Event::OpportunityDetected(e) => {
                info!(
                    market_id = %e.market_id,
                    edge = %e.edge,
                    profit = %e.expected_profit,
                    "Opportunity detected"
                );
            }
            Event::ExecutionCompleted(e) => {
                info!(
                    market_id = %e.market_id,
                    success = e.success,
                    details = %e.details,
                    "Execution completed"
                );
            }
            Event::RiskRejected(e) => {
                info!(
                    market_id = %e.market_id,
                    reason = %e.reason,
                    "Risk rejected"
                );
            }
            Event::CircuitBreakerActivated { reason } => {
                info!(reason = %reason, "Circuit breaker activated");
            }
            Event::CircuitBreakerReset => {
                info!("Circuit breaker reset");
            }
            Event::DailySummary(e) => {
                info!(
                    date = %e.date,
                    opportunities = e.opportunities_detected,
                    trades = e.trades_executed,
                    successful = e.trades_successful,
                    profit = %e.total_profit,
                    "Daily summary"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingNotifier {
        count: Arc<AtomicUsize>,
    }

    impl Notifier for CountingNotifier {
        fn notify(&self, _event: Event) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_registry_notify_all() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut registry = NotifierRegistry::new();

        registry.register(Box::new(CountingNotifier { count: count.clone() }));
        registry.register(Box::new(CountingNotifier { count: count.clone() }));

        registry.notify_all(Event::CircuitBreakerReset);

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_null_notifier() {
        let notifier = NullNotifier;
        notifier.notify(Event::CircuitBreakerReset);
        // Just verify it doesn't panic
    }
}
```

**Step 2: Update src/service/mod.rs**

```rust
//! Cross-cutting services - risk management, notifications, etc.

mod notifier;
mod risk;

pub use notifier::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier,
    OpportunityEvent, RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};
```

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Notifier trait and registry"
```

---

### Task 11: Implement Telegram notifier

**Files:**
- Create: `src/service/telegram.rs`
- Modify: `src/service/mod.rs`

**Step 1: Create src/service/telegram.rs**

```rust
//! Telegram notification implementation.
//!
//! Requires the `telegram` feature to be enabled.

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::{Event, Notifier};

/// Configuration for Telegram notifier.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot token from @BotFather.
    pub bot_token: String,
    /// Chat ID to send notifications to.
    pub chat_id: i64,
    /// Whether to send opportunity alerts (can be noisy).
    pub notify_opportunities: bool,
    /// Whether to send execution alerts.
    pub notify_executions: bool,
    /// Whether to send risk rejections.
    pub notify_risk_rejections: bool,
}

impl TelegramConfig {
    /// Create config from environment variables.
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .ok()
            .and_then(|s| s.parse().ok())?;

        Some(Self {
            bot_token,
            chat_id,
            notify_opportunities: std::env::var("TELEGRAM_NOTIFY_OPPORTUNITIES")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            notify_executions: true,
            notify_risk_rejections: true,
        })
    }
}

/// Telegram notifier that sends messages to a chat.
pub struct TelegramNotifier {
    sender: mpsc::UnboundedSender<Event>,
}

impl TelegramNotifier {
    /// Create a new Telegram notifier and spawn the background task.
    pub fn new(config: TelegramConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn background task to handle messages
        tokio::spawn(telegram_worker(config, receiver));

        Self { sender }
    }
}

impl Notifier for TelegramNotifier {
    fn notify(&self, event: Event) {
        if self.sender.send(event).is_err() {
            warn!("Telegram notifier channel closed");
        }
    }
}

/// Background worker that sends Telegram messages.
async fn telegram_worker(config: TelegramConfig, mut receiver: mpsc::UnboundedReceiver<Event>) {
    let bot = Bot::new(&config.bot_token);
    let chat_id = ChatId(config.chat_id);

    info!(chat_id = config.chat_id, "Telegram notifier started");

    while let Some(event) = receiver.recv().await {
        let message = match &event {
            Event::OpportunityDetected(e) if config.notify_opportunities => {
                Some(format!(
                    "🎯 *Opportunity Detected*\n\n\
                     Market: `{}`\n\
                     Question: {}\n\
                     Edge: {:.2}%\n\
                     Volume: ${:.2}\n\
                     Expected Profit: ${:.2}",
                    e.market_id,
                    escape_markdown(&e.question),
                    e.edge * rust_decimal::Decimal::from(100),
                    e.volume,
                    e.expected_profit
                ))
            }
            Event::ExecutionCompleted(e) if config.notify_executions => {
                let emoji = if e.success { "✅" } else { "❌" };
                Some(format!(
                    "{} *Execution {}*\n\n\
                     Market: `{}`\n\
                     Details: {}",
                    emoji,
                    if e.success { "Success" } else { "Failed" },
                    e.market_id,
                    escape_markdown(&e.details)
                ))
            }
            Event::RiskRejected(e) if config.notify_risk_rejections => {
                Some(format!(
                    "⚠️ *Risk Rejected*\n\n\
                     Market: `{}`\n\
                     Reason: {}",
                    e.market_id,
                    escape_markdown(&e.reason)
                ))
            }
            Event::CircuitBreakerActivated { reason } => {
                Some(format!(
                    "🚨 *CIRCUIT BREAKER ACTIVATED*\n\n\
                     Reason: {}\n\n\
                     All trading has been halted.",
                    escape_markdown(reason)
                ))
            }
            Event::CircuitBreakerReset => {
                Some("✅ *Circuit Breaker Reset*\n\nTrading has resumed.".to_string())
            }
            Event::DailySummary(e) => {
                Some(format!(
                    "📊 *Daily Summary - {}*\n\n\
                     Opportunities: {}\n\
                     Trades Executed: {}\n\
                     Successful: {}\n\
                     Total Profit: ${:.2}\n\
                     Current Exposure: ${:.2}",
                    e.date,
                    e.opportunities_detected,
                    e.trades_executed,
                    e.trades_successful,
                    e.total_profit,
                    e.current_exposure
                ))
            }
            _ => None,
        };

        if let Some(text) = message {
            if let Err(e) = bot
                .send_message(chat_id, &text)
                .parse_mode(ParseMode::MarkdownV2)
                .await
            {
                error!(error = %e, "Failed to send Telegram message");
            }
        }
    }

    warn!("Telegram worker shutting down");
}

/// Escape special characters for Telegram MarkdownV2.
fn escape_markdown(text: &str) -> String {
    let special_chars = ['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];
    let mut result = String::with_capacity(text.len() * 2);

    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("hello"), "hello");
        assert_eq!(escape_markdown("hello_world"), "hello\\_world");
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
        assert_eq!(escape_markdown("test.com"), "test\\.com");
    }
}
```

**Step 2: Update src/service/mod.rs**

```rust
//! Cross-cutting services - risk management, notifications, etc.

mod notifier;
mod risk;

#[cfg(feature = "telegram")]
mod telegram;

pub use notifier::{
    Event, ExecutionEvent, LogNotifier, Notifier, NotifierRegistry, NullNotifier,
    OpportunityEvent, RiskEvent, SummaryEvent,
};
pub use risk::{RiskCheckResult, RiskManager};

#[cfg(feature = "telegram")]
pub use telegram::{TelegramConfig, TelegramNotifier};
```

**Step 3: Run tests (with feature)**

```bash
cargo test --features telegram
```
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement Telegram notifier"
```

---

### Task 12: Add telegram config to app config

**Files:**
- Modify: `src/app/config.rs`

**Step 1: Add TelegramConfig to config (feature-gated)**

Add to config.rs:

```rust
/// Telegram notification configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TelegramAppConfig {
    /// Enable telegram notifications.
    #[serde(default)]
    pub enabled: bool,
    /// Send opportunity alerts (can be noisy).
    #[serde(default)]
    pub notify_opportunities: bool,
    /// Send execution alerts.
    #[serde(default = "default_true")]
    pub notify_executions: bool,
    /// Send risk rejection alerts.
    #[serde(default = "default_true")]
    pub notify_risk_rejections: bool,
}

fn default_true() -> bool {
    true
}
```

**Step 2: Add to Config struct**

```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub telegram: TelegramAppConfig,
}
```

**Step 3: Update Default impl**

```rust
telegram: TelegramAppConfig::default(),
```

**Step 4: Update exports in mod.rs**

```rust
pub use config::{
    Config, LoggingConfig, NetworkConfig, RiskConfig, StrategiesConfig, TelegramAppConfig,
    WalletConfig,
};
```

**Step 5: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add telegram config to app configuration"
```

---

## Part 4: Wire Everything Together

### Task 13: Update orchestrator to use services

**Files:**
- Modify: `src/app/orchestrator.rs`

**Step 1: Refactor orchestrator to use AppState, RiskManager, and NotifierRegistry**

Replace the entire orchestrator.rs with:

```rust
//! App orchestration module.
//!
//! This module contains the main application logic for running
//! the edgelord arbitrage detection and execution system.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::adapter::polymarket::{
    ArbitrageExecutionResult, MarketRegistry, PolymarketClient, PolymarketExecutor,
    WebSocketHandler, WsMessage,
};
use crate::app::config::Config;
use crate::app::state::AppState;
use crate::domain::strategy::{
    CombinatorialStrategy, DetectionContext, MarketRebalancingStrategy, SingleConditionStrategy,
    StrategyRegistry,
};
use crate::domain::{MarketPair, Opportunity, OrderBookCache};
use crate::error::Result;
use crate::service::{
    Event, ExecutionEvent, LogNotifier, NotifierRegistry, OpportunityEvent, RiskCheckResult,
    RiskEvent, RiskManager,
};

#[cfg(feature = "telegram")]
use crate::service::{TelegramConfig, TelegramNotifier};

/// Main application struct.
pub struct App;

impl App {
    /// Run the main application loop.
    pub async fn run(config: Config) -> Result<()> {
        // Initialize shared state
        let state = Arc::new(AppState::new(config.risk.clone().into()));

        // Initialize risk manager
        let risk_manager = Arc::new(RiskManager::new(state.clone()));

        // Initialize notifiers
        let notifiers = Arc::new(build_notifier_registry(&config));
        info!(notifiers = notifiers.len(), "Notifiers initialized");

        // Initialize executor (optional)
        let executor = init_executor(&config).await;

        // Build strategy registry
        let strategies = Arc::new(build_strategy_registry(&config));
        info!(
            strategies = ?strategies.strategies().iter().map(|s| s.name()).collect::<Vec<_>>(),
            "Strategies loaded"
        );

        // Fetch markets
        let client = PolymarketClient::new(config.network.api_url.clone());
        let markets = client.get_active_markets(20).await?;

        if markets.is_empty() {
            warn!("No active markets found");
            return Ok(());
        }

        let registry = MarketRegistry::from_markets(&markets);

        info!(
            total_markets = markets.len(),
            yes_no_pairs = registry.len(),
            "Markets loaded"
        );

        if registry.is_empty() {
            warn!("No YES/NO market pairs found");
            return Ok(());
        }

        for pair in registry.pairs() {
            debug!(
                market_id = %pair.market_id(),
                question = %pair.question(),
                "Tracking market"
            );
        }

        let token_ids: Vec<String> = registry
            .pairs()
            .iter()
            .flat_map(|p| vec![p.yes_token().to_string(), p.no_token().to_string()])
            .collect();

        info!(tokens = token_ids.len(), "Subscribing to tokens");

        let cache = Arc::new(OrderBookCache::new());
        let registry = Arc::new(registry);

        let handler = WebSocketHandler::new(config.network.ws_url);

        // Clone Arcs for closure
        let cache_clone = cache.clone();
        let registry_clone = registry.clone();
        let strategies_clone = strategies.clone();
        let executor_clone = executor.clone();
        let risk_manager_clone = risk_manager.clone();
        let notifiers_clone = notifiers.clone();
        let state_clone = state.clone();

        handler
            .run(token_ids, move |msg| {
                handle_message(
                    msg,
                    &cache_clone,
                    &registry_clone,
                    &strategies_clone,
                    executor_clone.clone(),
                    &risk_manager_clone,
                    &notifiers_clone,
                    &state_clone,
                );
            })
            .await?;

        Ok(())
    }
}

/// Build notifier registry from configuration.
fn build_notifier_registry(config: &Config) -> NotifierRegistry {
    let mut registry = NotifierRegistry::new();

    // Always add log notifier
    registry.register(Box::new(LogNotifier));

    // Add telegram notifier if configured
    #[cfg(feature = "telegram")]
    if config.telegram.enabled {
        if let Some(tg_config) = TelegramConfig::from_env() {
            let tg_config = TelegramConfig {
                notify_opportunities: config.telegram.notify_opportunities,
                notify_executions: config.telegram.notify_executions,
                notify_risk_rejections: config.telegram.notify_risk_rejections,
                ..tg_config
            };
            registry.register(Box::new(TelegramNotifier::new(tg_config)));
            info!("Telegram notifier enabled");
        } else {
            warn!("Telegram enabled but TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID not set");
        }
    }

    registry
}

/// Build strategy registry from configuration.
fn build_strategy_registry(config: &Config) -> StrategyRegistry {
    let mut registry = StrategyRegistry::new();

    for name in &config.strategies.enabled {
        match name.as_str() {
            "single_condition" => {
                registry.register(Box::new(SingleConditionStrategy::new(
                    config.strategies.single_condition.clone(),
                )));
            }
            "market_rebalancing" => {
                registry.register(Box::new(MarketRebalancingStrategy::new(
                    config.strategies.market_rebalancing.clone(),
                )));
            }
            "combinatorial" => {
                if config.strategies.combinatorial.enabled {
                    registry.register(Box::new(CombinatorialStrategy::new(
                        config.strategies.combinatorial.clone(),
                    )));
                }
            }
            unknown => {
                warn!(strategy = unknown, "Unknown strategy in config, skipping");
            }
        }
    }

    registry
}

/// Initialize the executor if wallet is configured.
async fn init_executor(config: &Config) -> Option<Arc<PolymarketExecutor>> {
    if config.wallet.private_key.is_some() {
        match PolymarketExecutor::new(config).await {
            Ok(exec) => {
                info!("Executor initialized - trading ENABLED");
                Some(Arc::new(exec))
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize executor - detection only");
                None
            }
        }
    } else {
        info!("No wallet configured - detection only mode");
        None
    }
}

/// Handle incoming WebSocket messages.
fn handle_message(
    msg: WsMessage,
    cache: &OrderBookCache,
    registry: &MarketRegistry,
    strategies: &StrategyRegistry,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &NotifierRegistry,
    state: &AppState,
) {
    match msg {
        WsMessage::Book(book) => {
            let orderbook = book.to_orderbook();
            let token_id = orderbook.token_id().clone();
            cache.update(orderbook);

            if let Some(pair) = registry.get_market_for_token(&token_id) {
                let ctx = DetectionContext::new(pair, cache);
                let opportunities = strategies.detect_all(&ctx);

                for opp in opportunities {
                    handle_opportunity(opp, executor.clone(), risk_manager, notifiers, state);
                }
            }
        }
        WsMessage::PriceChange(_) => {}
        _ => {}
    }
}

/// Handle a detected opportunity.
fn handle_opportunity(
    opp: Opportunity,
    executor: Option<Arc<PolymarketExecutor>>,
    risk_manager: &RiskManager,
    notifiers: &NotifierRegistry,
    state: &AppState,
) {
    // Notify opportunity detected
    notifiers.notify_all(Event::OpportunityDetected(OpportunityEvent::from(&opp)));

    // Check risk
    match risk_manager.check(&opp) {
        RiskCheckResult::Approved => {
            if let Some(exec) = executor {
                spawn_execution(exec, opp, notifiers.clone(), state.clone());
            }
        }
        RiskCheckResult::Rejected(error) => {
            notifiers.notify_all(Event::RiskRejected(RiskEvent::new(
                opp.market_id().as_str(),
                &error,
            )));
        }
    }
}

/// Spawn async execution without blocking message processing.
fn spawn_execution(
    executor: Arc<PolymarketExecutor>,
    opportunity: Opportunity,
    notifiers: NotifierRegistry,
    state: AppState,
) {
    let market_id = opportunity.market_id().to_string();

    tokio::spawn(async move {
        match executor.execute_arbitrage(&opportunity).await {
            Ok(result) => {
                // Record position in shared state
                if matches!(result, ArbitrageExecutionResult::Success { .. }) {
                    record_position(&state, &opportunity);
                }

                // Notify execution result
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent::from_result(
                    &market_id, &result,
                )));
            }
            Err(e) => {
                error!(error = %e, "Execution failed");
                notifiers.notify_all(Event::ExecutionCompleted(ExecutionEvent {
                    market_id,
                    success: false,
                    details: e.to_string(),
                }));
            }
        }
    });
}

/// Record a position in shared state.
fn record_position(state: &AppState, opportunity: &Opportunity) {
    use crate::domain::{Position, PositionLeg, PositionStatus};

    let mut positions = state.positions_mut();
    let position = Position::new(
        positions.next_id(),
        opportunity.market_id().clone(),
        vec![
            PositionLeg::new(
                opportunity.yes_token().clone(),
                opportunity.volume(),
                opportunity.yes_ask(),
            ),
            PositionLeg::new(
                opportunity.no_token().clone(),
                opportunity.volume(),
                opportunity.no_ask(),
            ),
        ],
        opportunity.total_cost() * opportunity.volume(),
        opportunity.volume(),
        chrono::Utc::now(),
        PositionStatus::Open,
    );
    positions.add(position);
}

/// Log market pair being tracked.
#[allow(dead_code)]
fn log_market(pair: &MarketPair) {
    debug!(
        market_id = %pair.market_id(),
        question = %pair.question(),
        "Tracking market"
    );
}
```

**Step 2: Fix executor.rs to not own PositionTracker**

In `src/adapter/polymarket/executor.rs`, remove the position tracking since AppState now owns it. Remove:
- The `positions: Mutex<PositionTracker>` field
- The position recording in `execute_arbitrage`
- The `total_exposure` and `open_position_count` methods

Actually, let's keep the executor simpler - just remove the position ownership. The state management is now centralized.

**Step 3: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 4: Run with telegram feature**

```bash
cargo test --features telegram
```
Expected: All tests pass

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: wire risk manager and notifiers into orchestrator"
```

---

### Task 14: Update executor to remove position ownership

**Files:**
- Modify: `src/adapter/polymarket/executor.rs`

**Step 1: Remove position tracking from executor**

Remove these lines from the struct:
```rust
/// Position tracker for recording executed arbitrage positions.
positions: Mutex<PositionTracker>,
```

Remove from `new()`:
```rust
positions: Mutex::new(PositionTracker::new()),
```

Remove position recording from `execute_arbitrage()` (the whole block that creates and adds a position).

Remove these methods:
```rust
pub fn total_exposure(&self) -> Price { ... }
pub fn open_position_count(&self) -> usize { ... }
```

Remove unused imports:
```rust
use crate::domain::{Position, PositionLeg, PositionStatus, PositionTracker, Price};
```

Keep only:
```rust
use crate::domain::{Opportunity, TokenId};
```

**Step 2: Run tests**

```bash
cargo test
```
Expected: All tests pass

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: remove position ownership from executor"
```

---

### Task 15: Update example config and documentation

**Files:**
- Modify: `config.toml`
- Modify: `docs/architecture/system-design.md`
- Modify: `README.md`

**Step 1: Update config.toml with new sections**

Add to config.toml:
```toml
[risk]
max_position_per_market = 1000
max_total_exposure = 10000
min_profit_threshold = 0.05
max_slippage = 0.02

[telegram]
enabled = false
notify_opportunities = false
notify_executions = true
notify_risk_rejections = true
```

**Step 2: Update system-design.md**

Update the module structure section and add documentation for the new services.

**Step 3: Update README.md**

Add configuration examples for risk and telegram.

**Step 4: Run final verification**

```bash
cargo test
cargo test --features telegram
cargo clippy -- -D warnings
```

**Step 5: Commit**

```bash
git add -A && git commit -m "docs: update config and documentation for phase 4"
```

---

## Final Verification

### Task 16: Final cleanup and verification

**Step 1: Run full test suite**

```bash
cargo test --all-features
```
Expected: All tests pass

**Step 2: Check for warnings**

```bash
cargo clippy --all-features -- -D warnings
```
Expected: No errors

**Step 3: Verify build**

```bash
cargo build --release --all-features
```
Expected: Builds successfully

**Step 4: Final commit if any cleanup needed**

```bash
git status
# If changes:
git add -A && git commit -m "chore: final cleanup for phase 4"
```

---

## Summary

**Files Created:**
- `src/adapter/mod.rs`
- `src/adapter/polymarket/` (moved from `src/polymarket/`)
- `src/app/mod.rs`
- `src/app/orchestrator.rs` (from `src/app.rs`)
- `src/app/config.rs` (from `src/config.rs`)
- `src/app/state.rs`
- `src/service/mod.rs`
- `src/service/risk.rs`
- `src/service/notifier.rs`
- `src/service/telegram.rs`

**Files Modified:**
- `src/lib.rs`
- `src/main.rs`
- `src/error.rs`
- `Cargo.toml`
- `config.toml`
- `docs/architecture/system-design.md`
- `README.md`

**New Features:**
- `telegram` feature flag for Telegram notifications

**Environment Variables (for Telegram):**
- `TELEGRAM_BOT_TOKEN` - Bot token from @BotFather
- `TELEGRAM_CHAT_ID` - Chat ID to send notifications to
