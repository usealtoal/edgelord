-- Opportunities: every detected arbitrage opportunity
CREATE TABLE opportunities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy TEXT NOT NULL,
    market_ids TEXT NOT NULL,
    edge REAL NOT NULL,
    expected_profit REAL NOT NULL,
    detected_at TEXT NOT NULL,
    executed INTEGER NOT NULL DEFAULT 0,
    rejected_reason TEXT
);

CREATE INDEX idx_opportunities_detected_at ON opportunities(detected_at);
CREATE INDEX idx_opportunities_strategy ON opportunities(strategy);

-- Trades: executed positions
CREATE TABLE trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    opportunity_id INTEGER NOT NULL,
    strategy TEXT NOT NULL,
    market_ids TEXT NOT NULL,
    legs TEXT NOT NULL,
    size REAL NOT NULL,
    expected_profit REAL NOT NULL,
    realized_profit REAL,
    status TEXT NOT NULL DEFAULT 'open',
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    close_reason TEXT,
    FOREIGN KEY (opportunity_id) REFERENCES opportunities(id)
);

CREATE INDEX idx_trades_opened_at ON trades(opened_at);
CREATE INDEX idx_trades_status ON trades(status);

-- Daily stats: pre-aggregated for fast queries
CREATE TABLE daily_stats (
    date TEXT PRIMARY KEY NOT NULL,
    opportunities_detected INTEGER NOT NULL DEFAULT 0,
    opportunities_executed INTEGER NOT NULL DEFAULT 0,
    opportunities_rejected INTEGER NOT NULL DEFAULT 0,
    trades_opened INTEGER NOT NULL DEFAULT 0,
    trades_closed INTEGER NOT NULL DEFAULT 0,
    profit_realized REAL NOT NULL DEFAULT 0,
    loss_realized REAL NOT NULL DEFAULT 0,
    win_count INTEGER NOT NULL DEFAULT 0,
    loss_count INTEGER NOT NULL DEFAULT 0,
    total_volume REAL NOT NULL DEFAULT 0,
    peak_exposure REAL NOT NULL DEFAULT 0,
    latency_sum_ms INTEGER NOT NULL DEFAULT 0,
    latency_count INTEGER NOT NULL DEFAULT 0
);

-- Per-strategy daily breakdown
CREATE TABLE strategy_daily_stats (
    date TEXT NOT NULL,
    strategy TEXT NOT NULL,
    opportunities_detected INTEGER NOT NULL DEFAULT 0,
    opportunities_executed INTEGER NOT NULL DEFAULT 0,
    trades_opened INTEGER NOT NULL DEFAULT 0,
    trades_closed INTEGER NOT NULL DEFAULT 0,
    profit_realized REAL NOT NULL DEFAULT 0,
    win_count INTEGER NOT NULL DEFAULT 0,
    loss_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (date, strategy)
);
