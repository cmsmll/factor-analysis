SELECT
    market_data.datetime,
    market_data.change_percent,
    market_data.open,
    market_data.close,
    market_data.high,
    market_data.low,
    market_data.volume,
    market_data.turnover,
    market_data.turnover_rate,
    market_data.is_st,
    NULL,
    NULL,
    NULL,
    NULL,
    NULL
FROM market_data
WHERE market_data.datetime >= ?1 AND market_data.datetime < ?2
ORDER BY market_data.datetime;