SELECT
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
FROM market_data
ORDER BY datetime DESC
LIMIT 1;
