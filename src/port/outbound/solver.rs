//! Solver ports for linear and integer programming.
//!
//! Defines traits for mathematical optimization solvers used in combinatorial
//! arbitrage detection. Supports both linear programming (LP) and integer
//! linear programming (ILP) formulations.
//!
//! # Overview
//!
//! - [`Solver`]: Core LP/ILP solver interface
//! - [`ProjectionSolver`]: Projection-based optimization (e.g., Frank-Wolfe)
//! - [`LpProblem`] / [`IlpProblem`]: Problem definitions
//! - [`LpSolution`]: Solution representation

use rust_decimal::Decimal;

use crate::domain::{constraint::Constraint, constraint::VariableBounds};
use crate::error::Result;

/// Linear and integer programming solver.
///
/// Implementations wrap specific solver backends (HiGHS, Gurobi, GLPK, etc.)
/// and provide a unified interface for optimization problems.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support concurrent
/// optimization requests from multiple strategies.
///
/// # Implementation Notes
///
/// - Handle numerical precision appropriately for financial calculations
/// - Consider warm-starting for repeated similar problems
/// - Return appropriate status codes for infeasible or unbounded problems
pub trait Solver: Send + Sync {
    /// Return the solver name for logging and configuration.
    fn name(&self) -> &'static str;

    /// Solve a linear programming problem.
    ///
    /// Minimizes the objective function `c * x` subject to the constraints.
    ///
    /// # Arguments
    ///
    /// * `problem` - The LP problem definition.
    ///
    /// # Errors
    ///
    /// Returns an error if the problem is infeasible, unbounded, or the solver
    /// encounters an internal error.
    fn solve_lp(&self, problem: &LpProblem) -> Result<LpSolution>;

    /// Solve an integer linear programming problem.
    ///
    /// Minimizes the objective function with integer constraints on specified
    /// variables.
    ///
    /// # Arguments
    ///
    /// * `problem` - The ILP problem definition.
    ///
    /// # Errors
    ///
    /// Returns an error if the problem is infeasible or the solver encounters
    /// an internal error.
    fn solve_ilp(&self, problem: &IlpProblem) -> Result<LpSolution>;
}

/// Result of projecting prices onto a feasible polytope.
///
/// Used by projection-based arbitrage detection algorithms to find the nearest
/// feasible point and measure the gap (arbitrage signal).
#[derive(Debug, Clone)]
pub struct ProjectionResult {
    /// Projected values in the feasible region.
    pub values: Vec<Decimal>,

    /// Distance between input and projected values.
    ///
    /// A positive gap indicates arbitrage potential; the magnitude corresponds
    /// to expected profit.
    pub gap: Decimal,

    /// Number of iterations performed by the projection algorithm.
    pub iterations: usize,

    /// Whether the projection algorithm converged to a solution.
    pub converged: bool,
}

/// Projection-based optimization solver.
///
/// Implements projection algorithms (e.g., Frank-Wolfe, projected gradient)
/// for finding the nearest feasible point to a given price vector.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait ProjectionSolver: Send + Sync {
    /// Return the solver name for logging and configuration.
    fn name(&self) -> &'static str;

    /// Project values onto the feasible region defined by the problem constraints.
    ///
    /// # Arguments
    ///
    /// * `theta` - Input price vector to project.
    /// * `problem` - Problem defining the feasible region.
    ///
    /// # Errors
    ///
    /// Returns an error if the projection fails to converge or the problem
    /// has no feasible region.
    fn project(&self, theta: &[Decimal], problem: &IlpProblem) -> Result<ProjectionResult>;
}

/// Linear programming problem definition.
///
/// Represents a minimization problem of the form:
///
/// ```text
/// minimize    c^T * x
/// subject to  constraints
///             bounds on x
/// ```
#[derive(Debug, Clone)]
pub struct LpProblem {
    /// Objective function coefficients.
    ///
    /// The solver minimizes `c^T * x` where `c` is this vector.
    pub objective: Vec<Decimal>,

    /// Linear constraints on the variables.
    pub constraints: Vec<Constraint>,

    /// Lower and upper bounds for each variable.
    pub bounds: Vec<VariableBounds>,
}

impl LpProblem {
    /// Create a new LP problem with the specified number of variables.
    ///
    /// Initializes all objective coefficients to zero and all variable bounds
    /// to their defaults.
    ///
    /// # Arguments
    ///
    /// * `num_vars` - Number of decision variables.
    #[must_use]
    pub fn new(num_vars: usize) -> Self {
        Self {
            objective: vec![Decimal::ZERO; num_vars],
            constraints: Vec::new(),
            bounds: vec![VariableBounds::default(); num_vars],
        }
    }

    /// Return the number of decision variables.
    #[must_use]
    pub fn num_vars(&self) -> usize {
        self.objective.len()
    }
}

/// Integer linear programming problem definition.
///
/// Extends a linear programming problem with integer constraints on specified
/// variables.
#[derive(Debug, Clone)]
pub struct IlpProblem {
    /// Underlying linear programming problem.
    pub lp: LpProblem,

    /// Indices of variables constrained to integer values.
    ///
    /// Variables not in this list are continuous (relaxed).
    pub integer_vars: Vec<usize>,
}

impl IlpProblem {
    /// Create an ILP problem from an LP with specified integer variables.
    ///
    /// # Arguments
    ///
    /// * `lp` - Base linear programming problem.
    /// * `integer_vars` - Indices of variables that must take integer values.
    #[must_use]
    pub const fn new(lp: LpProblem, integer_vars: Vec<usize>) -> Self {
        Self { lp, integer_vars }
    }

    /// Create an ILP with all variables constrained to binary (0 or 1) values.
    ///
    /// # Arguments
    ///
    /// * `lp` - Base linear programming problem.
    #[must_use]
    pub fn all_binary(lp: LpProblem) -> Self {
        let integer_vars: Vec<usize> = (0..lp.num_vars()).collect();
        Self { lp, integer_vars }
    }
}

/// Solution to a linear or integer programming problem.
#[derive(Debug, Clone)]
pub struct LpSolution {
    /// Optimal values for each decision variable.
    pub values: Vec<Decimal>,

    /// Optimal objective function value.
    pub objective: Decimal,

    /// Termination status of the solver.
    pub status: SolutionStatus,
}

impl LpSolution {
    /// Return `true` if the solver found an optimal solution.
    #[must_use]
    pub fn is_optimal(&self) -> bool {
        self.status == SolutionStatus::Optimal
    }
}

/// Termination status of an optimization solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    /// Solver found a globally optimal solution.
    Optimal,

    /// No feasible solution exists.
    Infeasible,

    /// Objective function is unbounded.
    Unbounded,

    /// Solver encountered an internal error.
    Error,
}
