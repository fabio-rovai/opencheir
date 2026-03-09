# Enforcer Hot-Reload & Domain Locking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the `notify` crate to hot-reload enforcer rules from `config.toml` → SQLite DB, and add a domain-locking mechanism to prevent concurrent hive memory write races.

**Architecture:** TOML rules are seeded into the `rules` DB table on startup; the `Enforcer` loads its rule-set from DB at startup and after each config file change detected by a `notify` watcher. Domain locks live in a new `locks` table; agents call `hive_claim_domain` before writing and `hive_release_domain` when done; `hive_memory_store` rejects writes to a locked domain unless the caller presents the matching token.

**Tech Stack:** Rust, `notify` v8 (already in `Cargo.toml`), `tokio::sync::watch`, `rusqlite`, `rmcp` MCP macros, `serde_json` for condition serialisation.

---

## Task 1: Add `Deserialize` to enforcer types + `reload_from_db` + `seed_to_db`

**Files:**
- Modify: `src/orchestration/enforcer.rs`

### Step 1: Write the failing test

Add at the bottom of `src/orchestration/enforcer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::state::StateDb;
    use tempfile::tempdir;

    fn test_db() -> StateDb {
        let dir = tempdir().unwrap();
        StateDb::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn test_seed_and_reload() {
        let db = test_db();
        Enforcer::seed_builtins_to_db(&db).unwrap();
        let mut e = Enforcer::new();
        e.reload_from_db(&db).unwrap();
        assert!(!e.rules().is_empty());
        assert_eq!(
            e.rules().iter().find(|r| r.name == "qa_after_docx_write").unwrap().enabled,
            true
        );
    }

    #[test]
    fn test_condition_roundtrip() {
        let cond = RuleCondition::MissingInWindow {
            trigger: "write_doc".into(),
            required: "qa_".into(),
            window: 3,
        };
        let json = serde_json::to_string(&cond).unwrap();
        let decoded: RuleCondition = serde_json::from_str(&json).unwrap();
        match decoded {
            RuleCondition::MissingInWindow { trigger, required, window } => {
                assert_eq!(trigger, "write_doc");
                assert_eq!(required, "qa_");
                assert_eq!(window, 3);
            }
            _ => panic!("wrong variant"),
        }
    }
}
```

### Step 2: Run test to verify it fails

```bash
cd /Users/fabio/projects/opencheir
cargo test test_seed_and_reload -- --nocapture 2>&1 | tail -20
```

Expected: compile error — `seed_builtins_to_db` and `reload_from_db` not found; `Deserialize` not derived.

### Step 3: Implement

In `src/orchestration/enforcer.rs`, make these changes:

**a) Add `Deserialize` to `Action` and `RuleCondition`:**

```rust
// Change from:
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Action {

// To:
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
```

```rust
// Change from:
#[derive(Debug, Clone)]
pub enum RuleCondition {

// To:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
```

Add `use serde::Deserialize;` to the top imports (already has `Serialize`):
```rust
use serde::{Deserialize, Serialize};
```

**b) Add `seed_builtins_to_db` and `reload_from_db` as associated/instance methods.**

Add these inside `impl Enforcer`, after `set_rule_enabled`:

```rust
/// Seed the built-in hardcoded rules into the `rules` table.
/// Uses INSERT OR IGNORE so existing DB customisations are preserved.
pub fn seed_builtins_to_db(db: &StateDb) -> anyhow::Result<()> {
    let builtins = Enforcer::new();
    let conn = db.conn();
    for rule in &builtins.rules {
        let action_str = match rule.action {
            Action::Block => "block",
            Action::Warn => "warn",
            Action::Allow => "allow",
        };
        let condition_json = serde_json::to_string(&rule.condition)?;
        conn.execute(
            "INSERT OR IGNORE INTO rules (name, description, condition, action, enabled) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                rule.name,
                rule.description,
                condition_json,
                action_str,
                rule.enabled as i32,
            ],
        )?;
    }
    Ok(())
}

/// Replace the in-memory rule-set with everything enabled in the `rules` table.
/// The sliding window (recent_calls) is left untouched.
pub fn reload_from_db(&mut self, db: &StateDb) -> anyhow::Result<()> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT name, description, condition, action, enabled FROM rules",
    )?;

    let rules: Vec<Rule> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i32>(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .filter_map(|(name, description, condition_json, action_str, enabled)| {
            let condition: RuleCondition = serde_json::from_str(&condition_json).ok()?;
            let action = match action_str.as_str() {
                "block" => Action::Block,
                "warn" => Action::Warn,
                _ => Action::Allow,
            };
            Some(Rule {
                name,
                description,
                action,
                enabled: enabled != 0,
                condition,
            })
        })
        .collect();

    self.rules = rules;
    Ok(())
}
```

### Step 4: Run tests to verify they pass

```bash
cargo test test_seed_and_reload test_condition_roundtrip -- --nocapture 2>&1 | tail -20
```

Expected: both tests PASS.

### Step 5: Commit

```bash
cd /Users/fabio/projects/opencheir
git add src/orchestration/enforcer.rs
git commit -m "feat: add serde to enforcer types, seed_builtins_to_db, reload_from_db"
```

---

## Task 2: `RuleConditionConfig` + `RuleConfig` in config.rs + `seed_config_rules_to_db`

**Files:**
- Modify: `src/config.rs`
- Modify: `src/orchestration/enforcer.rs`

### Step 1: Write the failing test

Add to the `tests` module in `src/orchestration/enforcer.rs`:

```rust
#[test]
fn test_seed_config_rules() {
    use crate::config::RuleConfig;
    let db = test_db();
    Enforcer::seed_builtins_to_db(&db).unwrap();

    let custom = vec![RuleConfig {
        name: "custom_test_rule".into(),
        description: Some("test".into()),
        action: "warn".into(),
        enabled: Some(true),
        condition: crate::config::RuleConditionConfig {
            kind: "MissingInWindow".into(),
            trigger: Some("foo".into()),
            required: Some("bar".into()),
            window: Some(2),
            category: None,
            count: None,
        },
    }];
    Enforcer::seed_config_rules_to_db(&db, &custom).unwrap();

    let mut e = Enforcer::new();
    e.reload_from_db(&db).unwrap();
    assert!(e.rules().iter().any(|r| r.name == "custom_test_rule"));
}
```

### Step 2: Run to verify it fails

```bash
cargo test test_seed_config_rules -- --nocapture 2>&1 | tail -20
```

Expected: compile error — `RuleConfig`, `RuleConditionConfig`, `seed_config_rules_to_db` not found.

### Step 3: Add `RuleConditionConfig` and `RuleConfig` to `src/config.rs`

Add after the existing `EnforcerConfig` block:

```rust
/// A single enforcer rule defined in TOML config.
#[derive(Debug, Deserialize, Clone)]
pub struct RuleConfig {
    pub name: String,
    pub description: Option<String>,
    pub action: String,
    pub enabled: Option<bool>,
    pub condition: RuleConditionConfig,
}

/// Flat TOML representation of a rule condition (avoids enum in TOML).
#[derive(Debug, Deserialize, Clone)]
pub struct RuleConditionConfig {
    /// "MissingInWindow" or "RepeatWithout"
    #[serde(rename = "type")]
    pub kind: String,
    pub trigger: Option<String>,
    pub required: Option<String>,
    pub window: Option<usize>,
    pub category: Option<String>,
    pub count: Option<usize>,
}
```

Update `EnforcerConfig` to include a `rules` vec:

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnforcerConfig {
    pub enabled: bool,
    pub default_action: String,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}
```

Also add `lock_ttl_seconds` to `HiveConfig`:

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct HiveConfig {
    pub max_agents: u32,
    pub claude_path: String,
    pub default_model: String,
    pub agent_timeout: String,
    pub lock_ttl_seconds: u32,   // ← add this
}

impl Default for HiveConfig {
    fn default() -> Self {
        Self {
            max_agents: 5,
            claude_path: "claude".into(),
            default_model: "claude-sonnet-4-6".into(),
            agent_timeout: "300s".into(),
            lock_ttl_seconds: 60,   // ← add this
        }
    }
}
```

### Step 4: Add `seed_config_rules_to_db` to `src/orchestration/enforcer.rs`

Add import at the top:
```rust
use crate::config::RuleConditionConfig;
```

Add conversion on `RuleConditionConfig` — add inside `impl Enforcer` (or as a free function):

```rust
/// Seed rules from TOML config into the DB.
/// Uses INSERT OR REPLACE so TOML always wins over previous DB state for named rules.
pub fn seed_config_rules_to_db(
    db: &StateDb,
    rules: &[crate::config::RuleConfig],
) -> anyhow::Result<()> {
    let conn = db.conn();
    for rule in rules {
        let condition = match Self::condition_from_config(&rule.condition) {
            Some(c) => c,
            None => {
                tracing::warn!("skipping rule '{}': unrecognised condition type '{}'", rule.name, rule.condition.kind);
                continue;
            }
        };
        let condition_json = serde_json::to_string(&condition)?;
        let enabled = rule.enabled.unwrap_or(true) as i32;
        conn.execute(
            "INSERT OR REPLACE INTO rules (name, description, condition, action, enabled) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                rule.name,
                rule.description.as_deref().unwrap_or(""),
                condition_json,
                rule.action,
                enabled,
            ],
        )?;
    }
    Ok(())
}

fn condition_from_config(cfg: &crate::config::RuleConditionConfig) -> Option<RuleCondition> {
    match cfg.kind.as_str() {
        "MissingInWindow" => Some(RuleCondition::MissingInWindow {
            trigger: cfg.trigger.clone()?,
            required: cfg.required.clone()?,
            window: cfg.window?,
        }),
        "RepeatWithout" => Some(RuleCondition::RepeatWithout {
            category: cfg.category.clone()?,
            count: cfg.count?,
            required: cfg.required.clone()?,
        }),
        _ => None,
    }
}
```

### Step 5: Run tests to verify they pass

```bash
cargo test test_seed_config_rules -- --nocapture 2>&1 | tail -20
```

Expected: PASS.

### Step 6: Run all tests

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass, no warnings about unused imports.

### Step 7: Commit

```bash
git add src/config.rs src/orchestration/enforcer.rs
git commit -m "feat: add RuleConfig to config, seed_config_rules_to_db to Enforcer"
```

---

## Task 3: Fix `enforcer_toggle_rule` persistence + wire startup seeding in `server.rs` + `main.rs`

**Files:**
- Modify: `src/gateway/server.rs`
- Modify: `src/main.rs`

### Step 1: Update `OpenCheirServer::new` signature

In `src/gateway/server.rs`, change `new` to accept a pre-built enforcer:

```rust
impl OpenCheirServer {
    pub fn new(db: StateDb, enforcer: Arc<Mutex<Enforcer>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            enforcer,
            graph: Arc::new(GraphStore::new()),
        }
    }
    // ...
}
```

### Step 2: Fix `enforcer_toggle_rule` to persist to DB

Replace the existing `enforcer_toggle_rule` handler:

```rust
#[tool(name = "enforcer_toggle_rule", description = "Enable or disable an enforcer rule")]
async fn enforcer_toggle_rule(&self, Parameters(input): Parameters<EnforcerRuleToggleInput>) -> String {
    // Persist to DB first
    {
        let conn = self.db.conn();
        let rows = conn.execute(
            "UPDATE rules SET enabled = ?1 WHERE name = ?2",
            rusqlite::params![input.enabled as i32, input.rule_name],
        );
        if rows.map(|n| n == 0).unwrap_or(true) {
            return format!(r#"{{"error":"Rule '{}' not found in DB"}}"#, input.rule_name);
        }
    }
    // Update in-memory cache
    let mut enforcer = self.enforcer.lock().unwrap();
    enforcer.set_rule_enabled(&input.rule_name, input.enabled);
    format!(r#"{{"ok":true,"rule":"{}","enabled":{}}}"#, input.rule_name, input.enabled)
}
```

### Step 3: Update `src/main.rs` to seed rules and pass enforcer to server

Replace the `Commands::Serve` arm:

```rust
Commands::Serve { config: config_path } => {
    use opencheir::config::Config;
    use opencheir::orchestration::enforcer::Enforcer;

    let config_path = expand_tilde(&config_path);
    let cfg = Config::load(std::path::Path::new(&config_path))
        .unwrap_or_default();
    let data_dir = expand_tilde(&cfg.general.data_dir);
    let db_path = std::path::Path::new(&data_dir).join("opencheir.db");
    let db = StateDb::open(&db_path)?;

    // Seed built-in rules, then TOML overrides
    Enforcer::seed_builtins_to_db(&db)?;
    if !cfg.enforcer.rules.is_empty() {
        Enforcer::seed_config_rules_to_db(&db, &cfg.enforcer.rules)?;
    }

    // Load initial rule-set from DB
    let enforcer = {
        let mut e = Enforcer::new();
        e.reload_from_db(&db)?;
        std::sync::Arc::new(std::sync::Mutex::new(e))
    };

    let server = OpenCheirServer::new(db, enforcer);
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
}
```

Also add `Config` to the imports at the top of `main.rs`:
```rust
use opencheir::config::{Config, expand_tilde};
```

### Step 4: Build to verify

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: no errors. Fix any type mismatches before proceeding.

### Step 5: Commit

```bash
git add src/gateway/server.rs src/main.rs
git commit -m "feat: persist toggle to DB, wire enforcer seeding on startup"
```

---

## Task 4: Spawn `notify` watcher + background reload task in `main.rs`

**Files:**
- Modify: `src/main.rs`

### Step 1: Add imports

At the top of `src/main.rs`, add:

```rust
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
```

### Step 2: Splice watcher into the `Serve` arm

After the `enforcer` binding and before `OpenCheirServer::new`, add:

```rust
// ── File watcher for hot-reload ──────────────────────────────────────────
let (reload_tx, reload_rx) = watch::channel(());

let mut watcher = {
    let tx = reload_tx.clone();
    recommended_watcher(move |res: notify::Result<Event>| {
        if res.map(|e| e.kind.is_modify() || e.kind.is_create()).unwrap_or(false) {
            let _ = tx.send(());
        }
    })?
};
watcher.watch(
    std::path::Path::new(&config_path),
    RecursiveMode::NonRecursive,
)?;

// Background task: re-seed DB and reload enforcer in-memory on config change
{
    let enforcer_arc = Arc::clone(&enforcer);
    let db_watch = db.clone();
    let path_watch = config_path.clone();
    tokio::spawn(async move {
        let mut rx = reload_rx;
        loop {
            if rx.changed().await.is_err() {
                break;
            }
            let new_cfg = match Config::load(std::path::Path::new(&path_watch)) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("config reload failed: {e}");
                    continue;
                }
            };
            if let Err(e) = Enforcer::seed_config_rules_to_db(&db_watch, &new_cfg.enforcer.rules) {
                tracing::warn!("seed rules on reload failed: {e}");
                continue;
            }
            let mut e = enforcer_arc.lock().unwrap();
            if let Err(e) = e.reload_from_db(&db_watch) {
                tracing::warn!("reload_from_db failed: {e}");
            } else {
                tracing::info!("enforcer rules hot-reloaded from {path_watch}");
            }
        }
    });
}

// Keep watcher alive until server exits
let _watcher = watcher;
```

### Step 3: Build

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: clean build. Common fix: if `StateDb` isn't `Clone`, derive it (`StateDb` already has `Arc<Mutex<_>>` inside so it's cheap to clone — check `state.rs` and add `#[derive(Clone)]` if missing — it already has it via the `Arc`).

### Step 4: Smoke test

```bash
cargo run -- init 2>&1
cargo run -- serve &
sleep 1
kill %1
```

Expected: starts and stops cleanly, no panics.

### Step 5: Commit

```bash
git add src/main.rs
git commit -m "feat: spawn notify watcher for enforcer hot-reload"
```

---

## Task 5: Domain locks — migration + `LockService`

**Files:**
- Create: `src/store/migrations/003_domain_locks.sql`
- Modify: `src/store/state.rs`
- Create: `src/orchestration/hive/locks.rs`
- Modify: `src/orchestration/hive/mod.rs`

### Step 1: Write the failing test

Create `src/orchestration/hive/locks.rs` with just the test module first:

```rust
use anyhow::Result;
use serde::Serialize;
use crate::store::state::StateDb;

#[derive(Debug, Clone, Serialize)]
pub struct ClaimResult {
    pub token: String,
    pub domain: String,
    pub locked_by: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockInfo {
    pub token: String,
    pub locked_by: String,
    pub expires_at: String,
}

pub struct LockService;

impl LockService {
    pub fn claim(_db: &StateDb, _domain: &str, _locked_by: &str, _ttl_seconds: u32) -> Result<ClaimResult> {
        todo!()
    }
    pub fn release(_db: &StateDb, _domain: &str, _token: &str) -> Result<()> {
        todo!()
    }
    pub fn check(_db: &StateDb, _domain: &str) -> Result<Option<LockInfo>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> StateDb {
        let dir = tempdir().unwrap();
        StateDb::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn test_claim_and_release() {
        let db = test_db();
        let result = LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        assert_eq!(result.domain, "ops");
        assert!(!result.token.is_empty());

        let info = LockService::check(&db, "ops").unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().locked_by, "agent-1");

        LockService::release(&db, "ops", &result.token).unwrap();
        assert!(LockService::check(&db, "ops").unwrap().is_none());
    }

    #[test]
    fn test_claim_conflict() {
        let db = test_db();
        LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        let err = LockService::claim(&db, "ops", "agent-2", 60);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("locked by"));
    }

    #[test]
    fn test_release_is_idempotent() {
        let db = test_db();
        let r = LockService::claim(&db, "ops", "agent-1", 60).unwrap();
        LockService::release(&db, "ops", &r.token).unwrap();
        LockService::release(&db, "ops", &r.token).unwrap(); // second release is fine
    }
}
```

Add `pub mod locks;` to `src/orchestration/hive/mod.rs`.

### Step 2: Create the migration

`src/store/migrations/003_domain_locks.sql`:

```sql
CREATE TABLE IF NOT EXISTS locks (
    id          TEXT PRIMARY KEY,
    domain      TEXT NOT NULL UNIQUE,
    locked_by   TEXT NOT NULL,
    locked_at   TEXT NOT NULL DEFAULT (datetime('now')),
    ttl_seconds INTEGER NOT NULL DEFAULT 60,
    expires_at  TEXT NOT NULL
);
```

Add to `src/store/state.rs`:

```rust
const MIGRATION_003: &str = include_str!("migrations/003_domain_locks.sql");
```

And in `StateDb::open`, after `conn.execute_batch(MIGRATION_002)?;`:

```rust
conn.execute_batch(MIGRATION_003)?;
```

### Step 3: Run test to verify it fails with `todo!()`

```bash
cargo test test_claim_and_release -- --nocapture 2>&1 | tail -10
```

Expected: panics with "not yet implemented".

### Step 4: Implement `LockService`

Replace the `todo!()` stubs in `src/orchestration/hive/locks.rs`:

```rust
impl LockService {
    pub fn claim(db: &StateDb, domain: &str, locked_by: &str, ttl_seconds: u32) -> Result<ClaimResult> {
        let token = uuid::Uuid::new_v4().to_string();
        let conn = db.conn();

        // Purge expired lock for this domain
        conn.execute(
            "DELETE FROM locks WHERE domain = ?1 AND expires_at < datetime('now')",
            rusqlite::params![domain],
        )?;

        // Try to insert; UNIQUE on domain will fail if someone else holds it
        let result = conn.execute(
            "INSERT INTO locks (id, domain, locked_by, ttl_seconds, expires_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now', ?5))",
            rusqlite::params![
                token,
                domain,
                locked_by,
                ttl_seconds,
                format!("+{ttl_seconds} seconds"),
            ],
        );

        if let Err(rusqlite::Error::SqliteFailure(err, _)) = &result {
            if err.code == rusqlite::ErrorCode::ConstraintViolation {
                // Find who holds it
                let info: (String, String) = conn.query_row(
                    "SELECT locked_by, expires_at FROM locks WHERE domain = ?1",
                    rusqlite::params![domain],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                return Err(anyhow::anyhow!(
                    "domain '{}' is locked by '{}' until {}",
                    domain, info.0, info.1
                ));
            }
        }
        result?;

        let expires_at: String = conn.query_row(
            "SELECT expires_at FROM locks WHERE id = ?1",
            rusqlite::params![token],
            |row| row.get(0),
        )?;

        Ok(ClaimResult {
            token,
            domain: domain.to_string(),
            locked_by: locked_by.to_string(),
            expires_at,
        })
    }

    pub fn release(db: &StateDb, domain: &str, token: &str) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "DELETE FROM locks WHERE domain = ?1 AND id = ?2",
            rusqlite::params![domain, token],
        )?;
        Ok(())
    }

    pub fn check(db: &StateDb, domain: &str) -> Result<Option<LockInfo>> {
        let conn = db.conn();

        // Purge expired
        conn.execute(
            "DELETE FROM locks WHERE domain = ?1 AND expires_at < datetime('now')",
            rusqlite::params![domain],
        )?;

        let result = conn.query_row(
            "SELECT id, locked_by, expires_at FROM locks WHERE domain = ?1",
            rusqlite::params![domain],
            |row| {
                Ok(LockInfo {
                    token: row.get(0)?,
                    locked_by: row.get(1)?,
                    expires_at: row.get(2)?,
                })
            },
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
```

### Step 5: Run tests

```bash
cargo test test_claim_and_release test_claim_conflict test_release_is_idempotent -- --nocapture 2>&1 | tail -20
```

Expected: all 3 PASS.

### Step 6: Run full test suite

```bash
cargo test 2>&1 | tail -20
```

Expected: all pass.

### Step 7: Commit

```bash
git add src/store/migrations/003_domain_locks.sql src/store/state.rs \
        src/orchestration/hive/locks.rs src/orchestration/hive/mod.rs
git commit -m "feat: add domain locks migration and LockService"
```

---

## Task 6: Wire `hive_claim_domain` + `hive_release_domain` MCP tools

**Files:**
- Modify: `src/gateway/server.rs`

### Step 1: Add input structs

After the existing `MemoryByDomainInput` struct in `server.rs`, add:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct HiveClaimDomainInput {
    /// Domain to lock (e.g. "ops", "research")
    pub domain: String,
    /// Identifier for the agent claiming the lock
    pub locked_by: String,
    /// Lock TTL in seconds (default: from config, typically 60)
    pub ttl_seconds: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HiveReleaseDomainInput {
    /// Domain to release
    pub domain: String,
    /// Token returned by hive_claim_domain
    pub token: String,
}
```

### Step 2: Add tool handlers

Inside the `#[tool_router] impl OpenCheirServer` block, after `hive_memory_by_domain`:

```rust
#[tool(name = "hive_claim_domain", description = "Claim exclusive write access to a hive memory domain. Returns a token required for hive_memory_store.")]
async fn hive_claim_domain(&self, Parameters(input): Parameters<HiveClaimDomainInput>) -> String {
    use crate::orchestration::hive::locks::LockService;
    let ttl = input.ttl_seconds.unwrap_or(60);
    match LockService::claim(&self.db, &input.domain, &input.locked_by, ttl) {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}

#[tool(name = "hive_release_domain", description = "Release a previously claimed domain lock.")]
async fn hive_release_domain(&self, Parameters(input): Parameters<HiveReleaseDomainInput>) -> String {
    use crate::orchestration::hive::locks::LockService;
    match LockService::release(&self.db, &input.domain, &input.token) {
        Ok(()) => r#"{"ok":true}"#.to_string(),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}
```

### Step 3: Build

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: clean.

### Step 4: Commit

```bash
git add src/gateway/server.rs
git commit -m "feat: add hive_claim_domain and hive_release_domain MCP tools"
```

---

## Task 7: Guard `hive_memory_store` with lock check

**Files:**
- Modify: `src/gateway/server.rs`

### Step 1: Add `token` field to `MemoryStoreInput`

```rust
// Find MemoryStoreInput and add:
#[derive(Deserialize, JsonSchema)]
pub struct MemoryStoreInput {
    pub domain: String,
    pub lesson: String,
    pub context: Option<String>,
    pub tags: Option<Vec<String>>,
    /// Lock token from hive_claim_domain (required if domain is locked)
    pub token: Option<String>,
}
```

### Step 2: Update the `hive_memory_store` handler

```rust
#[tool(name = "hive_memory_store", description = "Store a learning/insight in the persistent memory system")]
async fn hive_memory_store(&self, Parameters(input): Parameters<MemoryStoreInput>) -> String {
    use crate::orchestration::hive::locks::LockService;
    use crate::orchestration::hive::memory::MemoryService;

    // Check domain lock
    if let Ok(Some(lock)) = LockService::check(&self.db, &input.domain) {
        let caller_token = input.token.as_deref().unwrap_or("");
        if lock.token != caller_token {
            return format!(
                r#"{{"error":"domain '{}' is locked by '{}' until {}"}}"#,
                input.domain, lock.locked_by, lock.expires_at
            );
        }
    }

    let tags: Vec<&str> = input.tags.as_ref()
        .map(|t| t.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();
    match MemoryService::store(&self.db, &input.domain, &input.lesson, input.context.as_deref(), &tags) {
        Ok(id) => format!(r#"{{"id":{id}}}"#),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}
```

### Step 3: Build + full test suite

```bash
cargo build 2>&1 | grep -E "^error" | head -20
cargo test 2>&1 | tail -20
```

Expected: clean build, all tests pass.

### Step 4: Commit

```bash
git add src/gateway/server.rs
git commit -m "feat: guard hive_memory_store with domain lock check"
```

---

## Task 8: Update README

**Files:**
- Modify: `README.md`

### Step 1: Read the current README

```bash
head -80 /Users/fabio/projects/opencheir/README.md
```

### Step 2: Add sections

Find a suitable location (after the tool list or before Contributing) and add:

```markdown
## Enforcer hot-reload

Enforcement rules are loaded from the `rules` table in the SQLite database on startup. Built-in rules are seeded automatically; custom rules can be added in `config.toml`:

```toml
[[enforcer.rules]]
name = "my_rule"
description = "Block write without prior read in last 5 calls"
action = "warn"
enabled = true

[enforcer.rules.condition]
type = "MissingInWindow"
trigger = "write_document"
required = "read_document"
window = 5
```

While the server is running, edit and save `config.toml`. OpenCheir detects the change and reloads rules within milliseconds — no restart needed. The sliding window of recent tool calls is preserved across reloads.

Toggles via `enforcer_toggle_rule` are written to the DB and survive hot-reloads.

---

## Domain locking

When two agents write to the same hive memory domain concurrently, last write wins — unless they use domain locking.

**Pattern:**

```
1. Agent calls hive_claim_domain → receives { token, expires_at }
2. Agent calls hive_memory_store with the token → write succeeds
3. Agent calls hive_release_domain → lock released
```

If another agent tries to write to a locked domain without the token, it receives:
```json
{"error": "domain 'ops' is locked by 'agent-1' until 2026-03-09T12:01:00"}
```

Locks expire automatically (default TTL: 60 seconds, configurable per-claim and via `[hive] lock_ttl_seconds` in `config.toml`). Locking is opt-in — unlocked domains work exactly as before.
```

### Step 3: Build + final test run

```bash
cargo build --release 2>&1 | grep -E "^error" | head -20
cargo test 2>&1 | tail -20
```

Expected: clean release build, all tests pass.

### Step 4: Commit

```bash
git add README.md
git commit -m "docs: add hot-reload and domain locking sections to README"
```

---

## Done

All tasks complete. The two features are fully wired:

- **Hot-reload**: TOML → DB on startup, `notify` watcher → DB → in-memory enforcer on config change, toggle writes persisted to DB.
- **Domain locking**: `hive_claim_domain` / `hive_release_domain` tools, `hive_memory_store` lock guard, configurable TTL, lazy expiry, idempotent release.
