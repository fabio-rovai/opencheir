CREATE TABLE IF NOT EXISTS ontology_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    content     TEXT NOT NULL,
    format      TEXT NOT NULL DEFAULT 'ntriples',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
