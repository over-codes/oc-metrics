CREATE TABLE Metrics (
    name TEXT NOT NULL,
    time TEXT NOT NULL,
    value_type TEXT NOT NULL,
    dvalue REAL,
    tvalue TEXT,
    PRIMARY KEY (name, time)
)