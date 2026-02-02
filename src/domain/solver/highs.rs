//! HiGHS solver implementation via good_lp.
//!
//! HiGHS is a high-performance open-source linear/mixed-integer programming solver.
//! This implementation wraps it using the good_lp crate for ergonomic Rust usage.

use good_lp::solvers::highs::highs;
use good_lp::{constraint, variable, variables, Expression, Solution, SolverModel};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use super::{ConstraintSense, IlpProblem, LpProblem, LpSolution, SolutionStatus, Solver};
use crate::error::Result;

/// HiGHS-based LP/ILP solver.
#[derive(Debug, Default, Clone)]
pub struct HiGHSSolver;

impl HiGHSSolver {
    /// Create a new HiGHS solver instance.
    pub fn new() -> Self {
        Self
    }
}

impl Solver for HiGHSSolver {
    fn name(&self) -> &'static str {
        "highs"
    }

    fn solve_lp(&self, problem: &LpProblem) -> Result<LpSolution> {
        solve_with_good_lp(problem, &[])
    }

    fn solve_ilp(&self, problem: &IlpProblem) -> Result<LpSolution> {
        solve_with_good_lp(&problem.lp, &problem.integer_vars)
    }
}

/// Internal solver implementation using good_lp.
fn solve_with_good_lp(problem: &LpProblem, integer_vars: &[usize]) -> Result<LpSolution> {
    let n = problem.num_vars();

    // Handle empty problem
    if n == 0 {
        return Ok(LpSolution {
            values: vec![],
            objective: Decimal::ZERO,
            status: SolutionStatus::Optimal,
        });
    }

    // Create variables
    let mut vars = variables!();
    let mut var_list = Vec::with_capacity(n);

    for (i, bounds) in problem.bounds.iter().enumerate() {
        let mut v = variable();

        // Apply bounds
        if let Some(lb) = bounds.lower {
            v = v.min(lb.to_f64().unwrap_or(0.0));
        }
        if let Some(ub) = bounds.upper {
            v = v.max(ub.to_f64().unwrap_or(f64::INFINITY));
        }

        // Mark as integer if needed
        if integer_vars.contains(&i) {
            v = v.integer();
        }

        var_list.push(vars.add(v));
    }

    // Build objective function
    let objective: Expression = var_list
        .iter()
        .zip(problem.objective.iter())
        .map(|(v, c)| c.to_f64().unwrap_or(0.0) * *v)
        .sum();

    // Start building the model
    let mut model = vars.minimise(&objective).using(highs);

    // Add constraints
    for constr in &problem.constraints {
        let lhs: Expression = var_list
            .iter()
            .zip(constr.coefficients.iter())
            .map(|(v, c)| c.to_f64().unwrap_or(0.0) * *v)
            .sum();

        let rhs = constr.rhs.to_f64().unwrap_or(0.0);

        match constr.sense {
            ConstraintSense::GreaterEqual => {
                model = model.with(constraint!(lhs >= rhs));
            }
            ConstraintSense::LessEqual => {
                model = model.with(constraint!(lhs <= rhs));
            }
            ConstraintSense::Equal => {
                model = model.with(constraint!(lhs == rhs));
            }
        }
    }

    // Solve
    match model.solve() {
        Ok(solution) => {
            let values: Vec<Decimal> = var_list
                .iter()
                .map(|v| Decimal::try_from(solution.value(*v)).unwrap_or(Decimal::ZERO))
                .collect();

            // Re-evaluate objective with the solved values
            let obj_value: f64 = values
                .iter()
                .zip(problem.objective.iter())
                .map(|(v, c)| v.to_f64().unwrap_or(0.0) * c.to_f64().unwrap_or(0.0))
                .sum();

            Ok(LpSolution {
                values,
                objective: Decimal::try_from(obj_value).unwrap_or(Decimal::ZERO),
                status: SolutionStatus::Optimal,
            })
        }
        Err(_) => {
            // good_lp returns error for infeasible/unbounded
            Ok(LpSolution {
                values: vec![Decimal::ZERO; n],
                objective: Decimal::ZERO,
                status: SolutionStatus::Infeasible,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::solver::{Constraint, VariableBounds};
    use rust_decimal_macros::dec;

    #[test]
    fn test_solver_name() {
        let solver = HiGHSSolver::new();
        assert_eq!(solver.name(), "highs");
    }

    #[test]
    fn test_simple_lp() {
        // Minimize: x + y
        // Subject to: x + y >= 1
        //            x, y >= 0
        let solver = HiGHSSolver::new();

        let problem = LpProblem {
            objective: vec![Decimal::ONE, Decimal::ONE],
            constraints: vec![Constraint::geq(
                vec![Decimal::ONE, Decimal::ONE],
                Decimal::ONE,
            )],
            bounds: vec![VariableBounds::non_negative(); 2],
        };

        let solution = solver.solve_lp(&problem).unwrap();

        assert!(solution.is_optimal());
        // Optimal: x=1, y=0 or x=0, y=1 or some combination summing to 1
        let sum: Decimal = solution.values.iter().sum();
        assert!(
            (sum - Decimal::ONE).abs() < dec!(0.01),
            "Sum should be ~1, got {}",
            sum
        );
    }

    #[test]
    fn test_binary_ilp() {
        // Minimize: -x - y (maximize x + y)
        // Subject to: x + y <= 1
        //            x, y in {0, 1}
        let solver = HiGHSSolver::new();

        let lp = LpProblem {
            objective: vec![-Decimal::ONE, -Decimal::ONE],
            constraints: vec![Constraint::leq(
                vec![Decimal::ONE, Decimal::ONE],
                Decimal::ONE,
            )],
            bounds: vec![VariableBounds::binary(); 2],
        };

        let ilp = IlpProblem::all_binary(lp);
        let solution = solver.solve_ilp(&ilp).unwrap();

        assert!(solution.is_optimal());
        // Optimal: one variable is 1, other is 0
        let sum: Decimal = solution.values.iter().sum();
        assert!(
            (sum - Decimal::ONE).abs() < dec!(0.01),
            "Sum should be 1, got {}",
            sum
        );
    }

    #[test]
    fn test_equality_constraint() {
        // Minimize: x
        // Subject to: x + y = 2
        //            x, y >= 0
        let solver = HiGHSSolver::new();

        let problem = LpProblem {
            objective: vec![Decimal::ONE, Decimal::ZERO],
            constraints: vec![Constraint::eq(
                vec![Decimal::ONE, Decimal::ONE],
                dec!(2),
            )],
            bounds: vec![VariableBounds::non_negative(); 2],
        };

        let solution = solver.solve_lp(&problem).unwrap();

        assert!(solution.is_optimal());
        // Optimal: x=0, y=2
        assert!(
            solution.values[0].abs() < dec!(0.01),
            "x should be ~0, got {}",
            solution.values[0]
        );
        assert!(
            (solution.values[1] - dec!(2)).abs() < dec!(0.01),
            "y should be ~2, got {}",
            solution.values[1]
        );
    }

    #[test]
    fn test_empty_problem() {
        let solver = HiGHSSolver::new();
        let problem = LpProblem::new(0);
        let solution = solver.solve_lp(&problem).unwrap();

        assert!(solution.is_optimal());
        assert!(solution.values.is_empty());
    }

    #[test]
    fn test_probability_simplex() {
        // Project onto probability simplex: minimize ||x - p||^2
        // For simplicity, minimize sum(x) subject to sum(x) = 1
        let solver = HiGHSSolver::new();

        let problem = LpProblem {
            objective: vec![Decimal::ONE, Decimal::ONE, Decimal::ONE],
            constraints: vec![Constraint::eq(
                vec![Decimal::ONE, Decimal::ONE, Decimal::ONE],
                Decimal::ONE,
            )],
            bounds: vec![VariableBounds::bounded(Decimal::ZERO, Decimal::ONE); 3],
        };

        let solution = solver.solve_lp(&problem).unwrap();

        assert!(solution.is_optimal());
        let sum: Decimal = solution.values.iter().sum();
        assert!(
            (sum - Decimal::ONE).abs() < dec!(0.01),
            "Probabilities should sum to 1, got {}",
            sum
        );
    }
}
