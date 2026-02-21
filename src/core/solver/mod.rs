//! Solver abstraction for linear and integer programming.
//!
//! This module re-exports from `crate::adapters::solvers` for backward compatibility.

pub use crate::adapters::solvers::{
    bregman_divergence, bregman_gradient, lmsr_cost, lmsr_prices, Constraint, ConstraintSense,
    FrankWolfe, FrankWolfeConfig, FrankWolfeResult, HiGHSSolver, IlpProblem, LpProblem, LpSolution,
    SolutionStatus, Solver, VariableBounds,
};
