//! Mathematical constraint types for optimization.
//!
//! These types represent linear constraints used in both domain modeling
//! (market relations) and solver interfaces (LP/ILP problems).
//!
//! # Linear Constraints
//!
//! A linear constraint has the form: `sum(coeffs[i] * x[i]) {>=, <=, =} rhs`
//!
//! For example, "at most one of A, B, C can be true" becomes:
//! `1*A + 1*B + 1*C <= 1`
//!
//! # Examples
//!
//! Creating constraints:
//!
//! ```
//! use edgelord::domain::constraint::{Constraint, ConstraintSense, VariableBounds};
//! use rust_decimal_macros::dec;
//!
//! // At most one of three outcomes: x0 + x1 + x2 <= 1
//! let mutual_exclusion = Constraint::leq(
//!     vec![dec!(1), dec!(1), dec!(1)],
//!     dec!(1),
//! );
//!
//! // Exactly one must be true: x0 + x1 = 1
//! let exactly_one = Constraint::eq(
//!     vec![dec!(1), dec!(1)],
//!     dec!(1),
//! );
//!
//! // Variable bounds for binary variables
//! let bounds = VariableBounds::binary();
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single linear constraint of the form `sum(coeffs[i] * x[i]) {>=, <=, =} rhs`.
///
/// Used to encode logical relationships between market probabilities for
/// arbitrage detection via linear programming.
///
/// # Examples
///
/// ```
/// use edgelord::domain::constraint::Constraint;
/// use rust_decimal_macros::dec;
///
/// // Implication: P(A) <= P(B), encoded as A - B <= 0
/// let implies = Constraint::leq(vec![dec!(1), dec!(-1)], dec!(0));
///
/// // Mutual exclusion: A + B + C <= 1
/// let exclusive = Constraint::leq(vec![dec!(1), dec!(1), dec!(1)], dec!(1));
/// ```
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Coefficient for each decision variable.
    pub coefficients: Vec<Decimal>,
    /// Comparison operator (>=, <=, or =).
    pub sense: ConstraintSense,
    /// Right-hand side constant value.
    pub rhs: Decimal,
}

impl Constraint {
    /// Creates a greater-than-or-equal constraint (>=).
    #[must_use]
    pub const fn geq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::GreaterEqual,
            rhs,
        }
    }

    /// Creates a less-than-or-equal constraint (<=).
    #[must_use]
    pub const fn leq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::LessEqual,
            rhs,
        }
    }

    /// Creates an equality constraint (=).
    #[must_use]
    pub const fn eq(coefficients: Vec<Decimal>, rhs: Decimal) -> Self {
        Self {
            coefficients,
            sense: ConstraintSense::Equal,
            rhs,
        }
    }
}

/// Comparison operator for a linear constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSense {
    /// Greater than or equal (>=).
    GreaterEqual,
    /// Less than or equal (<=).
    LessEqual,
    /// Equal (=).
    Equal,
}

/// Bounds on a decision variable in an optimization problem.
///
/// Specifies the allowable range for a variable. Use `None` to indicate
/// unbounded in that direction.
///
/// # Examples
///
/// ```
/// use edgelord::domain::constraint::VariableBounds;
/// use rust_decimal_macros::dec;
///
/// // Binary variable: 0 <= x <= 1
/// let binary = VariableBounds::binary();
///
/// // Non-negative: 0 <= x < infinity
/// let positive = VariableBounds::non_negative();
///
/// // Specific range: 10 <= x <= 100
/// let bounded = VariableBounds::bounded(dec!(10), dec!(100));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct VariableBounds {
    /// Lower bound, or `None` for negative infinity.
    pub lower: Option<Decimal>,
    /// Upper bound, or `None` for positive infinity.
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
    /// Creates bounds for a binary variable [0, 1].
    #[must_use]
    pub const fn binary() -> Self {
        Self {
            lower: Some(Decimal::ZERO),
            upper: Some(Decimal::ONE),
        }
    }

    /// Creates bounds for a free (unbounded) variable.
    #[must_use]
    pub const fn free() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Creates bounds for a non-negative variable [0, +infinity).
    #[must_use]
    pub fn non_negative() -> Self {
        Self::default()
    }

    /// Creates bounds with specific lower and upper limits.
    #[must_use]
    pub const fn bounded(lower: Decimal, upper: Decimal) -> Self {
        Self {
            lower: Some(lower),
            upper: Some(upper),
        }
    }
}
