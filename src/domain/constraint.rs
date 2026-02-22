//! Mathematical constraint types for optimization.
//!
//! These types represent linear constraints used in both domain modeling
//! (market relations) and solver interfaces (LP/ILP problems).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single linear constraint: `sum(coeffs[i] * x[i]) {>=, <=, =} rhs`.
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

/// Constraint sense (comparison operator).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSense {
    /// Greater than or equal (>=).
    GreaterEqual,
    /// Less than or equal (<=).
    LessEqual,
    /// Equal (=).
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
