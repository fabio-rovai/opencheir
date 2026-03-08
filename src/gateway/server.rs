use std::sync::{Arc, Mutex};

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, Tool},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::orchestration::enforcer::{Action, Enforcer};
use crate::sentinel_core::state::StateDb;

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

// Patterns
#[derive(Deserialize, JsonSchema)]
pub struct PatternListInput {
    /// Optional category filter
    pub category: Option<String>,
}

// ─── OpenCheirServer ────────────────────────────────────────────────────────

/// MCP server that exposes all OpenCheir tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct OpenCheirServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    enforcer: Arc<Mutex<Enforcer>>,
}

impl OpenCheirServer {
    /// Create a new server with all tools wired to domain/orchestration services.
    pub fn new(db: StateDb) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            enforcer: Arc::new(Mutex::new(Enforcer::new())),
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
                "sentinel_core": "active",
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
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_fonts(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_dashes", description = "Check for dash/hyphen inconsistencies in a DOCX document")]
    async fn qa_check_dashes(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_dashes(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_word_counts", description = "Check word counts against limits in a DOCX document")]
    async fn qa_check_word_counts(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_word_counts(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_check_signatures", description = "Check for signature placeholders in a DOCX document")]
    async fn qa_check_signatures(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&QaService::check_signatures(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "qa_full_check", description = "Run all QA checks on a DOCX document (fonts, dashes, smart quotes, word counts, signatures)")]
    async fn qa_full_check(&self, Parameters(input): Parameters<QaFullCheckInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                serde_json::to_string(&QaService::full_check(&doc, None, &[]))
                    .unwrap_or_default()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Search ──────────────────────────────────────────────────────────────

    #[tool(name = "search_documents", description = "Full-text search across indexed document content")]
    async fn search_documents(&self, Parameters(input): Parameters<SearchInput>) -> String {
        use crate::sentinel_core::search::SearchService;
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
        let mut enforcer = self.enforcer.lock().unwrap();
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
    fn enforcer_rules(&self) -> String {
        let enforcer = self.enforcer.lock().unwrap();
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
        let mut enforcer = self.enforcer.lock().unwrap();
        if enforcer.set_rule_enabled(&input.rule_name, input.enabled) {
            format!(r#"{{"ok":true,"rule":"{}","enabled":{}}}"#, input.rule_name, input.enabled)
        } else {
            format!(r#"{{"error":"Rule '{}' not found"}}"#, input.rule_name)
        }
    }

    // ── Memory ──────────────────────────────────────────────────────────────

    #[tool(name = "hive_memory_store", description = "Store a learning/insight in the persistent memory system")]
    async fn hive_memory_store(&self, Parameters(input): Parameters<MemoryStoreInput>) -> String {
        use crate::orchestration::hive::memory::MemoryService;
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
}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for OpenCheirServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("OpenCheir: an MCP meta-server for orchestrating tools, policies, and agents")
    }
}
