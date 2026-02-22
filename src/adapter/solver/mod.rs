//! Solver implementations for linear and integer programming.
//!
//! Implements the `port::Solver` trait with concrete backends.

#![allow(clippy::result_large_err)]

mod bregman;
mod frank_wolfe;
mod highs;

pub use bregman::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};
pub use highs::HiGHSSolver;
