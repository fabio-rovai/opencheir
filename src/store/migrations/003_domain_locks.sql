CREATE TABLE IF NOT EXISTS locks (
    id          TEXT PRIMARY KEY,
    domain      TEXT NOT NULL UNIQUE,
    locked_by   TEXT NOT NULL,
    locked_at   TEXT NOT NULL DEFAULT (datetime('now')),
    ttl_seconds INTEGER NOT NULL DEFAULT 60,
    expires_at  TEXT NOT NULL
);
