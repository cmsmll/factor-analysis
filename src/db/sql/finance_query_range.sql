SELECT
    datetime,
    total_shares,
    float_shares,
    total_market,
    float_market
FROM financial
WHERE datetime >= ?1 AND datetime < ?2
ORDER BY datetime;