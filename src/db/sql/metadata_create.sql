CREATE TABLE IF NOT EXISTS metadata (
    code TEXT PRIMARY KEY,
    exchange TEXT NOT NULL,
    name TEXT NOT NULL,
    prov TEXT NOT NULL,
    city TEXT NOT NULL,
    sw1 TEXT NOT NULL,
    sw2 TEXT NOT NULL,
    sw3 TEXT NOT NULL,
    indice TEXT NOT NULL,
    listing_date TEXT NOT NULL
);
