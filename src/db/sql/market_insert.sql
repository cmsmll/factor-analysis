INSERT OR REPLACE INTO market_data (
    datetime,
    change_percent,
    open,
    close,
    high,
    low,
    volume,
    turnover,
    turnover_rate,
    is_st
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);
