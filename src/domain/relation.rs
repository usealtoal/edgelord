//! Relation and cluster types for market dependency inference.
//!
//! This module defines the domain types for expressing logical relationships
//! between prediction markets. These relations are inferred by the LLM and
//! converted to solver constraints for combinatorial arbitrage.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::id::{ClusterId, MarketId, RelationId};
use crate::ports::{Constraint, ConstraintSense};

/// A logical relation between prediction markets.
///
/// Relations are inferred by the LLM inference system and represent
/// logical dependencies like "A implies B" or "exactly one of A, B, C".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    /// Unique identifier for this relation.
    pub id: RelationId,
    /// The type and semantics of the relation.
    pub kind: RelationKind,
    /// Confidence score (0.0 - 1.0) from the inferrer.
    pub confidence: f64,
    /// Human-readable reasoning (for debugging/audit).
    pub reasoning: String,
    /// When this relation was inferred.
    pub inferred_at: DateTime<Utc>,
    /// When this relation expires (needs re-validation).
    pub expires_at: DateTime<Utc>,
}

impl Relation {
    /// Create a new relation with the given kind and confidence.
    pub fn new(kind: RelationKind, confidence: f64, reasoning: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: RelationId::new(),
            kind,
            confidence,
            reasoning: reasoning.into(),
            inferred_at: now,
            expires_at: now + chrono::Duration::hours(1), // Default 1 hour TTL
        }
    }

    /// Create a relation with a custom TTL.
    pub fn with_ttl(mut self, ttl: chrono::Duration) -> Self {
        self.expires_at = self.inferred_at + ttl;
        self
    }

    /// Check if this relation has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get all market IDs referenced by this relation.
    pub fn market_ids(&self) -> Vec<&MarketId> {
        self.kind.market_ids()
    }
}

/// The type of logical relationship between markets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelationKind {
    /// If market A resolves YES, market B must resolve YES.
    /// Constraint: P(A) ≤ P(B), encoded as μ_A - μ_B ≤ 0
    Implies {
        /// Market that implies the other.
        if_yes: MarketId,
        /// Market that must be true if `if_yes` is true.
        then_yes: MarketId,
    },

    /// At most one of these markets can resolve YES.
    /// Constraint: Σ μ_i ≤ 1
    MutuallyExclusive {
        /// Markets in the mutually exclusive set.
        markets: Vec<MarketId>,
    },

    /// Exactly one of these markets must resolve YES.
    /// Constraint: Σ μ_i = 1
    ExactlyOne {
        /// Markets where exactly one must be true.
        markets: Vec<MarketId>,
    },

    /// Custom linear constraint: Σ (coeff_i × μ_i) {≤, =, ≥} rhs
    Linear {
        /// Coefficient terms (market, coefficient) pairs.
        terms: Vec<(MarketId, Decimal)>,
        /// Constraint sense (<=, =, >=).
        sense: ConstraintSense,
        /// Right-hand side value.
        rhs: Decimal,
    },
}

impl RelationKind {
    /// Get all market IDs referenced by this relation kind.
    pub fn market_ids(&self) -> Vec<&MarketId> {
        match self {
            Self::Implies { if_yes, then_yes } => vec![if_yes, then_yes],
            Self::MutuallyExclusive { markets } | Self::ExactlyOne { markets } => {
                markets.iter().collect()
            }
            Self::Linear { terms, .. } => terms.iter().map(|(m, _)| m).collect(),
        }
    }

    /// Get the type name of this relation kind.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Implies { .. } => "implies",
            Self::MutuallyExclusive { .. } => "mutually_exclusive",
            Self::ExactlyOne { .. } => "exactly_one",
            Self::Linear { .. } => "linear",
        }
    }

    /// Convert this relation to solver constraints.
    ///
    /// # Arguments
    /// * `market_indices` - Mapping from MarketId to variable index in the ILP
    ///
    /// # Returns
    /// Vector of solver constraints representing this relation.
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

/// A cluster of related markets with pre-computed solver constraints.
///
/// Clusters are built from connected relations and cache the converted
/// solver constraints for fast access during the detection hot path.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Unique identifier for this cluster.
    pub id: ClusterId,
    /// Markets in this cluster (ordered for ILP variable mapping).
    pub markets: Vec<MarketId>,
    /// Source relations within this cluster.
    pub relations: Vec<Relation>,
    /// Pre-computed ILP constraints for the solver (hot path).
    pub constraints: Vec<Constraint>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Cluster {
    /// Create a new cluster from a set of relations.
    ///
    /// Automatically extracts all referenced markets and builds
    /// the market index mapping for constraint conversion.
    pub fn from_relations(relations: Vec<Relation>) -> Self {
        // Collect all unique markets, sorted for deterministic ordering
        let mut markets: Vec<MarketId> = relations
            .iter()
            .flat_map(|r| r.market_ids().into_iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        markets.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        // Build index mapping
        let market_indices: HashMap<MarketId, usize> = markets
            .iter()
            .enumerate()
            .map(|(i, m)| (m.clone(), i))
            .collect();

        // Convert relations to solver constraints
        let constraints: Vec<Constraint> = relations
            .iter()
            .flat_map(|r| r.kind.to_solver_constraints(&market_indices))
            .collect();

        Self {
            id: ClusterId::new(),
            markets,
            relations,
            constraints,
            updated_at: Utc::now(),
        }
    }

    /// Check if this cluster contains a specific market.
    pub fn contains_market(&self, market_id: &MarketId) -> bool {
        self.markets.iter().any(|m| m == market_id)
    }

    /// Get the number of markets in this cluster.
    pub fn market_count(&self) -> usize {
        self.markets.len()
    }

    /// Get the number of constraints in this cluster.
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Helper to create test market IDs
    fn market(s: &str) -> MarketId {
        MarketId::new(s)
    }

    // === Relation tests ===

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

    // === RelationKind tests ===

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

    // === Cluster tests ===

    #[test]
    fn cluster_from_relations_extracts_markets() {
        let relations = vec![
            Relation::new(
                RelationKind::Implies {
                    if_yes: market("a"),
                    then_yes: market("b"),
                },
                0.9,
                "test",
            ),
            Relation::new(
                RelationKind::MutuallyExclusive {
                    markets: vec![market("b"), market("c")],
                },
                0.85,
                "test",
            ),
        ];

        let cluster = Cluster::from_relations(relations);

        // Should have 3 unique markets: a, b, c
        assert_eq!(cluster.market_count(), 3);
        assert!(cluster.contains_market(&market("a")));
        assert!(cluster.contains_market(&market("b")));
        assert!(cluster.contains_market(&market("c")));
    }

    #[test]
    fn cluster_from_relations_builds_constraints() {
        let relations = vec![
            Relation::new(
                RelationKind::Implies {
                    if_yes: market("a"),
                    then_yes: market("b"),
                },
                0.9,
                "test",
            ),
            Relation::new(
                RelationKind::MutuallyExclusive {
                    markets: vec![market("b"), market("c")],
                },
                0.85,
                "test",
            ),
        ];

        let cluster = Cluster::from_relations(relations);

        // Should have 2 constraints (1 from implies, 1 from mutually_exclusive)
        assert_eq!(cluster.constraint_count(), 2);
    }

    #[test]
    fn cluster_markets_are_sorted() {
        let relations = vec![Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("z"), market("a"), market("m")],
            },
            0.9,
            "test",
        )];

        let cluster = Cluster::from_relations(relations);

        let market_strs: Vec<&str> = cluster.markets.iter().map(|m| m.as_str()).collect();
        assert_eq!(market_strs, vec!["a", "m", "z"]);
    }

    #[test]
    fn cluster_contains_market_returns_false_for_missing() {
        let relations = vec![Relation::new(
            RelationKind::MutuallyExclusive {
                markets: vec![market("a"), market("b")],
            },
            0.9,
            "test",
        )];

        let cluster = Cluster::from_relations(relations);

        assert!(!cluster.contains_market(&market("c")));
    }
}
