//! Solver factory.
//!
//! Provides factory functions for constructing optimization solvers used
//! by arbitrage detection strategies.

use std::sync::Arc;

use crate::adapter::outbound::solver::highs::HiGHSSolver;
use crate::application::solver::frank_wolfe::FrankWolfeConfig;
use crate::application::solver::projection::FrankWolfeProjectionSolver;
use crate::port::outbound::solver::ProjectionSolver;

/// Build the default projection solver for cluster and combinatorial detection.
///
/// Creates a Frank-Wolfe projection solver backed by the HiGHS linear
/// programming solver. Used for optimizing trade allocations across
/// multi-market arbitrage opportunities.
pub fn build_projection_solver() -> Arc<dyn ProjectionSolver> {
    Arc::new(FrankWolfeProjectionSolver::new(
        FrankWolfeConfig::default(),
        Arc::new(HiGHSSolver::new()),
    ))
}
