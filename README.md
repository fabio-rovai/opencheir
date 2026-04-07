# OpenCheir

Verified tender pipeline written in [Tardygrada](https://github.com/fabio-rovai/tardygrada).

OpenCheir (from Greek χείρ, "hand") provides document QA, compliance checking, workflow enforcement, and end-to-end tender governance — all as verified `.tardy` agents. Every document operation goes through Tardygrada's 8-layer verification pipeline.

## Architecture

```
opencheir/
├── tender.tardy              # pipeline orchestrator (25 instructions)
├── agents/
│   ├── parse_docx.tardy      # DOCX → structured claims via unzip
│   ├── parse_pdf.tardy       # PDF → PageIndex tree via pdftotext
│   ├── qa_fonts.tardy        # font consistency check
│   ├── qa_dashes.tardy       # dash/hyphen consistency
│   ├── qa_signatures.tardy   # unfilled signature detection
│   ├── qa_wordcount.tardy    # word count vs limits
│   ├── search.tardy          # FTS5 indexing and querying
│   ├── enforcer.tardy        # workflow rule engine
│   ├── compliance.tardy      # requirements ↔ responses matrix
│   ├── validate.tardy        # hedging, fallacies, reading level
│   ├── firewall.tardy        # prompt injection guard @sovereign
│   ├── bid_analysis.tardy    # bid/no-bid scoring
│   ├── extract_reqs.tardy    # spec → verified requirements
│   ├── draft.tardy           # response drafting grounded in spec
│   ├── source_verify.tardy   # quote verification
│   └── finalize.tardy        # @sovereign lock on submission
└── ontologies/
    ├── document.ttl           # DOCX/PDF structure ontology
    ├── tender.ttl             # tender domain ontology
    └── compliance.ttl         # requirements/responses ontology
```

## Tender Pipeline

```
1. Ingest    → parse spec PDFs and DOCX files into verified claims
2. Extract   → pull requirements with page references
3. Bid/no-bid → score opportunity against company capabilities
4. Compliance → match requirements to response sections, flag gaps
5. Draft     → write responses grounded in spec + case studies
6. QA        → check fonts, dashes, signatures, word counts
7. Validate  → detect hedging, fallacies, fabrication
8. Finalize  → lock submission with @sovereign immutability
```

Every step produces verified claims. The pipeline cannot proceed with unverified data.

## Requirements

- [Tardygrada](https://github.com/fabio-rovai/tardygrada) (build from source)
- System tools: `unzip`, `sqlite3`, `pdftotext` (poppler)

## Install

```bash
# Build Tardygrada
git clone https://github.com/fabio-rovai/tardygrada.git
cd tardygrada && make

# Clone OpenCheir
git clone https://github.com/fabio-rovai/opencheir.git
```

## Usage

```bash
# Check all agents compile
for f in agents/*.tardy; do tardygrada check "$f"; done

# Run the full pipeline
tardygrada serve tender.tardy

# Run individual agents
tardygrada run agents/qa_fonts.tardy
tardygrada run agents/compliance.tardy
```

## Verification Guarantees

Every agent runs inside Tardygrada's verification pipeline:

| Agent | Trust Level | What's Verified |
|-------|-------------|-----------------|
| `firewall.tardy` | `@sovereign` | Cannot be bypassed — BFT + ed25519 signed |
| `finalize.tardy` | `@sovereign` | Submission locked — cryptographically immutable |
| `enforcer.tardy` | `@sovereign` | Workflow rules cannot be skipped |
| All QA agents | `@verified` | SHA-256 checked — results are tamper-proof |
| All analysis agents | `@verified` | Claims grounded in ontology + evidence |

## Ontologies

| Ontology | Classes | Purpose |
|----------|---------|---------|
| `document.ttl` | Document, Section, Paragraph, Table, Run, Font, TocEntry | DOCX/PDF structure |
| `tender.ttl` | Tender, Specification, Requirement, Lot, CaseStudy, StaffMember, PricingSchedule | Tender domain |
| `compliance.ttl` | ComplianceMatch, CoverageStatus, Evidence, Gap, Quote | Requirements mapping |

## History

OpenCheir v1 was a Rust MCP server (see git history). v2 is a complete rewrite in Tardygrada's `.tardy` language — replacing ~5,000 lines of Rust with 17 verified agents totalling ~160 instructions.

## License

MIT
