SELECT
    exchange,
    name,
    code,
    prov,
    city,
    sw1,
    sw2,
    sw3,
    indice,
    listing_date
FROM metadata
WHERE code = ?1;
