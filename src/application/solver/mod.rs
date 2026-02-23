//! Core projection solver components used by application strategies.
//!
//! Implements mathematical algorithms for arbitrage detection in prediction markets:
//!
//! - [`bregman`]: Bregman divergence calculations for LMSR markets
//! - [`frank_wolfe`]: Frank-Wolfe algorithm for projecting onto the marginal polytope
//! - [`projection`]: Adapter implementing the projection solver port

pub mod bregman;
pub mod frank_wolfe;
pub mod projection;
