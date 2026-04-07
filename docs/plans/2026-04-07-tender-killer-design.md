# OpenCheir v2: Tender Killer — Design Document

**Date:** 2026-04-07
**Status:** Approved

## Vision

OpenCheir stops being a Rust MCP server. It becomes a **collection of `.tardy` programs** — a verified tender pipeline written in Tardygrada's machine-first language.

No Rust. No Python. No sidecars. Just `.tardy` files + ontologies + system CLI tools (`unzip`, `sqlite3`, `pdftotext`).

## Architecture

```
tardygrada/examples/opencheir/
├── tender.tardy              # main pipeline orchestrator
├── agents/
│   ├── parse_docx.tardy      # DOCX → structured claims
│   ├── parse_pdf.tardy       # PDF → PageIndex tree (verified)
│   ├── qa_fonts.tardy        # font consistency check
│   ├── qa_dashes.tardy       # dash/hyphen consistency
│   ├── qa_signatures.tardy   # unsigned placeholder detection
│   ├── qa_wordcount.tardy    # word count vs limits
│   ├── search.tardy          # FTS5 indexing and querying
│   ├── enforcer.tardy        # workflow rule engine
│   ├── compliance.tardy      # requirements <> responses matrix
│   ├── validate.tardy        # hedging, fallacies, reading level
│   ├── firewall.tardy        # prompt injection guard @sovereign
│   ├── bid_analysis.tardy    # bid/no-bid scoring
│   ├── extract_reqs.tardy    # spec → verified requirements
│   ├── draft.tardy           # write responses, grounded in spec
│   ├── source_verify.tardy   # quote verification
│   └── finalize.tardy        # @sovereign lock on submission
└── ontologies/
    ├── document.ttl           # DOCX/PDF structure ontology
    ├── tender.ttl             # tender domain ontology
    └── compliance.ttl         # requirements/responses ontology
```

## Decisions

1. **No Rust binary** — OpenCheir is pure `.tardy`
2. **No MCP server** — Tardygrada IS the server; `.tardy` agents are the tools
3. **No sidecars** — single Tardygrada binary runs everything
4. **`exec()` for I/O** — shell out to `unzip`, `sqlite3`, `pdftotext` for document operations
5. **PageIndex logic in Tardygrada C** — PDF parsing, TOC detection, tree building added to Tardygrada's C codebase (not `.tardy`)
6. **All intelligence in `.tardy`** — compliance matching, validation, enforcer rules, bid/no-bid, drafting
7. **Ontology-grounded** — every claim grounded in `document`, `tender`, or `compliance` ontology
8. **Features inherited from successors** — source verification (BITF), validation signals (BITF), prompt firewall (BITF), design pattern enforcement (Open Ontologies), document immutability (Tardygrada) — all written as `.tardy` agents

## Tender Pipeline (End-to-End)

```
Step 1: Ingest spec PDFs → Tardygrada PageIndex builds verified trees
Step 2: Extract requirements → verified claims with page refs
Step 3: Bid/no-bid analysis → .tardy agent scores against company profile
Step 4: Compliance matrix → FTS5 keyword + tag matching, gap detection
Step 5: Draft responses → agents write, grounded in spec + case studies
Step 6: QA → font, dash, signature, word count checks
Step 7: Validation → hedging, fallacies, reading level, fabrication
Step 8: Final lock → @sovereign on submitted documents
```

## OpenCheir Socket API (Eliminated)

Previously OpenCheir exposed ~30 MCP tools over a socket. Now replaced by:

| Old OpenCheir Tool | New `.tardy` Approach |
|---|---|
| `qa_check_fonts` | `qa_fonts.tardy` — `exec("unzip -p doc.docx word/document.xml")` + parse fonts |
| `qa_check_dashes` | `qa_dashes.tardy` — same XML extraction, scan for dash variants |
| `qa_check_signatures` | `qa_signatures.tardy` — scan for `[sign here]` patterns |
| `qa_check_word_counts` | `qa_wordcount.tardy` — count words per table cell |
| `doc_parse` | `parse_docx.tardy` — `exec("unzip")` + XML walk |
| `search_documents` | `search.tardy` — `exec("sqlite3")` FTS5 queries |
| `enforcer_check` | `enforcer.tardy` — rule evaluation as verified claims |
| `lineage_record` | Tardygrada native (already has lineage) |
| `hive_memory_store` | Tardygrada native (already has agent memory) |
| `pattern_analyze` | `validate.tardy` — pattern discovery as verified claims |

## Ontologies

### document.ttl — Document Structure

Models DOCX/PDF structure: paragraphs, tables, runs, fonts, sections, pages, TOC entries.

### tender.ttl — Tender Domain

Models tender concepts: specifications, requirements, lots, evaluation criteria, case studies, staff, pricing, compliance status.

### compliance.ttl — Requirements Mapping

Models the relationship between spec requirements and response sections: covers, partially_covers, gap, evidence, quote, page_reference.

## External Dependencies

System tools only (available on macOS/Linux):
- `unzip` — extract DOCX XML (DOCX = ZIP archive)
- `sqlite3` — FTS5 search, key-value storage
- `pdftotext` (poppler) — PDF text extraction (for PageIndex input)

## What Happens to the Rust Codebase

The existing OpenCheir Rust project at `/Users/fabio/projects/opencheir` is **archived**. The new OpenCheir lives inside Tardygrada's examples as the flagship `.tardy` project.

## Promotion Value

This becomes the headline example in Tardygrada's README:

> "OpenCheir: a production-grade verified tender pipeline in 16 `.tardy` files. Parses DOCX and PDF specs, extracts requirements, checks compliance, drafts responses, runs QA, detects fabrication, and locks final submissions with `@sovereign` immutability. Zero dependencies beyond system tools."

Proves Tardy is a machine-first language that can replace traditional application codebases.
