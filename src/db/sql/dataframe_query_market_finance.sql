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
    financial.total_shares,
    financial.float_shares,
    financial.total_market,
    financial.float_market
FROM market_data
INNER JOIN financial ON financial.datetime = market_data.datetime
WHERE market_data.datetime >= ?1 AND market_data.datetime < ?2
ORDER BY market_data.datetime;
