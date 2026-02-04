//! LLM-powered relation inferrer.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use super::{Inferrer, MarketSummary};
use crate::core::domain::{MarketId, Relation, RelationKind};
use crate::core::llm::Llm;
use crate::error::Result;

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

    fn build_prompt(&self, markets: &[MarketSummary]) -> String {
        let market_list = markets
            .iter()
            .enumerate()
            .map(|(i, m)| {
                format!(
                    "{}. [{}] {}\n   Outcomes: {}",
                    i + 1,
                    m.id.as_str(),
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

## Output (JSON only, no explanation)
```json
{{
  "relations": [
    {{
      "type": "implies",
      "if_yes": "market_id",
      "then_yes": "market_id",
      "confidence": 0.95,
      "reasoning": "Brief explanation"
    }},
    {{
      "type": "mutually_exclusive",
      "markets": ["id1", "id2"],
      "confidence": 0.99,
      "reasoning": "Brief explanation"
    }}
  ]
}}
```

Rules:
- Only high-confidence relations (>0.7)
- Use exact market IDs from the list
- Empty array if no relations found
"#
        )
    }

    fn parse_response(
        &self,
        response: &str,
        markets: &[MarketSummary],
    ) -> Result<Vec<Relation>> {
        let json_str = extract_json(response)?;
        let parsed: LlmResponse = serde_json::from_str(json_str)
            .map_err(|e| crate::error::Error::Parse(format!("Invalid JSON: {e}")))?;

        let valid_ids: HashSet<_> = markets.iter().map(|m| m.id.as_str()).collect();
        let now = Utc::now();
        let expires = now + self.ttl;

        let relations = parsed
            .relations
            .into_iter()
            .filter_map(|r| {
                if !r.markets_valid(&valid_ids) {
                    warn!(relation = ?r, "Invalid market ID in relation");
                    return None;
                }

                let confidence = r.confidence;
                let reasoning = r.reasoning.clone();
                let kind = r.into_kind()?;

                Some(Relation {
                    id: crate::core::domain::RelationId::default(),
                    kind,
                    confidence,
                    reasoning,
                    inferred_at: now,
                    expires_at: expires,
                })
            })
            .collect();

        Ok(relations)
    }
}

#[async_trait]
impl Inferrer for LlmInferrer {
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
    fn markets_valid(&self, valid: &HashSet<&str>) -> bool {
        match self.kind.as_str() {
            "implies" => {
                self.if_yes.as_ref().map_or(false, |a| valid.contains(a.as_str()))
                    && self.then_yes.as_ref().map_or(false, |c| valid.contains(c.as_str()))
            }
            "mutually_exclusive" | "exactly_one" => {
                self.markets
                    .as_ref()
                    .map_or(false, |ms| ms.iter().all(|m| valid.contains(m.as_str())))
            }
            _ => false,
        }
    }

    fn into_kind(self) -> Option<RelationKind> {
        match self.kind.as_str() {
            "implies" => Some(RelationKind::Implies {
                if_yes: MarketId::new(self.if_yes?),
                then_yes: MarketId::new(self.then_yes?),
            }),
            "mutually_exclusive" => Some(RelationKind::MutuallyExclusive {
                markets: self.markets?.into_iter().map(MarketId::new).collect(),
            }),
            "exactly_one" => Some(RelationKind::ExactlyOne {
                markets: self.markets?.into_iter().map(MarketId::new).collect(),
            }),
            _ => None,
        }
    }
}

fn extract_json(text: &str) -> Result<&str> {
    // Find JSON in markdown code block or raw
    if let Some(start) = text.find("```json") {
        let start = start + 7;
        let end = text[start..].find("```").map(|i| start + i).unwrap_or(text.len());
        Ok(text[start..end].trim())
    } else if let Some(start) = text.find('{') {
        let end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
        Ok(&text[start..end])
    } else {
        Err(crate::error::Error::Parse("No JSON found in response".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::tests::MockLlm;

    #[tokio::test]
    async fn parses_valid_response() {
        let response = r#"{"relations": [
            {
                "type": "mutually_exclusive",
                "markets": ["m1", "m2"],
                "confidence": 0.95,
                "reasoning": "Same election"
            }
        ]}"#;

        let llm = Arc::new(MockLlm::new(response));
        let inferrer = LlmInferrer::new(llm, Duration::hours(1));

        let markets = vec![
            MarketSummary {
                id: MarketId::new("m1"),
                question: "Will A win?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
            MarketSummary {
                id: MarketId::new("m2"),
                question: "Will B win?".into(),
                outcomes: vec!["Yes".into(), "No".into()],
            },
        ];

        let relations = inferrer.infer(&markets).await.unwrap();
        assert_eq!(relations.len(), 1);
        assert!(relations[0].confidence > 0.9);
    }

    #[tokio::test]
    async fn filters_invalid_market_ids() {
        let response = r#"{"relations": [
            {
                "type": "mutually_exclusive",
                "markets": ["m1", "invalid_id"],
                "confidence": 0.95,
                "reasoning": "Test"
            }
        ]}"#;

        let llm = Arc::new(MockLlm::new(response));
        let inferrer = LlmInferrer::new(llm, Duration::hours(1));

        let markets = vec![MarketSummary {
            id: MarketId::new("m1"),
            question: "Test".into(),
            outcomes: vec!["Yes".into()],
        }];

        let relations = inferrer.infer(&markets).await.unwrap();
        assert!(relations.is_empty()); // Filtered out
    }
}
