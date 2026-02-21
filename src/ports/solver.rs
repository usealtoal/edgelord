//! Solver port for linear and integer programming.
//!
//! This module defines the trait for LP/ILP solvers used in
//! combinatorial arbitrage detection.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A linear/integer programming solver.
///
/// Implementations wrap specific solver backends (HiGHS, Gurobi, etc.)
/// and provide a unified interface for optimization problems.
///
/// # Implementation Notes
///
/// - Implementations must be thread-safe (`Send + Sync`)
/// - The solver should handle numerical precision appropriately
/// - Consider caching or warm-starting for repeated similar problems
pub trait Solver: Send + Sync {
    /// Solver name for logging/config.
    fn name(&self) -> &'static str;

    /// Solve: minimize c*x subject to constraints.
    ///
    /// # Arguments
    ///
    /// * `problem` - The linear programming problem to solve
    ///
    /// # Returns
    ///
    /// The optimal solution, or an error if the problem is infeasible/unbounded.
    fn solve_lp(&self, problem: &LpProblem) -> Result<LpSolution>;

    /// Solve with integer constraints on specified variables.
    ///
    /// # Arguments
    ///
    /// * `problem` - The integer linear programming problem to solve
    ///
    /// # Returns
    ///
    /// The optimal integer solution, or an error if infeasible.
    fn solve_ilp(&self, problem: &IlpProblem) -> Result<LpSolution>;
}

/// Linear programming problem definition.
#[derive(Debug, Clone)]
pub struct LpProblem {
    /// Objective coefficients (minimize c*x).
    pub objective: Vec<Decimal>,
    /// Constraints.
    pub constraints: Vec<Constraint>,
    /// Variable bounds.
    pub bounds: Vec<VariableBounds>,
}

impl LpProblem {
    /// Create a new LP problem.
    #[must_use]
    pub fn new(num_vars: usize) -> Self {
        Self {
            objective: vec![Decimal::ZERO; num_vars],
            constraints: Vec::new(),
            bounds: vec![VariableBounds::default(); num_vars],
        }
    }

    /// Number of variables.
    #[must_use]
    pub const fn num_vars(&self) -> usize {
        self.objective.len()
    }
}

/// Integer linear programming problem.
#[derive(Debug, Clone)]
pub struct IlpProblem {
    /// Base LP problem.
    pub lp: LpProblem,
    /// Indices of variables that must be integer.
    pub integer_vars: Vec<usize>,
}

impl IlpProblem {
    /// Create from an LP problem with specified integer variables.
    #[must_use]
    pub const fn new(lp: LpProblem, integer_vars: Vec<usize>) -> Self {
        Self { lp, integer_vars }
    }

    /// Create with all variables as binary (0-1).
    #[must_use]
    pub fn all_binary(lp: LpProblem) -> Self {
        let integer_vars: Vec<usize> = (0..lp.num_vars()).collect();
        Self { lp, integer_vars }
    }
}

/// A single constraint: `sum(coeffs[i] * x[i]) {>=, <=, =} rhs`.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Coefficients for each variable.
    pub coefficients: Vec<Decimal>,
    /// Constraint sense (>=, <=, =).
    pub sense: ConstraintSense,
    /// Right-hand side value.
    pub rhs: Decimal,
}

impl Constraint {
    /// Create a >= constraint.
    #[must_use]
    pub const fn geq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::GreaterEqual,
            rhs,
        }
    }

    /// Create a <= constraint.
    #[must_use]
    pub const fn leq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::LessEqual,
            rhs,
        }
    }

    /// Create an = constraint.
    #[must_use]
    pub const fn eq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::Equal,
            rhs,
        }
    }
}

/// Constraint sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSense {
    GreaterEqual,
    LessEqual,
    Equal,
}

/// Bounds on a variable.
#[derive(Debug, Clone, Copy)]
pub struct VariableBounds {
    /// Lower bound (None = -infinity).
    pub lower: Option<Decimal>,
    /// Upper bound (None = +infinity).
    pub upper: Option<Decimal>,
}

impl Default for VariableBounds {
    fn default() -> Self {
        Self {
            lower: Some(Decimal::ZERO),
            upper: None,
        }
    }
}

impl VariableBounds {
    /// Binary variable bounds [0, 1].
    #[must_use]
    pub const fn binary() -> Self {
        Self {
            lower: Some(Decimal::ZERO),
            upper: Some(Decimal::ONE),
        }
    }

    /// Free variable (no bounds).
    #[must_use]
    pub const fn free() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Non-negative variable [0, +inf).
    #[must_use]
    pub fn non_negative() -> Self {
        Self::default()
    }

    /// Bounded variable [lower, upper].
    #[must_use]
    pub const fn bounded(lower: Decimal, upper: Decimal) -> Self {
        Self {
            lower: Some(lower),
            upper: Some(upper),
        }
    }
}

/// Solution to an LP/ILP problem.
#[derive(Debug, Clone)]
pub struct LpSolution {
    /// Optimal variable values.
    pub values: Vec<Decimal>,
    /// Optimal objective value.
    pub objective: Decimal,
    /// Solver status.
    pub status: SolutionStatus,
}

impl LpSolution {
    /// Check if solution is optimal.
    #[must_use]
    pub fn is_optimal(&self) -> bool {
        self.status == SolutionStatus::Optimal
    }
}

/// Solver solution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    /// Found optimal solution.
    Optimal,
    /// Problem is infeasible.
    Infeasible,
    /// Problem is unbounded.
    Unbounded,
    /// Solver error.
    Error,
}
