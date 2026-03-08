use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, Tool},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::domain::bid::BidService;
use crate::domain::social_value::SocialValueService;
use crate::orchestration::enforcer::{Action, Enforcer};
use crate::sentinel_core::state::StateDb;

// ─── MCP tool input structs ─────────────────────────────────────────────────

// Social Value
#[derive(Deserialize, JsonSchema)]
pub struct SvGetMeasureInput {
    /// TOMs measure reference (e.g. "NT1")
    pub reference: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvSearchInput {
    /// Search query for TOMs measures
    pub query: String,
    /// Optional theme filter
    pub theme: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvGetByThemeInput {
    /// Theme key (e.g. "jobs", "growth", "environment")
    pub theme: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvCommitment {
    /// TOMs measure reference
    pub reference: String,
    /// Number of units committed
    pub units: f64,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvCalculateInput {
    /// List of commitments to calculate social value for
    pub commitments: Vec<SvCommitment>,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvSuggestInput {
    /// Type of contract (e.g. "facilities_management", "construction")
    pub contract_type: String,
    /// Optional contract value
    pub contract_value: Option<String>,
    /// Optional sector
    pub sector: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct SvDraftInput {
    /// TOMs measure references to include
    pub measure_refs: Vec<String>,
    /// Description of the contract
    pub contract_description: String,
}

// Bid Writing
#[derive(Deserialize, JsonSchema)]
pub struct BidListFrameworksInput {
    /// Optional category filter
    pub category: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidGetFrameworkInput {
    /// Framework ID
    pub framework_id: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidCompareInput {
    /// Framework IDs to compare
    pub framework_ids: Vec<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidRecommendInput {
    /// Type of deal
    pub deal_type: String,
    /// Size of deal
    pub deal_size: String,
    /// Level of competition
    pub competition_level: String,
    /// Buyer sophistication level
    pub buyer_sophistication: String,
    /// Type of proposal
    pub proposal_type: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidSearchInput {
    /// Search query across frameworks, templates, guides
    pub query: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidScoreInput {
    /// Framework to score against
    pub framework_id: String,
    /// Criteria scores (criterion name -> score 1-10)
    pub scores: HashMap<String, u32>,
    /// Optional notes
    pub notes: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidWinThemesInput {
    /// Client's industry
    pub client_industry: String,
    /// Client's main challenge
    pub client_challenge: String,
    /// Our key strengths
    pub our_strengths: Vec<String>,
    /// Competitor weaknesses (optional)
    pub competitor_weaknesses: Option<Vec<String>>,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidComplianceReqInput {
    /// Requirement ID
    pub id: String,
    /// Requirement description
    pub description: String,
    /// Whether the requirement is mandatory
    pub mandatory: bool,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidComplianceMatrixInput {
    /// Requirements to build compliance matrix for
    pub requirements: Vec<BidComplianceReqInput>,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidNoBidInput {
    /// Opportunity name
    pub opportunity_name: String,
    /// Deal value
    pub deal_value: Option<String>,
    /// Customer relationship score (1-10)
    pub customer_relationship: u32,
    /// Competitive position score (1-10)
    pub competitive_position: u32,
    /// Solution fit score (1-10)
    pub solution_fit: u32,
    /// Business value score (1-10)
    pub business_value: u32,
    /// Proposal feasibility score (1-10)
    pub proposal_feasibility: u32,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidExecSummaryInput {
    /// Framework ID to structure against
    pub framework_id: String,
    /// Client name
    pub client_name: String,
    /// Client's main challenge
    pub client_challenge: String,
    /// Our proposed solution
    pub our_solution: String,
    /// Key benefit
    pub key_benefit: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidProposalOutlineInput {
    /// Framework ID
    pub framework_id: String,
    /// Client name
    pub client_name: String,
    /// Project name
    pub project_name: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidGetSectionInput {
    /// Section template ID
    pub section_id: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidGetIndustryInput {
    /// Industry guide ID
    pub industry_id: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct BidGetPersuasionInput {
    /// Persuasion technique ID
    pub technique_id: String,
}

// Tender
#[derive(Deserialize, JsonSchema)]
pub struct DocPathInput {
    /// Path to the DOCX document
    pub path: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct TenderReadAnswerInput {
    /// Path to the DOCX document
    pub path: String,
    /// Table index
    pub table_index: usize,
    /// Row index
    pub row_index: usize,
    /// Cell index
    pub cell_index: usize,
}
#[derive(Deserialize, JsonSchema)]
pub struct TenderSubmissionInput {
    /// Folder containing submission files
    pub folder: String,
    /// Expected files as [filename, description] pairs
    pub expected_files: Vec<[String; 2]>,
}

// QA
#[derive(Deserialize, JsonSchema)]
pub struct QaFolderInput {
    /// Folder path to check filenames in
    pub folder: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct QaSensitiveInput {
    /// Path to the DOCX document
    pub path: String,
    /// Sensitive key-value pairs to check for (if empty, reads from company DB)
    pub sensitive_values: Option<Vec<[String; 2]>>,
}
#[derive(Deserialize, JsonSchema)]
pub struct QaFullCheckInput {
    /// Path to the DOCX document
    pub path: String,
    /// Optional folder to check filenames in
    pub folder: Option<String>,
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
    /// Knowledge domain (e.g. "rust", "tender-writing")
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

// ─── SentinelServer ─────────────────────────────────────────────────────────

/// MCP server that exposes all Sentinel tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct SentinelServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    sv: Arc<SocialValueService>,
    bid: Arc<BidService>,
    enforcer: Arc<Mutex<Enforcer>>,
}

impl SentinelServer {
    /// Create a new server with all tools wired to domain/orchestration services.
    pub fn new(db: StateDb) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            sv: Arc::new(SocialValueService::new()),
            bid: Arc::new(BidService::new()),
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
impl SentinelServer {

    // ── Sentinel ────────────────────────────────────────────────────────────

    #[tool(name = "sentinel_status", description = "Returns a high-level system health summary for the Sentinel platform")]
    fn sentinel_status(&self) -> String {
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

    #[tool(name = "sentinel_health", description = "Returns detailed health information for each Sentinel component")]
    fn sentinel_health(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        serde_json::json!({
            "components": {
                "gateway": { "status": "ok", "tools": tool_count },
                "state_db": { "status": "available" },
                "documents": { "status": "available" },
                "search": { "status": "available" },
                "tender": { "status": "available" },
                "qa": { "status": "available" },
                "social_value": { "status": "available" },
                "bid": { "status": "available" },
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

    // ── Social Value ────────────────────────────────────────────────────────

    #[tool(name = "list_toms_themes", description = "List all Social Value TOMs themes with measure counts")]
    fn list_toms_themes(&self) -> String {
        serde_json::to_string(&self.sv.list_themes()).unwrap_or_default()
    }

    #[tool(name = "get_toms_by_theme", description = "Get all TOMs measures for a given theme")]
    async fn get_toms_by_theme(&self, Parameters(input): Parameters<SvGetByThemeInput>) -> String {
        serde_json::to_string(&self.sv.get_by_theme(&input.theme)).unwrap_or_default()
    }

    #[tool(name = "get_toms_measure", description = "Get details of a specific TOMs measure by reference")]
    async fn get_toms_measure(&self, Parameters(input): Parameters<SvGetMeasureInput>) -> String {
        match self.sv.get_measure(&input.reference) {
            Some(m) => serde_json::to_string(m).unwrap_or_default(),
            None => r#"{"error":"Measure not found"}"#.to_string(),
        }
    }

    #[tool(name = "search_toms", description = "Search TOMs measures by keyword, optionally filtered by theme")]
    async fn search_toms(&self, Parameters(input): Parameters<SvSearchInput>) -> String {
        let results = self.sv.search(&input.query, input.theme.as_deref());
        serde_json::to_string(&results).unwrap_or_default()
    }

    #[tool(name = "calculate_social_value", description = "Calculate the total social value in GBP for a set of TOMs commitments")]
    async fn calculate_social_value(&self, Parameters(input): Parameters<SvCalculateInput>) -> String {
        let commitments: Vec<(&str, f64)> = input.commitments.iter()
            .map(|c| (c.reference.as_str(), c.units))
            .collect();
        serde_json::to_string(&self.sv.calculate(&commitments)).unwrap_or_default()
    }

    #[tool(name = "suggest_social_value", description = "Suggest relevant TOMs measures based on contract type")]
    async fn suggest_social_value(&self, Parameters(input): Parameters<SvSuggestInput>) -> String {
        let suggestions = self.sv.suggest(
            &input.contract_type,
            input.contract_value.as_deref(),
            input.sector.as_deref(),
        );
        serde_json::to_string(&suggestions).unwrap_or_default()
    }

    #[tool(name = "draft_social_value_response", description = "Draft a social value response for specified TOMs measures")]
    async fn draft_social_value_response(&self, Parameters(input): Parameters<SvDraftInput>) -> String {
        let refs: Vec<&str> = input.measure_refs.iter().map(|s| s.as_str()).collect();
        serde_json::to_string(&self.sv.draft_response(&refs, &input.contract_description))
            .unwrap_or_default()
    }

    // ── Bid Writing ─────────────────────────────────────────────────────────

    #[tool(name = "list_frameworks", description = "List available bid writing frameworks, optionally filtered by category")]
    async fn list_frameworks(&self, Parameters(input): Parameters<BidListFrameworksInput>) -> String {
        serde_json::to_string(&self.bid.list_frameworks(input.category.as_deref()))
            .unwrap_or_default()
    }

    #[tool(name = "get_framework", description = "Get full details of a bid writing framework")]
    async fn get_framework(&self, Parameters(input): Parameters<BidGetFrameworkInput>) -> String {
        match self.bid.get_framework(&input.framework_id) {
            Some(f) => serde_json::to_string(f).unwrap_or_default(),
            None => r#"{"error":"Framework not found"}"#.to_string(),
        }
    }

    #[tool(name = "compare_frameworks", description = "Compare multiple bid writing frameworks side by side")]
    async fn compare_frameworks(&self, Parameters(input): Parameters<BidCompareInput>) -> String {
        let ids: Vec<&str> = input.framework_ids.iter().map(|s| s.as_str()).collect();
        serde_json::to_string(&self.bid.compare_frameworks(&ids)).unwrap_or_default()
    }

    #[tool(name = "recommend_framework", description = "Get framework recommendations based on deal characteristics")]
    async fn recommend_framework(&self, Parameters(input): Parameters<BidRecommendInput>) -> String {
        let recs = self.bid.recommend_framework(
            &input.deal_type, &input.deal_size, &input.competition_level,
            &input.buyer_sophistication, &input.proposal_type,
        );
        serde_json::to_string(&recs).unwrap_or_default()
    }

    #[tool(name = "search_resources", description = "Search across all bid writing resources (frameworks, templates, guides)")]
    async fn search_resources(&self, Parameters(input): Parameters<BidSearchInput>) -> String {
        serde_json::to_string(&self.bid.search_resources(&input.query)).unwrap_or_default()
    }

    #[tool(name = "score_proposal", description = "Score a proposal against a framework's criteria")]
    async fn score_proposal(&self, Parameters(input): Parameters<BidScoreInput>) -> String {
        match self.bid.score_proposal(&input.framework_id, &input.scores, input.notes.as_deref()) {
            Some(s) => serde_json::to_string(&s).unwrap_or_default(),
            None => r#"{"error":"Framework not found"}"#.to_string(),
        }
    }

    #[tool(name = "generate_win_themes", description = "Generate win themes based on client challenges and our strengths")]
    async fn generate_win_themes(&self, Parameters(input): Parameters<BidWinThemesInput>) -> String {
        let strengths: Vec<&str> = input.our_strengths.iter().map(|s| s.as_str()).collect();
        let weaknesses: Option<Vec<&str>> = input.competitor_weaknesses.as_ref()
            .map(|w| w.iter().map(|s| s.as_str()).collect());
        let themes = self.bid.generate_win_themes(
            &input.client_industry, &input.client_challenge,
            &strengths, weaknesses.as_deref(),
        );
        serde_json::to_string(&themes).unwrap_or_default()
    }

    #[tool(name = "generate_compliance_matrix", description = "Generate a compliance matrix from a list of requirements")]
    async fn generate_compliance_matrix(&self, Parameters(input): Parameters<BidComplianceMatrixInput>) -> String {
        use crate::domain::bid::ComplianceRequirement;
        let reqs: Vec<ComplianceRequirement> = input.requirements.into_iter()
            .map(|r| ComplianceRequirement { id: r.id, description: r.description, mandatory: r.mandatory })
            .collect();
        serde_json::to_string(&self.bid.generate_compliance_matrix(&reqs)).unwrap_or_default()
    }

    #[tool(name = "bid_no_bid_analysis", description = "Run a bid/no-bid analysis with weighted scoring")]
    async fn bid_no_bid_analysis(&self, Parameters(input): Parameters<BidNoBidInput>) -> String {
        let result = self.bid.bid_no_bid_analysis(
            &input.opportunity_name, input.deal_value.as_deref(),
            input.customer_relationship, input.competitive_position,
            input.solution_fit, input.business_value, input.proposal_feasibility,
        );
        serde_json::to_string(&result).unwrap_or_default()
    }

    #[tool(name = "generate_executive_summary", description = "Generate a structured executive summary using a framework")]
    async fn generate_executive_summary(&self, Parameters(input): Parameters<BidExecSummaryInput>) -> String {
        serde_json::to_string(&self.bid.generate_executive_summary(
            &input.framework_id, &input.client_name, &input.client_challenge,
            &input.our_solution, &input.key_benefit,
        ))
        .unwrap_or_default()
    }

    #[tool(name = "generate_proposal_outline", description = "Generate a proposal outline based on a framework")]
    async fn generate_proposal_outline(&self, Parameters(input): Parameters<BidProposalOutlineInput>) -> String {
        match self.bid.generate_proposal_outline(&input.framework_id, &input.client_name, &input.project_name) {
            Some(o) => serde_json::to_string(&o).unwrap_or_default(),
            None => r#"{"error":"Framework not found"}"#.to_string(),
        }
    }

    #[tool(name = "get_section_template", description = "Get a bid writing section template by ID")]
    async fn get_section_template(&self, Parameters(input): Parameters<BidGetSectionInput>) -> String {
        match self.bid.get_section_template(&input.section_id) {
            Some(t) => serde_json::to_string(t).unwrap_or_default(),
            None => r#"{"error":"Section template not found"}"#.to_string(),
        }
    }

    #[tool(name = "get_industry_guide", description = "Get a bid writing industry guide by ID")]
    async fn get_industry_guide(&self, Parameters(input): Parameters<BidGetIndustryInput>) -> String {
        match self.bid.get_industry_guide(&input.industry_id) {
            Some(g) => serde_json::to_string(g).unwrap_or_default(),
            None => r#"{"error":"Industry guide not found"}"#.to_string(),
        }
    }

    #[tool(name = "get_persuasion_technique", description = "Get a persuasion technique by ID")]
    async fn get_persuasion_technique(&self, Parameters(input): Parameters<BidGetPersuasionInput>) -> String {
        match self.bid.get_persuasion_technique(&input.technique_id) {
            Some(t) => serde_json::to_string(t).unwrap_or_default(),
            None => r#"{"error":"Persuasion technique not found"}"#.to_string(),
        }
    }

    // ── Tender ──────────────────────────────────────────────────────────────

    #[tool(name = "parse_tender", description = "Parse a DOCX tender document and extract questions, word limits, and structure")]
    async fn parse_tender(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::tender::TenderService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => serde_json::to_string(&TenderService::parse_tender(&doc)).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "read_answer", description = "Read the content of a specific table cell from a tender document")]
    async fn read_answer(&self, Parameters(input): Parameters<TenderReadAnswerInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::tender::TenderService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                match TenderService::read_answer(&doc, input.table_index, input.row_index, input.cell_index) {
                    Some(r) => serde_json::to_string(&r).unwrap_or_default(),
                    None => r#"{"error":"Cell not found"}"#.to_string(),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "check_compliance", description = "Check a tender document for compliance issues (unanswered questions, missing content)")]
    async fn check_compliance(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::tender::TenderService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                let structure = TenderService::parse_tender(&doc);
                serde_json::to_string(&TenderService::check_compliance(&structure)).unwrap_or_default()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "check_pass_fail_questions", description = "Check pass/fail questions in a tender document")]
    async fn check_pass_fail_questions(&self, Parameters(input): Parameters<DocPathInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::tender::TenderService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                let structure = TenderService::parse_tender(&doc);
                serde_json::to_string(&TenderService::check_pass_fail(&structure)).unwrap_or_default()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "check_submission_files", description = "Check that all expected submission files are present in a folder")]
    async fn check_submission_files(&self, Parameters(input): Parameters<TenderSubmissionInput>) -> String {
        use crate::domain::tender::TenderService;
        let expected: Vec<(String, String)> = input.expected_files.into_iter()
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect();
        serde_json::to_string(&TenderService::check_submission_files(&input.folder, &expected))
            .unwrap_or_default()
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

    #[tool(name = "qa_check_sensitive_info", description = "Check for sensitive company information in a DOCX document")]
    async fn qa_check_sensitive_info(&self, Parameters(input): Parameters<QaSensitiveInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                let sensitive: Vec<(String, String)> = match input.sensitive_values {
                    Some(vals) => vals.into_iter().map(|pair| (pair[0].clone(), pair[1].clone())).collect(),
                    None => {
                        // Read sensitive values from company table
                        let conn = self.db.conn();
                        let mut stmt = conn.prepare(
                            "SELECT key, value FROM company WHERE sensitive = 1"
                        ).unwrap_or_else(|_| conn.prepare("SELECT '', ''").unwrap());
                        stmt.query_map([], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        })
                        .map(|rows| rows.filter_map(|r| r.ok()).collect())
                        .unwrap_or_default()
                    }
                };
                serde_json::to_string(&QaService::check_sensitive_info(&doc, &sensitive))
                    .unwrap_or_default()
            }
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

    #[tool(name = "qa_check_filenames", description = "Check filename conventions in a submission folder")]
    async fn qa_check_filenames(&self, Parameters(input): Parameters<QaFolderInput>) -> String {
        use crate::domain::qa::QaService;
        serde_json::to_string(&QaService::check_filenames(&input.folder)).unwrap_or_default()
    }

    #[tool(name = "qa_full_check", description = "Run all QA checks on a DOCX document")]
    async fn qa_full_check(&self, Parameters(input): Parameters<QaFullCheckInput>) -> String {
        use crate::sentinel_core::documents::DocumentService;
        use crate::domain::qa::QaService;
        match DocumentService::parse(&input.path) {
            Ok(doc) => {
                // Get sensitive values from DB for the sensitive info check
                let sensitive: Vec<(String, String)> = {
                    let conn = self.db.conn();
                    conn.prepare("SELECT key, value FROM company WHERE sensitive = 1")
                        .and_then(|mut stmt| {
                            stmt.query_map([], |row| {
                                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                            })
                            .map(|rows| rows.filter_map(|r| r.ok()).collect())
                        })
                        .unwrap_or_default()
                };
                serde_json::to_string(&QaService::full_check(&doc, input.folder.as_deref(), &sensitive))
                    .unwrap_or_default()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Search ──────────────────────────────────────────────────────────────

    #[tool(name = "search_tenders", description = "Full-text search across indexed tender content")]
    async fn search_tenders(&self, Parameters(input): Parameters<SearchInput>) -> String {
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
impl ServerHandler for SentinelServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Sentinel: an MCP meta-server for orchestrating tools, policies, and agents")
    }
}
