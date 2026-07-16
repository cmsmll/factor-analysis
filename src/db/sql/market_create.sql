CREATE TABLE IF NOT EXISTS market_data (
    datetime TEXT PRIMARY KEY,
    change_percent REAL,
    open REAL,
    close REAL,
    high REAL,
    low REAL,
    volume REAL,
    turnover REAL,
    turnover_rate REAL,
    is_st INTEGER NOT NULL DEFAULT 0
);
