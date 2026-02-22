//! Solver abstraction for linear and integer programming.
//!
//! This module provides a trait-based abstraction over LP/ILP solvers,
//! allowing different backends (`HiGHS`, Gurobi, etc.) to be swapped.
//!
//! The core types (`Solver`, `LpProblem`, `Constraint`, etc.) are defined
//! in `crate::port::solver` and re-exported here for convenience.

// Allow large error types in Result - Error includes WebSocket variant for unified error handling
#![allow(clippy::result_large_err)]

mod bregman;
mod frank_wolfe;
mod highs;

pub use highs::HiGHSSolver;

pub use bregman::{bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices};
pub use frank_wolfe::{FrankWolfe, FrankWolfeConfig, FrankWolfeResult};

// Re-export solver types from port (canonical definitions)
pub use crate::port::{
    Constraint, ConstraintSense, IlpProblem, LpProblem, LpSolution, SolutionStatus, Solver,
    VariableBounds,
};
