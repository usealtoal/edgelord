//! Cluster types for grouped market relations.
//!
//! A [`Cluster`] represents a group of markets connected by logical relations.
//! Clusters cache pre-computed solver constraints for fast arbitrage detection.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::constraint::Constraint;
use super::id::{ClusterId, MarketId};
use super::relation::Relation;

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
    use crate::domain::relation::RelationKind;

    fn market(s: &str) -> MarketId {
        MarketId::new(s)
    }

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
