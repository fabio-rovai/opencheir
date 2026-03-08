PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;

CREATE TABLE IF NOT EXISTS sessions (
    id          TEXT PRIMARY KEY,
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at    TEXT,
    project     TEXT,
    summary     TEXT
);

CREATE TABLE IF NOT EXISTS documents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    path        TEXT NOT NULL,
    filename    TEXT NOT NULL,
    doc_type    TEXT NOT NULL,
    project_name TEXT,
    text        TEXT,
    mtime       INTEGER,
    parsed_at   TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(path)
);

CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
    source,
    source_id,
    title,
    content,
    tags,
    tokenize='porter unicode61'
);

CREATE TABLE IF NOT EXISTS qa_results (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    doc_path    TEXT NOT NULL,
    check_type  TEXT NOT NULL,
    status      TEXT NOT NULL,
    details     TEXT,
    checked_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS events (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    timestamp   INTEGER NOT NULL,
    event_type  TEXT NOT NULL,
    path        TEXT,
    tool        TEXT,
    meta        TEXT
);

CREATE TABLE IF NOT EXISTS goals (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    session_id  TEXT REFERENCES sessions(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    goal_id     TEXT NOT NULL REFERENCES goals(id),
    description TEXT NOT NULL,
    role        TEXT,
    status      TEXT NOT NULL DEFAULT 'pending',
    depends_on  TEXT,
    agent_pid   INTEGER,
    artifacts   TEXT,
    stdout      TEXT,
    stderr      TEXT,
    exit_code   INTEGER,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS learnings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    domain      TEXT NOT NULL,
    lesson      TEXT NOT NULL,
    context     TEXT,
    tags        TEXT,
    source_task TEXT,
    outcome     TEXT,
    times_used  INTEGER DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS learnings_fts USING fts5(
    domain, lesson, context, tags,
    content=learnings,
    content_rowid=id,
    tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS learnings_ai AFTER INSERT ON learnings BEGIN
    INSERT INTO learnings_fts(rowid, domain, lesson, context, tags)
    VALUES (new.id, new.domain, new.lesson, new.context, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS learnings_ad AFTER DELETE ON learnings BEGIN
    INSERT INTO learnings_fts(learnings_fts, rowid, domain, lesson, context, tags)
    VALUES ('delete', old.id, old.domain, old.lesson, old.context, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS learnings_au AFTER UPDATE ON learnings BEGIN
    INSERT INTO learnings_fts(learnings_fts, rowid, domain, lesson, context, tags)
    VALUES ('delete', old.id, old.domain, old.lesson, old.context, old.tags);
    INSERT INTO learnings_fts(rowid, domain, lesson, context, tags)
    VALUES (new.id, new.domain, new.lesson, new.context, new.tags);
END;

CREATE TABLE IF NOT EXISTS health (
    component   TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    status      TEXT NOT NULL,
    last_check  TEXT,
    error       TEXT,
    restart_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS enforcement (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    timestamp   TEXT NOT NULL DEFAULT (datetime('now')),
    rule        TEXT NOT NULL,
    action      TEXT NOT NULL,
    tool_call   TEXT,
    reason      TEXT
);

CREATE TABLE IF NOT EXISTS rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    condition   TEXT NOT NULL,
    action      TEXT NOT NULL DEFAULT 'block',
    enabled     INTEGER DEFAULT 1,
    auto_learned INTEGER DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS patterns (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category    TEXT NOT NULL,
    description TEXT NOT NULL,
    evidence    TEXT,
    confidence  REAL DEFAULT 0.5,
    occurrences INTEGER DEFAULT 1,
    first_seen  TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen   TEXT NOT NULL DEFAULT (datetime('now')),
    actionable  INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS skills (
    name        TEXT PRIMARY KEY,
    source      TEXT NOT NULL,
    file_path   TEXT NOT NULL,
    description TEXT,
    last_loaded TEXT,
    load_count  INTEGER DEFAULT 0,
    healthy     INTEGER DEFAULT 1
);
