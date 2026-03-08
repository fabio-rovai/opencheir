# Sentinel Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a single Rust binary that replaces 12 MCP servers, 3 hooks, and 1 plugin system with shared memory, enforcement, and self-healing.

**Architecture:** Layered Rust monolith — shared core (state, documents, search) at the bottom, domain modules in the middle, orchestration + gateway on top. One SQLite DB, one MCP server, one process. External third-party servers supervised as child processes.

**Tech Stack:** Rust, `rmcp` (MCP SDK), `axum` (HTTP), `tokio` (async), `rusqlite` (SQLite+FTS5), `docx-rs` (DOCX), `serde` (JSON/YAML/TOML), `clap` (CLI), `notify` (file watcher), `tokio-tungstenite` (WebSocket).

**Design doc:** `docs/plans/2026-03-08-sentinel-design.md`

---

## Phasing Strategy

The project is split into 7 phases. Each phase delivers a working, testable increment. Phases 1-3 are the foundation — nothing else works without them. Phases 4-6 are domain modules that can be built in parallel. Phase 7 is the orchestration layer.

```
Phase 1: Skeleton + State (foundation)
  ↓
Phase 2: MCP Gateway + Router (can talk to Claude)
  ↓
Phase 3: External Server Supervisor (proxies 3rd-party servers)
  ↓
Phase 4: Domain — Tender + QA + Social Value + Bid (parallel)
Phase 5: Domain — Eyes + Lineage (parallel)
Phase 6: Domain — Hive + Skills Engine (parallel)
  ↓
Phase 7: Supervisor + Enforcer + Init Command (the brain)
```

After Phase 3, the system is functional as a pass-through proxy. Each subsequent phase replaces one old server at a time. Old servers can run alongside sentinel during migration.

---

## Phase 1: Skeleton + State

**Goal:** Cargo project scaffolded, SQLite DB with full schema, config loading, CLI entry point.

**Estimated tasks:** 18

### Task 1.1: Initialize Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

**Step 1: Create the project**

```bash
cd /Users/fabio/projects/claude-sentinel
cargo init --name sentinel
```

**Step 2: Set up Cargo.toml with all dependencies**

Replace `Cargo.toml` with:

```toml
[package]
name = "sentinel"
version = "0.1.0"
edition = "2024"

[dependencies]
# MCP protocol
rmcp = { version = "0.1", features = ["server", "transport-io"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP server (lineage API, eyes)
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors"] }

# WebSocket (eyes)
tokio-tungstenite = "0.24"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Database
rusqlite = { version = "0.32", features = ["bundled", "fts5"] }

# Document handling
docx-rs = "0.4"

# Search
regex = "1"
rust-stemmers = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# File watching
notify = "7"
walkdir = "2"
glob = "0.3"

# Image handling (eyes)
image = "0.25"

# HTTP client (health checks)
reqwest = { version = "0.12", features = ["json"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
anyhow = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

**Step 3: Set up .gitignore**

```
/target
sentinel.db
*.db-shm
*.db-wal
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors (warnings OK)

**Step 5: Commit**

```bash
git init
git add Cargo.toml src/main.rs .gitignore
git commit -m "feat: initialize sentinel cargo project with dependencies"
```

---

### Task 1.2: Project Directory Structure

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/state.rs`
- Create: `src/core/documents.rs`
- Create: `src/core/search.rs`
- Create: `src/domain/mod.rs`
- Create: `src/domain/tender.rs`
- Create: `src/domain/qa.rs`
- Create: `src/domain/social_value.rs`
- Create: `src/domain/bid.rs`
- Create: `src/domain/eyes.rs`
- Create: `src/orchestration/mod.rs`
- Create: `src/orchestration/supervisor.rs`
- Create: `src/orchestration/enforcer.rs`
- Create: `src/orchestration/lineage.rs`
- Create: `src/orchestration/hive/mod.rs`
- Create: `src/orchestration/hive/planner.rs`
- Create: `src/orchestration/hive/coordinator.rs`
- Create: `src/orchestration/hive/spawner.rs`
- Create: `src/orchestration/hive/memory.rs`
- Create: `src/orchestration/skills.rs`
- Create: `src/gateway/mod.rs`
- Create: `src/gateway/server.rs`
- Create: `src/gateway/router.rs`
- Create: `src/gateway/proxy.rs`
- Create: `src/config.rs`

**Step 1: Create all module files with stub content**

Each file should contain:

```rust
// src/core/mod.rs
pub mod state;
pub mod documents;
pub mod search;
```

```rust
// src/domain/mod.rs
pub mod tender;
pub mod qa;
pub mod social_value;
pub mod bid;
pub mod eyes;
```

```rust
// src/orchestration/mod.rs
pub mod supervisor;
pub mod enforcer;
pub mod lineage;
pub mod hive;
pub mod skills;
```

```rust
// src/orchestration/hive/mod.rs
pub mod planner;
pub mod coordinator;
pub mod spawner;
pub mod memory;
```

```rust
// src/gateway/mod.rs
pub mod server;
pub mod router;
pub mod proxy;
```

All leaf modules (state.rs, documents.rs, etc.) contain:

```rust
// TODO: Phase N implementation
```

```rust
// src/main.rs
mod core;
mod domain;
mod orchestration;
mod gateway;
mod config;

fn main() {
    println!("sentinel: not yet implemented");
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles

**Step 3: Commit**

```bash
git add src/
git commit -m "feat: scaffold full module structure"
```

---

### Task 1.3: CLI Entry Point with Clap

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the failing test**

```rust
// tests/cli_test.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    Command::cargo_bin("sentinel")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sentinel"));
}

#[test]
fn test_cli_init_subcommand_exists() {
    Command::cargo_bin("sentinel")
        .unwrap()
        .arg("init")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_serve_subcommand_exists() {
    Command::cargo_bin("sentinel")
        .unwrap()
        .arg("serve")
        .arg("--help")
        .assert()
        .success();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test cli_test`
Expected: FAIL

**Step 3: Implement CLI**

```rust
// src/main.rs
mod core;
mod domain;
mod orchestration;
mod gateway;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sentinel", about = "One brain to rule them all")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize sentinel: create DB, seed data, update settings
    Init {
        /// Path to sentinel data directory
        #[arg(long, default_value = "~/.sentinel")]
        data_dir: String,
    },
    /// Start the MCP server
    Serve {
        /// Path to config file
        #[arg(long, default_value = "~/.sentinel/config.toml")]
        config: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { data_dir } => {
            println!("Initializing sentinel at {data_dir}...");
            // TODO: Phase 7
        }
        Commands::Serve { config } => {
            println!("Starting sentinel with config {config}...");
            // TODO: Phase 2
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test cli_test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: CLI with init and serve subcommands"
```

---

### Task 1.4: Config Loading

**Files:**
- Modify: `src/config.rs`
- Create: `config.toml.example`
- Create: `tests/config_test.rs`

**Step 1: Write the failing test**

```rust
// tests/config_test.rs
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_load_config_from_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"
[general]
data_dir = "/tmp/sentinel-test"
tenders_root = "/tmp/tenders"

[supervisor]
health_check_interval = "5s"
max_restart_attempts = 3

[hive]
max_agents = 5
claude_path = "claude"
"#).unwrap();

    let config = sentinel::config::Config::load(f.path()).unwrap();
    assert_eq!(config.general.data_dir, "/tmp/sentinel-test");
    assert_eq!(config.hive.max_agents, 5);
}

#[test]
fn test_config_defaults() {
    let config = sentinel::config::Config::default();
    assert_eq!(config.hive.max_agents, 5);
    assert_eq!(config.supervisor.max_restart_attempts, 3);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test config_test`
Expected: FAIL

**Step 3: Implement config**

```rust
// src/config.rs
use serde::Deserialize;
use std::path::Path;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub supervisor: SupervisorConfig,
    #[serde(default)]
    pub enforcer: EnforcerConfig,
    #[serde(default)]
    pub hive: HiveConfig,
    #[serde(default)]
    pub eyes: EyesConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub lineage: LineageConfig,
    #[serde(default)]
    pub external_servers: HashMap<String, ExternalServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_tenders_root")]
    pub tenders_root: String,
    #[serde(default = "default_skills_dir")]
    pub skills_dir: String,
    #[serde(default = "default_personal_skills_dir")]
    pub personal_skills_dir: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SupervisorConfig {
    #[serde(default = "default_health_interval")]
    pub health_check_interval: String,
    #[serde(default = "default_max_restarts")]
    pub max_restart_attempts: u32,
    #[serde(default = "default_restart_cooldown")]
    pub restart_cooldown: String,
    #[serde(default = "default_pattern_interval")]
    pub pattern_analysis_interval: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnforcerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_action")]
    pub default_action: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HiveConfig {
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,
    #[serde(default = "default_claude_path")]
    pub claude_path: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_agent_timeout")]
    pub agent_timeout: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EyesConfig {
    #[serde(default)]
    pub port: u16,
    #[serde(default = "default_max_image_width")]
    pub max_image_width: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_max_features")]
    pub max_features: u32,
    #[serde(default = "default_ngram_range")]
    pub ngram_range: [u32; 2],
    #[serde(default = "default_min_df")]
    pub min_df: u32,
    #[serde(default = "default_max_df")]
    pub max_df: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LineageConfig {
    #[serde(default)]
    pub http_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExternalServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// Default value functions
fn default_data_dir() -> String { "~/.sentinel".into() }
fn default_tenders_root() -> String { "~/Desktop/Tenders".into() }
fn default_skills_dir() -> String { "~/.sentinel/skills".into() }
fn default_personal_skills_dir() -> String { "~/.claude/skills".into() }
fn default_health_interval() -> String { "5s".into() }
fn default_max_restarts() -> u32 { 3 }
fn default_restart_cooldown() -> String { "60s".into() }
fn default_pattern_interval() -> u32 { 100 }
fn default_true() -> bool { true }
fn default_action() -> String { "block".into() }
fn default_max_agents() -> u32 { 5 }
fn default_claude_path() -> String { "claude".into() }
fn default_model() -> String { "claude-sonnet-4-6".into() }
fn default_agent_timeout() -> String { "300s".into() }
fn default_max_image_width() -> u32 { 800 }
fn default_max_features() -> u32 { 20000 }
fn default_ngram_range() -> [u32; 2] { [1, 2] }
fn default_min_df() -> u32 { 2 }
fn default_max_df() -> f64 { 0.9 }

impl Default for Config {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

impl Default for GeneralConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for SupervisorConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for EnforcerConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for HiveConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for EyesConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for SearchConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}
impl Default for LineageConfig {
    fn default() -> Self { toml::from_str("").unwrap() }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
```

Also add to `src/main.rs` after `mod config;`:

```rust
// Make config module public for integration tests
pub use config as config;
```

Update `src/main.rs` to add `pub` visibility:

```rust
// src/lib.rs (NEW FILE — needed for integration tests to access modules)
pub mod config;
// Re-export core/domain/etc as needed later
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test config_test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs src/lib.rs config.toml.example tests/config_test.rs
git commit -m "feat: config loading with TOML and sensible defaults"
```

---

### Task 1.5: State Module — SQLite DB + Full Schema

**Files:**
- Modify: `src/core/state.rs`
- Create: `src/core/migrations/001_initial.sql`
- Create: `tests/state_test.rs`

**Step 1: Write the failing test**

```rust
// tests/state_test.rs
use tempfile::TempDir;

#[test]
fn test_create_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("sentinel.db");
    let db = sentinel::core::state::StateDb::open(&db_path).unwrap();
    assert!(db_path.exists());
}

#[test]
fn test_schema_tables_exist() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("sentinel.db");
    let db = sentinel::core::state::StateDb::open(&db_path).unwrap();

    let tables = db.list_tables().unwrap();
    assert!(tables.contains(&"sessions".to_string()));
    assert!(tables.contains(&"documents".to_string()));
    assert!(tables.contains(&"questions".to_string()));
    assert!(tables.contains(&"events".to_string()));
    assert!(tables.contains(&"goals".to_string()));
    assert!(tables.contains(&"tasks".to_string()));
    assert!(tables.contains(&"learnings".to_string()));
    assert!(tables.contains(&"company".to_string()));
    assert!(tables.contains(&"toms".to_string()));
    assert!(tables.contains(&"frameworks".to_string()));
    assert!(tables.contains(&"health".to_string()));
    assert!(tables.contains(&"enforcement".to_string()));
    assert!(tables.contains(&"rules".to_string()));
    assert!(tables.contains(&"patterns".to_string()));
    assert!(tables.contains(&"skills".to_string()));
    assert!(tables.contains(&"qa_results".to_string()));
}

#[test]
fn test_create_session() {
    let dir = TempDir::new().unwrap();
    let db = sentinel::core::state::StateDb::open(&dir.path().join("test.db")).unwrap();
    let session_id = db.create_session(Some("test-project")).unwrap();
    assert!(!session_id.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test state_test`
Expected: FAIL

**Step 3: Write the SQL migration**

```sql
-- src/core/migrations/001_initial.sql
-- Full schema from design doc

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
    tender_name TEXT,
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

CREATE TABLE IF NOT EXISTS questions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL REFERENCES documents(id),
    question_id     TEXT NOT NULL,
    text            TEXT,
    word_limit      INTEGER,
    question_type   TEXT NOT NULL,
    answer          TEXT,
    table_index     INTEGER,
    row_index       INTEGER,
    cell_index      INTEGER,
    score_weight    REAL,
    parsed_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS company (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    sensitive   INTEGER DEFAULT 0,
    category    TEXT
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

CREATE TABLE IF NOT EXISTS toms (
    reference       TEXT PRIMARY KEY,
    theme           TEXT NOT NULL,
    outcome         TEXT NOT NULL,
    title           TEXT NOT NULL,
    units           TEXT,
    proxy_value     REAL,
    definition      TEXT,
    target_requirements TEXT,
    evidence_required   TEXT,
    unit_guidance       TEXT,
    tags            TEXT
);

CREATE TABLE IF NOT EXISTS frameworks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    category    TEXT,
    content     TEXT NOT NULL,
    tags        TEXT
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

-- FTS5 sync triggers
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
```

**Step 4: Implement StateDb**

```rust
// src/core/state.rs
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const MIGRATION: &str = include_str!("migrations/001_initial.sql");

pub struct StateDb {
    conn: Arc<Mutex<Connection>>,
}

impl StateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(MIGRATION)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )?;
        let tables = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tables)
    }

    pub fn create_session(&self, project: Option<&str>) -> Result<String> {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project) VALUES (?1, ?2)",
            rusqlite::params![id, project],
        )?;
        Ok(id)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}
```

**Step 5: Update lib.rs to export core**

```rust
// src/lib.rs
pub mod config;
pub mod core;
```

**Step 6: Run tests**

Run: `cargo test --test state_test`
Expected: PASS

**Step 7: Commit**

```bash
git add src/core/state.rs src/core/migrations/ tests/state_test.rs src/lib.rs
git commit -m "feat: SQLite state DB with full schema and migrations"
```

---

### Task 1.6: Seed Data — Company

**Files:**
- Create: `data/company.json`
- Modify: `src/core/state.rs` (add seed_company method)
- Create: `tests/seed_test.rs`

**Step 1: Write the failing test**

```rust
// tests/seed_test.rs
use tempfile::TempDir;

#[test]
fn test_seed_company_data() {
    let dir = TempDir::new().unwrap();
    let db = sentinel::core::state::StateDb::open(&dir.path().join("test.db")).unwrap();
    db.seed_company().unwrap();

    let conn = db.conn();
    let vat: String = conn.query_row(
        "SELECT value FROM company WHERE key = 'vat_number'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(!vat.is_empty());

    let sensitive_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM company WHERE sensitive = 1",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(sensitive_count >= 3); // UTR, CDP password, mobile, PSC DOB
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test seed_test -- test_seed_company`
Expected: FAIL

**Step 3: Create company.json from company_data.py**

Read the existing data from `/Users/fabio/projects/tender-qa-mcp/tender_qa_mcp/company_data.py` and convert to JSON. Store at `data/company.json`.

```json
{
  "company": [
    {"key": "name", "value": "Kampakis and Co. Ltd", "sensitive": false, "category": "company"},
    {"key": "trading_as", "value": "The Tesseract Academy", "sensitive": false, "category": "company"},
    {"key": "registration_number", "value": "<from source>", "sensitive": false, "category": "company"},
    {"key": "vat_number", "value": "<from source>", "sensitive": false, "category": "company"},
    {"key": "address", "value": "<from source>", "sensitive": false, "category": "company"}
  ],
  "psc": [
    {"key": "psc_name", "value": "Dr. Stylianos Kampakis", "sensitive": false, "category": "psc"},
    {"key": "psc_dob", "value": "<from source>", "sensitive": true, "category": "psc"}
  ],
  "sensitive_fields": [
    {"key": "utr", "value": "<from source>", "sensitive": true, "category": "sensitive"},
    {"key": "cdp_password", "value": "<from source>", "sensitive": true, "category": "sensitive"},
    {"key": "mobile", "value": "<from source>", "sensitive": true, "category": "sensitive"}
  ]
}
```

> **Note to implementer:** Read actual values from `/Users/fabio/projects/tender-qa-mcp/tender_qa_mcp/company_data.py`. Do NOT commit actual sensitive values to git — use `data/company.json` as a local-only file listed in `.gitignore`.

**Step 4: Implement seed_company**

```rust
// Add to src/core/state.rs
impl StateDb {
    pub fn seed_company(&self) -> Result<()> {
        let json_str = include_str!("../../data/company.json");
        let data: serde_json::Value = serde_json::from_str(json_str)?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO company (key, value, sensitive, category) VALUES (?1, ?2, ?3, ?4)"
        )?;

        for section in ["company", "psc", "sensitive_fields"] {
            if let Some(items) = data.get(section).and_then(|v| v.as_array()) {
                for item in items {
                    stmt.execute(rusqlite::params![
                        item["key"].as_str().unwrap_or_default(),
                        item["value"].as_str().unwrap_or_default(),
                        item["sensitive"].as_bool().unwrap_or(false) as i32,
                        item["category"].as_str().unwrap_or_default(),
                    ])?;
                }
            }
        }
        Ok(())
    }
}
```

**Step 5: Run tests**

Run: `cargo test --test seed_test -- test_seed_company`
Expected: PASS

**Step 6: Commit**

```bash
git add data/company.json src/core/state.rs tests/seed_test.rs .gitignore
git commit -m "feat: seed company data from JSON into sentinel.db"
```

---

## Phase 2: MCP Gateway + Router

**Goal:** A working MCP server that Claude Code can connect to. Routes tool calls to stub handlers. Enforcer and lineage hooks are no-ops.

### Task 2.1: MCP Server Skeleton with rmcp

**Files:**
- Modify: `src/gateway/server.rs`
- Create: `tests/gateway_test.rs`

**Step 1: Write the failing test**

```rust
// tests/gateway_test.rs
#[test]
fn test_tool_list_not_empty() {
    let tools = sentinel::gateway::server::tool_definitions();
    assert!(!tools.is_empty());
    // Should have at least sentinel_status
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"sentinel_status"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test gateway_test`
Expected: FAIL

**Step 3: Implement MCP server skeleton**

Implement `src/gateway/server.rs` using the `rmcp` crate to:
- Register all tool definitions (65 tools from design doc)
- Handle MCP initialize/call handshake
- Route to stub handlers that return `{"status": "not_implemented"}`

> **Reference:** Check `rmcp` crate docs for exact API. The server reads JSON-RPC from stdin, writes to stdout.

**Step 4: Run test**

Run: `cargo test --test gateway_test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/gateway/ tests/gateway_test.rs
git commit -m "feat: MCP gateway skeleton with tool definitions"
```

---

### Task 2.2: Tool Router

**Files:**
- Modify: `src/gateway/router.rs`

Implement routing table from design doc:

```
tender_*     → domain::tender
qa_*         → domain::qa
sv_*         → domain::social_value
bid_*        → domain::bid
eyes_*       → domain::eyes
lineage_*    → orchestration::lineage
hive_*       → orchestration::hive
skill_*      → orchestration::skills
sentinel_*   → orchestration::supervisor
word_*       → gateway::proxy (word-document-server)
mermaid_*    → gateway::proxy (mermaid-kroki)
puppeteer_*  → gateway::proxy (puppeteer)
threejs_*    → gateway::proxy (threejs)
```

Each route dispatches to a trait method. Stub implementations return "not implemented" errors until the domain module is built.

---

### Task 2.3: Wire Serve Command to MCP Server

**Files:**
- Modify: `src/main.rs`

Connect `sentinel serve` to start the MCP server on stdin/stdout. At this point Claude Code can connect:

```json
{
  "mcpServers": {
    "sentinel": {
      "command": "/path/to/sentinel",
      "args": ["serve"]
    }
  }
}
```

---

## Phase 3: External Server Supervisor

**Goal:** Spawn, health-check, and proxy tool calls to the 4 third-party MCP servers.

### Task 3.1: External Process Spawner

Spawn child processes for word-document-server, mermaid-kroki, puppeteer, threejs. Hold stdin/stdout handles.

### Task 3.2: Health Check Loop

Every 5s, send MCP `initialize` to each child. Track health status in `health` table.

### Task 3.3: Auto-Restart on Failure

If health check fails, kill and respawn. Track restart count. After 3 failures in 60s, mark as `down`.

### Task 3.4: Proxy Tool Calls

Forward `word_*`, `mermaid_*`, `puppeteer_*`, `threejs_*` calls to the appropriate child process via its stdin/stdout.

---

## Phase 4: Domain Modules (can parallelize)

Build each domain module independently. Each module:
1. Implements the tool handler trait
2. Uses shared core (StateDb, DocumentService, SearchService)
3. Has integration tests

### Task 4.1: Social Value Module (simplest — static data)

Port `social_value_mcp` to Rust. Pure data lookups + calculations. 7 tools.

### Task 4.2: QA Module

Port `tender_qa_mcp` to Rust. DOCX parsing for font/dash/word count checks. 13 tools.

### Task 4.3: Tender Module

Port `tender_orchestrator` to Rust. DOCX parsing, cell reading/writing, subprocess calls to LibreOffice. 12 tools.

### Task 4.4: Bid Module

Port `bid-writing-mcp` to Rust. Framework lookups, scoring algorithms. 14 tools. Convert 264KB TypeScript data to JSON.

### Task 4.5: Documents Service (shared core)

Implement `src/core/documents.rs` — DOCX parsing with `docx-rs`, PDF extraction via subprocess, rendering via LibreOffice.

### Task 4.6: Search Service (shared core)

Implement `src/core/search.rs` — FTS5 index management, TF-IDF vector construction, cosine similarity search.

---

## Phase 5: Eyes + Lineage

### Task 5.1: Lineage Module

Port lineage TypeScript to Rust. Event store in SQLite, graph builder, rules engine, HTTP API for VSCode extension.

### Task 5.2: Eyes Module

Port eyes Go to Rust. HTTP server, WebSocket endpoint, browser client serving, PNG capture handling.

---

## Phase 6: Hive + Skills Engine

### Task 6.1: Skills Engine

Port superpowers `skills-core.js` to Rust. Directory scanning, YAML frontmatter parsing, skill resolution, hot-reload with `notify`.

### Task 6.2: Hive Planner

Port hive planner from Go. Spawns Claude CLI for goal decomposition, parses JSON DAG.

### Task 6.3: Hive Coordinator

Port coordinator from Go. DAG execution, dependency resolution, parallel spawning.

### Task 6.4: Hive Spawner

Port spawner from Go. `tokio::process` for Claude CLI management.

### Task 6.5: Hive Memory

Port memory from Go. Shared `learnings` + `learnings_fts` tables (already in schema).

---

## Phase 7: Supervisor + Enforcer + Init

### Task 7.1: Enforcer Engine

Implement rule matching against recent tool call history. 6 built-in rules from design doc. Block/warn verdicts.

### Task 7.2: Supervisor Health Dashboard

`sentinel_status` tool returning full health report. Pattern analysis on enforcement log.

### Task 7.3: `sentinel init` Command

Full initialization flow: create DB, seed data, migrate hive.db, update settings.json, generate CLAUDE.md.

### Task 7.4: Cross-Session Pattern Discovery

Analyze enforcement log + QA results to discover patterns. Store in `patterns` table. Surface actionable insights.

---

## Testing Strategy

- **Unit tests** per module (`cargo test`)
- **Integration tests** per phase (MCP tool call → response round-trip)
- **Smoke test:** Configure Claude Code to use sentinel, run a tender workflow, verify all tools respond
- **Migration test:** Run sentinel alongside old servers, compare tool outputs

## Commit Cadence

- Every task gets a commit
- Every phase gets a tag: `v0.1.0` (Phase 1), `v0.2.0` (Phase 2), etc.
- Phase 3 is the first "usable" release (proxy mode)

---

Plan complete and saved to `docs/plans/2026-03-08-sentinel-implementation.md`.

**Two execution options:**

**1. Subagent-Driven (this session)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** — Open new session with executing-plans, batch execution with checkpoints

Which approach?
