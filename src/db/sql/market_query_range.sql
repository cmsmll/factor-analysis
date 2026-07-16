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
WHERE datetime >= ?1 AND datetime < ?2
ORDER BY datetime;