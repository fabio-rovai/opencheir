# Sentinel: One Brain to Rule Them All

> A single Rust binary that replaces 12 MCP servers, 3 bash hooks, 1 JavaScript plugin, and 5 languages — with shared memory, self-healing, and enforcement that gets smarter over time.

---

## Problem

The current Claude Code setup is spaghetti:

- **12 MCP servers** in 4 languages (Python, TypeScript, Go, Node) running as isolated processes
- **3 bash hooks** that fail silently when scripts break or servers are offline
- **1 JS plugin** (superpowers) for skill management that drifts out of sync
- **5 separate data stores** (SQLite, pickle, in-memory, hardcoded dicts x2) with zero shared state
- **4 things parse DOCX independently**, 4 search engines share no index, double event logging
- **No enforcement** — CLAUDE.md has rules but nothing stops Claude from ignoring them
- **No health monitoring** — a server crashes and Claude discovers it mid-task
- **No cross-session intelligence** — every session starts from zero
- **250-500MB RAM** for 12 Python/Node processes

## Solution

**Sentinel** — a single Rust binary that:

1. **Replaces** all custom MCP servers, hooks, and plugin infrastructure
2. **Shares** one SQLite database across all modules and sessions
3. **Enforces** orchestration rules with blocking, not suggestions
4. **Self-heals** by restarting crashed external servers automatically
5. **Learns** patterns across sessions and adapts enforcement

## Design Principles

1. **One process, one database, one binary** — no deployment complexity
2. **Shared core, independent modules** — DOCX parsing happens once, everyone uses it
3. **Enforce, don't suggest** — if a rule matters, block violations, don't print reminders
4. **Persist everything useful** — cross-session intelligence is the killer feature
5. **Supervise what you can't replace** — third-party servers get health-checked and auto-restarted

---

## Architecture

```
                        ┌─────────────────────────┐
                        │      Claude Code         │
                        │    (stdin/stdout MCP)     │
                        └───────────┬─────────────┘
                                    │
                        ┌───────────▼─────────────┐
                        │     MCP GATEWAY          │
                        │                          │
                        │  • single MCP server     │
                        │  • routes tool calls     │
                        │  • enforces rules BEFORE │
                        │  • logs to lineage AFTER │
                        └───────────┬─────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           │                        │                        │
┌──────────▼──────────┐ ┌──────────▼──────────┐ ┌──────────▼──────────┐
│    SHARED CORE      │ │   DOMAIN MODULES    │ │ ORCHESTRATION LAYER │
│                     │ │                     │ │                     │
│  documents (docx,   │ │  tender (parse,     │ │  supervisor (health,│
│   pdf, render)      │ │   comply, fill)     │ │   enforce, heal)    │
│                     │ │                     │ │                     │
│  search (unified    │ │  qa (check, fix)    │ │  lineage (events,   │
│   FTS5 + TF-IDF)    │ │                     │ │   graph, warnings)  │
│                     │ │  social_value       │ │                     │
│  state (SQLite,     │ │   (toms, calc)      │ │  hive (plan, spawn, │
│   sentinel.db)      │ │                     │ │   coordinate, learn)│
│                     │ │  bid (frameworks,   │ │                     │
│                     │ │   win themes, score)│ │  skills (load,      │
│                     │ │                     │ │   resolve, serve)   │
│                     │ │  eyes (ws, capture, │ │                     │
│                     │ │   screenshot)       │ │                     │
└──────────┬──────────┘ └──────────┬──────────┘ └──────────┬──────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    │
                        ┌───────────▼─────────────┐
                        │    EXTERNAL SUPERVISOR   │
                        │                          │
                        │  word-document-server    │
                        │  mermaid-kroki           │
                        │  puppeteer               │
                        │  threejs                 │
                        └─────────────────────────┘
```

---

## Shared Core

Three modules that every domain module depends on. No domain module does its own I/O.

### Documents Module

Single entry point for all document operations.

**Responsibilities:**
- Parse DOCX (tables, paragraphs, runs, fonts, formatting)
- Parse PDF (via subprocess `pdftotext` — no Rust PDF text extraction is mature enough)
- Extract text from TXT, MD files
- Render DOCX to PNG (via subprocess `soffice` + `pdftoppm`)
- Render DOCX to PDF (via subprocess `soffice`)
- Cache parsed document structure in state DB

**Interface:**
```rust
pub struct DocumentService {
    db: Arc<StateDb>,
}

impl DocumentService {
    /// Parse and cache a DOCX. Returns structured representation.
    /// If already parsed and file hasn't changed (mtime check), returns cached.
    fn parse_docx(&self, path: &Path) -> Result<Document>;

    /// Extract plain text from any supported format.
    fn extract_text(&self, path: &Path) -> Result<String>;

    /// Render document to PNG images (one per page).
    fn render_to_png(&self, path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>>;

    /// Render document to PDF.
    fn render_to_pdf(&self, path: &Path, output_dir: &Path) -> Result<PathBuf>;

    /// Read a specific cell from a DOCX table.
    fn read_cell(&self, path: &Path, table: usize, row: usize, cell: usize) -> Result<String>;

    /// Write to a specific cell, preserving formatting.
    fn write_cell(&self, path: &Path, table: usize, row: usize, cell: usize, content: &str) -> Result<()>;
}
```

**Crate dependencies:** `docx-rs`, `walkdir`, `tokio::process` (for soffice/pdftotext)

### Search Module

Unified search over all indexed content.

**Responsibilities:**
- Maintain FTS5 index over all searchable content (tenders, TOMs, learnings, frameworks)
- TF-IDF ranking with cosine similarity for semantic search
- Keyword/exact matching with context windows
- Auto-index new documents when parsed by Documents module
- Rebuild index on demand

**Interface:**
```rust
pub struct SearchService {
    db: Arc<StateDb>,
}

impl SearchService {
    /// Semantic search across all content types.
    /// source_filter: optional filter by content type.
    fn search(&self, query: &str, source_filter: Option<ContentSource>, top_k: usize) -> Result<Vec<SearchResult>>;

    /// Exact keyword search with surrounding context.
    fn keyword_search(&self, keyword: &str, source_filter: Option<ContentSource>) -> Result<Vec<KeywordHit>>;

    /// Index a document (called automatically by Documents module).
    fn index_document(&self, doc_id: i64, source: ContentSource, content: &str, tags: &[String]) -> Result<()>;

    /// Force rebuild entire index.
    fn rebuild_index(&self) -> Result<IndexStats>;
}

pub enum ContentSource {
    Tender,
    Toms,
    Learning,
    Framework,
    CaseStudy,
}
```

**Crate dependencies:** `rusqlite` (FTS5), `rust-stemmers`, `ndarray` (sparse TF-IDF matrices)

### State Module

Single SQLite database shared by all modules.

**File:** `~/.sentinel/sentinel.db`

**Schema:**

```sql
-- ============================================================
-- SESSIONS
-- ============================================================
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at    TEXT,
    project     TEXT,
    summary     TEXT
);

-- ============================================================
-- DOCUMENTS (shared core — parsed once, used by all)
-- ============================================================
CREATE TABLE documents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    path        TEXT NOT NULL,
    filename    TEXT NOT NULL,
    doc_type    TEXT NOT NULL,           -- submission|feedback|cv|pricing|spec|other
    tender_name TEXT,
    text        TEXT,                    -- full extracted text
    mtime       INTEGER,                -- file modification time (cache invalidation)
    parsed_at   TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(path)
);

-- ============================================================
-- SEARCH INDEX (unified FTS5 over everything)
-- ============================================================
CREATE VIRTUAL TABLE search_fts USING fts5(
    source,         -- tender|toms|learning|framework|case_study
    source_id,      -- FK to source table
    title,
    content,
    tags,
    tokenize='porter unicode61'
);

-- ============================================================
-- TENDER (domain: parsing, compliance)
-- ============================================================
CREATE TABLE questions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL REFERENCES documents(id),
    question_id     TEXT NOT NULL,       -- e.g. "5.4", "6.1"
    text            TEXT,
    word_limit      INTEGER,
    question_type   TEXT NOT NULL,       -- SCORED|PASS_FAIL|INFORMATION|DECLARATION
    answer          TEXT,
    table_index     INTEGER,
    row_index       INTEGER,
    cell_index      INTEGER,
    score_weight    REAL,
    parsed_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- QA (domain: quality checks)
-- ============================================================
CREATE TABLE company (
    key         TEXT PRIMARY KEY,        -- vat_number|registration|address|...
    value       TEXT NOT NULL,
    sensitive   INTEGER DEFAULT 0,       -- 1 = must not appear in submissions
    category    TEXT                     -- company|psc|insurance|turnover|reference
);

CREATE TABLE qa_results (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    doc_path    TEXT NOT NULL,
    check_type  TEXT NOT NULL,           -- fonts|dashes|word_counts|sensitive|signatures
    status      TEXT NOT NULL,           -- pass|fail|warning
    details     TEXT,                    -- JSON array of findings
    checked_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- SOCIAL VALUE (domain: TOMs framework)
-- ============================================================
CREATE TABLE toms (
    reference       TEXT PRIMARY KEY,    -- NT1, NT14, HE1
    theme           TEXT NOT NULL,
    outcome         TEXT NOT NULL,
    title           TEXT NOT NULL,
    units           TEXT,
    proxy_value     REAL,               -- GBP per unit
    definition      TEXT,
    target_requirements TEXT,
    evidence_required   TEXT,
    unit_guidance       TEXT,
    tags            TEXT                -- JSON array
);

-- ============================================================
-- BID (domain: frameworks, win themes)
-- ============================================================
CREATE TABLE frameworks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    category    TEXT,                    -- persuasion|structure|scoring
    content     TEXT NOT NULL,
    tags        TEXT                    -- JSON array
);

-- ============================================================
-- LINEAGE (orchestration: event tracking)
-- ============================================================
CREATE TABLE events (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    timestamp   INTEGER NOT NULL,       -- unix ms
    event_type  TEXT NOT NULL,           -- R|W|C|D|T|E
    path        TEXT,
    tool        TEXT,
    meta        TEXT
);

-- ============================================================
-- HIVE (orchestration: multi-agent)
-- ============================================================
CREATE TABLE goals (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    session_id  TEXT REFERENCES sessions(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY,
    goal_id     TEXT NOT NULL REFERENCES goals(id),
    description TEXT NOT NULL,
    role        TEXT,
    status      TEXT NOT NULL DEFAULT 'pending',
    depends_on  TEXT,                   -- JSON array of task IDs
    agent_pid   INTEGER,
    artifacts   TEXT,                   -- JSON
    stdout      TEXT,
    stderr      TEXT,
    exit_code   INTEGER,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE learnings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    domain      TEXT NOT NULL,
    lesson      TEXT NOT NULL,
    context     TEXT,
    tags        TEXT,                   -- JSON array
    source_task TEXT,
    outcome     TEXT,
    times_used  INTEGER DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE learnings_fts USING fts5(
    domain, lesson, context, tags,
    content=learnings,
    content_rowid=id,
    tokenize='porter unicode61'
);

-- ============================================================
-- SUPERVISOR (orchestration: health + enforcement)
-- ============================================================
CREATE TABLE health (
    component   TEXT PRIMARY KEY,       -- tender|qa|hive|word_document_server|...
    kind        TEXT NOT NULL,           -- internal|external
    status      TEXT NOT NULL,           -- ok|degraded|down
    last_check  TEXT,
    error       TEXT,
    restart_count INTEGER DEFAULT 0
);

CREATE TABLE enforcement (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT REFERENCES sessions(id),
    timestamp   TEXT NOT NULL DEFAULT (datetime('now')),
    rule        TEXT NOT NULL,           -- qa_after_docx_write|rag_before_bid|...
    action      TEXT NOT NULL,           -- block|warn|allow
    tool_call   TEXT,                    -- the tool that was blocked/warned
    reason      TEXT
);

CREATE TABLE rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    condition   TEXT NOT NULL,           -- JSON: trigger condition
    action      TEXT NOT NULL DEFAULT 'block',  -- block|warn
    enabled     INTEGER DEFAULT 1,
    auto_learned INTEGER DEFAULT 0,     -- 1 = discovered by supervisor
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- PATTERNS (cross-session intelligence)
-- ============================================================
CREATE TABLE patterns (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category    TEXT NOT NULL,           -- failure|optimization|preference
    description TEXT NOT NULL,
    evidence    TEXT,                    -- JSON: supporting data
    confidence  REAL DEFAULT 0.5,       -- 0.0 to 1.0
    occurrences INTEGER DEFAULT 1,
    first_seen  TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen   TEXT NOT NULL DEFAULT (datetime('now')),
    actionable  INTEGER DEFAULT 0       -- 1 = supervisor can act on this
);

-- ============================================================
-- SKILLS (engine)
-- ============================================================
CREATE TABLE skills (
    name        TEXT PRIMARY KEY,
    source      TEXT NOT NULL,           -- builtin|personal
    file_path   TEXT NOT NULL,
    description TEXT,
    last_loaded TEXT,
    load_count  INTEGER DEFAULT 0,
    healthy     INTEGER DEFAULT 1
);
```

**Crate dependencies:** `rusqlite` with `bundled` and `fts5` features

**Pragmas (applied on open):**
```sql
PRAGMA journal_mode = WAL;          -- concurrent reads during writes
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;        -- fast writes, safe enough for local
PRAGMA cache_size = -64000;         -- 64MB cache
```

---

## Domain Modules

Each module owns business logic only. No file I/O, no database access except through shared core.

### Tender Module

**Replaces:** `tender-orchestrator` (Python)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `tender_parse` | Parse DOCX tender, extract questions/word limits/scoring → stores in `questions` table |
| `tender_parse_spec` | Parse PDF specification for lots/KPIs/delivery requirements |
| `tender_read_answer` | Read answer from a specific cell |
| `tender_write_answer` | Write answer to a specific cell (via Documents.write_cell) |
| `tender_fill_appendices` | Fill appendix tables in PSQ documents |
| `tender_render` | Render DOCX to PNG (via Documents.render_to_png) |
| `tender_convert_pdf` | Convert DOCX to PDF (via Documents.render_to_pdf) |
| `tender_check_compliance` | Check word counts against limits (reads from `questions` table) |
| `tender_check_pass_fail` | Verify all pass/fail questions answered |
| `tender_check_submission` | Validate required files exist in folder |
| `tender_full_check` | Run all compliance checks at once |

**Internal logic:**
- Regex-based question ID extraction (`"5.4"`, `"6.1"`)
- Word limit pattern matching (`"Limited to 1000 words"`)
- Question type classification (SCORED, PASS_FAIL, INFORMATION, DECLARATION)
- Compliance scoring against parsed limits

**Dependencies:** Shared Core only (Documents for I/O, State for persistence, Search for indexing)

### QA Module

**Replaces:** `tender-qa-mcp` (Python)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `qa_full_check` | Run all checks, return aggregated report |
| `qa_check_fonts` | Detect font inconsistencies across paragraphs and tables |
| `qa_check_dashes` | Find em/en dashes that should be hyphens |
| `qa_check_word_counts` | Validate word counts against limits |
| `qa_check_sensitive` | Detect sensitive info (UTR, CDP password, mobile, PSC DOB) |
| `qa_check_company` | Verify VAT, registration, company name |
| `qa_check_signatures` | Check signature fields and confirmation ticks |
| `qa_check_filenames` | Validate submission filenames (no spaces, correct extensions) |
| `qa_fix_dashes` | Auto-replace em/en dashes with hyphens |
| `qa_fix_quotes` | Replace curly quotes with straight |
| `qa_fix_fonts` | Normalize all fonts to target |
| `qa_fix_all` | Run all fixes |
| `qa_get_company` | Return company details by field |

**Internal logic:**
- Font analysis: walk all runs, count font/size occurrences, report inconsistencies
- Unicode character detection: U+2014, U+2013, U+201C-201F
- Sensitive data patterns: regex for UTR, VAT, phone formats
- Company data read from `company` table (not hardcoded)
- Results stored in `qa_results` table for cross-session tracking

**Dependencies:** Shared Core only (Documents for parsed DOCX, State for company data + results)

### Social Value Module

**Replaces:** `social-value-mcp` (Python)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `sv_get_measure` | Full TOMs measure details by reference (NT1, HE1, etc.) |
| `sv_search` | Search TOMs measures by keyword + optional theme filter |
| `sv_list_themes` | List all 5 themes with outcomes |
| `sv_by_theme` | All measures for a theme |
| `sv_suggest` | Smart recommendations by contract type/value/sector |
| `sv_draft` | Generate response outline for selected measures |
| `sv_calculate` | Calculate total proxy GBP value from commitments |

**Internal logic:**
- TOMs data loaded from `toms` table (seeded on first run from embedded data)
- Suggestion rules based on contract type keywords
- Proxy value calculation: quantity x proxy_value per unit
- All TOMs data also indexed in unified `search_fts`

**Dependencies:** Shared Core only (Search for querying, State for TOMs data)

### Bid Module

**Replaces:** `bid-writing-mcp` (TypeScript/Node)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `bid_no_bid` | Bid/no-bid analysis with scoring |
| `bid_win_themes` | Generate win themes for a tender |
| `bid_proposal_outline` | Generate proposal structure |
| `bid_executive_summary` | Draft executive summary |
| `bid_compare_frameworks` | Compare persuasion/scoring frameworks |
| `bid_get_framework` | Get a specific framework |
| `bid_list_frameworks` | List available frameworks |
| `bid_recommend_framework` | Recommend best framework for context |
| `bid_section_template` | Get template for a proposal section |
| `bid_score_proposal` | Score a draft proposal against criteria |
| `bid_get_industry_guide` | Industry-specific bidding guidance |
| `bid_get_persuasion` | Get a persuasion technique |
| `bid_compliance_matrix` | Generate compliance matrix |
| `bid_search_resources` | Search bid writing resources |

**Internal logic:**
- Framework definitions stored in `frameworks` table
- All searchable content indexed in unified `search_fts`
- Scoring algorithms for bid/no-bid decisions
- Template generation for proposal sections

**Dependencies:** Shared Core only (Search for resources, State for frameworks)

### Eyes Module

**Replaces:** `eyes` (Go)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `eyes_get` | Return latest canvas screenshot (PNG) + console logs |
| `eyes_get_logs` | Return console logs only (cheaper, no image) |

**Internal architecture:**
- HTTP server on configurable port (random ephemeral by default)
- Serves `eyes.js` client script at `/eyes.js`
- WebSocket endpoint for browser client connection
- Browser client captures: largest canvas as PNG, console.log/error/warn (up to 50), unhandled exceptions
- Images downscaled to max 800px width
- Latest capture held in memory (single-slot buffer)
- Auto-reconnect on client side (3s retry)

**Crate dependencies:** `axum` (HTTP), `tokio-tungstenite` (WebSocket), `image` (PNG resize)

---

## Orchestration Layer

### Supervisor Module (NEW)

The core innovation. Does not exist in current setup.

**MCP Tools:**
| Tool | Description |
|------|------------|
| `sentinel_status` | Full health report: all components, external servers, recent enforcement |
| `sentinel_rules` | List active enforcement rules |
| `sentinel_add_rule` | Add a custom enforcement rule |
| `sentinel_disable_rule` | Temporarily disable a rule |
| `sentinel_patterns` | Show discovered cross-session patterns |

**Startup sequence:**
```
1. Open/create sentinel.db, run migrations
2. Create session record
3. Load config.toml
4. Initialize shared core (Documents, Search, State)
5. Initialize all domain modules
6. Initialize orchestration modules (Lineage, Hive, Skills)
7. Spawn external servers (word-doc, mermaid, puppeteer, threejs)
8. Health-check everything
9. Load enforcement rules from DB + config
10. Start MCP server on stdin/stdout
11. Report: "Sentinel online. 8 internal modules OK, 4/4 external servers OK."
```

**Continuous health checks (every 5s):**
```
For each external server:
  Send MCP initialize request with 2s timeout
  If no response:
    Mark as degraded
    Attempt restart (spawn process again)
    If restart fails 3x in 60s:
      Mark as down
      Log to enforcement table
      Warn Claude on next tool call: "word-document-server is down. Restart failed."
  If response OK:
    Mark as ok, reset restart_count
```

**Enforcement engine:**

The enforcer intercepts every tool call at the MCP Gateway level, before routing.

```rust
pub struct Enforcer {
    db: Arc<StateDb>,
    rules: Vec<Rule>,
    recent_calls: VecDeque<ToolCall>,  // sliding window of last 20 calls
}

impl Enforcer {
    /// Called BEFORE every tool call is routed to its module.
    fn pre_check(&mut self, tool_name: &str, params: &Value) -> Verdict {
        // Check each active rule
        for rule in &self.rules {
            if rule.matches(tool_name, &self.recent_calls) {
                match rule.action {
                    Action::Block => return Verdict::Block(rule.reason()),
                    Action::Warn => self.log_warning(rule),
                }
            }
        }
        Verdict::Allow
    }

    /// Called AFTER every tool call completes.
    fn post_check(&mut self, tool_name: &str, result: &Value) {
        self.recent_calls.push_back(ToolCall::new(tool_name));
        if self.recent_calls.len() > 20 {
            self.recent_calls.pop_front();
        }
    }
}
```

**Built-in enforcement rules:**

| Rule | Trigger | Action |
|------|---------|--------|
| `qa_after_docx_write` | `tender_write_answer` called but no `qa_*` in last 3 calls | BLOCK |
| `render_after_edit` | 3+ document edits without `tender_render` | WARN |
| `rag_before_bid` | `bid_*` generation tool without `search` in last 5 calls | WARN |
| `parse_before_write` | `tender_write_answer` without `tender_parse` in session | BLOCK |
| `health_gate` | Tool call to module whose health is `down` | BLOCK with restart attempt |
| `sensitive_data_gate` | `tender_write_answer` content matches sensitive patterns | BLOCK |

**Cross-session pattern discovery:**
```
Every 100 tool calls (or on session end):
  Analyze enforcement log:
    - If a rule blocks 0 times over 50+ sessions → suggest disabling
    - If Claude retries the same blocked call 3+ times → flag as friction point
    - If a sequence of tools always appears together → suggest creating a pipeline
  Analyze qa_results:
    - If a check_type always passes → suggest removing from required pipeline
    - If a check_type fails on specific question_types → create targeted rule
  Store findings in patterns table with confidence score
```

### Lineage Module

**Replaces:** `lineage` (TypeScript) + `lineage-track.sh` (Bash hook)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `lineage_log` | Log an event (compiled-in, no HTTP round-trip) |
| `lineage_warnings` | Get current session warnings |
| `lineage_query` | Query lineage for a file or full session graph |
| `lineage_clear` | Reset session tracking |
| `lineage_history` | Query events across past sessions (NEW — cross-session) |

**Internal architecture:**
- Events written directly to `events` table (no in-memory-only store)
- Graph built on-demand from events table (same DAG logic as TypeScript version)
- Rules engine detects: edit-without-read, multi-edit (3+ edits to same file)
- HTTP API at `/events`, `/warnings`, `/graph`, `/health` for VSCode extension polling
- **No hook needed** — lineage logging is compiled into the MCP Gateway's post-routing step

**Cross-session queries (new capability):**
```sql
-- "What files did I touch most across all sessions?"
SELECT path, COUNT(*) as touches
FROM events WHERE event_type IN ('W', 'C')
GROUP BY path ORDER BY touches DESC LIMIT 10;

-- "Show me all sessions that worked on tender X"
SELECT s.id, s.started_at, COUNT(e.seq) as events
FROM sessions s JOIN events e ON e.session_id = s.id
WHERE e.path LIKE '%tender_x%'
GROUP BY s.id;
```

### Hive Module

**Replaces:** `hive` (Go)

**MCP Tools:**
| Tool | Description |
|------|------------|
| `hive_orchestrate` | Full auto: decompose goal → spawn agents → coordinate → return |
| `hive_plan` | Decompose goal into task DAG without executing |
| `hive_status` | Check goal/task progress |
| `hive_learn` | Store a learning by domain + tags |
| `hive_recall` | FTS5 search over learnings |
| `hive_stop` | Kill a specific task or all tasks in a goal |

**Internal architecture:**
- Planner spawns a Claude CLI agent with decomposition prompt, parses JSON DAG
- Coordinator resolves dependencies, spawns ready tasks in parallel (up to max_agents)
- Spawner manages Claude CLI processes via `tokio::process::Command`
- Memory queries learnings from shared `learnings` + `learnings_fts` tables
- Goals/tasks stored in shared `goals` + `tasks` tables
- Agent output stored in `tasks.stdout`/`tasks.stderr`

**Claude CLI invocation:**
```rust
Command::new(&config.claude_path)
    .arg("--print")
    .arg("--output-format").arg("json")
    .arg("--system-prompt").arg(&system_prompt)
    .arg("--dangerously-skip-permissions")
    .arg(&task.description)
    .current_dir(&working_dir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
```

**Cross-module integration (new):**
- Before spawning an agent, queries lineage for recent file activity → injects as context
- After agents complete, stores patterns in `patterns` table
- If agent fails, checks `qa_results` for related issues → enriches error context

### Skills Engine

**Replaces:** `superpowers` JavaScript plugin + custom skills

**MCP Tools:**
| Tool | Description |
|------|------------|
| `skill_list` | List all available skills with descriptions |
| `skill_get` | Load a skill's content by name |
| `skill_resolve` | Resolve a skill name (personal overrides builtin) |
| `skill_health` | Check which skills are loadable/broken |

**Internal architecture:**
- Scans two directories on startup:
  - Built-in: `~/.sentinel/skills/` (ships with binary or extracted on first run)
  - Personal: `~/.claude/skills/` (user overrides)
- Parses YAML frontmatter from `SKILL.md` files
- Personal skills shadow built-in skills with same name
- File watcher (`notify` crate) hot-reloads skills when files change on disk
- Skill metadata stored in `skills` table for health tracking
- Load count tracked → discover unused skills

**Bundled skills (20 total):**

From superpowers (forked, customized):
1. brainstorming
2. dispatching-parallel-agents
3. executing-plans
4. finishing-a-development-branch
5. receiving-code-review
6. requesting-code-review
7. subagent-driven-development
8. systematic-debugging
9. test-driven-development
10. using-git-worktrees
11. using-superpowers (renamed: using-sentinel)
12. verification-before-completion
13. writing-plans
14. writing-skills

Custom:
15. researcher
16. heartbeat
17. mcporter
18. ui-ux-pro-max
19. hive (updated for sentinel)
20. lineage (updated for sentinel)

---

## MCP Gateway

Single MCP server that Claude Code connects to. Routes all tool calls.

**Architecture:**
```rust
pub struct Gateway {
    enforcer: Enforcer,
    lineage: LineageService,
    router: Router,
}

impl Gateway {
    /// Handle every incoming MCP tool call.
    async fn handle_tool_call(&mut self, tool: &str, params: Value) -> Result<Value> {
        // 1. PRE-CHECK: enforcement rules
        match self.enforcer.pre_check(tool, &params) {
            Verdict::Block(reason) => {
                self.lineage.log_enforcement(tool, "block", &reason);
                return Err(McpError::blocked(reason));
            }
            Verdict::Warn(msg) => {
                self.lineage.log_enforcement(tool, "warn", &msg);
                // Continue but include warning in response
            }
            Verdict::Allow => {}
        }

        // 2. ROUTE: dispatch to correct module
        let result = self.router.dispatch(tool, params).await?;

        // 3. POST-CHECK: log to lineage + update enforcer state
        self.lineage.log_tool_call(tool, &result);
        self.enforcer.post_check(tool, &result);

        Ok(result)
    }
}
```

**Tool routing table:**
```
tender_*        → Tender module
qa_*            → QA module
sv_*            → Social Value module
bid_*           → Bid module
eyes_*          → Eyes module
lineage_*       → Lineage module
hive_*          → Hive module
skill_*         → Skills engine
sentinel_*      → Supervisor module
word_*          → Proxy to external word-document-server
mermaid_*       → Proxy to external mermaid-kroki
puppeteer_*     → Proxy to external puppeteer
threejs_*       → Proxy to external threejs
```

---

## External Server Supervisor

Manages third-party MCP servers that can't be rewritten in Rust.

**Managed servers:**
| Server | Spawn Command | Health Check |
|--------|--------------|-------------|
| word-document-server | `uvx --from office-word-mcp-server word_mcp_server` | MCP initialize |
| mermaid-kroki | `python3 /path/to/server.py` | MCP initialize |
| puppeteer | `npx @modelcontextprotocol/server-puppeteer` | MCP initialize |
| threejs | `npx @modelcontextprotocol/server-threejs` | MCP initialize |

**Proxy pattern:**
```rust
pub struct ExternalProxy {
    name: String,
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    status: HealthStatus,
    restart_count: u32,
}

impl ExternalProxy {
    /// Forward a tool call to the external server via MCP stdin/stdout.
    async fn forward(&mut self, tool: &str, params: Value) -> Result<Value> {
        if self.status == HealthStatus::Down {
            self.attempt_restart().await?;
        }
        // Send JSON-RPC request to child's stdin
        // Read JSON-RPC response from child's stdout
    }

    /// Health check: send MCP initialize, expect response within 2s.
    async fn health_check(&mut self) -> HealthStatus { ... }

    /// Kill and respawn the process.
    async fn attempt_restart(&mut self) -> Result<()> { ... }
}
```

---

## Configuration

**File:** `~/.sentinel/config.toml`

```toml
[general]
data_dir = "~/.sentinel"                    # sentinel.db location
tenders_root = "~/Desktop/Tenders"          # tender corpus for RAG indexing
skills_dir = "~/.sentinel/skills"           # built-in skills
personal_skills_dir = "~/.claude/skills"    # user skill overrides

[supervisor]
health_check_interval = "5s"
max_restart_attempts = 3
restart_cooldown = "60s"
pattern_analysis_interval = 100             # every N tool calls

[enforcer]
enabled = true
default_action = "block"                    # block|warn for unmatched rules

[hive]
max_agents = 5
claude_path = "claude"
default_model = "claude-sonnet-4-6"
agent_timeout = "300s"

[eyes]
port = 0                                    # 0 = random ephemeral
max_image_width = 800

[search]
max_features = 20000
ngram_range = [1, 2]
min_df = 2
max_df = 0.9

[external_servers]
[external_servers.word_document_server]
command = "uvx"
args = ["--from", "office-word-mcp-server", "word_mcp_server"]
enabled = true

[external_servers.mermaid_kroki]
command = "python3"
args = ["/path/to/mermaid-mcp-server/src/server/server.py"]
enabled = true

[external_servers.puppeteer]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-puppeteer"]
enabled = true

[external_servers.threejs]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-threejs"]
enabled = true
```

---

## Project Structure

```
claude-sentinel/
├── Cargo.toml
├── config.toml.example
├── README.md
│
├── src/
│   ├── main.rs                     # CLI entry point: sentinel serve
│   │
│   ├── core/                       # SHARED CORE
│   │   ├── mod.rs
│   │   ├── documents.rs            # DOCX/PDF parsing, rendering
│   │   ├── search.rs               # Unified FTS5 + TF-IDF
│   │   ├── state.rs                # SQLite connection + migrations
│   │   └── migrations/             # SQL migration files
│   │       ├── 001_initial.sql
│   │       └── ...
│   │
│   ├── domain/                     # DOMAIN MODULES
│   │   ├── mod.rs
│   │   ├── tender.rs               # Parse, comply, fill, render
│   │   ├── qa.rs                   # Check, fix, company data
│   │   ├── social_value.rs         # TOMs, calculate, suggest
│   │   ├── bid.rs                  # Frameworks, win themes, score
│   │   └── eyes.rs                 # WebSocket capture, PNG
│   │
│   ├── orchestration/              # ORCHESTRATION LAYER
│   │   ├── mod.rs
│   │   ├── supervisor.rs           # Health, enforce, auto-heal
│   │   ├── enforcer.rs             # Rule engine, pre/post checks
│   │   ├── lineage.rs              # Event log, graph, warnings
│   │   ├── hive/                   # Multi-agent orchestrator
│   │   │   ├── mod.rs
│   │   │   ├── planner.rs
│   │   │   ├── coordinator.rs
│   │   │   ├── spawner.rs
│   │   │   └── memory.rs
│   │   └── skills.rs               # Skill loader, resolver, watcher
│   │
│   ├── gateway/                    # MCP GATEWAY
│   │   ├── mod.rs
│   │   ├── server.rs               # MCP protocol handler
│   │   ├── router.rs               # Tool call routing
│   │   └── proxy.rs                # External server proxy
│   │
│   └── config.rs                   # Config loading
│
├── skills/                         # Bundled skill markdowns
│   ├── brainstorming/SKILL.md
│   ├── debugging/SKILL.md
│   ├── tdd/SKILL.md
│   ├── ...
│   └── sentinel/SKILL.md           # "How to use Sentinel"
│
├── data/                           # Seed data (embedded at compile time)
│   ├── toms_2022.json              # TOMs framework
│   ├── company.json                # Company details
│   ├── frameworks.json             # Bid writing frameworks
│   └── rules.json                  # Default enforcement rules
│
└── tests/
    ├── core/
    ├── domain/
    ├── orchestration/
    └── gateway/
```

---

## Claude Code Integration

**Settings change:** Replace 12 MCP server entries with one:

```json
{
  "mcpServers": {
    "sentinel": {
      "command": "sentinel",
      "args": ["serve"]
    }
  }
}
```

**Hooks:** None needed. All hook logic is compiled into the gateway.

**Skills:** Sentinel exposes `skill_list` and `skill_get` MCP tools. Claude Code's existing Skill tool mechanism works unchanged — skills are still SKILL.md files on disk.

---

## Migration Path

1. Build and test Sentinel alongside existing setup
2. Run both in parallel (Sentinel on different MCP name) to compare behavior
3. Disable old servers one module at a time as Sentinel proves stable
4. Remove old Python/TypeScript/Go/Bash code
5. Single `mcpServers` entry in settings.json

---

## What Sentinel Replaces

| Before | After |
|--------|-------|
| 12 MCP server processes | 1 Sentinel process |
| 4 languages (Python, TS, Go, Node) | 1 language (Rust) |
| 3 bash hook scripts | Compiled-in gateway hooks |
| 1 JS plugin (superpowers) | Built-in skills engine |
| 5 separate data stores | 1 SQLite database |
| 250-500MB RAM | ~15MB RAM |
| No enforcement | Active blocking + learning |
| No health monitoring | Continuous health + auto-heal |
| No shared memory | Full cross-module shared state |
| No cross-session intelligence | Pattern discovery + adaptive rules |
| 12 things to deploy/configure | `cargo install sentinel && sentinel serve` |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| `docx-rs` less mature than `python-docx` | Test thoroughly against real tender documents. Fallback: shell out to Python for complex DOCX ops until crate matures |
| Single process = single point of failure | Rust's safety guarantees + supervisor self-monitoring. Add watchdog systemd/launchd service if needed |
| Large scope (rewriting everything) | Incremental build: shared core first, then one module at a time, parallel run with old servers |
| TF-IDF in Rust more work than scikit-learn | ~100 lines of code. TF-IDF is simple math, not ML. Sparse matrix with ndarray is straightforward |
| Loss of VSCode lineage extension | Keep thin TS extension (~50 lines) that polls Sentinel's HTTP API. No logic in extension |
| Enforcement too strict | All rules configurable, disableable. Start with warn-only mode. Graduate to block after confidence builds |

---

## Success Criteria

1. `sentinel serve` starts in <100ms
2. All existing MCP tool calls work identically (backward compatible)
3. Memory usage <20MB for the entire system
4. External server crash → auto-restart within 5s
5. Claude cannot skip QA after document writes
6. Cross-session search works: "find case studies about X from any past tender"
7. Pattern discovery surfaces at least 1 actionable insight after 10 sessions

---

## Addendum: Audit Findings and Gaps (2026-03-08)

Full audit of all MCP tools, skills, hooks, data, and configuration revealed the following gaps in the original design. All items below are now incorporated.

### Gap 1: Missing MCP Tool — `verify_question_placement`

The tender-orchestrator exposes `verify_question_placement(filepath, question_id, answer_snippet)` which renders a document and visually confirms an answer landed in the correct cell. This was missing from the Tender Module tools table.

**Fix:** Added to Tender Module:

| Tool | Description |
|------|------------|
| `tender_verify_placement` | Render doc to PNG, verify answer appears in correct cell via text matching |

### Gap 2: Superpowers Slash Commands Not Preserved

Superpowers provides 3 slash commands (`/brainstorm`, `/execute-plan`, `/write-plan`) which are thin wrappers that invoke skills. The Skills Engine design didn't account for command registration.

**Fix:** Skills Engine now supports command aliases:

```rust
// skills/brainstorming/SKILL.md frontmatter:
// ---
// name: brainstorming
// command: brainstorm        ← NEW: registers /brainstorm as alias
// description: ...
// ---
```

Skills with a `command:` field in frontmatter are registered as slash commands with Claude Code. The Skills Engine parses and exposes these via `skill_list`.

### Gap 3: Code Reviewer Agent Definition

Superpowers includes an agent definition (`agents/code-reviewer.md`) used by the `requesting-code-review` skill. This is a specialized Claude Code subagent, not an MCP tool.

**Fix:** Agent definitions bundled alongside skills:

```
skills/
├── requesting-code-review/
│   ├── SKILL.md
│   └── code-reviewer.md         ← agent prompt template
├── subagent-driven-development/
│   ├── SKILL.md
│   ├── implementer-prompt.md    ← agent prompt template
│   ├── spec-reviewer-prompt.md
│   └── code-quality-reviewer-prompt.md
```

These are NOT MCP tools — they're prompt templates loaded by skills at runtime. The Skills Engine serves them via `skill_get` when the parent skill references them.

### Gap 4: Skill Sub-Documents

Several skills contain multiple markdown files (not just SKILL.md):

- `systematic-debugging/`: 7 sub-documents (root-cause-tracing.md, defense-in-depth.md, condition-based-waiting.md, test-pressure-1/2/3.md, find-polluter.sh)
- `subagent-driven-development/`: 3 prompt templates
- `writing-skills/`: examples/, anthropic-best-practices.md, graphviz-conventions.dot, persuasion-principles.md, render-graphs.js, testing-skills-with-subagents.md
- `test-driven-development/`: testing-anti-patterns.md

**Fix:** Skills Engine loads entire skill directories, not just SKILL.md:

```rust
pub struct Skill {
    name: String,
    description: String,
    command: Option<String>,        // slash command alias
    main_content: String,           // SKILL.md content
    sub_documents: HashMap<String, String>,  // all other .md/.sh/.js files
    directory: PathBuf,
}
```

`skill_get("systematic-debugging")` returns SKILL.md. `skill_get("systematic-debugging/root-cause-tracing")` returns the sub-document.

### Gap 5: ui-ux-pro-max Has Runtime Dependencies

The `ui-ux-pro-max` skill includes:
- `scripts/search.py` — Python search script for design data
- `data/stacks/` — Framework stack configuration files

These are Python scripts called at runtime, not just markdown.

**Fix:** Two options:
1. **Rewrite search.py in Rust** — embed the design data and search logic in sentinel. The search is simple keyword matching over JSON data.
2. **Keep as external script** — sentinel serves the skill markdown, the Python script runs independently.

**Decision:** Option 1 (rewrite). Add `domain/design.rs` module with embedded design data. Adds ~200 lines. The skill markdown references sentinel tools instead of `python3 scripts/search.py`.

### Gap 6: Superpowers Session-Start Hook

Superpowers has its own session-start hook (`hooks/session-start` via `run-hook.cmd`) separate from the Hive session hook. This hook runs `skills-core.js` to discover and register skills on startup.

**Fix:** Sentinel's startup sequence already replaces this:
- Step 3 in startup: "Load config.toml"
- Step 5 in startup: Skills Engine initializes, scans directories, registers all skills

No separate hook needed. The Skills Engine IS the session-start logic.

### Gap 7: Bid-Writing Data Migration (264KB)

The bid-writing MCP server embeds 6 TypeScript data files totaling 264KB:
- `copywriting-frameworks.ts` (52KB)
- `formal-frameworks.ts` (40KB)
- `industry-techniques.ts` (30KB)
- `sales-frameworks.ts` (66KB)
- `section-templates.ts` (34KB)
- `strategy-frameworks.ts` (43KB)

**Fix:** Convert to JSON, embed at compile time via `include_str!()`, seed into `frameworks` table on first run:

```rust
// data/frameworks.json — converted from TypeScript
const FRAMEWORKS_JSON: &str = include_str!("../../data/frameworks.json");

fn seed_frameworks(db: &Connection) -> Result<()> {
    let frameworks: Vec<Framework> = serde_json::from_str(FRAMEWORKS_JSON)?;
    for fw in frameworks {
        db.execute(
            "INSERT OR IGNORE INTO frameworks (name, category, content, tags) VALUES (?1, ?2, ?3, ?4)",
            params![fw.name, fw.category, fw.content, serde_json::to_string(&fw.tags)?],
        )?;
    }
    Ok(())
}
```

### Gap 8: Tender-RAG Index Migration (17.7MB pickle)

The current TF-IDF index is a Python pickle file at `~/.cache/tender-rag/index.pkl`. This is Python-specific and can't be loaded by Rust.

**Fix:** Don't migrate the pickle. Rebuild the index on first run:

```
sentinel serve (first run):
  1. Detect no search index exists in sentinel.db
  2. Walk ~/Desktop/Tenders/ (377MB, ~500 documents)
  3. Extract text from each DOCX/PDF/TXT
  4. Build TF-IDF matrix + FTS5 index
  5. Store in sentinel.db
  6. Log: "Index built: 500 documents, 20,000 features, 45s"
```

Subsequent startups skip rebuild (mtime-based cache invalidation).

### Gap 9: Hive DB Migration (48KB)

Existing `~/.hive/hive.db` contains 1 learning, 0 goals, 0 tasks.

**Fix:** One-time migration on first sentinel run:

```rust
fn migrate_hive_db(sentinel_db: &Connection, hive_db_path: &Path) -> Result<()> {
    let hive = Connection::open(hive_db_path)?;
    // Copy learnings
    let mut stmt = hive.prepare("SELECT domain, lesson, context, tags, source_task, outcome FROM learnings")?;
    // Insert into sentinel_db.learnings
    // Copy goals, tasks, agent_logs if any exist
}
```

### Gap 10: Company Data — Sensitive Field Handling

`company_data.py` contains sensitive fields (UTR, CDP password, mobile, PSC DOB) marked with `SENSITIVE_FIELDS`. These must:
1. Be stored in `company` table with `sensitive = 1`
2. Be checked by QA module's `qa_check_sensitive` against document content
3. Never appear in enforcement logs or pattern data

**Fix:** Already handled by schema (`company.sensitive` column). Add to enforcer:

```rust
// In qa.rs — sensitive data check
fn check_sensitive(&self, doc: &Document) -> Vec<Finding> {
    let sensitive = self.db.query_company_sensitive_fields();
    for (label, value) in sensitive {
        if doc.text.contains(&value) {
            findings.push(Finding::critical(
                format!("Sensitive field '{}' found in document", label)
            ));
        }
    }
    findings
}
```

### Gap 11: CLAUDE.md Needs Updating

When sentinel is active, CLAUDE.md sections need changes:

| Section | Current | Updated |
|---------|---------|---------|
| Task Start Protocol step 1 | "scan all available MCP servers" | "sentinel provides all tools — check sentinel_status" |
| Task Start Protocol step 5 | "use parallel agents or hive orchestrator" | "use hive_orchestrate via sentinel" |
| Tool Orchestration | "chain tools, verify" | "sentinel enforces chains automatically — focus on the task" |
| Mandatory Verification Loop | Manual QA → Render → Inspect | "sentinel blocks if QA is skipped — run qa_full_check, sentinel handles the rest" |
| Tender Work Roles | Lists 8 roles | "sentinel maps roles to modules automatically" |
| Memory | "save to memory files" | "use hive_learn to store learnings in sentinel.db" |

**Fix:** Generate updated CLAUDE.md as part of sentinel install:

```
sentinel init
  → Creates ~/.sentinel/
  → Seeds sentinel.db with embedded data
  → Generates updated CLAUDE.md with sentinel references
  → Updates ~/.claude/settings.json (replaces 12 servers with 1)
  → Migrates ~/.hive/hive.db if exists
```

### Gap 12: Lineage VSCode Extension

The current lineage VSCode extension (`src/extension/`) polls an HTTP endpoint. Sentinel needs to serve this same HTTP API for backward compatibility.

**Fix:** Already covered — Lineage Module includes HTTP API at `/events`, `/warnings`, `/graph`, `/health`. The existing thin TS extension continues to work, pointing to sentinel's HTTP port instead.

Add to config:

```toml
[lineage]
http_port = 0    # 0 = random, written to /tmp/lineage-port (same as before)
```

### Gap 13: Permissions and Additional Directories

`settings.json` contains 110+ permission allow rules and 7 additional directories. These are Claude Code settings, NOT sentinel settings. They stay in `settings.json`.

**Fix:** No migration needed. Sentinel only replaces `mcpServers` and `hooks` in settings.json. Permissions and additional directories remain untouched.

### Gap 14: Unrelated Projects in ~/projects/

The audit found projects NOT related to sentinel:
- `Tesseract-Gov-Website` — company website
- `hyper-sublimation` — unknown
- `GATEEVONEW` — unknown
- `legacy-ui` — legacy codebase
- `MCP-Dandan` — unknown MCP server
- `high-annealer` / `high-annealer-vscode` — code analysis tool (separate project)

**Decision:** These are out of scope. Sentinel doesn't touch them.

### Gap 15: Auto-Memory Directory

Claude Code's auto-memory lives at `~/.claude/projects/-Users-fabio/memory/` (currently empty). This is Claude Code's built-in feature, not something sentinel replaces.

**Decision:** Keep as-is. Sentinel's `learnings` table is for hive cross-session memory. Claude Code's auto-memory is a separate concern managed by Claude Code itself.

### Gap 16: `sentinel init` Command

The design mentions `sentinel serve` but doesn't describe first-time setup.

**Fix:** Add `sentinel init` subcommand:

```
sentinel init
  ├── Create ~/.sentinel/ directory
  ├── Create sentinel.db with full schema
  ├── Seed TOMs data (24 measures → toms table)
  ├── Seed company data (company_data.py → company table)
  ├── Seed bid frameworks (264KB TypeScript → frameworks table)
  ├── Seed default enforcement rules (6 built-in → rules table)
  ├── Copy bundled skills to ~/.sentinel/skills/
  ├── Build search index from ~/Desktop/Tenders/ (~45s)
  ├── Migrate ~/.hive/hive.db if exists
  ├── Update ~/.claude/settings.json:
  │   ├── Replace 12 mcpServers entries with 1 sentinel entry
  │   ├── Remove 3 hooks entries (compiled into sentinel)
  │   └── Keep permissions and additionalDirectories unchanged
  ├── Generate updated CLAUDE.md with sentinel references
  └── Print: "Sentinel initialized. Run 'sentinel serve' to start."

sentinel serve
  ├── Open sentinel.db
  ├── Start all internal modules
  ├── Spawn external servers
  ├── Health-check everything
  ├── Start MCP server on stdin/stdout
  └── Print: "Sentinel online. Ready."
```

---

## Updated Totals After Audit

| Category | Count |
|----------|-------|
| MCP tools (custom, migrated to Rust) | **65** (was 64 — added verify_placement) |
| MCP tools (external, proxied) | **~40** (word-doc, mermaid, puppeteer, threejs) |
| Skills (with sub-documents) | **20** (14 superpowers + 6 custom) |
| Slash commands | **3** (/brainstorm, /execute-plan, /write-plan) |
| Agent prompt templates | **4** (code-reviewer, implementer, spec-reviewer, code-quality-reviewer) |
| Seed data files | **4** (toms, company, frameworks, rules) |
| DB tables | **17** |
| Enforcement rules (built-in) | **6** |
| External servers (supervised) | **4** |
| Config sections | **7** |

All gaps accounted for. Design is complete.
