//! Frank-Wolfe algorithm for Bregman projection.
//!
//! The Frank-Wolfe (conditional gradient) algorithm solves:
//!   min_{μ ∈ M} D(μ || θ)
//!
//! where D is the Bregman divergence and M is the marginal polytope.
//!
//! Instead of full projection, it uses a linear minimization oracle (ILP)
//! to iteratively improve the solution.

// Allow large error types - inherited from crate's unified Error type
#![allow(clippy::result_large_err)]

use rust_decimal::Decimal;

use crate::domain::solver::{IlpProblem, LpProblem, Solver, SolutionStatus};
use crate::error::Result;

use super::bregman::{bregman_divergence, bregman_gradient};

/// Configuration for Frank-Wolfe algorithm.
#[derive(Debug, Clone)]
pub struct FrankWolfeConfig {
    /// Maximum iterations.
    pub max_iterations: usize,
    /// Convergence tolerance.
    pub tolerance: Decimal,
}

impl Default for FrankWolfeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            tolerance: Decimal::new(1, 4), // 0.0001
        }
    }
}

/// Frank-Wolfe algorithm state.
pub struct FrankWolfe {
    config: FrankWolfeConfig,
}

impl FrankWolfe {
    /// Create a new Frank-Wolfe instance with the given configuration.
    #[must_use] 
    pub fn new(config: FrankWolfeConfig) -> Self {
        Self { config }
    }

    /// Get the configuration.
    #[must_use] 
    pub fn config(&self) -> &FrankWolfeConfig {
        &self.config
    }

    /// Run Frank-Wolfe projection.
    ///
    /// The Frank-Wolfe algorithm (also known as conditional gradient descent) projects
    /// market prices onto the marginal polytope M by iteratively solving:
    ///   min_{mu ∈ M} D(mu || theta)
    ///
    /// where D is the Bregman divergence. This is useful for arbitrage detection:
    /// if theta lies outside M, the projection distance indicates arbitrage profit.
    ///
    /// # Arguments
    /// * `theta` - Current market prices (may be outside M)
    /// * `ilp_problem` - ILP defining the feasible set M
    /// * `solver` - ILP solver to use
    ///
    /// # Returns
    /// * `FrankWolfeResult` with projected prices and arbitrage gap
    pub fn project<S: Solver>(
        &self,
        theta: &[Decimal],
        ilp_problem: &IlpProblem,
        solver: &S,
    ) -> Result<FrankWolfeResult> {
        let n = theta.len();
        if n == 0 {
            return Ok(FrankWolfeResult {
                mu: vec![],
                gap: Decimal::ZERO,
                iterations: 0,
                converged: true,
            });
        }

        // ========================================================================
        // STEP 1: Initialize with current prices
        // ========================================================================
        // Start with mu = theta. If theta is already in M, we converge immediately.
        // If theta is outside M, the algorithm will iteratively move mu toward M.
        let mut mu = theta.to_vec();
        let mut iterations = 0;
        let mut gap = Decimal::MAX;

        for _ in 0..self.config.max_iterations {
            iterations += 1;

            // ====================================================================
            // STEP 2: Compute Bregman gradient
            // ====================================================================
            // The gradient of D(mu || theta) with respect to mu tells us the
            // direction of steepest ascent in divergence. For LMSR's Bregman
            // divergence, this is: grad_i = log(mu_i) - log(theta_i)
            let grad = bregman_gradient(&mu, theta);

            // ====================================================================
            // STEP 3: Solve ILP oracle to find minimizing vertex
            // ====================================================================
            // The key insight of Frank-Wolfe: instead of projecting directly onto M
            // (which requires solving a complex optimization), we solve a LINEAR
            // minimization over M: find s = argmin_{s ∈ M} <grad, s>
            //
            // For prediction markets, M is the marginal polytope (valid probability
            // distributions), and the ILP finds the vertex (extreme point) of M
            // that most decreases the Bregman divergence.
            let oracle_problem = IlpProblem {
                lp: LpProblem {
                    objective: grad.clone(),
                    constraints: ilp_problem.lp.constraints.clone(),
                    bounds: ilp_problem.lp.bounds.clone(),
                },
                integer_vars: ilp_problem.integer_vars.clone(),
            };

            let solution = solver.solve_ilp(&oracle_problem)?;

            if solution.status != SolutionStatus::Optimal {
                break;
            }

            let s = &solution.values;

            // ====================================================================
            // STEP 5: Check convergence via duality gap
            // ====================================================================
            // The Frank-Wolfe gap <grad, mu - s> provides an upper bound on the
            // suboptimality of the current solution. When gap ≈ 0, we've found
            // the projection. This gap also approximates the arbitrage profit:
            // it measures how far theta is from the nearest valid price vector.
            gap = grad
                .iter()
                .zip(mu.iter())
                .zip(s.iter())
                .map(|((g, m), si)| *g * (*m - *si))
                .sum();

            if gap.abs() < self.config.tolerance {
                break;
            }

            // ====================================================================
            // STEP 4: Update toward the minimizing vertex with step size
            // ====================================================================
            // Move mu toward the oracle solution s using a convex combination:
            //   mu_new = (1 - gamma) * mu + gamma * s
            //
            // The step size gamma ∈ [0,1] determines how far to move. We use the
            // classic 2/(t+2) schedule which guarantees O(1/t) convergence rate.
            // For LMSR, exact line search is complex, so this schedule works well.
            let gamma = Decimal::TWO / Decimal::from(iterations + 2);

            let one_minus_gamma = Decimal::ONE - gamma;
            for i in 0..n {
                mu[i] = one_minus_gamma * mu[i] + gamma * s[i];
            }
        }

        // Compute final divergence (arbitrage profit potential)
        // This measures how far the original prices theta were from the polytope M
        let divergence = bregman_divergence(&mu, theta);

        Ok(FrankWolfeResult {
            mu,
            gap: divergence,
            iterations,
            converged: gap.abs() < self.config.tolerance,
        })
    }
}

/// Result of Frank-Wolfe projection.
#[derive(Debug, Clone)]
pub struct FrankWolfeResult {
    /// Projected prices (on or near the marginal polytope).
    pub mu: Vec<Decimal>,
    /// Final gap (approximates arbitrage profit).
    pub gap: Decimal,
    /// Number of iterations run.
    pub iterations: usize,
    /// Whether algorithm converged.
    pub converged: bool,
}

impl FrankWolfeResult {
    /// Check if significant arbitrage exists.
    #[must_use] 
    pub fn has_arbitrage(&self, threshold: Decimal) -> bool {
        self.gap > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::solver::{Constraint, ConstraintSense, HiGHSSolver, VariableBounds};
    use rust_decimal_macros::dec;

    #[test]
    fn test_frank_wolfe_config_default() {
        let config = FrankWolfeConfig::default();
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.tolerance, dec!(0.0001));
    }

    #[test]
    fn test_frank_wolfe_empty_input() {
        let config = FrankWolfeConfig::default();
        let fw = FrankWolfe::new(config);
        let solver = HiGHSSolver::new();

        let ilp = IlpProblem {
            lp: LpProblem {
                objective: vec![],
                constraints: vec![],
                bounds: vec![],
            },
            integer_vars: vec![],
        };

        let result = fw.project(&[], &ilp, &solver).unwrap();

        assert!(result.mu.is_empty());
        assert_eq!(result.gap, Decimal::ZERO);
        assert!(result.converged);
    }

    #[test]
    fn test_frank_wolfe_simple_projection() {
        let config = FrankWolfeConfig {
            max_iterations: 10,
            tolerance: dec!(0.001),
        };
        let fw = FrankWolfe::new(config);
        let solver = HiGHSSolver::new();

        // Simple 2-outcome market: probabilities must sum to 1
        // theta = [0.3, 0.3] sums to 0.6 (arbitrage!)
        let theta = vec![dec!(0.3), dec!(0.3)];

        // ILP: x1 + x2 = 1, x in [0,1]
        let ilp = IlpProblem {
            lp: LpProblem {
                objective: vec![Decimal::ZERO; 2], // Will be replaced by gradient
                constraints: vec![Constraint {
                    coefficients: vec![Decimal::ONE, Decimal::ONE],
                    sense: ConstraintSense::Equal,
                    rhs: Decimal::ONE,
                }],
                bounds: vec![VariableBounds::binary(); 2],
            },
            integer_vars: vec![], // LP relaxation for this test
        };

        let result = fw.project(&theta, &ilp, &solver).unwrap();

        // Projected prices should sum closer to 1
        let sum: Decimal = result.mu.iter().sum();
        assert!(sum > dec!(0.9), "Sum should be close to 1, got {}", sum);
    }

    #[test]
    fn test_frank_wolfe_no_arbitrage() {
        let config = FrankWolfeConfig {
            max_iterations: 10,
            tolerance: dec!(0.001),
        };
        let fw = FrankWolfe::new(config);
        let solver = HiGHSSolver::new();

        // theta = [0.5, 0.5] already sums to 1 (no arbitrage)
        let theta = vec![dec!(0.5), dec!(0.5)];

        let ilp = IlpProblem {
            lp: LpProblem {
                objective: vec![Decimal::ZERO; 2],
                constraints: vec![Constraint {
                    coefficients: vec![Decimal::ONE, Decimal::ONE],
                    sense: ConstraintSense::Equal,
                    rhs: Decimal::ONE,
                }],
                bounds: vec![VariableBounds::binary(); 2],
            },
            integer_vars: vec![],
        };

        let result = fw.project(&theta, &ilp, &solver).unwrap();

        // Should have minimal gap
        assert!(
            result.gap < dec!(0.01),
            "No arbitrage case should have small gap, got {}",
            result.gap
        );
    }

    #[test]
    fn test_frank_wolfe_result_has_arbitrage() {
        let result = FrankWolfeResult {
            mu: vec![dec!(0.5), dec!(0.5)],
            gap: dec!(0.05),
            iterations: 5,
            converged: true,
        };

        assert!(result.has_arbitrage(dec!(0.02)));
        assert!(!result.has_arbitrage(dec!(0.10)));
    }
}
