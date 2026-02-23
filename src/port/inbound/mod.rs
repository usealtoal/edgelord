//! Inbound ports (driving side): interfaces consumed by inbound adapters.
//!
//! These contracts expose application capabilities to drivers such as CLI,
//! control surfaces, and orchestration entry points.

pub mod operator;
pub mod risk;
pub mod runtime;
pub mod strategy;
