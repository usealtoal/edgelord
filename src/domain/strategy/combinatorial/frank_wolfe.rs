//! Frank-Wolfe algorithm for Bregman projection.
//!
//! The Frank-Wolfe (conditional gradient) algorithm solves:
//!   min_{μ ∈ M} D(μ || θ)
//!
//! where D is the Bregman divergence and M is the marginal polytope.
//!
//! Instead of full projection, it uses a linear minimization oracle (ILP)
//! to iteratively improve the solution.

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
    pub fn new(config: FrankWolfeConfig) -> Self {
        Self { config }
    }

    /// Get the configuration.
    pub fn config(&self) -> &FrankWolfeConfig {
        &self.config
    }

    /// Run Frank-Wolfe projection.
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

        // Initialize mu at theta (or feasible point if theta infeasible)
        let mut mu = theta.to_vec();
        let mut iterations = 0;
        let mut gap = Decimal::MAX;

        for _ in 0..self.config.max_iterations {
            iterations += 1;

            // Compute gradient of Bregman divergence at mu
            let grad = bregman_gradient(&mu, theta);

            // Solve linear minimization oracle: min_{s ∈ M} <grad, s>
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

            // Compute Frank-Wolfe gap: <grad, mu - s>
            gap = grad
                .iter()
                .zip(mu.iter())
                .zip(s.iter())
                .map(|((g, m), si)| *g * (*m - *si))
                .sum();

            // Check convergence
            if gap.abs() < self.config.tolerance {
                break;
            }

            // Line search: find optimal step size gamma
            // For LMSR, closed-form gamma is complex; use simple 2/(t+2) schedule
            let gamma = Decimal::TWO / Decimal::from(iterations + 2);

            // Update: mu = (1 - gamma) * mu + gamma * s
            let one_minus_gamma = Decimal::ONE - gamma;
            for i in 0..n {
                mu[i] = one_minus_gamma * mu[i] + gamma * s[i];
            }
        }

        // Compute final divergence (arbitrage profit potential)
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
