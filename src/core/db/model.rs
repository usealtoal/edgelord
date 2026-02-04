//! Database model types for Diesel ORM.

use diesel::prelude::*;

use super::schema::{clusters, relations};

/// Database row for a relation.
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = relations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RelationRow {
    pub id: String,
    pub kind: String,
    pub confidence: f32,
    pub reasoning: String,
    pub inferred_at: String,
    pub expires_at: String,
    pub market_ids: String,
}

/// Database row for a cluster.
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = clusters)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ClusterRow {
    pub id: String,
    pub market_ids: String,
    pub relation_ids: String,
    pub constraints_json: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;

    #[test]
    fn relation_row_is_insertable() {
        // Type check - if this compiles, the Insertable derive works
        let _row = RelationRow {
            id: "test".to_string(),
            kind: "{}".to_string(),
            confidence: 0.9,
            reasoning: "test".to_string(),
            inferred_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-02T00:00:00Z".to_string(),
            market_ids: "[]".to_string(),
        };
    }

    #[test]
    fn cluster_row_is_insertable() {
        let _row = ClusterRow {
            id: "test".to_string(),
            market_ids: "[]".to_string(),
            relation_ids: "[]".to_string(),
            constraints_json: "[]".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
    }
}
