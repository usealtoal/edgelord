//! Relation types for market dependency inference.
//!
//! This module provides types for representing logical relationships between
//! prediction markets:
//!
//! - [`Relation`] - A logical relation with confidence and expiration
//! - [`RelationKind`] - The type of relationship (implies, exclusive, etc.)
//!
//! # Relation Types
//!
//! - **Implies**: If market A resolves YES, market B must resolve YES
//! - **MutuallyExclusive**: At most one of the markets can resolve YES
//! - **ExactlyOne**: Exactly one of the markets must resolve YES
//! - **Linear**: Custom linear constraint on market probabilities
//!
//! # Examples
//!
//! Creating a mutual exclusion relation:
//!
//! ```
//! use edgelord::domain::relation::{Relation, RelationKind};
//! use edgelord::domain::id::MarketId;
//!
//! let relation = Relation::new(
//!     RelationKind::MutuallyExclusive {
//!         markets: vec![
//!             MarketId::new("trump-wins"),
//!             MarketId::new("biden-wins"),
//!             MarketId::new("other-wins"),
//!         ],
//!     },
//!     0.95,  // 95% confidence
//!     "Only one candidate can win the election",
//! );
//!
//! assert!(!relation.is_expired());
//! assert_eq!(relation.market_ids().len(), 3);
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::constraint::{Constraint, ConstraintSense};
use super::id::{MarketId, RelationId};

/// A logical relation between prediction markets with confidence and expiration.
///
/// Relations are inferred by the LLM inference system and represent
/// logical dependencies like "A implies B" or "exactly one of A, B, C".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    /// Unique identifier for this relation.
    pub id: RelationId,
    /// The type and semantics of the relation.
    pub kind: RelationKind,
    /// Confidence score (0.0 to 1.0) from the inferrer.
    pub confidence: f64,
    /// Human-readable reasoning for debugging and audit.
    pub reasoning: String,
    /// Timestamp when this relation was inferred.
    pub inferred_at: DateTime<Utc>,
    /// Timestamp when this relation expires and needs re-validation.
    pub expires_at: DateTime<Utc>,
}

impl Relation {
    /// Creates a new relation with the given kind and confidence.
    ///
    /// The relation is assigned a default TTL of 1 hour. Use [`with_ttl`](Self::with_ttl)
    /// to customize the expiration.
    pub fn new(kind: RelationKind, confidence: f64, reasoning: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: RelationId::new(),
            kind,
            confidence,
            reasoning: reasoning.into(),
            inferred_at: now,
            expires_at: now + chrono::Duration::hours(1),
        }
    }

    /// Sets a custom time-to-live for this relation.
    pub fn with_ttl(mut self, ttl: chrono::Duration) -> Self {
        self.expires_at = self.inferred_at + ttl;
        self
    }

    /// Returns true if this relation has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Returns all market IDs referenced by this relation.
    pub fn market_ids(&self) -> Vec<&MarketId> {
        self.kind.market_ids()
    }
}

/// The type of logical relationship between markets.
///
/// Each variant encodes a different logical constraint that can be converted
/// to linear constraints for optimization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelationKind {
    /// If market A resolves YES, market B must resolve YES.
    ///
    /// Constraint: P(A) <= P(B), encoded as A - B <= 0
    Implies {
        /// Market that implies the other (the "if" market).
        if_yes: MarketId,
        /// Market that must be true when `if_yes` is true (the "then" market).
        then_yes: MarketId,
    },

    /// At most one of these markets can resolve YES.
    ///
    /// Constraint: sum of all market probabilities <= 1
    MutuallyExclusive {
        /// Markets in the mutually exclusive set.
        markets: Vec<MarketId>,
    },

    /// Exactly one of these markets must resolve YES.
    ///
    /// Constraint: sum of all market probabilities = 1
    ExactlyOne {
        /// Markets where exactly one must resolve YES.
        markets: Vec<MarketId>,
    },

    /// Custom linear constraint on market probabilities.
    ///
    /// Constraint: sum(coeff_i * P(market_i)) {<=, =, >=} rhs
    Linear {
        /// Coefficient terms as (market, coefficient) pairs.
        terms: Vec<(MarketId, Decimal)>,
        /// Comparison operator (<=, =, >=).
        sense: ConstraintSense,
        /// Right-hand side constant.
        rhs: Decimal,
    },
}

impl RelationKind {
    /// Returns all market IDs referenced by this relation kind.
    pub fn market_ids(&self) -> Vec<&MarketId> {
        match self {
            Self::Implies { if_yes, then_yes } => vec![if_yes, then_yes],
            Self::MutuallyExclusive { markets } | Self::ExactlyOne { markets } => {
                markets.iter().collect()
            }
            Self::Linear { terms, .. } => terms.iter().map(|(m, _)| m).collect(),
        }
    }

    /// Returns the type name as a static string.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Implies { .. } => "implies",
            Self::MutuallyExclusive { .. } => "mutually_exclusive",
            Self::ExactlyOne { .. } => "exactly_one",
            Self::Linear { .. } => "linear",
        }
    }

    /// Converts this relation to solver constraints.
    ///
    /// # Arguments
    ///
    /// * `market_indices` - Mapping from market ID to variable index in the ILP
    ///
    /// # Returns
    ///
    /// A vector of linear constraints encoding this relation.
    pub fn to_solver_constraints(
        &self,
        market_indices: &HashMap<MarketId, usize>,
    ) -> Vec<Constraint> {
        let num_vars = market_indices.len();

        match self {
            Self::Implies { if_yes, then_yes } => {
                // μ_A - μ_B ≤ 0  =>  μ_A ≤ μ_B
                let mut coeffs = vec![Decimal::ZERO; num_vars];
                if let (Some(&i), Some(&j)) =
                    (market_indices.get(if_yes), market_indices.get(then_yes))
                {
                    coeffs[i] = Decimal::ONE;
                    coeffs[j] = -Decimal::ONE;
                }
                vec![Constraint::leq(coeffs, Decimal::ZERO)]
            }
            Self::MutuallyExclusive { markets } => {
                // Σ μ_i ≤ 1
                let mut coeffs = vec![Decimal::ZERO; num_vars];
                for m in markets {
                    if let Some(&i) = market_indices.get(m) {
                        coeffs[i] = Decimal::ONE;
                    }
                }
                vec![Constraint::leq(coeffs, Decimal::ONE)]
            }
            Self::ExactlyOne { markets } => {
                // Σ μ_i = 1
                let mut coeffs = vec![Decimal::ZERO; num_vars];
                for m in markets {
                    if let Some(&i) = market_indices.get(m) {
                        coeffs[i] = Decimal::ONE;
                    }
                }
                vec![Constraint::eq(coeffs, Decimal::ONE)]
            }
            Self::Linear { terms, sense, rhs } => {
                let mut coeffs = vec![Decimal::ZERO; num_vars];
                for (market_id, coeff) in terms {
                    if let Some(&i) = market_indices.get(market_id) {
                        coeffs[i] = *coeff;
                    }
                }
                vec![match sense {
                    ConstraintSense::LessEqual => Constraint::leq(coeffs, *rhs),
                    ConstraintSense::GreaterEqual => Constraint::geq(coeffs, *rhs),
                    ConstraintSense::Equal => Constraint::eq(coeffs, *rhs),
                }]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn market(s: &str) -> MarketId {
        MarketId::new(s)
    }

    #[test]
    fn relation_new_sets_timestamps() {
        let rel = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("a"), market("b")],
            },
            0.9,
            "test reasoning",
        );

        assert!(rel.inferred_at <= Utc::now());
        assert!(rel.expires_at > rel.inferred_at);
    }

    #[test]
    fn relation_with_ttl_sets_expiration() {
        let rel = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("a"), market("b")],
            },
            0.9,
            "test",
        )
        .with_ttl(chrono::Duration::hours(24));

        let expected_expiry = rel.inferred_at + chrono::Duration::hours(24);
        assert_eq!(rel.expires_at, expected_expiry);
    }

    #[test]
    fn relation_is_expired_returns_false_for_fresh() {
        let rel = Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("a"), market("b")],
            },
            0.9,
            "test",
        );

        assert!(!rel.is_expired());
    }

    #[test]
    fn relation_market_ids_returns_all_markets() {
        let rel = Relation::new(
            RelationKind::ExactlyOne {
                markets: vec![market("a"), market("b"), market("c")],
            },
            0.95,
            "test",
        );

        let ids: Vec<&str> = rel.market_ids().iter().map(|m| m.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn relation_kind_implies_market_ids() {
        let kind = RelationKind::Implies {
            if_yes: market("pa"),
            then_yes: market("national"),
        };

        let ids: Vec<&str> = kind.market_ids().iter().map(|m| m.as_str()).collect();
        assert_eq!(ids, vec!["pa", "national"]);
    }

    #[test]
    fn relation_kind_mutually_exclusive_market_ids() {
        let kind = RelationKind::MutuallyExclusive {
            markets: vec![market("trump"), market("biden"), market("other")],
        };

        assert_eq!(kind.market_ids().len(), 3);
    }

    #[test]
    fn relation_kind_linear_market_ids() {
        let kind = RelationKind::Linear {
            terms: vec![(market("a"), dec!(0.5)), (market("b"), dec!(0.5))],
            sense: ConstraintSense::LessEqual,
            rhs: dec!(1.0),
        };

        assert_eq!(kind.market_ids().len(), 2);
    }

    #[test]
    fn relation_kind_implies_to_solver_constraints() {
        let kind = RelationKind::Implies {
            if_yes: market("a"),
            then_yes: market("b"),
        };

        let indices: HashMap<MarketId, usize> =
            [(market("a"), 0), (market("b"), 1)].into_iter().collect();

        let constraints = kind.to_solver_constraints(&indices);

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].coefficients, vec![dec!(1), dec!(-1)]);
        assert_eq!(constraints[0].sense, ConstraintSense::LessEqual);
        assert_eq!(constraints[0].rhs, dec!(0));
    }

    #[test]
    fn relation_kind_mutually_exclusive_to_solver_constraints() {
        let kind = RelationKind::MutuallyExclusive {
            markets: vec![market("a"), market("b"), market("c")],
        };

        let indices: HashMap<MarketId, usize> =
            [(market("a"), 0), (market("b"), 1), (market("c"), 2)]
                .into_iter()
                .collect();

        let constraints = kind.to_solver_constraints(&indices);

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].coefficients, vec![dec!(1), dec!(1), dec!(1)]);
        assert_eq!(constraints[0].sense, ConstraintSense::LessEqual);
        assert_eq!(constraints[0].rhs, dec!(1));
    }

    #[test]
    fn relation_kind_exactly_one_to_solver_constraints() {
        let kind = RelationKind::ExactlyOne {
            markets: vec![market("a"), market("b")],
        };

        let indices: HashMap<MarketId, usize> =
            [(market("a"), 0), (market("b"), 1)].into_iter().collect();

        let constraints = kind.to_solver_constraints(&indices);

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].coefficients, vec![dec!(1), dec!(1)]);
        assert_eq!(constraints[0].sense, ConstraintSense::Equal);
        assert_eq!(constraints[0].rhs, dec!(1));
    }

    #[test]
    fn relation_kind_linear_to_solver_constraints() {
        let kind = RelationKind::Linear {
            terms: vec![(market("a"), dec!(2)), (market("b"), dec!(3))],
            sense: ConstraintSense::GreaterEqual,
            rhs: dec!(5),
        };

        let indices: HashMap<MarketId, usize> =
            [(market("a"), 0), (market("b"), 1)].into_iter().collect();

        let constraints = kind.to_solver_constraints(&indices);

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].coefficients, vec![dec!(2), dec!(3)]);
        assert_eq!(constraints[0].sense, ConstraintSense::GreaterEqual);
        assert_eq!(constraints[0].rhs, dec!(5));
    }
}
