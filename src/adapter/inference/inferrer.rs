//! LLM-powered relation inferrer.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::adapter::llm::Llm;
use crate::domain::{MarketId, Relation, RelationKind};
use crate::error::Result;
use crate::port::{MarketSummary, RelationInferrer};

/// LLM-powered relation inferrer.
pub struct LlmInferrer {
    llm: Arc<dyn Llm>,
    ttl: Duration,
}

impl LlmInferrer {
    /// Create a new LLM inferrer.
    pub fn new(llm: Arc<dyn Llm>, ttl: Duration) -> Self {
        Self { llm, ttl }
    }

    /// Build prompt using short reference IDs (M1, M2, etc) for reliability.
    fn build_prompt(&self, markets: &[MarketSummary]) -> String {
        let market_list = markets
            .iter()
            .enumerate()
            .map(|(i, m)| {
                format!(
                    "M{}: {}\n   Outcomes: {}",
                    i + 1,
                    m.question,
                    m.outcomes.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"Analyze these prediction markets for logical relationships.

## Markets
{market_list}

## Relation Types
- **implies**: If A=YES then B=YES. Example: "Trump wins PA" implies "Trump wins swing state"
- **mutually_exclusive**: At most one can be YES. Example: "Trump wins" vs "Biden wins"
- **exactly_one**: Exactly one must be YES. Example: All candidates in single-winner election

## Output (JSON only)
```json
{{
  "relations": [
    {{
      "type": "implies",
      "if_yes": "M1",
      "then_yes": "M2",
      "confidence": 0.95,
      "reasoning": "Brief explanation"
    }},
    {{
      "type": "mutually_exclusive",
      "markets": ["M1", "M2", "M3"],
      "confidence": 0.99,
      "reasoning": "Brief explanation"
    }}
  ]
}}
```

Rules:
- Use market IDs exactly as shown (M1, M2, etc)
- Only relations with confidence > 0.7
- Return empty array if no relations found
"#
        )
    }

    /// Parse response, mapping short IDs back to real market IDs.
    fn parse_response(&self, response: &str, markets: &[MarketSummary]) -> Result<Vec<Relation>> {
        let json_str = extract_json(response)?;
        let parsed: LlmResponse = serde_json::from_str(json_str)
            .map_err(|e| crate::error::Error::Parse(format!("Invalid JSON: {e}")))?;

        // Build mapping from short ID (M1, M2) to real market ID
        let id_map: HashMap<String, &MarketId> = markets
            .iter()
            .enumerate()
            .map(|(i, m)| (format!("M{}", i + 1), &m.id))
            .collect();

        let now = Utc::now();
        let expires = now + self.ttl;

        let relations = parsed
            .relations
            .into_iter()
            .filter_map(|r| {
                let confidence = r.confidence;
                let reasoning = r.reasoning.clone();
                let relation_type = r.kind.clone();

                match r.into_kind_mapped(&id_map) {
                    Some(Some(kind)) => Some(Relation {
                        id: crate::domain::RelationId::default(),
                        kind,
                        confidence,
                        reasoning,
                        inferred_at: now,
                        expires_at: expires,
                    }),
                    _ => {
                        warn!(relation_type = %relation_type, "Failed to map market IDs");
                        None
                    }
                }
            })
            .collect();

        Ok(relations)
    }
}

#[async_trait]
impl RelationInferrer for LlmInferrer {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn infer(&self, markets: &[MarketSummary]) -> Result<Vec<Relation>> {
        if markets.len() < 2 {
            return Ok(vec![]);
        }

        let prompt = self.build_prompt(markets);
        let response = self.llm.complete(&prompt).await?;
        debug!(provider = self.llm.name(), "LLM inference complete");

        self.parse_response(&response, markets)
    }

    fn batch_limit(&self) -> usize {
        30
    }
}

#[derive(Deserialize)]
struct LlmResponse {
    relations: Vec<RawRelation>,
}

#[derive(Deserialize, Debug)]
struct RawRelation {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    if_yes: Option<String>,
    #[serde(default)]
    then_yes: Option<String>,
    #[serde(default)]
    markets: Option<Vec<String>>,
    confidence: f64,
    reasoning: String,
}

impl RawRelation {
    /// Map short IDs (M1, M2) to real market IDs and build RelationKind.
    fn into_kind_mapped(self, id_map: &HashMap<String, &MarketId>) -> Option<Option<RelationKind>> {
        match self.kind.as_str() {
            "implies" => {
                let if_yes = id_map.get(self.if_yes.as_ref()?)?;
                let then_yes = id_map.get(self.then_yes.as_ref()?)?;
                Some(Some(RelationKind::Implies {
                    if_yes: (*if_yes).clone(),
                    then_yes: (*then_yes).clone(),
                }))
            }
            "mutually_exclusive" => {
                let markets: Option<Vec<MarketId>> = self
                    .markets?
                    .iter()
                    .map(|m| id_map.get(m).map(|id| (*id).clone()))
                    .collect();
                Some(markets.map(|ms| RelationKind::MutuallyExclusive { markets: ms }))
            }
            "exactly_one" => {
                let markets: Option<Vec<MarketId>> = self
                    .markets?
                    .iter()
                    .map(|m| id_map.get(m).map(|id| (*id).clone()))
                    .collect();
                Some(markets.map(|ms| RelationKind::ExactlyOne { markets: ms }))
            }
            _ => Some(None),
        }
    }
}

fn extract_json(text: &str) -> Result<&str> {
    // Find JSON in markdown code block or raw
    if let Some(start) = text.find("```json") {
        let start = start + 7;
        let end = text[start..]
            .find("```")
            .map(|i| start + i)
            .unwrap_or(text.len());
        Ok(text[start..end].trim())
    } else if let Some(start) = text.find('{') {
        let end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
        Ok(&text[start..end])
    } else {
        Err(crate::error::Error::Parse(
            "No JSON found in response".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::llm::tests::MockLlm;

    #[tokio::test]
    async fn parses_valid_response_with_short_ids() {
        // LLM returns short IDs (M1, M2) which we map to real IDs
        let response = r#"{"relations": [
            {
                "type": "mutually_exclusive",
                "markets": ["M1", "M2"],
                "confidence": 0.95,
                "reasoning": "Same election"
            }
        ]}"#;

        let llm = Arc::new(MockLlm::new(response));
        let inferrer = LlmInferrer::new(llm, Duration::hours(1));

        let markets = vec![
            MarketSummary {
                id: MarketId::new("real-market-id-1"),
                question: "Will A win?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
            MarketSummary {
                id: MarketId::new("real-market-id-2"),
                question: "Will B win?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
        ];

        let relations = inferrer.infer(&markets).await.unwrap();
        assert_eq!(relations.len(), 1);
        assert!(relations[0].confidence > 0.9);

        // Verify the real market IDs were used
        match &relations[0].kind {
            RelationKind::MutuallyExclusive { markets } => {
                assert_eq!(markets[0].as_str(), "real-market-id-1");
                assert_eq!(markets[1].as_str(), "real-market-id-2");
            }
            _ => panic!("Expected MutuallyExclusive"),
        }
    }

    #[tokio::test]
    async fn filters_invalid_short_ids() {
        // LLM returns an ID that doesn't exist (M99)
        let response = r#"{"relations": [
            {
                "type": "mutually_exclusive",
                "markets": ["M1", "M99"],
                "confidence": 0.95,
                "reasoning": "Test"
            }
        ]}"#;

        let llm = Arc::new(MockLlm::new(response));
        let inferrer = LlmInferrer::new(llm, Duration::hours(1));

        let markets = vec![MarketSummary {
            id: MarketId::new("real-id"),
            question: "Test".into(),
            outcomes: vec!["Yes".into()],
        }];

        let relations = inferrer.infer(&markets).await.unwrap();
        assert!(relations.is_empty()); // Filtered out because M99 doesn't exist
    }

    #[tokio::test]
    async fn parses_implies_relation() {
        let response = r#"{"relations": [
            {
                "type": "implies",
                "if_yes": "M1",
                "then_yes": "M2",
                "confidence": 0.9,
                "reasoning": "If PA then swing state"
            }
        ]}"#;

        let llm = Arc::new(MockLlm::new(response));
        let inferrer = LlmInferrer::new(llm, Duration::hours(1));

        let markets = vec![
            MarketSummary {
                id: MarketId::new("trump-pa"),
                question: "Trump wins PA?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
            MarketSummary {
                id: MarketId::new("trump-swing"),
                question: "Trump wins swing state?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
        ];

        let relations = inferrer.infer(&markets).await.unwrap();
        assert_eq!(relations.len(), 1);

        match &relations[0].kind {
            RelationKind::Implies { if_yes, then_yes } => {
                assert_eq!(if_yes.as_str(), "trump-pa");
                assert_eq!(then_yes.as_str(), "trump-swing");
            }
            _ => panic!("Expected Implies"),
        }
    }
}
