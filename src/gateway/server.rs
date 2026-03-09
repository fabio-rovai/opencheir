use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, Tool},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::orchestration::enforcer::{Action, Enforcer};
use crate::store::graph::GraphStore;
use crate::store::state::StateDb;

// ─── MCP tool input structs ─────────────────────────────────────────────────

// Document
#[derive(Deserialize, JsonSchema)]
pub struct DocPathInput {
    /// Path to the DOCX document
    pub path: String,
}

// QA
#[derive(Deserialize, JsonSchema)]
pub struct QaFullCheckInput {
    /// Path to the DOCX document
    pub path: String,
}

// Search
#[derive(Deserialize, JsonSchema)]
pub struct SearchInput {
    /// Full-text search query
    pub query: String,
    /// Optional source filter
    pub source: Option<String>,
    /// Maximum results to return
    pub limit: Option<usize>,
}

// Lineage
#[derive(Deserialize, JsonSchema)]
pub struct LineageRecordInput {
    /// Session ID
    pub session_id: String,
    /// Event type (tool_call, tool_result, file_read, file_write)
    pub event_type: String,
    /// Optional file path
    pub path: Option<String>,
    /// Optional tool name
    pub tool: Option<String>,
    /// Optional metadata (JSON)
    pub meta: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct LineageEventsInput {
    /// Optional session ID filter
    pub session_id: Option<String>,
    /// Optional event type filter
    pub event_type: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
pub struct LineageTimelineInput {
    /// Session ID to get timeline for
    pub session_id: String,
}

// Enforcer
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerCheckInput {
    /// Tool name to check against enforcer rules
    pub tool_name: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerLogInput {
    /// Optional session ID filter
    pub session_id: Option<String>,
    /// Maximum entries to return
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerRuleToggleInput {
    /// Rule name to enable or disable
    pub rule_name: String,
    /// Whether to enable the rule
    pub enabled: bool,
}

// Memory
#[derive(Deserialize, JsonSchema)]
pub struct MemoryStoreInput {
    /// Knowledge domain (e.g. "rust", "writing")
    pub domain: String,
    /// The lesson or insight to store
    pub lesson: String,
    /// Optional context about where this was learned
    pub context: Option<String>,
    /// Tags for categorisation
    pub tags: Option<Vec<String>>,
    /// Lock token from hive_claim_domain (required if domain is locked)
    pub token: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct MemoryRecallInput {
    /// Full-text search query
    pub query: String,
    /// Maximum results
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
pub struct MemoryByDomainInput {
    /// Domain to filter by
    pub domain: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct HiveClaimDomainInput {
    /// Domain to lock (e.g. "ops", "research")
    pub domain: String,
    /// Identifier for the agent claiming the lock
    pub locked_by: String,
    /// Lock TTL in seconds (default 60)
    pub ttl_seconds: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HiveReleaseDomainInput {
    /// Domain to release
    pub domain: String,
    /// Token returned by hive_claim_domain
    pub token: String,
}

// Patterns
#[derive(Deserialize, JsonSchema)]
pub struct PatternListInput {
    /// Optional category filter
    pub category: Option<String>,
}

// Ontology
#[derive(Deserialize, JsonSchema)]
pub struct OntoValidateInput {
    /// Path to an RDF file OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content rather than a file path
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoConvertInput {
    /// Path to source RDF file
    pub path: String,
    /// Target format: turtle, ntriples, rdfxml, nquads, trig
    pub to: String,
    /// Optional output file path (if omitted, returns content)
    pub output: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLoadInput {
    /// Path to RDF file to load into the in-memory store
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoQueryInput {
    /// SPARQL query string
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSaveInput {
    /// Output file path
    pub path: String,
    /// Format: turtle, ntriples, rdfxml, nquads, trig
    pub format: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDiffInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLintInput {
    /// Path to RDF file to lint, OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPullInput {
    /// Remote URL or SPARQL endpoint to fetch ontology from
    pub url: String,
    /// If true, treat url as a SPARQL endpoint and run a CONSTRUCT query
    pub sparql: Option<bool>,
    /// Optional SPARQL CONSTRUCT query (required if sparql=true)
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPushInput {
    /// Remote SPARQL endpoint URL
    pub endpoint: String,
    /// Optional named graph IRI
    pub graph: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoImportInput {
    /// Resolve and load all owl:imports from the currently loaded ontology
    pub max_depth: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoVersionInput {
    /// Version label (e.g. "v1.0", "draft-2026-03-09")
    pub label: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRollbackInput {
    /// Version label to restore
    pub label: String,
}

// ─── OpenCheirServer ────────────────────────────────────────────────────────

/// MCP server that exposes all OpenCheir tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct OpenCheirServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    enforcer: Arc<Mutex<Enforcer>>,
    graph: Arc<GraphStore>,
    lock_ttl_seconds: u32,
}

impl OpenCheirServer {
    /// Create a new server with all tools wired to domain/orchestration services.
    pub fn new(db: StateDb, enforcer: Arc<Mutex<Enforcer>>, lock_ttl_seconds: u32) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            enforcer,
            graph: Arc::new(GraphStore::new()),
            lock_ttl_seconds,
        }
    }

    /// Return the list of all registered tool definitions.
    pub fn list_tool_definitions(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }
}

// ─── Tool definitions ───────────────────────────────────────────────────────

#[tool_router]
impl OpenCheirServer {

    // ── OpenCheir ────────────────────────────────────────────────────────────

    #[tool(name = "opencheir_status", description = "Returns a high-level system health summary for the OpenCheir platform")]
    fn opencheir_status(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "modules": {
                "gateway": "active",
                "store": "active",
                "domain": "active",
                "orchestration": "active"
            },
            "tool_count": tool_count
        })
        .to_string()
    }

    #[tool(name = "opencheir_health", description = "Returns detailed health information for each OpenCheir component")]
    fn opencheir_health(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        serde_json::json!({
            "components": {
                "gateway": { "status": "ok", "tools": tool_count },
                "state_db": { "status": "available" },
                "documents": { "status": "available" },
                "search": { "status": "available" },
                "qa": { "status": "available" },
                "eyes": { "status": "available" },
                "lineage": { "status": "available" },
                "hive": { "status": "available" },
                "skills": { "status": "available" },
                "enforcer": { "status": "available" },
                "supervisor": { "status": "available" }
            },
            "version": env!("CARGO_PKG_VERSION")
        })
        .to_string()
    }

    // ── QA ───────────────────────────────────────────────────────────────────

    #[tool(name = "qa_check_fonts", description = "Check for font consistency issues in a DOCX document")]
    async fn qa_check_fonts(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::store::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_fonts(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_dashes", description = "Check for dash/hyphen inconsistencies in a DOCX document")]
    async fn qa_check_dashes(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::store::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_dashes(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_word_counts", description = "Check word counts against limits in a DOCX document")]
    async fn qa_check_word_counts(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::store::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_word_counts(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_signatures", description = "Check for signature placeholders in a DOCX document")]
    async fn qa_check_signatures(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::store::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_signatures(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_full_check", description = "Run all QA checks on a DOCX document (fonts, dashes, smart quotes, word counts, signatures)")]
    async fn qa_full_check(&self, Parameters(input): Parameters<QaFullCheckInput>) -> String {
        use crate::store::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                serde_json::to_string(&QaService::full_check(&doc))
                    .unwrap_or_default()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Search ──────────────────────────────────────────────────────────────

    #[tool(name = "search_documents", description = "Full-text search across indexed document content")]
    async fn search_documents(&self, Parameters(input): Parameters<SearchInput>) -> String {
        use crate::store::search::SearchService;
        match SearchService::search(&self.db, &input.query, input.source.as_deref(), input.limit.unwrap_or(10)) {
            Ok(results) => serde_json::to_string(&results).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Lineage ─────────────────────────────────────────────────────────────

    #[tool(name = "lineage_record", description = "Record a lineage event (tool call, file read/write, etc.)")]
    async fn lineage_record(&self, Parameters(input): Parameters<LineageRecordInput>) -> String {
        use crate::orchestration::lineage::{LineageEvent, LineageService};
        let meta = input.meta.and_then(|s| serde_json::from_str(&s).ok());
        let event = LineageEvent {
            seq: None,
            session_id: Some(input.session_id),
            timestamp: chrono::Utc::now().timestamp_millis(),
            event_type: input.event_type,
            path: input.path,
            tool: input.tool,
            meta,
        };
        match LineageService::record_event(&self.db, &event) {
            Ok(seq) => format!(r#"{{"seq":{seq}}}"#),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "lineage_events", description = "Query lineage events, optionally filtered by session or type")]
    async fn lineage_events(&self, Parameters(input): Parameters<LineageEventsInput>) -> String {
        use crate::orchestration::lineage::LineageService;
        match LineageService::get_events(&self.db, input.session_id.as_deref(), input.event_type.as_deref(), input.limit.unwrap_or(50)) {
            Ok(events) => serde_json::to_string(&events).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "lineage_timeline", description = "Get a timeline of events for a session")]
    async fn lineage_timeline(&self, Parameters(input): Parameters<LineageTimelineInput>) -> String {
        use crate::orchestration::lineage::LineageService;
        match LineageService::get_timeline(&self.db, &input.session_id) {
            Ok(timeline) => serde_json::to_string(&timeline).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Enforcer ────────────────────────────────────────────────────────────

    #[tool(name = "enforcer_check", description = "Check if a tool call is allowed by enforcer rules and record it")]
    async fn enforcer_check(&self, Parameters(input): Parameters<EnforcerCheckInput>) -> String {
        let mut enforcer = self.enforcer.lock().await;
        let verdict = enforcer.pre_check(&input.tool_name);
        enforcer.post_check(&input.tool_name);
        let action_str = match verdict.action {
            Action::Block => "block",
            Action::Warn => "warn",
            Action::Allow => "allow",
        };
        serde_json::json!({
            "action": action_str,
            "rule": verdict.rule,
            "reason": verdict.reason,
        })
        .to_string()
    }

    #[tool(name = "enforcer_log", description = "View the enforcement log, optionally filtered by session")]
    async fn enforcer_log(&self, Parameters(input): Parameters<EnforcerLogInput>) -> String {
        let log = Enforcer::get_log(&self.db, input.session_id.as_deref(), input.limit.unwrap_or(20));
        serde_json::to_string(&log).unwrap_or_default()
    }

    #[tool(name = "enforcer_rules", description = "List all enforcer rules and their enabled status")]
    async fn enforcer_rules(&self) -> String {
        let enforcer = self.enforcer.lock().await;
        let rules: Vec<serde_json::Value> = enforcer.rules().iter().map(|r| {
            serde_json::json!({
                "name": r.name,
                "description": r.description,
                "action": format!("{:?}", r.action),
                "enabled": r.enabled,
            })
        }).collect();
        serde_json::to_string(&rules).unwrap_or_default()
    }

    #[tool(name = "enforcer_toggle_rule", description = "Enable or disable an enforcer rule")]
    async fn enforcer_toggle_rule(&self, Parameters(input): Parameters<EnforcerRuleToggleInput>) -> String {
        // Persist to DB first so the toggle survives hot-reloads
        {
            let conn = self.db.conn();
            match conn.execute(
                "UPDATE rules SET enabled = ?1 WHERE name = ?2",
                rusqlite::params![input.enabled as i32, input.rule_name],
            ) {
                Ok(0) => return format!(r#"{{"error":"Rule '{}' not found in DB"}}"#, input.rule_name),
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
                Ok(_) => {}
            }
        }
        // Update in-memory cache
        let mut enforcer = self.enforcer.lock().await;
        let in_memory_updated = enforcer.set_rule_enabled(&input.rule_name, input.enabled);
        if !in_memory_updated {
            return format!(
                r#"{{"ok":true,"rule":"{}","enabled":{},"warning":"rule updated in DB but not found in memory cache; restart to sync"}}"#,
                input.rule_name, input.enabled
            );
        }
        format!(r#"{{"ok":true,"rule":"{}","enabled":{}}}"#, input.rule_name, input.enabled)
    }

    // ── Memory ──────────────────────────────────────────────────────────────

    #[tool(name = "hive_memory_store", description = "Store a learning/insight in the persistent memory system")]
    async fn hive_memory_store(&self, Parameters(input): Parameters<MemoryStoreInput>) -> String {
        use crate::orchestration::hive::locks::LockService;
        use crate::orchestration::hive::memory::MemoryService;

        // Advisory lock check — Err blocks write to avoid silent data races
        match LockService::check(&self.db, &input.domain) {
            Err(e) => return format!(r#"{{"error":"lock check failed: {}"}}"#, e),
            Ok(Some(lock)) => {
                let caller_token = input.token.as_deref().unwrap_or("");
                if lock.token != caller_token {
                    return format!(
                        r#"{{"error":"domain '{}' is locked by '{}' until {}"}}"#,
                        input.domain, lock.locked_by, lock.expires_at
                    );
                }
            }
            Ok(None) => {}
        }

        let tags: Vec<&str> = input.tags.as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        match MemoryService::store(&self.db, &input.domain, &input.lesson, input.context.as_deref(), &tags) {
            Ok(id) => format!(r#"{{"id":{id}}}"#),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "hive_memory_recall", description = "Search the memory system for relevant learnings using full-text search")]
    async fn hive_memory_recall(&self, Parameters(input): Parameters<MemoryRecallInput>) -> String {
        use crate::orchestration::hive::memory::MemoryService;
        match MemoryService::recall(&self.db, &input.query, input.limit.unwrap_or(10)) {
            Ok(results) => serde_json::to_string(&results).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "hive_memory_by_domain", description = "Get all stored learnings for a specific domain")]
    async fn hive_memory_by_domain(&self, Parameters(input): Parameters<MemoryByDomainInput>) -> String {
        use crate::orchestration::hive::memory::MemoryService;
        match MemoryService::by_domain(&self.db, &input.domain) {
            Ok(results) => serde_json::to_string(&results).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "hive_claim_domain", description = "Claim exclusive write access to a hive memory domain. Returns a token required for hive_memory_store.")]
    async fn hive_claim_domain(&self, Parameters(input): Parameters<HiveClaimDomainInput>) -> String {
        use crate::orchestration::hive::locks::LockService;
        let ttl = input.ttl_seconds.unwrap_or(self.lock_ttl_seconds);
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

    // ── Patterns ────────────────────────────────────────────────────────────

    #[tool(name = "pattern_analyze", description = "Analyze enforcement log to discover patterns across sessions")]
    fn pattern_analyze(&self) -> String {
        use crate::orchestration::patterns::PatternService;
        match PatternService::analyze_enforcement(&self.db) {
            Ok(patterns) => serde_json::to_string(&patterns).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "pattern_list", description = "List discovered patterns, optionally filtered by category")]
    async fn pattern_list(&self, Parameters(input): Parameters<PatternListInput>) -> String {
        use crate::orchestration::patterns::PatternService;
        match PatternService::list(&self.db, input.category.as_deref()) {
            Ok(patterns) => serde_json::to_string(&patterns).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Ontology ─────────────────────────────────────────────────────────

    #[tool(name = "onto_validate", description = "Validate RDF/OWL syntax. Accepts a file path or inline Turtle content.")]
    async fn onto_validate(&self, Parameters(input): Parameters<OntoValidateInput>) -> String {
        use crate::domain::ontology::OntologyService;
        if input.inline.unwrap_or(false) {
            OntologyService::validate_string(&input.input).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
        } else {
            OntologyService::validate_file(&input.input).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
        }
    }

    #[tool(name = "onto_convert", description = "Convert an RDF file between formats: turtle, ntriples, rdfxml, nquads, trig")]
    async fn onto_convert(&self, Parameters(input): Parameters<OntoConvertInput>) -> String {
        let store = GraphStore::new();
        match store.load_file(&input.path) {
            Ok(_) => {
                match store.serialize(&input.to) {
                    Ok(content) => {
                        if let Some(output) = input.output {
                            match std::fs::write(&output, &content) {
                                Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, output, input.to),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        } else {
                            content
                        }
                    }
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_load", description = "Load an RDF file into the in-memory ontology store for querying")]
    async fn onto_load(&self, Parameters(input): Parameters<OntoLoadInput>) -> String {
        match self.graph.load_file(&input.path) {
            Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"path":"{}"}}"#, count, input.path),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_query", description = "Run a SPARQL query against the loaded ontology store")]
    async fn onto_query(&self, Parameters(input): Parameters<OntoQueryInput>) -> String {
        self.graph.sparql_select(&input.query).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_save", description = "Save the current ontology store to a file")]
    async fn onto_save(&self, Parameters(input): Parameters<OntoSaveInput>) -> String {
        let format = input.format.as_deref().unwrap_or("turtle");
        match self.graph.save_file(&input.path, format) {
            Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, input.path, format),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_stats", description = "Get statistics about the loaded ontology (triple count, classes, properties, individuals)")]
    fn onto_stats(&self) -> String {
        self.graph.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_diff", description = "Compare two ontology files and show added/removed triples")]
    async fn onto_diff(&self, Parameters(input): Parameters<OntoDiffInput>) -> String {
        use crate::domain::ontology::OntologyService;
        let old = match std::fs::read_to_string(&input.old_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.old_path, e),
        };
        let new = match std::fs::read_to_string(&input.new_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.new_path, e),
        };
        OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_lint", description = "Check an ontology for quality issues: missing labels, comments, domains, ranges")]
    async fn onto_lint(&self, Parameters(input): Parameters<OntoLintInput>) -> String {
        use crate::domain::ontology::OntologyService;
        let content = if input.inline.unwrap_or(false) {
            input.input.clone()
        } else {
            match std::fs::read_to_string(&input.input) {
                Ok(c) => c,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
            }
        };
        OntologyService::lint(&content).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_clear", description = "Clear all triples from the in-memory ontology store")]
    fn onto_clear(&self) -> String {
        match self.graph.clear() {
            Ok(_) => r#"{"ok":true,"message":"Store cleared"}"#.to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_pull", description = "Fetch an ontology from a remote URL or SPARQL endpoint and load it into the store")]
    async fn onto_pull(&self, Parameters(input): Parameters<OntoPullInput>) -> String {
        use crate::store::graph::GraphStore;
        if input.sparql.unwrap_or(false) {
            let query = input.query.as_deref().unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
            match GraphStore::fetch_sparql(&input.url, query).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else {
            match GraphStore::fetch_url(&input.url).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        }
    }

    #[tool(name = "onto_push", description = "Push the current ontology store to a remote SPARQL endpoint")]
    async fn onto_push(&self, Parameters(input): Parameters<OntoPushInput>) -> String {
        use crate::store::graph::GraphStore;
        match self.graph.serialize("ntriples") {
            Ok(content) => {
                match GraphStore::push_sparql(&input.endpoint, &content).await {
                    Ok(msg) => format!(r#"{{"ok":true,"message":"{}"}}"#, msg),
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_import", description = "Resolve and load all owl:imports from the currently loaded ontology")]
    async fn onto_import(&self, Parameters(input): Parameters<OntoImportInput>) -> String {
        use crate::store::graph::GraphStore;
        let max_depth = input.max_depth.unwrap_or(3);
        let mut imported = Vec::new();
        let mut to_import: Vec<String> = Vec::new();

        let query = "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
        if let Ok(result) = self.graph.sparql_select(query) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                if let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(uri) = row["import"].as_str() {
                            let uri = uri.trim_matches(|c| c == '<' || c == '>');
                            to_import.push(uri.to_string());
                        }
                    }
                }
            }
        }

        let mut depth = 0;
        while !to_import.is_empty() && depth < max_depth {
            let batch = to_import.drain(..).collect::<Vec<_>>();
            for url in batch {
                if imported.contains(&url) { continue; }
                match GraphStore::fetch_url(&url).await {
                    Ok(content) => {
                        match self.graph.load_turtle(&content, None) {
                            Ok(_count) => {
                                imported.push(url.clone());
                                if let Ok(result) = self.graph.sparql_select(query) {
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                                        if let Some(results) = parsed["results"].as_array() {
                                            for row in results {
                                                if let Some(uri) = row["import"].as_str() {
                                                    let uri = uri.trim_matches(|c| c == '<' || c == '>').to_string();
                                                    if !imported.contains(&uri) && !to_import.contains(&uri) {
                                                        to_import.push(uri);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                        }
                    }
                    Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                }
            }
            depth += 1;
        }

        serde_json::json!({
            "ok": true,
            "imported": imported,
            "total": imported.len(),
            "depth": depth,
        }).to_string()
    }

    #[tool(name = "onto_version", description = "Save a named snapshot of the current ontology store")]
    async fn onto_version(&self, Parameters(input): Parameters<OntoVersionInput>) -> String {
        use crate::domain::ontology::OntologyService;
        OntologyService::save_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_history", description = "List all saved ontology version snapshots")]
    fn onto_history(&self) -> String {
        use crate::domain::ontology::OntologyService;
        OntologyService::list_versions(&self.db)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_rollback", description = "Restore the ontology store to a previously saved version")]
    async fn onto_rollback(&self, Parameters(input): Parameters<OntoRollbackInput>) -> String {
        use crate::domain::ontology::OntologyService;
        OntologyService::rollback_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }
}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for OpenCheirServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("OpenCheir: an MCP meta-server for orchestrating tools, policies, and agents")
    }
}
