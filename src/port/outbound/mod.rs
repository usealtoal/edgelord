//! Outbound ports (driven side): interfaces implemented by outbound adapters.
//!
//! These contracts describe infrastructure dependencies such as exchanges,
//! storage, solvers, inference, and notifications.

pub mod approval;
pub mod dedup;
pub mod exchange;
pub mod filter;
pub mod inference;
pub mod llm;
pub mod notifier;
pub mod report;
pub mod solver;
pub mod stats;
pub mod store;
