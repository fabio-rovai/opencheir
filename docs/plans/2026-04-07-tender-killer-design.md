# OpenCheir v2: Tender Killer — Design Document

**Date:** 2026-04-07
**Status:** Approved

## Vision

OpenCheir is refactored from a Rust MCP server into a **collection of `.tardy` programs** — a verified tender pipeline written in Tardygrada's machine-first language.

No compiled binary. Just `.tardy` files + ontologies + system CLI tools (`unzip`, `sqlite3`, `pdftotext`).

## Architecture

```text
opencheir/
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
2. **No MCP server** — Tardygrada IS the runtime; `.tardy` agents are the tools
3. **`exec()` for I/O** — shell out to `unzip`, `sqlite3`, `pdftotext` for document operations
4. **All intelligence in `.tardy`** — compliance matching, validation, enforcer rules, bid/no-bid, drafting
5. **Ontology-grounded** — every claim grounded in `document`, `tender`, or `compliance` ontology
6. **Features inherited from successors** — source verification (BITF), validation signals (BITF), prompt firewall (BITF), design pattern enforcement (Open Ontologies), document immutability (Tardygrada) — all written as `.tardy` agents

## Tender Pipeline (End-to-End)

```text
Step 1: Ingest spec PDFs → Tardygrada PageIndex builds verified trees
Step 2: Extract requirements → verified claims with page refs
Step 3: Bid/no-bid analysis → .tardy agent scores against company profile
Step 4: Compliance matrix → FTS5 keyword + tag matching, gap detection
Step 5: Draft responses → agents write, grounded in spec + case studies
Step 6: QA → font, dash, signature, word count checks
Step 7: Validation → hedging, fallacies, reading level, fabrication
Step 8: Final lock → @sovereign on submitted documents
```

## Migration from Rust

The Rust v1 code is preserved in git history. The refactor replaces ~5,000 lines of Rust with 17 verified agents totalling ~160 VM instructions.

| Old Rust Module | New `.tardy` Agent |
| --- | --- |
| `domain/qa.rs` | `qa_fonts.tardy`, `qa_dashes.tardy`, `qa_signatures.tardy`, `qa_wordcount.tardy` |
| `store/documents.rs` | `parse_docx.tardy` |
| `store/search.rs` | `search.tardy` |
| `orchestration/enforcer.rs` | `enforcer.tardy` |
| `orchestration/lineage.rs` | Tardygrada native |
| `orchestration/hive/memory.rs` | Tardygrada native |
| `orchestration/patterns.rs` | `validate.tardy` |
| N/A (new) | `compliance.tardy`, `bid_analysis.tardy`, `extract_reqs.tardy`, `draft.tardy`, `source_verify.tardy`, `firewall.tardy`, `finalize.tardy`, `parse_pdf.tardy` |

## External Dependencies

System tools only (available on macOS/Linux):
- `unzip` — extract DOCX XML (DOCX = ZIP archive)
- `sqlite3` — FTS5 search, key-value storage
- `pdftotext` (poppler) — PDF text extraction
