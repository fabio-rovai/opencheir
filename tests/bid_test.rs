use sentinel::domain::bid::BidService;
use std::collections::HashMap;

// ── Loading ──

#[test]
fn test_loads_framework_data() {
    let svc = BidService::new();
    assert!(svc.framework_count() > 0);
    assert!(svc.section_template_count() > 0);
    assert!(svc.industry_guide_count() > 0);
    assert!(svc.persuasion_technique_count() > 0);
}

#[test]
fn test_framework_count_matches_expected() {
    let svc = BidService::new();
    assert_eq!(svc.framework_count(), 49);
}

#[test]
fn test_section_template_count() {
    let svc = BidService::new();
    assert_eq!(svc.section_template_count(), 21);
}

#[test]
fn test_industry_guide_count() {
    let svc = BidService::new();
    assert_eq!(svc.industry_guide_count(), 10);
}

#[test]
fn test_persuasion_technique_count() {
    let svc = BidService::new();
    assert_eq!(svc.persuasion_technique_count(), 11);
}

// ── List frameworks ──

#[test]
fn test_list_frameworks_all() {
    let svc = BidService::new();
    let all = svc.list_frameworks(None);
    assert_eq!(all.len(), 49);
}

#[test]
fn test_list_frameworks_by_category() {
    let svc = BidService::new();
    let sales = svc.list_frameworks(Some("Sales Methodology"));
    assert!(!sales.is_empty());
    assert!(sales.iter().all(|fw| fw.category == "Sales Methodology"));
}

#[test]
fn test_list_frameworks_category_case_insensitive() {
    let svc = BidService::new();
    let a = svc.list_frameworks(Some("copywriting"));
    let b = svc.list_frameworks(Some("Copywriting"));
    assert_eq!(a.len(), b.len());
    assert!(!a.is_empty());
}

#[test]
fn test_list_frameworks_unknown_category() {
    let svc = BidService::new();
    let empty = svc.list_frameworks(Some("Nonexistent Category"));
    assert!(empty.is_empty());
}

// ── Get framework ──

#[test]
fn test_get_framework_spin() {
    let svc = BidService::new();
    let fw = svc.get_framework("spin-selling").unwrap();
    assert_eq!(fw.name, "SPIN Selling");
    assert_eq!(fw.category, "Sales Methodology");
    assert!(!fw.checklist.is_empty());
    assert!(!fw.scoring_criteria.is_empty());
    assert!(!fw.best_practices.is_empty());
}

#[test]
fn test_get_framework_shipley() {
    let svc = BidService::new();
    let fw = svc.get_framework("shipley").unwrap();
    assert_eq!(fw.name, "Shipley Proposal Method");
    assert_eq!(fw.category, "Proposal Management");
}

#[test]
fn test_get_framework_aida() {
    let svc = BidService::new();
    let fw = svc.get_framework("aida").unwrap();
    assert_eq!(fw.name, "AIDA");
    assert_eq!(fw.category, "Copywriting & Persuasion");
}

#[test]
fn test_get_framework_not_found() {
    let svc = BidService::new();
    assert!(svc.get_framework("nonexistent").is_none());
}

// ── Compare frameworks ──

#[test]
fn test_compare_frameworks() {
    let svc = BidService::new();
    let compared = svc.compare_frameworks(&["spin-selling", "aida"]);
    assert_eq!(compared.len(), 2);
    assert_eq!(compared[0].id, "spin-selling");
    assert_eq!(compared[1].id, "aida");
}

#[test]
fn test_compare_frameworks_skips_invalid() {
    let svc = BidService::new();
    let compared = svc.compare_frameworks(&["spin-selling", "nonexistent", "aida"]);
    assert_eq!(compared.len(), 2);
}

// ── Section templates ──

#[test]
fn test_get_section_template() {
    let svc = BidService::new();
    let section = svc.get_section_template("cover-letter").unwrap();
    assert_eq!(section.name, "Proposal Cover Letter");
    assert!(!section.content.is_empty());
}

#[test]
fn test_get_section_template_not_found() {
    let svc = BidService::new();
    assert!(svc.get_section_template("nonexistent").is_none());
}

// ── Industry guides ──

#[test]
fn test_get_industry_guide() {
    let svc = BidService::new();
    let guide = svc.get_industry_guide("gov-federal").unwrap();
    assert_eq!(guide.name, "Federal Government Proposals");
    assert!(!guide.content.is_empty());
}

#[test]
fn test_get_industry_guide_not_found() {
    let svc = BidService::new();
    assert!(svc.get_industry_guide("nonexistent").is_none());
}

// ── Persuasion techniques ──

#[test]
fn test_get_persuasion_technique() {
    let svc = BidService::new();
    let tech = svc.get_persuasion_technique("social-proof").unwrap();
    assert_eq!(tech.name, "Social Proof in Proposals");
    assert!(!tech.content.is_empty());
}

#[test]
fn test_get_persuasion_technique_not_found() {
    let svc = BidService::new();
    assert!(svc.get_persuasion_technique("nonexistent").is_none());
}

// ── Recommend framework ──

#[test]
fn test_recommend_government() {
    let svc = BidService::new();
    let recs = svc.recommend_framework("government", "large", "high", "high", "rfp-response");
    assert!(!recs.is_empty());
    let texts: Vec<&str> = recs.iter().map(|r| r.recommendation.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("Shipley")));
    assert!(texts.iter().any(|t| t.contains("APMP")));
}

#[test]
fn test_recommend_enterprise() {
    let svc = BidService::new();
    let recs = svc.recommend_framework("enterprise", "medium", "medium", "high", "pitch");
    assert!(!recs.is_empty());
    let texts: Vec<&str> = recs.iter().map(|r| r.recommendation.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("MEDDPICC")));
}

#[test]
fn test_recommend_smb() {
    let svc = BidService::new();
    let recs = svc.recommend_framework("smb", "small", "low", "low", "unsolicited");
    assert!(!recs.is_empty());
    let texts: Vec<&str> = recs.iter().map(|r| r.recommendation.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("SNAP")));
}

#[test]
fn test_recommend_startup() {
    let svc = BidService::new();
    let recs = svc.recommend_framework("startup", "small", "medium", "low", "pitch");
    let texts: Vec<&str> = recs.iter().map(|r| r.recommendation.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("Gap Selling")));
    assert!(texts.iter().any(|t| t.contains("Blue Ocean")));
}

#[test]
fn test_recommend_high_competition_adds_persuasion() {
    let svc = BidService::new();
    let recs = svc.recommend_framework("government", "large", "high", "medium", "rfp-response");
    let texts: Vec<&str> = recs.iter().map(|r| r.recommendation.as_str()).collect();
    assert!(texts.iter().any(|t| t.contains("Social Proof")));
}

// ── Search resources ──

#[test]
fn test_search_framework_by_name() {
    let svc = BidService::new();
    let results = svc.search_resources("SPIN");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.id == "spin-selling"));
}

#[test]
fn test_search_section_template() {
    let svc = BidService::new();
    let results = svc.search_resources("cover letter");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.resource_type == "Section Template"));
}

#[test]
fn test_search_industry_guide() {
    let svc = BidService::new();
    let results = svc.search_resources("federal");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.resource_type == "Industry Guide"));
}

#[test]
fn test_search_persuasion_technique() {
    let svc = BidService::new();
    let results = svc.search_resources("social proof");
    assert!(!results.is_empty());
    assert!(results
        .iter()
        .any(|r| r.resource_type == "Persuasion Technique"));
}

#[test]
fn test_search_no_results() {
    let svc = BidService::new();
    let results = svc.search_resources("zzxxyyzznonsensicalquery");
    assert!(results.is_empty());
}

#[test]
fn test_search_case_insensitive() {
    let svc = BidService::new();
    let upper = svc.search_resources("SHIPLEY");
    let lower = svc.search_resources("shipley");
    assert_eq!(upper.len(), lower.len());
    assert!(!upper.is_empty());
}

// ── Score proposal ──

#[test]
fn test_score_proposal_strong() {
    let svc = BidService::new();
    let mut scores = HashMap::new();
    scores.insert("Technical Quality".to_string(), 9u32);
    scores.insert("Compliance".to_string(), 8);
    scores.insert("Innovation".to_string(), 9);
    let result = svc.score_proposal("spin-selling", &scores, None).unwrap();
    assert!(result.average >= 8.0);
    assert!(result.assessment.contains("Strong"));
    assert!(result.weak_areas.is_empty());
}

#[test]
fn test_score_proposal_weak_areas() {
    let svc = BidService::new();
    let mut scores = HashMap::new();
    scores.insert("Technical Quality".to_string(), 9u32);
    scores.insert("Cost Realism".to_string(), 4);
    let result = svc.score_proposal("shipley", &scores, None).unwrap();
    assert!(!result.weak_areas.is_empty());
    assert!(result.weak_areas.iter().any(|(k, _)| k == "Cost Realism"));
}

#[test]
fn test_score_proposal_framework_not_found() {
    let svc = BidService::new();
    let scores = HashMap::new();
    assert!(svc.score_proposal("nonexistent", &scores, None).is_none());
}

// ── Win themes ──

#[test]
fn test_generate_win_themes() {
    let svc = BidService::new();
    let themes = svc.generate_win_themes(
        "technology",
        "legacy system modernisation",
        &["Cloud expertise", "Agile delivery"],
        None,
    );
    assert_eq!(themes.len(), 2);
    assert_eq!(themes[0].index, 1);
    assert!(themes[0]
        .theme_statement
        .contains("legacy system modernisation"));
}

#[test]
fn test_generate_win_themes_max_five() {
    let svc = BidService::new();
    let themes = svc.generate_win_themes(
        "tech",
        "challenge",
        &["A", "B", "C", "D", "E", "F", "G"],
        None,
    );
    assert_eq!(themes.len(), 5);
}

// ── Compliance matrix ──

#[test]
fn test_generate_compliance_matrix() {
    let svc = BidService::new();
    let reqs = vec![
        sentinel::domain::bid::ComplianceRequirement {
            id: "R1".to_string(),
            description: "Must provide 24/7 support".to_string(),
            mandatory: true,
        },
        sentinel::domain::bid::ComplianceRequirement {
            id: "R2".to_string(),
            description: "Desirable: multi-language support".to_string(),
            mandatory: false,
        },
    ];
    let matrix = svc.generate_compliance_matrix(&reqs);
    assert_eq!(matrix.total_requirements, 2);
    assert_eq!(matrix.mandatory_count, 1);
    assert_eq!(matrix.desired_count, 1);
    assert_eq!(matrix.rows.len(), 2);
    assert!(matrix.rows[0].mandatory);
    assert!(!matrix.rows[1].mandatory);
}

// ── Bid/No-Bid analysis ──

#[test]
fn test_bid_no_bid_strong() {
    let svc = BidService::new();
    let result = svc.bid_no_bid_analysis("NHS Digital Transformation", Some("5M GBP"), 9, 8, 9, 8, 9);
    assert!(result.weighted_score >= 80.0);
    assert!(result.recommendation.contains("STRONG BID"));
    assert!(result.win_probability.contains("High"));
}

#[test]
fn test_bid_no_bid_no_bid() {
    let svc = BidService::new();
    let result = svc.bid_no_bid_analysis("Risky Opportunity", None, 2, 3, 2, 3, 2);
    assert!(result.weighted_score < 40.0);
    assert!(result.recommendation.contains("NO BID"));
    assert!(!result.weak_areas.is_empty());
}

#[test]
fn test_bid_no_bid_conditional() {
    let svc = BidService::new();
    let result = svc.bid_no_bid_analysis("Council IT Tender", None, 7, 6, 6, 7, 6);
    assert!(result.weighted_score >= 60.0);
    assert!(result.weighted_score < 80.0);
    assert!(result.recommendation.contains("CONDITIONAL"));
}

// ── Executive summary ──

#[test]
fn test_generate_executive_summary_aida() {
    let svc = BidService::new();
    let result = svc.generate_executive_summary(
        "aida",
        "Acme Corp",
        "slow digital adoption",
        "Cloud Migration Platform",
        "50% cost reduction",
    );
    assert_eq!(result.framework_name, "AIDA");
    assert_eq!(result.sections.len(), 4);
    assert_eq!(result.sections[0].heading, "ATTENTION");
    assert_eq!(result.sections[3].heading, "ACTION");
}

#[test]
fn test_generate_executive_summary_pas() {
    let svc = BidService::new();
    let result = svc.generate_executive_summary(
        "pas",
        "BigCo",
        "data silos",
        "Integration Hub",
        "unified view",
    );
    assert_eq!(result.sections.len(), 3);
    assert_eq!(result.sections[0].heading, "PROBLEM");
    assert!(result.sections[0].content.contains("BigCo"));
}

#[test]
fn test_generate_executive_summary_generic() {
    let svc = BidService::new();
    let result = svc.generate_executive_summary(
        "shipley",
        "GovOrg",
        "compliance gaps",
        "Audit Platform",
        "full compliance",
    );
    assert_eq!(result.sections.len(), 5);
    assert_eq!(result.sections[0].heading, "Client Understanding");
}

// ── Proposal outline ──

#[test]
fn test_generate_proposal_outline() {
    let svc = BidService::new();
    let outline = svc
        .generate_proposal_outline("spin-selling", "NHSX", "Digital Maturity")
        .unwrap();
    assert_eq!(outline.framework_name, "SPIN Selling");
    assert_eq!(outline.client_name, "NHSX");
    assert!(!outline.template.is_empty());
    assert!(!outline.checklist.is_empty());
    assert!(!outline.supplementary_sections.is_empty());
}

#[test]
fn test_generate_proposal_outline_not_found() {
    let svc = BidService::new();
    assert!(svc
        .generate_proposal_outline("nonexistent", "Client", "Project")
        .is_none());
}

// ── Categories ──

#[test]
fn test_categories() {
    let svc = BidService::new();
    let cats = svc.categories();
    assert!(cats.contains(&"Sales Methodology".to_string()));
    assert!(cats.contains(&"Proposal Management".to_string()));
    assert!(cats.contains(&"Copywriting & Persuasion".to_string()));
    assert!(cats.contains(&"Business Strategy".to_string()));
}
