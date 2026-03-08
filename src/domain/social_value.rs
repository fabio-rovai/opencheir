use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomsMeasure {
    pub reference: String,
    pub theme: String,
    pub outcome: String,
    pub title: String,
    pub units: String,
    pub proxy_value: Option<f64>,
    #[serde(default)]
    pub proxy_currency: String,
    #[serde(default)]
    pub localised_by_project: bool,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub target_requirements: String,
    #[serde(default)]
    pub evidence_required: String,
    #[serde(default)]
    pub unit_guidance: String,
    #[serde(default)]
    pub double_counting: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeSummary {
    pub theme_key: String,
    pub theme_name: String,
    pub measure_count: usize,
    pub outcomes: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalculationResult {
    pub items: Vec<CalculationItem>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalculationItem {
    pub reference: String,
    pub title: String,
    pub quantity: f64,
    pub units: String,
    pub proxy_value: Option<f64>,
    pub social_value_gbp: f64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub reference: String,
    pub title: String,
    pub relevance: String,
    pub reason: String,
    pub practical_tip: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftResponseResult {
    pub contract: String,
    pub sections: Vec<DraftSection>,
    pub writing_tips: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftSection {
    pub reference: String,
    pub title: String,
    pub theme: String,
    pub units: String,
    pub proxy_value_per_unit: Option<f64>,
    pub commitment: String,
    pub how: String,
    pub evidence: String,
    pub monitoring: String,
    pub double_counting_warning: Vec<String>,
    pub practical_tip: String,
}

// ── Static Lookups ──

fn themes() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("jobs", "Promote Local Skills and Employment"),
        ("growth", "Supporting Growth of Responsible Regional Business"),
        ("social", "Healthier, Safer and more Resilient Communities"),
        ("environment", "Decarbonising and Safeguarding our World"),
        ("innovation", "Promoting Social Innovation"),
    ])
}

fn outcomes() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("local_employment", "More local people in employment"),
        ("disadvantaged", "More opportunities for disadvantaged people"),
        ("skills", "Improved skills"),
        ("youth_employability", "Improved employability of young people"),
        ("local_business", "More opportunities for local MSMEs and VCSEs"),
        ("wellbeing", "Improving staff wellbeing and mental health"),
        ("inequalities", "Reducing inequalities"),
        ("ethical_procurement", "Ethical procurement is promoted"),
        ("community", "More working with the community"),
        ("carbon", "Carbon emissions are reduced"),
        (
            "resource_efficiency",
            "Resource efficiency and circular economy solutions",
        ),
        ("air_quality", "Air pollution is reduced"),
        ("biodiversity", "Safeguarding the natural environment"),
        (
            "social_innovation",
            "Social innovation to support community",
        ),
    ])
}

fn practical_tips() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("NT1", "Count all local FTE staff working on the contract. Define 'local' per the tender spec (usually within local authority boundary)."),
        ("NT1c", "Map your subcontractors and their local employees. Ask them to provide postcode data for their staff on the contract."),
        ("NT4", "Partner with local job centres, Prince's Trust, or youth employment charities to identify NEET candidates for junior roles."),
        ("HE1", "Offer guest lectures or workshops at local universities. Count preparation + delivery hours per staff member."),
        ("NT9", "Fund staff to undertake vocational qualifications (NVQ, BTEC). Only count weeks where they primarily work on the contract."),
        ("NT10", "Create apprenticeship positions on the contract. Remember to account for levy attribution if levy-funded."),
        ("NT13", "Offer 6+ week paid placements to students or graduates. Must pay at least minimum wage."),
        ("NT14", "Subcontract to social enterprises or charities where possible. Use Social Enterprise UK directory to find them."),
        ("NT18", "Prioritise local suppliers. Track spend by supplier postcode and report against the defined local area."),
        ("NT20", "Ensure your wellbeing programme covers: flexible working, nutrition, physical health, health risk assessment, and resources."),
        ("NT21", "Run EDI training sessions. Multiply session hours by attendees. Offer to supply chain at no cost."),
        ("NT28", "Donate materials, equipment, or cash to local community projects linked to the contract area."),
        ("NT29", "Organise team volunteering days for local causes. Only count paid working hours or overtime."),
        ("NT31", "Measure your baseline carbon footprint for the contract, then implement reductions. Get independent verification."),
        ("NT32", "Implement cycle-to-work scheme, remote working policy, or carpooling for contract staff."),
        ("NT33", "Use electric or hybrid vehicles for contract-related travel. Track miles and calculate CO2e savings."),
        ("NT40", "Publish gender pay gap data for contract staff. Implement targeted recruitment and promotion practices."),
        ("NT43", "Map your supply chain for modern slavery risk. Implement checks (right to work, bank account, address). Train staff."),
        ("NT58", "On renewed contracts, raise pay to Real Living Wage (currently GBP 12.00/hr UK, GBP 13.15/hr London)."),
        ("NT80", "Fund comprehensive upskilling programmes for existing staff on the contract. Must lead to recognised qualifications."),
    ])
}

fn get_practical_tip(reference: &str) -> String {
    let tips = practical_tips();
    tips.get(reference)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            "Refer to the full measure details for implementation guidance.".to_string()
        })
}

// ── Service ──

pub struct SocialValueService {
    measures: Vec<TomsMeasure>,
}

impl SocialValueService {
    pub fn new() -> Self {
        let json_str = include_str!("../../data/toms.json");
        let measures: Vec<TomsMeasure> =
            serde_json::from_str(json_str).expect("Failed to parse toms.json");
        Self { measures }
    }

    /// Get a single measure by its reference code (case-insensitive).
    pub fn get_measure(&self, reference: &str) -> Option<&TomsMeasure> {
        let ref_upper = reference.to_uppercase();
        let ref_upper = ref_upper.trim();
        self.measures.iter().find(|m| m.reference == ref_upper)
    }

    /// Search measures by keyword across title, definition, tags, and reference.
    /// Optionally filter by theme.
    pub fn search(&self, query: &str, theme: Option<&str>) -> Vec<&TomsMeasure> {
        let query_lower = query.to_lowercase();
        self.measures
            .iter()
            .filter(|m| {
                let matches_theme = theme.map_or(true, |t| m.theme == t.to_lowercase());
                if !matches_theme {
                    return false;
                }
                let searchable = format!(
                    "{} {} {} {}",
                    m.title,
                    m.definition,
                    m.tags.join(" "),
                    m.reference
                )
                .to_lowercase();
                searchable.contains(&query_lower)
            })
            .collect()
    }

    /// List all 5 TOMs themes with outcome breakdowns and measure counts.
    pub fn list_themes(&self) -> Vec<ThemeSummary> {
        let theme_map = themes();
        let outcome_map = outcomes();

        let mut result = Vec::new();
        // Iterate in a stable order
        for &theme_key in &["jobs", "growth", "social", "environment", "innovation"] {
            let theme_measures: Vec<&TomsMeasure> =
                self.measures.iter().filter(|m| m.theme == theme_key).collect();

            let mut outcome_groups: HashMap<String, Vec<String>> = HashMap::new();
            for m in &theme_measures {
                let outcome_name = outcome_map
                    .get(m.outcome.as_str())
                    .unwrap_or(&m.outcome.as_str())
                    .to_string();
                outcome_groups
                    .entry(outcome_name)
                    .or_default()
                    .push(m.reference.clone());
            }

            let theme_name = theme_map
                .get(theme_key)
                .unwrap_or(&theme_key)
                .to_string();

            result.push(ThemeSummary {
                theme_key: theme_key.to_string(),
                theme_name,
                measure_count: theme_measures.len(),
                outcomes: outcome_groups,
            });
        }

        result
    }

    /// Get all measures for a specific theme.
    pub fn get_by_theme(&self, theme: &str) -> Vec<&TomsMeasure> {
        let theme_lower = theme.to_lowercase();
        self.measures
            .iter()
            .filter(|m| m.theme == theme_lower)
            .collect()
    }

    /// Calculate total proxy social value for a set of commitments.
    /// Each commitment is a (reference, quantity) pair.
    /// Localised measures and measures without proxy values are included in items
    /// but excluded from the total.
    pub fn calculate(&self, commitments: &[(&str, f64)]) -> CalculationResult {
        let mut items = Vec::new();
        let mut total = 0.0;

        for &(ref_code, quantity) in commitments {
            let Some(m) = self.get_measure(ref_code) else {
                // Skip unknown references
                continue;
            };

            if m.proxy_value.is_none() || m.localised_by_project {
                let note = if m.localised_by_project {
                    "Localised measure - requires project-specific multiplier"
                } else {
                    "No proxy value available"
                };
                items.push(CalculationItem {
                    reference: m.reference.clone(),
                    title: m.title.clone(),
                    quantity,
                    units: m.units.clone(),
                    proxy_value: m.proxy_value,
                    social_value_gbp: 0.0,
                    note: Some(note.to_string()),
                });
                continue;
            }

            let proxy = m.proxy_value.unwrap();
            let sv = quantity * proxy;
            total += sv;

            items.push(CalculationItem {
                reference: m.reference.clone(),
                title: m.title.clone(),
                quantity,
                units: m.units.clone(),
                proxy_value: Some(proxy),
                social_value_gbp: (sv * 100.0).round() / 100.0,
                note: None,
            });
        }

        CalculationResult {
            items,
            total: (total * 100.0).round() / 100.0,
        }
    }

    /// Suggest relevant TOMs measures for a contract type.
    /// Ported from the Python social-value-mcp server.
    pub fn suggest(
        &self,
        contract_type: &str,
        _contract_value: Option<&str>,
        _sector: Option<&str>,
    ) -> Vec<Suggestion> {
        let ct_lower = contract_type.to_lowercase();
        let mut suggestions: Vec<Suggestion> = Vec::new();
        let mut seen: Vec<String> = Vec::new();

        let add_suggestions =
            |refs: &[&str], relevance: &str, reason: &str, svc: &SocialValueService, seen: &mut Vec<String>, out: &mut Vec<Suggestion>| {
                for &r in refs {
                    if seen.contains(&r.to_string()) {
                        continue;
                    }
                    if let Some(m) = svc.get_measure(r) {
                        seen.push(r.to_string());
                        out.push(Suggestion {
                            reference: m.reference.clone(),
                            title: m.title.clone(),
                            relevance: relevance.to_string(),
                            reason: reason.to_string(),
                            practical_tip: get_practical_tip(&m.reference),
                        });
                    }
                }
            };

        // Universal measures (relevant to all contracts)
        let universal = &["NT1", "NT1c", "NT20", "NT21", "NT29", "NT40", "NT43"];
        add_suggestions(
            universal,
            "high",
            "Applicable to most contract types",
            self,
            &mut seen,
            &mut suggestions,
        );

        // Training/education specific
        let training_keywords = ["training", "education", "teaching", "mentoring", "coaching"];
        if training_keywords.iter().any(|kw| ct_lower.contains(kw)) {
            let training_refs = &["HE1", "NT9", "NT10", "NT80", "NT13"];
            add_suggestions(
                training_refs,
                "high",
                "Directly relevant to training/education contracts",
                self,
                &mut seen,
                &mut suggestions,
            );
        }

        // VCSE/SME relevant
        let consultancy_keywords = ["consultancy", "research", "advisory", "support"];
        if consultancy_keywords.iter().any(|kw| ct_lower.contains(kw)) {
            let supply_refs = &["NT14", "NT18"];
            add_suggestions(
                supply_refs,
                "medium",
                "Relevant for supply chain social value",
                self,
                &mut seen,
                &mut suggestions,
            );
        }

        // Youth/disadvantaged
        let youth_keywords = ["training", "support", "mentoring", "community"];
        if youth_keywords.iter().any(|kw| ct_lower.contains(kw)) {
            let youth_refs = &["NT4", "NT13"];
            add_suggestions(
                youth_refs,
                "medium",
                "Opportunity for social impact through disadvantaged employment",
                self,
                &mut seen,
                &mut suggestions,
            );
        }

        // Carbon (all contracts)
        let carbon_refs = &["NT31", "NT32"];
        add_suggestions(
            carbon_refs,
            "medium",
            "Environmental social value - expected in most tenders",
            self,
            &mut seen,
            &mut suggestions,
        );

        // Community
        let community_keywords = ["community", "local", "support", "training"];
        if community_keywords.iter().any(|kw| ct_lower.contains(kw)) {
            let community_refs = &["NT28"];
            add_suggestions(
                community_refs,
                "medium",
                "Community engagement opportunity",
                self,
                &mut seen,
                &mut suggestions,
            );
        }

        suggestions
    }

    /// Generate a structured outline for a social value tender response.
    pub fn draft_response(
        &self,
        measure_refs: &[&str],
        contract_description: &str,
    ) -> DraftResponseResult {
        let theme_map = themes();
        let mut sections = Vec::new();

        for &ref_code in measure_refs {
            let Some(m) = self.get_measure(ref_code) else {
                continue;
            };

            let theme_name = theme_map
                .get(m.theme.as_str())
                .unwrap_or(&m.theme.as_str())
                .to_string();

            let target_preview: String = m.target_requirements.chars().take(200).collect();
            let evidence_preview: String = m.evidence_required.chars().take(200).collect();

            sections.push(DraftSection {
                reference: m.reference.clone(),
                title: m.title.clone(),
                theme: theme_name,
                units: m.units.clone(),
                proxy_value_per_unit: m.proxy_value,
                commitment: format!(
                    "[State your specific, measurable commitment for {}]",
                    m.reference
                ),
                how: format!("[Describe HOW you will deliver this - {}]", target_preview),
                evidence: format!(
                    "[How you will evidence delivery - {}]",
                    evidence_preview
                ),
                monitoring: "[How you will track and report progress]".to_string(),
                double_counting_warning: m.double_counting.clone(),
                practical_tip: get_practical_tip(&m.reference),
            });
        }

        DraftResponseResult {
            contract: contract_description.to_string(),
            sections,
            writing_tips: vec![
                "Be SPECIFIC - use numbers, dates, names of partner organisations".to_string(),
                "Show ADDITIONALITY - what you do beyond minimum requirements".to_string(),
                "Demonstrate MEASURABILITY - how you will track and evidence each commitment"
                    .to_string(),
                "Avoid DOUBLE COUNTING - check warnings on each measure".to_string(),
                "Link to CONTRACT DELIVERY - show how social value integrates with your core delivery"
                    .to_string(),
            ],
        }
    }

    /// Return the total number of loaded measures.
    pub fn measure_count(&self) -> usize {
        self.measures.len()
    }
}

impl Default for SocialValueService {
    fn default() -> Self {
        Self::new()
    }
}
