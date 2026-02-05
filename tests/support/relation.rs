use edgelord::core::domain::{MarketId, Relation, RelationKind};

pub fn mutually_exclusive(markets: &[&str], confidence: f64, reasoning: &str) -> Relation {
    Relation::new(
        RelationKind::MutuallyExclusive {
            markets: markets.iter().map(|m| MarketId::from(*m)).collect(),
        },
        confidence,
        reasoning,
    )
}

pub fn exactly_one(markets: &[&str], confidence: f64, reasoning: &str) -> Relation {
    Relation::new(
        RelationKind::ExactlyOne {
            markets: markets.iter().map(|m| MarketId::from(*m)).collect(),
        },
        confidence,
        reasoning,
    )
}
