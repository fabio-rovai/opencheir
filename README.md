# Sentinel

A single Rust binary that replaces 12 MCP servers (Python, TypeScript, Go, Node) with one fast, unified MCP server for Claude Code.

## What it does

Sentinel consolidates tender writing, bid analysis, QA, social value, search, enforcer rules, lineage tracking, memory, and pattern discovery into **49 MCP tools** served over stdio.

### Modules

| Module | Tools | Replaces |
|--------|-------|----------|
| Social Value | `list_toms_themes`, `search_toms`, `calculate_social_value`, `suggest_social_value`, `draft_social_value_response`, `get_toms_measure`, `get_toms_by_theme` | `social-value` (Python) |
| Bid Writing | `list_frameworks`, `get_framework`, `compare_frameworks`, `recommend_framework`, `search_resources`, `score_proposal`, `generate_win_themes`, `generate_compliance_matrix`, `bid_no_bid_analysis`, `generate_executive_summary`, `generate_proposal_outline`, `get_section_template`, `get_industry_guide`, `get_persuasion_technique` | `bid-writing` (Node) |
| Tender | `parse_tender`, `read_answer`, `check_compliance`, `check_pass_fail_questions`, `check_submission_files` | `tender-orchestrator` (Python) |
| QA | `qa_check_fonts`, `qa_check_dashes`, `qa_check_word_counts`, `qa_check_sensitive_info`, `qa_check_signatures`, `qa_check_filenames`, `qa_full_check` | `tender-qa` (Python) |
| Search | `search_tenders` | `tender-rag` (Python) |
| Lineage | `lineage_record`, `lineage_events`, `lineage_timeline` | `lineage` (Node) |
| Enforcer | `enforcer_check`, `enforcer_log`, `enforcer_rules`, `enforcer_toggle_rule` | new |
| Memory | `hive_memory_store`, `hive_memory_recall`, `hive_memory_by_domain` | `hive` (Go) |
| Patterns | `pattern_analyze`, `pattern_list` | new |
| Status | `sentinel_status`, `sentinel_health` | new |

## Requirements

- Rust 1.80+ (uses `edition = "2024"`)
- macOS or Linux

### Companion MCP Servers

Sentinel handles tender analysis, QA, bid writing, and orchestration — but it does **not** write DOCX files, automate browsers, or render diagrams. You need these MCP servers alongside Sentinel:

| Server | Purpose | Why Sentinel needs it |
|--------|---------|----------------------|
| `word-document-server` | Create and edit Word documents | Sentinel reads DOCX for QA/parsing but cannot write them |
| `puppeteer` | Browser automation, screenshots | Visual inspection of rendered documents |
| `mermaid-kroki` | Mermaid/Kroki diagram rendering | Generate architecture and flow diagrams |

## Install

```bash
# Clone
git clone https://github.com/YOUR_USERNAME/claude-sentinel.git
cd claude-sentinel

# Build
cargo build --release

# Initialize (creates ~/.sentinel/sentinel.db and config)
./target/release/sentinel init
```

## Configure Claude Code

Add Sentinel to your Claude Code MCP settings (`~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "sentinel": {
      "command": "/path/to/claude-sentinel/target/release/sentinel",
      "args": ["serve"]
    }
  }
}
```

**Important:** Remove or disable the MCP servers that Sentinel replaces to avoid duplicate tools:

- `bid-writing`
- `tender-orchestrator`
- `tender-rag`
- `social-value`
- `tender-qa`
- `hive`
- `lineage`

Servers Sentinel does **not** replace (keep these):

- `word-document-server` (DOCX writing)
- `puppeteer` (browser automation)
- `mermaid-kroki` (diagram rendering)
- `eyes` (visual inspection)

Restart Claude Code after changing settings.

## Verify

Start a new Claude Code session and check that Sentinel tools are available:

```
Tools should appear as mcp__sentinel__<tool_name>
e.g. mcp__sentinel__parse_tender, mcp__sentinel__qa_check_fonts
```

## Architecture

```
gateway/          MCP interface (rmcp), tool routing, JSON-RPC proxy
  server.rs       49 tool definitions, dispatches to domain/orchestration
domain/           Business logic (stateless services)
  tender.rs       DOCX parsing, question extraction, compliance
  qa.rs           Font, dash, word count, signature checks
  social_value.rs TOMs measures, calculations, suggestions
  bid.rs          Frameworks, scoring, win themes, compliance matrix
  eyes.rs         Capture storage, HTTP dashboard
orchestration/    Cross-cutting workflows
  enforcer.rs     Rule engine with sliding window
  lineage.rs      Event tracking, timelines, dependency graphs
  skills.rs       Skill directory scanning, YAML frontmatter
  patterns.rs     Cross-session pattern discovery
  hive/           Multi-agent orchestration
    planner.rs    Goal/task planning
    coordinator.rs DAG-based task scheduling
    spawner.rs    Claude CLI subprocess management
    memory.rs     FTS5-backed learning storage
sentinel_core/    State management
  state.rs        SQLite with WAL, Arc<Mutex<Connection>>
  documents.rs    DOCX parsing via docx-rs
  search.rs       FTS5 full-text search
```

## Data

- `~/.sentinel/sentinel.db` — SQLite database (WAL mode)
- `~/.sentinel/config.toml` — configuration
- Company data is seeded from embedded JSON on `sentinel init`

## Development

```bash
# Run tests (350 tests)
cargo test

# Run specific test file
cargo test --test qa_test

# Build debug
cargo build
```

## License

Private.
