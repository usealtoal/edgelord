//! Trait definitions (hexagonal ports). Depend only on domain.
//!
//! Ports define the extension points in the hexagonal architecture.
//! They are traits that adapters implement to integrate with external
//! systems (exchanges, databases, notification services, etc.).
//!
//! # Architecture
//!
//! ```text
//!                    ┌─────────────────────────┐
//!                    │      Application        │
//!                    │                         │
//!     ┌──────────────┤  Domain + Port          ├──────────────┐
//!     │              │                         │              │
//!     │              └─────────────────────────┘              │
//!     │                         │                             │
//!     ▼                         ▼                             ▼
//! ┌─────────┐            ┌─────────────┐              ┌───────────┐
//! │Exchange │            │  Notifier   │              │  Solver   │
//! │ Adapter │            │   Adapter   │              │  Adapter  │
//! └─────────┘            └─────────────┘              └───────────┘
//! ```
//!
//! # Available Ports
//!
//! - [`MarketDataStream`], [`MarketFetcher`], [`OrderExecutor`], [`ArbitrageExecutor`] - Exchange integration
//! - [`Notifier`] - Event notifications (Telegram, logging, etc.)
//! - [`Solver`] - LP/ILP optimization backend
//! - [`RelationInferrer`] - Market relation discovery
//! - [`Strategy`] - Arbitrage detection strategies
//!
//! Note: Risk management (`adapter::risk`) and storage (`adapter::cache`) use
//! concrete implementations rather than ports, as they are tightly coupled to
//! the application's internal state and data model.

mod exchange;
mod inference;
mod notifier;
mod risk;
mod solver;
mod strategy;

// Exchange ports
pub use exchange::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo,
    OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};

// Inference port
pub use inference::{MarketSummary, RelationInferrer};

// Test utilities
#[cfg(test)]
pub use inference::tests;

// Notifier port
pub use notifier::{
    Event, ExecutionEvent, Notifier, OpportunityEvent, RelationDetail, RelationsEvent, RiskEvent,
    SummaryEvent,
};

// Risk types
pub use risk::RiskCheckResult;

// Solver port
pub use solver::{
    Constraint, ConstraintSense, IlpProblem, LpProblem, LpSolution, SolutionStatus, Solver,
    VariableBounds,
};

// Strategy port
pub use strategy::{DetectionContext, DetectionResult, MarketContext, Strategy};
