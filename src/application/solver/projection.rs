//! Projection solver adapter backed by Frank-Wolfe + HiGHS.

use std::sync::Arc;

use rust_decimal::Decimal;

use crate::error::Result;
use crate::port::{
    outbound::solver::IlpProblem, outbound::solver::ProjectionResult,
    outbound::solver::ProjectionSolver, outbound::solver::Solver,
};

use super::frank_wolfe::{FrankWolfe, FrankWolfeConfig};

/// Projection adapter used by cluster/combinatorial detection.
#[derive(Clone)]
pub struct FrankWolfeProjectionSolver {
    frank_wolfe: FrankWolfe,
    ilp_solver: Arc<dyn Solver>,
}

impl FrankWolfeProjectionSolver {
    /// Create with explicit Frank-Wolfe configuration.
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
