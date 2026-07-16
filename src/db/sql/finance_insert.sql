INSERT OR REPLACE INTO financial (
    datetime,
    total_shares,
    float_shares,
    total_market,
    float_market
)
VALUES (?1, ?2, ?3, ?4, ?5);
