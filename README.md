# OpenCheir

Lightweight, open-source document governance MCP server written in Rust.

OpenCheir (from Greek χείρ, "hand") provides document QA, workflow enforcement, audit trails, and multi-agent orchestration as a single MCP binary.

## Features

| Module | Tools | Purpose |
|--------|-------|---------|
| Document QA | 5 | Font, dash, word count, signature checks |
| Document Parsing | 2 | DOCX structure extraction |
| Search | 1 | FTS5 full-text search |
| Enforcer | 4 | Workflow rule engine |
| Lineage | 3 | Audit trail & event tracking |
| Patterns | 2 | Cross-session pattern discovery |
| Memory | 3 | Persistent learning storage |
| Hive | - | Multi-agent orchestration |
| Ontology | 9 | RDF/OWL validation, SPARQL queries, format conversion, diff, lint |
| Status | 2 | Health monitoring |

## Requirements

- Rust 1.80+
- macOS or Linux

## Install

```bash
git clone https://github.com/fabio-rovai/opencheir.git
cd opencheir
cargo build --release
./target/release/opencheir init
```

## Configure Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "opencheir": {
      "command": "/path/to/opencheir",
      "args": ["serve"]
    }
  }
}
```

## Tools

Tools appear as `mcp__opencheir__<tool_name>` in Claude Code.

### Document QA

- `qa_check_fonts` — detect font inconsistencies in DOCX
- `qa_check_dashes` — detect dash/hyphen inconsistencies
- `qa_check_word_counts` — check word limits vs actual
- `qa_check_signatures` — detect unfilled signature placeholders
- `qa_full_check` — run all QA checks at once

### Document Parsing

- `parse_document` — extract text, tables, structure from DOCX
- `read_content` — read specific table cell content

### Search

- `search_documents` — full-text search across indexed documents

### Enforcer

- `enforcer_check` — check if tool call is allowed by rules
- `enforcer_log` — view enforcement log
- `enforcer_rules` — list all rules
- `enforcer_toggle_rule` — enable/disable rules

### Lineage

- `lineage_record` — record events
- `lineage_events` — query events
- `lineage_timeline` — session timeline

### Memory

- `hive_memory_store` — store learnings
- `hive_memory_recall` — search memory
- `hive_memory_by_domain` — get learnings by domain

### Patterns

- `pattern_analyze` — discover workflow patterns
- `pattern_list` — list discovered patterns

### Ontology

- `onto_validate` — validate RDF/OWL syntax (file or inline)
- `onto_convert` — convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG)
- `onto_load` — load RDF into in-memory store
- `onto_query` — run SPARQL queries against loaded ontology
- `onto_save` — save ontology store to file
- `onto_stats` — triple count, classes, properties, individuals
- `onto_diff` — compare two ontology files (added/removed triples)
- `onto_lint` — check for missing labels, comments, domains
- `onto_clear` — clear in-memory store

### Status

- `opencheir_status` — system health summary
- `opencheir_health` — detailed health info

## Architecture

```
opencheir/
├── src/
│   ├── gateway/     # MCP tool definitions & routing
│   ├── domain/      # Document QA, ontology engine, image capture
│   ├── orchestration/ # Enforcer, lineage, hive, patterns
│   └── core/        # SQLite state, document parsing, search
└── tests/
```

## License

MIT
