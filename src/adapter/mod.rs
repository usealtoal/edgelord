//! Adapter layer split by direction.
//!
//! - `inbound`: driving adapters (CLI, entrypoints)
//! - `outbound`: driven adapters (exchange, storage, notifier, solver, llm)

pub mod inbound;
pub mod outbound;
