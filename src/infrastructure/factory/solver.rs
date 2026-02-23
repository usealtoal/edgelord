//! Solver factory.

use std::sync::Arc;

use crate::adapter::outbound::solver::highs::HiGHSSolver;
use crate::application::solver::frank_wolfe::FrankWolfeConfig;
use crate::application::solver::projection::FrankWolfeProjectionSolver;
use crate::port::outbound::solver::ProjectionSolver;

/// Build the default projection solver for cluster/combinatorial detection.
pub fn build_projection_solver() -> Arc<dyn ProjectionSolver> {
    Arc::new(FrankWolfeProjectionSolver::new(
        FrankWolfeConfig::default(),
        Arc::new(HiGHSSolver::new()),
    ))
}
