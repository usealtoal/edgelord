# LLM-Assisted Dependency Discovery

> Deep dive into using LLMs to detect correlated markets for combinatorial arbitrage
> Based on: "Unravelling the Probabilistic Forest" (arXiv:2508.03474, August 2025)

---

## Executive Summary

The research paper that your existing `polymarket-arbitrage.md` references actually **implemented** LLM-based dependency discovery. Here's how they did it and how to adapt it for edgelord.

**Key finding:** They used **DeepSeek-R1-Distill-Qwen-32B** to detect dependencies between markets during the 2024 US election. Out of 46,360 market pairs checked, they found **13 genuinely dependent pairs** that satisfied combinatorial arbitrage conditions.

**Total combinatorial profit extracted:** ~$95K across those 13 pairs.

---

## Their Architecture

### Step 1: Pre-filtering (Reduce Search Space)

Don't throw all markets at the LLM. Filter first:

```
17,218 conditions total
    ↓ Filter by topic (using embeddings)
    ↓ Filter by end_date (same resolution time)
    ↓ Filter by activity (minimum volume)
~200-500 candidate pairs per topic/date
```

**Embedding model used:** `Linq-Embed-Mistral` (best open-source at time of research)
- Generate embeddings for market questions
- Cosine similarity to topic categories: Politics, Economy, Technology, Crypto, Sports, Culture
- Only compare markets within same topic + end date

### Step 2: Condition Reduction

LLMs struggle with too many conditions. Their solution:

```rust
// For markets with > 4 conditions:
// 1. Keep top 4 by trading volume
// 2. Add 5th "catch-all" condition: "Any other outcome"
// This preserves logical dependencies while staying within LLM context limits
```

**Why this works:** 90%+ of liquidity concentrates in top 4 conditions. The catch-all preserves the logical structure.

### Step 3: LLM Dependency Inference

**Model:** DeepSeek-R1-Distill-Qwen-32B (local deployment)

**Approach:** Give LLM the union of conditions from two markets, ask it to enumerate all valid outcome combinations.

```
Market A: "Who wins Pennsylvania?" [Trump, Harris, Other]
Market B: "Republican margin in PA?" [>5%, 0-5%, Dem wins]

Prompt: "Given these conditions, list all logically consistent 
combinations of True/False assignments..."

Response: JSON with valid state vectors
```

**Key insight:** If the LLM returns fewer than `n × m` state vectors, the markets are dependent.

### Step 4: Validation

Check LLM output for:
1. Valid JSON (LLMs sometimes loop or produce garbage)
2. Exactly one true condition per market in each state vector
3. State space size ≤ `n × m`

**Failure rate:** ~10% of prompts failed validation (loops, invalid JSON)

---

## Prompt Engineering Details

### Single Market Validation Prompt

```
You are a logic evaluator. Given a set of mutually exclusive conditions 
for a prediction market, enumerate all possible resolution states.

Conditions (exactly one must be TRUE):
1. "Trump wins Pennsylvania"
2. "Harris wins Pennsylvania"  
3. "Other candidate wins Pennsylvania"

Output a JSON array where each element is a boolean vector representing 
a valid state. The i-th element corresponds to condition i.

Rules:
- Exactly one condition can be TRUE in each state
- All conditions are mutually exclusive and exhaustive
- Output ONLY valid JSON, no explanation

Example output format:
[[true, false, false], [false, true, false], [false, false, true]]
```

### Two-Market Dependency Prompt

```
You are a logic evaluator analyzing dependencies between prediction markets.

Market A conditions (exactly one TRUE):
A1: "Trump wins Pennsylvania"
A2: "Harris wins Pennsylvania"
A3: "Other wins Pennsylvania"

Market B conditions (exactly one TRUE):
B1: "Republicans win PA by >5 points"
B2: "Republicans win PA by 0-5 points"
B3: "Democrats win PA"

Task: Enumerate ALL logically consistent combinations of outcomes.

Consider semantic implications:
- If B1 is TRUE (Rep >5%), what does that imply for Market A?
- If A2 is TRUE (Harris wins), what does that imply for Market B?

Output JSON array of valid state vectors [A1, A2, A3, B1, B2, B3].

Important: Include ONLY states that are logically possible in the real world.
Do not include states like [Harris wins, Republicans +5%] since they are 
mutually contradictory.
```

---

## Implementation for Edgelord

### New Module: `src/core/strategy/combinatorial/discovery.rs`

```rust
use serde::{Deserialize, Serialize};

/// Represents a discovered dependency between markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDependency {
    pub market_a: MarketId,
    pub market_b: MarketId,
    pub constraint_type: ConstraintType,
    pub dependent_subset_a: Vec<usize>,  // Condition indices
    pub dependent_subset_b: Vec<usize>,
    pub confidence: f64,
    pub llm_reasoning: String,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// If any in subset A is true, exactly one in subset B must be true
    Implies,
    /// Conditions are mutually exclusive across markets
    MutuallyExclusive,
    /// Custom linear constraint (for advanced cases)
    Linear { coefficients: Vec<f64>, rhs: f64 },
}

/// LLM client abstraction
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError>;
}

/// Dependency discovery service
pub struct DependencyDiscovery<L: LlmClient> {
    llm: L,
    embedder: TextEmbedder,
    cache: DependencyCache,
    config: DiscoveryConfig,
}

impl<L: LlmClient> DependencyDiscovery<L> {
    /// Discover dependencies between markets with same topic and end date
    pub async fn discover_batch(
        &self,
        markets: &[Market],
    ) -> Vec<DiscoveredDependency> {
        // 1. Filter to same topic/end_date
        let clusters = self.cluster_by_topic_date(markets);
        
        let mut dependencies = vec![];
        
        for cluster in clusters {
            // 2. Reduce conditions if needed
            let reduced: Vec<_> = cluster.iter()
                .map(|m| self.reduce_conditions(m))
                .collect();
            
            // 3. Check pairwise dependencies
            for (i, market_a) in reduced.iter().enumerate() {
                for market_b in reduced.iter().skip(i + 1) {
                    if let Some(dep) = self.check_dependency(market_a, market_b).await {
                        dependencies.push(dep);
                    }
                }
            }
        }
        
        dependencies
    }
    
    /// Reduce market to max 5 conditions (top 4 by volume + catch-all)
    fn reduce_conditions(&self, market: &Market) -> ReducedMarket {
        let mut conditions: Vec<_> = market.conditions()
            .iter()
            .enumerate()
            .collect();
        
        // Sort by volume descending
        conditions.sort_by(|a, b| b.1.volume().cmp(&a.1.volume()));
        
        if conditions.len() <= 4 {
            return ReducedMarket::from(market);
        }
        
        // Keep top 4, add catch-all
        let top_4: Vec<_> = conditions.iter().take(4).cloned().collect();
        let catch_all = Condition::catch_all(&conditions[4..]);
        
        ReducedMarket {
            id: market.id().clone(),
            conditions: top_4.into_iter()
                .map(|(_, c)| c.clone())
                .chain(std::iter::once(catch_all))
                .collect(),
        }
    }
    
    /// Check if two markets have dependent conditions
    async fn check_dependency(
        &self,
        market_a: &ReducedMarket,
        market_b: &ReducedMarket,
    ) -> Option<DiscoveredDependency> {
        let prompt = self.build_dependency_prompt(market_a, market_b);
        
        let response = match self.llm.complete(&prompt).await {
            Ok(r) => r,
            Err(_) => return None,
        };
        
        let state_vectors = match self.parse_state_vectors(&response) {
            Ok(v) => v,
            Err(_) => return None,  // LLM produced invalid output
        };
        
        // Validate: each vector should have exactly one true per market
        if !self.validate_vectors(&state_vectors, market_a.len(), market_b.len()) {
            return None;
        }
        
        let n = market_a.len();
        let m = market_b.len();
        let expected_independent = n * m;
        
        if state_vectors.len() >= expected_independent {
            return None;  // Markets are independent
        }
        
        // Markets are dependent - extract the constraint structure
        self.extract_constraint(&state_vectors, market_a, market_b)
    }
}
```

### Configuration

```toml
[strategies.combinatorial.discovery]
enabled = true
llm_endpoint = "http://localhost:8080/v1/completions"  # Local LLM
llm_model = "deepseek-r1-distill-qwen-32b"
embedding_model = "linq-embed-mistral"

# Pre-filtering
min_market_volume = 1000.0       # Minimum 24h volume to consider
same_topic_only = true
same_end_date_only = true

# LLM settings
max_conditions_per_prompt = 10   # 5 per market, 2 markets
timeout_ms = 30000
retry_attempts = 2

# Caching
cache_ttl_hours = 24
refresh_on_new_market = true
```

---

## Topic Clustering with Embeddings

The paper used Linq-Embed-Mistral for topic classification. Here's how to integrate:

```rust
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use hf_hub::api::sync::Api;

pub struct TextEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
}

impl TextEmbedder {
    pub fn new() -> Result<Self, EmbedError> {
        let api = Api::new()?;
        let repo = api.model("Linq-AI-Research/Linq-Embed-Mistral".to_string());
        // Load model and tokenizer...
    }
    
    pub fn embed(&self, text: &str) -> Vec<f32> {
        // Tokenize and run inference
    }
    
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }
    
    pub fn classify_topic(&self, question: &str) -> Topic {
        let question_emb = self.embed(question);
        
        let topics = [
            ("Politics", self.embed("Political elections government policy")),
            ("Sports", self.embed("Sports games matches tournaments")),
            ("Crypto", self.embed("Cryptocurrency blockchain bitcoin ethereum")),
            ("Economy", self.embed("Economy finance markets stocks")),
            ("Technology", self.embed("Technology AI software companies")),
            ("Culture", self.embed("Entertainment culture celebrities media")),
        ];
        
        topics.iter()
            .max_by(|a, b| {
                self.cosine_similarity(&question_emb, &a.1)
                    .partial_cmp(&self.cosine_similarity(&question_emb, &b.1))
                    .unwrap()
            })
            .map(|(name, _)| Topic::from(*name))
            .unwrap()
    }
}
```

---

## Integration with Frank-Wolfe

Once dependencies are discovered, feed them into the existing Frank-Wolfe infrastructure:

```rust
impl CombinatorialStrategy {
    fn detect(&self, ctx: &DetectionContext) -> Vec<Opportunity> {
        // 1. Get cached dependencies for this market
        let deps = self.dependency_cache
            .get_dependencies(ctx.market.id())
            .unwrap_or_default();
        
        if deps.is_empty() {
            return vec![];
        }
        
        // 2. For each dependent market pair, aggregate prices
        for dep in deps {
            let market_b = self.market_registry.get(&dep.market_b)?;
            
            // 3. Build joint price vector
            let prices = self.aggregate_prices(ctx.market, market_b, ctx.cache);
            
            // 4. Build ILP constraints from dependency
            let constraints = dep.to_ilp_constraints();
            
            // 5. Run Frank-Wolfe projection
            let result = self.frank_wolfe.project(&prices, &constraints);
            
            // 6. Check if gap exceeds threshold
            if result.gap > self.config.gap_threshold {
                return vec![self.build_opportunity(result, ctx.market, market_b)];
            }
        }
        
        vec![]
    }
}
```

---

## LLM Provider Options

### Option 1: Local Deployment (Recommended for Speed)
- **Model:** DeepSeek-R1-Distill-Qwen-32B or Llama-3.2-70B
- **Infra:** vLLM or llama.cpp on GPU server
- **Latency:** 2-5s per prompt
- **Cost:** Hardware only

### Option 2: Cloud API
- **Claude API:** Best reasoning, ~$0.015/1K tokens
- **OpenAI GPT-4:** Good, ~$0.03/1K tokens
- **DeepSeek API:** Cheapest, good quality

### Option 3: Hybrid
- Use local model for bulk discovery (background job)
- Use Claude/GPT-4 for validation of high-value dependencies

---

## Caching Strategy

Dependencies don't change frequently. Cache aggressively:

```rust
pub struct DependencyCache {
    // Market pair -> dependency
    dependencies: DashMap<(MarketId, MarketId), DiscoveredDependency>,
    
    // Market -> all related dependencies
    by_market: DashMap<MarketId, Vec<(MarketId, MarketId)>>,
    
    // TTL tracking
    expiry: DashMap<(MarketId, MarketId), Instant>,
}

impl DependencyCache {
    pub fn should_refresh(&self, market_a: &MarketId, market_b: &MarketId) -> bool {
        self.expiry
            .get(&(*market_a, *market_b))
            .map(|t| t.elapsed() > self.ttl)
            .unwrap_or(true)
    }
}
```

---

## Expected Results

Based on the paper's findings:

| Metric | Value |
|--------|-------|
| Market pairs checked | 46,360 |
| Valid dependencies found | 374 (after LLM) |
| True combinatorial arbitrage | 13 pairs |
| Total profit extracted | ~$95,000 |
| Profit per opportunity | ~$7,300 avg |

**Important:** Combinatorial is rare but high-value per trade. Most profit comes from simpler strategies.

---

## Recommended Roadmap

### Week 1: Infrastructure
1. Set up local LLM (vLLM + DeepSeek)
2. Implement `TextEmbedder` for topic clustering
3. Add `DependencyCache` to persist discoveries

### Week 2: Discovery Pipeline
1. Implement `DependencyDiscovery` service
2. Build prompt templates with chain-of-thought
3. Add validation logic for LLM outputs

### Week 3: Integration
1. Wire into existing `CombinatorialStrategy`
2. Add background job for periodic discovery
3. Connect to Frank-Wolfe projection

### Week 4: Testing & Tuning
1. Backtest on historical election data
2. Tune confidence thresholds
3. Add monitoring for false positives

---

## References

- [Unravelling the Probabilistic Forest](https://arxiv.org/abs/2508.03474) - The source paper
- [DeepSeek-R1-Distill-Qwen-32B](https://huggingface.co/deepseek-ai/DeepSeek-R1-Distill-Qwen-32B)
- [Linq-Embed-Mistral](https://huggingface.co/Linq-AI-Research/Linq-Embed-Mistral)
- [Chain-of-Thought Prompting](https://arxiv.org/abs/2201.11903)
