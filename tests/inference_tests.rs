//! Integration tests for the relation inference system.

use std::sync::Arc;

use chrono::Duration;

use edgelord::adapters::llm::Llm;
use edgelord::core::inference::{Inferrer, LlmInferrer, MarketSummary};
use edgelord::domain::{MarketId, Relation, RelationKind};
use edgelord::runtime::cache::ClusterCache;
use edgelord::error::Result;

/// Mock LLM that returns predefined responses.
struct MockLlm {
    response: String,
}

impl MockLlm {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }

    /// Returns mutual exclusion using short IDs (M1, M2)
    fn with_mutual_exclusion() -> Self {
        Self::new(
            r#"{
            "relations": [
                {
                    "type": "mutually_exclusive",
                    "markets": ["M1", "M2"],
                    "confidence": 0.98,
                    "reasoning": "Only one can win the election"
                }
            ]
        }"#,
        )
    }

    /// Returns implication using short IDs (M1, M2)
    fn with_implication() -> Self {
        Self::new(
            r#"{
            "relations": [
                {
                    "type": "implies",
                    "if_yes": "M1",
                    "then_yes": "M2",
                    "confidence": 0.95,
                    "reasoning": "PA is a swing state"
                }
            ]
        }"#,
        )
    }

    fn empty() -> Self {
        Self::new(r#"{"relations": []}"#)
    }
}

#[async_trait::async_trait]
impl Llm for MockLlm {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }
}

fn sample_markets() -> Vec<MarketSummary> {
    vec![
        MarketSummary {
            id: MarketId::new("trump-wins"),
            question: "Will Trump win the 2024 election?".into(),
            outcomes: vec!["Yes".into(), "No".into()],
        },
        MarketSummary {
            id: MarketId::new("biden-wins"),
            question: "Will Biden win the 2024 election?".into(),
            outcomes: vec!["Yes".into(), "No".into()],
        },
    ]
}

#[tokio::test]
async fn test_llm_inferrer_discovers_mutual_exclusion() {
    let llm = Arc::new(MockLlm::with_mutual_exclusion());
    let inferrer = LlmInferrer::new(llm, Duration::hours(1));

    let relations = inferrer.infer(&sample_markets()).await.unwrap();

    assert_eq!(relations.len(), 1);
    assert!(relations[0].confidence > 0.9);

    match &relations[0].kind {
        RelationKind::MutuallyExclusive { markets } => {
            assert_eq!(markets.len(), 2);
            assert!(markets.iter().any(|m| m.as_str() == "trump-wins"));
            assert!(markets.iter().any(|m| m.as_str() == "biden-wins"));
        }
        _ => panic!("Expected MutuallyExclusive relation"),
    }
}

#[tokio::test]
async fn test_llm_inferrer_discovers_implication() {
    let llm = Arc::new(MockLlm::with_implication());
    let inferrer = LlmInferrer::new(llm, Duration::hours(1));

    let markets = vec![
        MarketSummary {
            id: MarketId::new("trump-wins-pa"),
            question: "Will Trump win Pennsylvania?".into(),
            outcomes: vec!["Yes".into(), "No".into()],
        },
        MarketSummary {
            id: MarketId::new("trump-wins-swing"),
            question: "Will Trump win any swing state?".into(),
            outcomes: vec!["Yes".into(), "No".into()],
        },
    ];

    let relations = inferrer.infer(&markets).await.unwrap();

    assert_eq!(relations.len(), 1);
    match &relations[0].kind {
        RelationKind::Implies { if_yes, then_yes } => {
            assert_eq!(if_yes.as_str(), "trump-wins-pa");
            assert_eq!(then_yes.as_str(), "trump-wins-swing");
        }
        _ => panic!("Expected Implies relation"),
    }
}

#[tokio::test]
async fn test_llm_inferrer_handles_empty_response() {
    let llm = Arc::new(MockLlm::empty());
    let inferrer = LlmInferrer::new(llm, Duration::hours(1));

    let relations = inferrer.infer(&sample_markets()).await.unwrap();
    assert!(relations.is_empty());
}

#[tokio::test]
async fn test_cluster_cache_stores_and_retrieves() {
    let cache = ClusterCache::new(Duration::hours(1));

    // Create a relation
    let relation = Relation::new(
        RelationKind::MutuallyExclusive {
            markets: vec![MarketId::new("m1"), MarketId::new("m2")],
        },
        0.95,
        "Test relation".to_string(),
    );

    // Store via put_relations
    cache.put_relations(vec![relation]);

    // Should be retrievable by either market
    assert!(cache.has_relations(&MarketId::new("m1")));
    assert!(cache.has_relations(&MarketId::new("m2")));

    let cluster = cache.get_for_market(&MarketId::new("m1")).unwrap();
    assert_eq!(cluster.markets.len(), 2);
    assert_eq!(cluster.relations.len(), 1);
}

#[tokio::test]
async fn test_end_to_end_inference_to_cache() {
    // 1. Create mock LLM
    let llm = Arc::new(MockLlm::with_mutual_exclusion());

    // 2. Create inferrer
    let inferrer = LlmInferrer::new(llm, Duration::hours(1));

    // 3. Run inference
    let relations = inferrer.infer(&sample_markets()).await.unwrap();
    assert!(!relations.is_empty());

    // 4. Store in cache
    let cache = ClusterCache::new(Duration::hours(1));
    cache.put_relations(relations);

    // 5. Verify cache lookup works
    assert!(cache.has_relations(&MarketId::new("trump-wins")));
    assert!(cache.has_relations(&MarketId::new("biden-wins")));

    // 6. Get cluster and verify constraints were computed
    let cluster = cache.get_for_market(&MarketId::new("trump-wins")).unwrap();
    assert!(
        !cluster.constraints.is_empty(),
        "Constraints should be pre-computed"
    );
}

#[tokio::test]
async fn test_cache_invalidation() {
    let cache = ClusterCache::new(Duration::hours(1));

    let relation = Relation::new(
        RelationKind::MutuallyExclusive {
            markets: vec![MarketId::new("a"), MarketId::new("b")],
        },
        0.9,
        "test".to_string(),
    );
    cache.put_relations(vec![relation]);

    assert!(cache.has_relations(&MarketId::new("a")));

    // Invalidate one market - should remove entire cluster
    cache.invalidate(&MarketId::new("a"));

    assert!(!cache.has_relations(&MarketId::new("a")));
    assert!(!cache.has_relations(&MarketId::new("b"))); // Same cluster
}
