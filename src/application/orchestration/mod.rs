//! Application orchestration services.
//!
//! Core event processing and execution workflows that coordinate
//! strategy detection, risk gates, and trade execution.

mod context;
mod event;
mod execution;
pub mod handler;
mod opportunity;
mod position;
mod slippage;
