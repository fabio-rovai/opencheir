# Design: Enforcer Hot-Reload & Domain Locking

**Date:** 2026-03-09
**Status:** Approved

---

## Overview

Two independent features that close the two biggest operational gaps in OpenCheir:

1. **Enforcer hot-reload** — rule changes in `config.toml` take effect immediately without restarting the server.
2. **Domain locking** — agents can claim exclusive write access to a hive memory domain, preventing last-write-wins races in multi-agent setups.

---

## Feature 1: Enforcer Hot-Reload

### Problem

Built-in rules are hardcoded in `Enforcer::new()`. The `rules` DB table exists but is never read. `enforcer_toggle_rule` changes in-memory state only — lost on restart. No mechanism to add or modify rules without redeploying.

### Design

#### Config seeding (startup)

Add `[[enforcer.rules]]` array to `EnforcerConfig` in `config.rs`:

```toml
[[enforcer.rules]]
name = "my_custom_rule"
description = "Block write without prior read"
condition = { type = "MissingInWindow", trigger = "write_doc", required = "read_doc", window = 5 }
action = "warn"
enabled = true
```

Each entry maps to the existing `rules` table schema. On `serve`:

1. Load config from TOML.
2. Upsert TOML rules into `rules` table (`INSERT OR REPLACE`).
3. Upsert hardcoded built-in rules if not already present (so they appear in DB from first run).
4. `Enforcer::reload_from_db(&StateDb)` loads all enabled rules from DB into `self.rules`. This replaces `Enforcer::new()`'s hardcoded vec.

DB is now the single source of truth. TOML is the seed/override mechanism.

#### Live reload path

- Spawn a `notify` watcher on `config.toml` at server start.
- Watcher thread sends unit message on a `tokio::sync::watch::Sender<()>`.
- Background `tokio::spawn` task holds `Arc<Mutex<Enforcer>>`, `Arc<StateDb>`, and the watch receiver.
- On signal:
  1. Re-parse TOML.
  2. Upsert changed rules into DB.
  3. Lock enforcer, call `reload_from_db`.
- Sliding window (`recent_calls`) is preserved across reloads.

#### `enforcer_toggle_rule` persistence fix

Now writes `UPDATE rules SET enabled = ? WHERE name = ?` to DB before updating in-memory state. Toggle survives hot-reloads.

#### Condition serialisation

`RuleCondition` needs `serde` derive (`Serialize`, `Deserialize`) so conditions can be stored as JSON in the `rules.condition` column and round-tripped through TOML config.

---

## Feature 2: Domain Locking

### Problem

`hive_memory_store` has no concurrency protection. Two agents writing to the same domain race, with last write winning silently.

### Design

#### New migration: `003_domain_locks.sql`

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

One lock per domain. `expires_at` is computed at insert time: `datetime('now', '+N seconds')`.

#### New MCP tools

**`hive_claim_domain(domain, locked_by, ttl_seconds?)`**

1. Delete expired locks: `DELETE FROM locks WHERE expires_at < datetime('now')`.
2. `INSERT INTO locks (id, domain, locked_by, ttl_seconds, expires_at) VALUES (...)`.
3. On `UNIQUE` conflict → return error: `"domain 'X' is locked by <locked_by> until <expires_at>"`.
4. On success → return `{ token: "<uuid>", expires_at: "..." }`.

**`hive_release_domain(domain, token)`**

`DELETE FROM locks WHERE domain = ? AND id = ?`. Idempotent — no error if already released.

#### `hive_memory_store` guard

Add optional `token: Option<String>` to `MemoryStoreInput`.

Before inserting:
1. Delete expired locks for this domain.
2. Query: `SELECT id, locked_by, expires_at FROM locks WHERE domain = ?`.
3. If a live lock exists and `token` doesn't match → return error with lock owner info.
4. If no lock or token matches → proceed with insert.

Locking is **opt-in** — unlocked domains write as before.

#### TTL configuration

- Default: 60 seconds.
- Override per-claim via `ttl_seconds` param on `hive_claim_domain`.
- Add `lock_ttl_seconds: u32` (default 60) to `HiveConfig` in `config.rs`.

---

## README additions

- **Enforcement hot-reload** section: explain TOML → DB → live cycle, show `[[enforcer.rules]]` example.
- **Domain locking** section: show `claim → write → release` pattern with example tool calls.

---

## Files changed

| File | Change |
|------|--------|
| `src/config.rs` | Add `rules: Vec<RuleConfig>` to `EnforcerConfig`; `lock_ttl_seconds` to `HiveConfig` |
| `src/orchestration/enforcer.rs` | `RuleCondition` serde; `reload_from_db`; seed built-ins to DB; fix toggle persistence |
| `src/store/state.rs` | Include `003_domain_locks.sql` migration |
| `src/store/migrations/003_domain_locks.sql` | New `locks` table |
| `src/gateway/server.rs` | Wire `reload_from_db` on startup; add `hive_claim_domain`, `hive_release_domain` tools; guard `hive_memory_store` |
| `src/main.rs` | Spawn notify watcher + reload background task on `serve` |
| `README.md` | Document both features |

---

## Out of scope

- Row-level locking within a domain (all-or-nothing domain lock is sufficient).
- Distributed locking across multiple OpenCheir instances.
- Forced lock takeover / admin unlock tool (can add later).
