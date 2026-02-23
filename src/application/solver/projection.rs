//! Projection solver adapter using Frank-Wolfe algorithm.
//!
//! Implements the [`ProjectionSolver`] port by combining the Frank-Wolfe
//! algorithm with an ILP solver backend.

use std::sync::Arc;

use rust_decimal::Decimal;

use crate::error::Result;
use crate::port::{
    outbound::solver::IlpProblem, outbound::solver::ProjectionResult,
    outbound::solver::ProjectionSolver, outbound::solver::Solver,
};

use super::frank_wolfe::{FrankWolfe, FrankWolfeConfig};

/// Projection solver adapter for cluster and combinatorial detection.
///
/// Combines the Frank-Wolfe algorithm with an ILP solver (e.g., HiGHS)
/// to project market prices onto the marginal polytope and detect arbitrage.
#[derive(Clone)]
pub struct FrankWolfeProjectionSolver {
    /// Frank-Wolfe algorithm instance.
    frank_wolfe: FrankWolfe,
    /// ILP solver for the linear minimization oracle.
    ilp_solver: Arc<dyn Solver>,
}

impl FrankWolfeProjectionSolver {
    /// Create a new projection solver with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the Frank-Wolfe algorithm.
    /// * `ilp_solver` - ILP solver to use as the linear minimization oracle.
    #[must_use]
    pub fn new(config: FrankWolfeConfig, ilp_solver: Arc<dyn Solver>) -> Self {
        Self {
            frank_wolfe: FrankWolfe::new(config),
            ilp_solver,
        }
    }
}

impl ProjectionSolver for FrankWolfeProjectionSolver {
    fn name(&self) -> &'static str {
        "frank_wolfe"
    }

    fn project(&self, theta: &[Decimal], problem: &IlpProblem) -> Result<ProjectionResult> {
        let result = self
            .frank_wolfe
            .project(theta, problem, self.ilp_solver.as_ref())?;

        Ok(ProjectionResult {
            values: result.mu,
            gap: result.gap,
            iterations: result.iterations,
            converged: result.converged,
        })
    }
}
