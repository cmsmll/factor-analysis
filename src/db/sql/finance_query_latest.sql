SELECT
    datetime,
    total_shares,
    float_shares,
    total_market,
    float_market
FROM financial
ORDER BY datetime DESC
LIMIT 1;
