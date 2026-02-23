//! Cluster detection logic using Frank-Wolfe projection.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{info, trace};

use crate::domain::{
    book::Book, cluster::Cluster, constraint::VariableBounds, id::TokenId, market::MarketRegistry,
    opportunity::Opportunity, opportunity::OpportunityLeg,
};
use crate::error::{Error, Result};
use crate::port::{
    outbound::solver::IlpProblem, outbound::solver::LpProblem, outbound::solver::ProjectionSolver,
};

use super::service::{ClusterDetectionConfig, ClusterOpportunity};

/// Errors specific to cluster detection.
#[derive(Debug, Clone)]
pub enum DetectionError {
    /// Cluster not found in cache.
    ClusterNotFound(String),
    /// Not enough markets in cluster.
    InsufficientMarkets { cluster_id: String, count: usize },
    /// Missing price data for market.
    MissingPriceData { market_id: String },
    /// Frank-Wolfe solver failed.
    SolverFailed(String),
    /// Gap below threshold.
    GapBelowThreshold { gap: Decimal, threshold: Decimal },
}

impl std::fmt::Display for DetectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClusterNotFound(id) => write!(f, "Cluster not found: {id}"),
            Self::InsufficientMarkets { cluster_id, count } => {
                write!(f, "Cluster {cluster_id} has only {count} markets (need 2+)")
            }
            Self::MissingPriceData { market_id } => {
                write!(f, "Missing price data for market: {market_id}")
            }
            Self::SolverFailed(msg) => write!(f, "Solver failed: {msg}"),
            Self::GapBelowThreshold { gap, threshold } => {
                write!(f, "Gap {gap} below threshold {threshold}")
            }
        }
    }
}

impl std::error::Error for DetectionError {}

/// Function type for looking up order books.
pub type BookLookup<'a> = &'a dyn Fn(&TokenId) -> Option<Book>;

/// Cluster detector using Frank-Wolfe projection.
///
/// Encapsulates the detection logic separate from the service lifecycle.
pub struct ClusterDetector {
    config: ClusterDetectionConfig,
    projection_solver: Arc<dyn ProjectionSolver>,
}

impl ClusterDetector {
    /// Create a new detector with the given configuration.
    pub fn new(
        config: ClusterDetectionConfig,
        projection_solver: Arc<dyn ProjectionSolver>,
    ) -> Self {
        Self {
            config,
            projection_solver,
        }
    }

    /// Detect arbitrage in a cluster.
    ///
    /// Returns `Ok(Some(opportunity))` if arbitrage found,
    /// `Ok(None)` if gap below threshold,
    /// `Err` if detection failed.
    ///
    /// # Arguments
    /// * `cluster` - The cluster to analyze
    /// * `book_lookup` - Function to look up order books by token ID
    /// * `registry` - Market registry for resolving market metadata
    pub fn detect(
        &self,
        cluster: &Cluster,
        book_lookup: BookLookup<'_>,
        registry: &MarketRegistry,
    ) -> Result<Option<ClusterOpportunity>> {
        let cluster_id = cluster.id.to_string();

        // Gather prices
        let (prices, token_ids) = self.gather_prices(cluster, book_lookup, registry)?;

        if prices.len() < 2 {
            return Err(Error::Parse(
                DetectionError::InsufficientMarkets {
                    cluster_id: cluster_id.clone(),
                    count: prices.len(),
                }
                .to_string(),
            ));
        }

        // Build ILP problem
        let lp = LpProblem {
            objective: prices.clone(),
            constraints: cluster.constraints.clone(),
            bounds: vec![VariableBounds::binary(); prices.len()],
        };
        let ilp = IlpProblem::all_binary(lp);

        // Run Frank-Wolfe projection
        let result = self
            .projection_solver
            .project(&prices, &ilp)
            .map_err(|e| Error::Parse(DetectionError::SolverFailed(e.to_string()).to_string()))?;

        // Check threshold
        if result.gap < self.config.min_gap {
            trace!(
                cluster = %cluster_id,
                gap = %result.gap,
                threshold = %self.config.min_gap,
                "Gap below threshold"
            );
            return Ok(None);
        }

        info!(
            cluster = %cluster_id,
            gap = %result.gap,
            iterations = result.iterations,
            "Found cluster arbitrage"
        );

        // Build opportunity
        let opportunity =
            self.build_opportunity(cluster, &token_ids, &result.values, result.gap, registry)?;

        Ok(Some(ClusterOpportunity {
            cluster_id,
            markets: cluster.markets.clone(),
            gap: result.gap,
            opportunity,
        }))
    }

    /// Gather current prices for all markets in a cluster.
    fn gather_prices(
        &self,
        cluster: &Cluster,
        book_lookup: BookLookup<'_>,
        registry: &MarketRegistry,
    ) -> Result<(Vec<Decimal>, Vec<TokenId>)> {
        let mut prices = Vec::with_capacity(cluster.markets.len());
        let mut token_ids = Vec::with_capacity(cluster.markets.len());

        for market_id in &cluster.markets {
            let market = registry.get_by_market_id(market_id).ok_or_else(|| {
                Error::Parse(
                    DetectionError::MissingPriceData {
                        market_id: market_id.to_string(),
                    }
                    .to_string(),
                )
            })?;

            let yes_token = market.outcomes().first().ok_or_else(|| {
                Error::Parse(
                    DetectionError::MissingPriceData {
                        market_id: market_id.to_string(),
                    }
                    .to_string(),
                )
            })?;

            let book = book_lookup(yes_token.token_id()).ok_or_else(|| {
                Error::Parse(
                    DetectionError::MissingPriceData {
                        market_id: market_id.to_string(),
                    }
                    .to_string(),
                )
            })?;

            let price = book.best_ask().map(|l| l.price()).unwrap_or(Decimal::ONE);
            prices.push(price);
            token_ids.push(yes_token.token_id().clone());
        }

        Ok((prices, token_ids))
    }

    /// Build an opportunity from detection results.
    fn build_opportunity(
        &self,
        cluster: &Cluster,
        token_ids: &[TokenId],
        projected_prices: &[Decimal],
        gap: Decimal,
        registry: &MarketRegistry,
    ) -> Result<Opportunity> {
        let legs: Vec<OpportunityLeg> = token_ids
            .iter()
            .zip(projected_prices.iter())
            .map(|(token_id, &price)| OpportunityLeg::new(token_id.clone(), price))
            .collect();

        let market_id = cluster
            .markets
            .first()
            .ok_or_else(|| Error::Parse("Cluster has no markets".to_string()))?
            .clone();

        let question = registry
            .get_by_market_id(&market_id)
            .map(|m| m.question().to_string())
            .unwrap_or_else(|| format!("Cluster {}", cluster.id));

        Ok(Opportunity::new(
            market_id,
            question,
            legs,
            gap,
            Decimal::ONE,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{outbound::solver::ProjectionResult, outbound::solver::ProjectionSolver};

    struct MockProjectionSolver;

    impl ProjectionSolver for MockProjectionSolver {
        fn name(&self) -> &'static str {
            "mock_projection"
        }

        fn project(&self, theta: &[Decimal], _problem: &IlpProblem) -> Result<ProjectionResult> {
            Ok(ProjectionResult {
                values: theta.to_vec(),
                gap: Decimal::ZERO,
                iterations: 1,
                converged: true,
            })
        }
    }

    #[test]
    fn test_detector_creation() {
        let config = ClusterDetectionConfig::default();
        let detector = ClusterDetector::new(config, Arc::new(MockProjectionSolver));
        assert_eq!(detector.config.debounce_ms, 100);
    }

    #[test]
    fn test_detection_error_display() {
        let err = DetectionError::ClusterNotFound("test-id".to_string());
        assert!(err.to_string().contains("test-id"));

        let err = DetectionError::InsufficientMarkets {
            cluster_id: "c1".to_string(),
            count: 1,
        };
        assert!(err.to_string().contains("1 markets"));

        let err = DetectionError::GapBelowThreshold {
            gap: Decimal::new(1, 2),
            threshold: Decimal::new(2, 2),
        };
        assert!(err.to_string().contains("below threshold"));
    }
}
