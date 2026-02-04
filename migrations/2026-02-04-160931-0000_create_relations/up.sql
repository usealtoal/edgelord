-- Relations table: stores inferred market relationships
CREATE TABLE relations (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,  -- JSON: {"type": "implies", "if_yes": "...", "then_yes": "..."}
    confidence REAL NOT NULL,
    reasoning TEXT NOT NULL,
    inferred_at TEXT NOT NULL,  -- ISO 8601
    expires_at TEXT NOT NULL,
    market_ids TEXT NOT NULL  -- JSON array for indexing
);

CREATE INDEX idx_relations_expires_at ON relations(expires_at);

-- Clusters table: groups of related markets with pre-computed constraints
CREATE TABLE clusters (
    id TEXT PRIMARY KEY NOT NULL,
    market_ids TEXT NOT NULL,  -- JSON array
    relation_ids TEXT NOT NULL,  -- JSON array
    constraints_json TEXT NOT NULL,  -- Pre-computed solver constraints
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_clusters_updated_at ON clusters(updated_at);
