INSERT OR REPLACE INTO metadata (
    code,
    exchange,
    name,
    prov,
    city,
    sw1,
    sw2,
    sw3,
    indice,
    listing_date
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);
