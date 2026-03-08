use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    pub id: String,
    pub name: String,
    pub category: String,
    pub overview: String,
    pub template: String,
    pub checklist: Vec<String>,
    pub example: String,
    #[serde(rename = "scoringCriteria")]
    pub scoring_criteria: Vec<String>,
    #[serde(rename = "bestPractices")]
    pub best_practices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryGuide {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersuasionTechnique {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSummary {
    pub id: String,
    pub name: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkRecommendation {
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub resource_type: String,
    pub name: String,
    pub id: String,
    pub match_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WinTheme {
    pub index: usize,
    pub strength: String,
    pub theme_statement: String,
    pub feature: String,
    pub benefit_prompt: String,
    pub proof_prompt: String,
    pub discriminator_prompt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub description: String,
    pub mandatory: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceRow {
    pub req_id: String,
    pub description: String,
    pub mandatory: bool,
    pub proposal_section: String,
    pub compliance: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceMatrix {
    pub rows: Vec<ComplianceRow>,
    pub total_requirements: usize,
    pub mandatory_count: usize,
    pub desired_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalScore {
    pub framework_name: String,
    pub scores: Vec<(String, u32)>,
    pub average: f64,
    pub assessment: String,
    pub weak_areas: Vec<(String, u32)>,
    pub scoring_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BidNoBidResult {
    pub opportunity_name: String,
    pub deal_value: Option<String>,
    pub weighted_score: f64,
    pub recommendation: String,
    pub win_probability: String,
    pub scores: HashMap<String, u32>,
    pub weak_areas: Vec<(String, u32)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutiveSummary {
    pub framework_name: String,
    pub client_name: String,
    pub sections: Vec<ExecutiveSummarySection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutiveSummarySection {
    pub heading: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalOutline {
    pub client_name: String,
    pub project_name: String,
    pub framework_name: String,
    pub template: String,
    pub checklist: Vec<String>,
    pub supplementary_sections: Vec<FrameworkSummary>,
}

// ── Root JSON shape ──

#[derive(Debug, Deserialize)]
struct BidData {
    frameworks: Vec<Framework>,
    section_templates: Vec<SectionTemplate>,
    industry_guides: Vec<IndustryGuide>,
    persuasion_techniques: Vec<PersuasionTechnique>,
}

// ── Service ──

pub struct BidService {
    frameworks: Vec<Framework>,
    section_templates: Vec<SectionTemplate>,
    industry_guides: Vec<IndustryGuide>,
    persuasion_techniques: Vec<PersuasionTechnique>,
}

impl BidService {
    pub fn new() -> Self {
        let json_str = include_str!("../../data/bid_frameworks.json");
        let data: BidData =
            serde_json::from_str(json_str).expect("Failed to parse bid_frameworks.json");
        Self {
            frameworks: data.frameworks,
            section_templates: data.section_templates,
            industry_guides: data.industry_guides,
            persuasion_techniques: data.persuasion_techniques,
        }
    }

    // ── Framework lookups ──

    /// List all frameworks, optionally filtered by category (case-insensitive substring match).
    pub fn list_frameworks(&self, category: Option<&str>) -> Vec<FrameworkSummary> {
        self.frameworks
            .iter()
            .filter(|fw| {
                category.map_or(true, |c| {
                    fw.category.to_lowercase().contains(&c.to_lowercase())
                })
            })
            .map(|fw| FrameworkSummary {
                id: fw.id.clone(),
                name: fw.name.clone(),
                category: fw.category.clone(),
            })
            .collect()
    }

    /// Get complete details for a specific framework by ID.
    pub fn get_framework(&self, framework_id: &str) -> Option<&Framework> {
        self.frameworks.iter().find(|f| f.id == framework_id)
    }

    /// Compare two or more frameworks side by side.
    pub fn compare_frameworks(&self, framework_ids: &[&str]) -> Vec<&Framework> {
        framework_ids
            .iter()
            .filter_map(|id| self.get_framework(id))
            .collect()
    }

    /// Return all known categories (deduplicated).
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .frameworks
            .iter()
            .map(|fw| fw.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    // ── Section templates ──

    /// Get a section template by ID.
    pub fn get_section_template(&self, section_id: &str) -> Option<&SectionTemplate> {
        self.section_templates.iter().find(|s| s.id == section_id)
    }

    /// List all section templates.
    pub fn list_section_templates(&self) -> &[SectionTemplate] {
        &self.section_templates
    }

    // ── Industry guides ──

    /// Get an industry guide by ID.
    pub fn get_industry_guide(&self, industry_id: &str) -> Option<&IndustryGuide> {
        self.industry_guides.iter().find(|g| g.id == industry_id)
    }

    /// List all industry guides.
    pub fn list_industry_guides(&self) -> &[IndustryGuide] {
        &self.industry_guides
    }

    // ── Persuasion techniques ──

    /// Get a persuasion technique by ID.
    pub fn get_persuasion_technique(&self, technique_id: &str) -> Option<&PersuasionTechnique> {
        self.persuasion_techniques
            .iter()
            .find(|t| t.id == technique_id)
    }

    /// List all persuasion techniques.
    pub fn list_persuasion_techniques(&self) -> &[PersuasionTechnique] {
        &self.persuasion_techniques
    }

    // ── Recommend framework ──

    /// Recommend frameworks based on bid context.
    pub fn recommend_framework(
        &self,
        deal_type: &str,
        deal_size: &str,
        competition_level: &str,
        buyer_sophistication: &str,
        proposal_type: &str,
    ) -> Vec<FrameworkRecommendation> {
        let mut recs: Vec<FrameworkRecommendation> = Vec::new();

        let push = |recs: &mut Vec<FrameworkRecommendation>, text: &str| {
            recs.push(FrameworkRecommendation {
                recommendation: text.to_string(),
            });
        };

        // Primary framework recommendations by deal type
        match deal_type {
            "government" => {
                push(
                    &mut recs,
                    "Shipley Method — Industry standard for government proposals",
                );
                push(
                    &mut recs,
                    "APMP Body of Knowledge — Professional proposal management",
                );
                push(
                    &mut recs,
                    "Compliance Matrix Method — Ensure 100% compliance",
                );
                push(
                    &mut recs,
                    "Color Team Review Process — Quality assurance through structured reviews",
                );
                if competition_level == "high" {
                    push(
                        &mut recs,
                        "Win Theme Development — Differentiate in competitive environments",
                    );
                    push(
                        &mut recs,
                        "Capture Planning — Pre-RFP positioning is critical",
                    );
                }
            }
            "enterprise" => {
                push(
                    &mut recs,
                    "MEDDPICC — Comprehensive deal qualification",
                );
                push(
                    &mut recs,
                    "Miller Heiman Strategic Selling — Multi-stakeholder management",
                );
                push(
                    &mut recs,
                    "Challenger Sale — Lead with insight, not features",
                );
                if buyer_sophistication == "high" {
                    push(
                        &mut recs,
                        "Value Selling Framework — Quantify business value",
                    );
                }
            }
            "smb" => {
                push(
                    &mut recs,
                    "SNAP Selling — Simple, focused proposals for busy buyers",
                );
                push(
                    &mut recs,
                    "Baseline Selling — Practical, step-by-step approach",
                );
                push(&mut recs, "1-2-3-4 Formula — Concise and impactful");
            }
            "startup" => {
                push(
                    &mut recs,
                    "Gap Selling — Focus on the gap between current and future state",
                );
                push(&mut recs, "CHAMP Framework — Challenge-led approach");
                push(
                    &mut recs,
                    "Blue Ocean Strategy — Position as category creator",
                );
            }
            "nonprofit" => {
                push(
                    &mut recs,
                    "RAIN Selling — Relationship-focused approach",
                );
                push(
                    &mut recs,
                    "PASTOR Framework — Story-driven persuasion",
                );
                push(
                    &mut recs,
                    "Value Proposition Canvas — Align value to mission",
                );
            }
            "international" => {
                push(
                    &mut recs,
                    "Shipley Method — Globally recognised proposal process",
                );
                push(
                    &mut recs,
                    "APMP Body of Knowledge — International certification standard",
                );
            }
            _ => {}
        }

        // Copywriting framework recommendations by proposal type
        match proposal_type {
            "pitch" | "unsolicited" => {
                push(&mut recs, "AIDA — Attention-grabbing structure");
                push(
                    &mut recs,
                    "PAS (Problem-Agitate-Solution) — Create urgency",
                );
                push(&mut recs, "4Ps (Promise-Picture-Proof-Push) — Bold and visual");
            }
            "rfp-response" => {
                push(
                    &mut recs,
                    "Executive Summary Framework — Critical for RFP responses",
                );
                push(
                    &mut recs,
                    "FAB (Features-Advantages-Benefits) — Translate features to value",
                );
            }
            _ => {}
        }

        // Persuasion technique recommendations
        if competition_level == "high" {
            push(
                &mut recs,
                "Social Proof — Leverage client references and case studies",
            );
            push(
                &mut recs,
                "Contrast Principle — Differentiate visually and substantively",
            );
        }

        if buyer_sophistication == "high" {
            push(
                &mut recs,
                "Cognitive Ease — Make complex proposals easy to evaluate",
            );
            push(
                &mut recs,
                "Authority Principle — Leverage certifications and recognition",
            );
        }

        // Strategy frameworks for large deals
        if deal_size == "large" || deal_size == "enterprise" {
            push(
                &mut recs,
                "Risk Management Framework — Demonstrate maturity",
            );
            push(
                &mut recs,
                "Pricing Strategy Framework — Optimize value positioning",
            );
            push(&mut recs, "SWOT Analysis — Strategic bid planning");
        }

        recs
    }

    // ── Search ──

    /// Search across all bid writing resources by keyword (case-insensitive).
    pub fn search_resources(&self, query: &str) -> Vec<SearchResult> {
        let q = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();

        for fw in &self.frameworks {
            if fw.name.to_lowercase().contains(&q)
                || fw.overview.to_lowercase().contains(&q)
                || fw.category.to_lowercase().contains(&q)
            {
                let match_line = fw
                    .overview
                    .lines()
                    .find(|line| line.to_lowercase().contains(&q))
                    .unwrap_or(&fw.name);
                let truncated: String = match_line.trim().chars().take(120).collect();
                results.push(SearchResult {
                    resource_type: "Framework".to_string(),
                    name: fw.name.clone(),
                    id: fw.id.clone(),
                    match_text: truncated,
                });
            }
        }

        for section in &self.section_templates {
            if section.name.to_lowercase().contains(&q)
                || section.content.to_lowercase().contains(&q)
            {
                let match_line = section
                    .content
                    .lines()
                    .find(|line| line.to_lowercase().contains(&q))
                    .unwrap_or(&section.name);
                let truncated: String = match_line.trim().chars().take(120).collect();
                results.push(SearchResult {
                    resource_type: "Section Template".to_string(),
                    name: section.name.clone(),
                    id: section.id.clone(),
                    match_text: truncated,
                });
            }
        }

        for guide in &self.industry_guides {
            if guide.name.to_lowercase().contains(&q)
                || guide.content.to_lowercase().contains(&q)
            {
                let match_line = guide
                    .content
                    .lines()
                    .find(|line| line.to_lowercase().contains(&q))
                    .unwrap_or(&guide.name);
                let truncated: String = match_line.trim().chars().take(120).collect();
                results.push(SearchResult {
                    resource_type: "Industry Guide".to_string(),
                    name: guide.name.clone(),
                    id: guide.id.clone(),
                    match_text: truncated,
                });
            }
        }

        for tech in &self.persuasion_techniques {
            if tech.name.to_lowercase().contains(&q)
                || tech.content.to_lowercase().contains(&q)
            {
                let match_line = tech
                    .content
                    .lines()
                    .find(|line| line.to_lowercase().contains(&q))
                    .unwrap_or(&tech.name);
                let truncated: String = match_line.trim().chars().take(120).collect();
                results.push(SearchResult {
                    resource_type: "Persuasion Technique".to_string(),
                    name: tech.name.clone(),
                    id: tech.id.clone(),
                    match_text: truncated,
                });
            }
        }

        results
    }

    // ── Score proposal ──

    /// Score a proposal against a framework's criteria.
    /// `scores` maps criterion name to a 1-10 score.
    pub fn score_proposal(
        &self,
        framework_id: &str,
        scores: &HashMap<String, u32>,
        _notes: Option<&str>,
    ) -> Option<ProposalScore> {
        let fw = self.get_framework(framework_id)?;

        let values: Vec<u32> = scores.values().copied().collect();
        let avg = if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<u32>() as f64 / values.len() as f64
        };

        let assessment = if avg >= 8.0 {
            "Strong proposal. Ready for submission with minor refinements."
        } else if avg >= 6.0 {
            "Good foundation but needs improvement in key areas."
        } else if avg >= 4.0 {
            "Significant gaps. Recommend major revision before submission."
        } else {
            "Not ready. Fundamental rework needed."
        };

        let weak_areas: Vec<(String, u32)> = scores
            .iter()
            .filter(|(_, s)| **s < 7)
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        let score_vec: Vec<(String, u32)> = scores
            .iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();

        Some(ProposalScore {
            framework_name: fw.name.clone(),
            scores: score_vec,
            average: (avg * 10.0).round() / 10.0,
            assessment: assessment.to_string(),
            weak_areas,
            scoring_criteria: fw.scoring_criteria.clone(),
        })
    }

    // ── Generate win themes ──

    /// Generate win theme suggestions from strengths and context.
    pub fn generate_win_themes(
        &self,
        _client_industry: &str,
        client_challenge: &str,
        our_strengths: &[&str],
        _competitor_weaknesses: Option<&[&str]>,
    ) -> Vec<WinTheme> {
        let mut themes = Vec::new();
        for (i, strength) in our_strengths.iter().enumerate().take(5) {
            themes.push(WinTheme {
                index: i + 1,
                strength: strength.to_string(),
                theme_statement: format!(
                    "Our {} directly addresses {}, delivering [quantified benefit] as proven by [evidence].",
                    strength.to_lowercase(),
                    client_challenge.to_lowercase()
                ),
                feature: strength.to_string(),
                benefit_prompt: format!(
                    "[Quantify how this helps with: {}]",
                    client_challenge
                ),
                proof_prompt: "[Insert case study or data point]".to_string(),
                discriminator_prompt: "[What makes this unique vs. competitors]".to_string(),
            });
        }
        themes
    }

    // ── Generate compliance matrix ──

    /// Generate a compliance matrix from a list of requirements.
    pub fn generate_compliance_matrix(
        &self,
        requirements: &[ComplianceRequirement],
    ) -> ComplianceMatrix {
        let mandatory_count = requirements.iter().filter(|r| r.mandatory).count();
        let rows: Vec<ComplianceRow> = requirements
            .iter()
            .map(|req| ComplianceRow {
                req_id: req.id.clone(),
                description: req.description.clone(),
                mandatory: req.mandatory,
                proposal_section: "[Section TBD]".to_string(),
                compliance: "[TBD]".to_string(),
                notes: String::new(),
            })
            .collect();

        ComplianceMatrix {
            total_requirements: requirements.len(),
            mandatory_count,
            desired_count: requirements.len() - mandatory_count,
            rows,
        }
    }

    // ── Bid/No-Bid analysis ──

    /// Run a weighted bid/no-bid decision analysis.
    pub fn bid_no_bid_analysis(
        &self,
        opportunity_name: &str,
        deal_value: Option<&str>,
        customer_relationship: u32,
        competitive_position: u32,
        solution_fit: u32,
        business_value: u32,
        proposal_feasibility: u32,
    ) -> BidNoBidResult {
        let weighted_score = (customer_relationship as f64 * 0.25
            + competitive_position as f64 * 0.25
            + solution_fit as f64 * 0.20
            + business_value as f64 * 0.15
            + proposal_feasibility as f64 * 0.15)
            * 10.0;

        let recommendation = if weighted_score >= 80.0 {
            "STRONG BID - Proceed with full commitment and resources"
        } else if weighted_score >= 60.0 {
            "CONDITIONAL BID - Proceed if key gaps can be addressed"
        } else if weighted_score >= 40.0 {
            "CAUTIOUS - Only bid if strategic value justifies the investment"
        } else {
            "NO BID - Redirect resources to stronger opportunities"
        };

        let win_probability = if weighted_score >= 80.0 {
            "High (60-80%): Strong position across all dimensions"
        } else if weighted_score >= 60.0 {
            "Medium (30-50%): Competitive but with gaps to address"
        } else if weighted_score >= 40.0 {
            "Low (15-30%): Uphill battle"
        } else {
            "Very Low (<15%): Resources better spent elsewhere"
        };

        let mut scores_map = HashMap::new();
        scores_map.insert("Customer Relationship".to_string(), customer_relationship);
        scores_map.insert("Competitive Position".to_string(), competitive_position);
        scores_map.insert("Solution Fit".to_string(), solution_fit);
        scores_map.insert("Business Value".to_string(), business_value);
        scores_map.insert("Proposal Feasibility".to_string(), proposal_feasibility);

        let weak_areas: Vec<(String, u32)> = scores_map
            .iter()
            .filter(|(_, s)| **s < 6)
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        BidNoBidResult {
            opportunity_name: opportunity_name.to_string(),
            deal_value: deal_value.map(|v| v.to_string()),
            weighted_score: (weighted_score * 10.0).round() / 10.0,
            recommendation: recommendation.to_string(),
            win_probability: win_probability.to_string(),
            scores: scores_map,
            weak_areas,
        }
    }

    // ── Generate executive summary ──

    /// Generate an executive summary structure using a specific framework.
    pub fn generate_executive_summary(
        &self,
        framework_id: &str,
        client_name: &str,
        client_challenge: &str,
        our_solution: &str,
        key_benefit: &str,
    ) -> ExecutiveSummary {
        let fw_name = self
            .get_framework(framework_id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| framework_id.to_string());

        let sections = match framework_id {
            "aida" => vec![
                ExecutiveSummarySection {
                    heading: "ATTENTION".to_string(),
                    content: format!(
                        "[Startling statistic or bold claim about {}]",
                        client_challenge
                    ),
                },
                ExecutiveSummarySection {
                    heading: "INTEREST".to_string(),
                    content: format!(
                        "{} faces a critical challenge: {}. [Expand with data, industry context, and relevance to their specific situation.]",
                        client_name, client_challenge
                    ),
                },
                ExecutiveSummarySection {
                    heading: "DESIRE".to_string(),
                    content: format!(
                        "Imagine a world where {} is resolved. {} delivers {}. [Add case studies, testimonials, and vivid future-state description.]",
                        client_challenge, our_solution, key_benefit
                    ),
                },
                ExecutiveSummarySection {
                    heading: "ACTION".to_string(),
                    content: "[Clear next step with timeline and contact. Risk-free option if possible.]".to_string(),
                },
            ],
            "pas" => vec![
                ExecutiveSummarySection {
                    heading: "PROBLEM".to_string(),
                    content: format!(
                        "{} is experiencing {}. [Quantify the problem with data.]",
                        client_name, client_challenge
                    ),
                },
                ExecutiveSummarySection {
                    heading: "AGITATE".to_string(),
                    content: "[What happens if this continues? Financial impact, competitive risk, personal impact on stakeholders. Build urgency across multiple dimensions.]".to_string(),
                },
                ExecutiveSummarySection {
                    heading: "SOLUTION".to_string(),
                    content: format!(
                        "{} addresses this challenge directly, delivering {}. [Proof points, methodology, and next steps.]",
                        our_solution, key_benefit
                    ),
                },
            ],
            "bab" => vec![
                ExecutiveSummarySection {
                    heading: "BEFORE (Today)".to_string(),
                    content: format!(
                        "{}'s current reality: {}. [Vivid description of current pain, metrics, and daily impact.]",
                        client_name, client_challenge
                    ),
                },
                ExecutiveSummarySection {
                    heading: "AFTER (With Our Solution)".to_string(),
                    content: format!(
                        "[Paint a vivid picture of life after {} is implemented. Specific improvements: {}. Day-in-the-life scenarios.]",
                        our_solution, key_benefit
                    ),
                },
                ExecutiveSummarySection {
                    heading: "BRIDGE (How We Get There)".to_string(),
                    content: format!(
                        "{} is the bridge from before to after. [Implementation plan, proof it works, and why only you can deliver this.]",
                        our_solution
                    ),
                },
            ],
            _ => vec![
                ExecutiveSummarySection {
                    heading: "Client Understanding".to_string(),
                    content: format!(
                        "{} faces: {}\n[Demonstrate deep understanding of their situation.]",
                        client_name, client_challenge
                    ),
                },
                ExecutiveSummarySection {
                    heading: "Our Solution".to_string(),
                    content: format!(
                        "{}\n[Detail the approach and methodology.]",
                        our_solution
                    ),
                },
                ExecutiveSummarySection {
                    heading: "Expected Outcomes".to_string(),
                    content: format!(
                        "{}\n[Quantify all benefits with proof points.]",
                        key_benefit
                    ),
                },
                ExecutiveSummarySection {
                    heading: "Why Us".to_string(),
                    content: "[Win themes, differentiators, proof of capability.]".to_string(),
                },
                ExecutiveSummarySection {
                    heading: "Next Steps".to_string(),
                    content: "[Clear call to action.]".to_string(),
                },
            ],
        };

        ExecutiveSummary {
            framework_name: fw_name,
            client_name: client_name.to_string(),
            sections,
        }
    }

    // ── Generate proposal outline ──

    /// Generate a customised proposal outline based on a framework.
    pub fn generate_proposal_outline(
        &self,
        framework_id: &str,
        client_name: &str,
        project_name: &str,
    ) -> Option<ProposalOutline> {
        let fw = self.get_framework(framework_id)?;

        let supplementary: Vec<FrameworkSummary> = self
            .section_templates
            .iter()
            .take(5)
            .map(|s| FrameworkSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                category: s.category.clone(),
            })
            .collect();

        Some(ProposalOutline {
            client_name: client_name.to_string(),
            project_name: project_name.to_string(),
            framework_name: fw.name.clone(),
            template: fw.template.clone(),
            checklist: fw.checklist.clone(),
            supplementary_sections: supplementary,
        })
    }

    // ── Counts ──

    pub fn framework_count(&self) -> usize {
        self.frameworks.len()
    }

    pub fn section_template_count(&self) -> usize {
        self.section_templates.len()
    }

    pub fn industry_guide_count(&self) -> usize {
        self.industry_guides.len()
    }

    pub fn persuasion_technique_count(&self) -> usize {
        self.persuasion_techniques.len()
    }
}

impl Default for BidService {
    fn default() -> Self {
        Self::new()
    }
}
