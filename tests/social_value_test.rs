use sentinel::domain::social_value::SocialValueService;

#[test]
fn test_get_measure_by_reference() {
    let svc = SocialValueService::new();
    let measure = svc.get_measure("NT1").unwrap();
    assert_eq!(measure.reference, "NT1");
    assert_eq!(measure.theme, "jobs");
}

#[test]
fn test_get_measure_case_insensitive() {
    let svc = SocialValueService::new();
    let measure = svc.get_measure("nt1").unwrap();
    assert_eq!(measure.reference, "NT1");
}

#[test]
fn test_get_measure_not_found() {
    let svc = SocialValueService::new();
    assert!(svc.get_measure("INVALID").is_none());
}

#[test]
fn test_search_measures() {
    let svc = SocialValueService::new();
    let results = svc.search("apprentice", None);
    assert!(!results.is_empty());
}

#[test]
fn test_search_with_theme_filter() {
    let svc = SocialValueService::new();
    let results = svc.search("local", Some("jobs"));
    assert!(results.iter().all(|m| m.theme == "jobs"));
}

#[test]
fn test_search_by_reference() {
    let svc = SocialValueService::new();
    let results = svc.search("HE1", None);
    assert!(!results.is_empty());
    assert!(results.iter().any(|m| m.reference == "HE1"));
}

#[test]
fn test_list_themes() {
    let svc = SocialValueService::new();
    let themes = svc.list_themes();
    assert_eq!(themes.len(), 5);
}

#[test]
fn test_list_themes_has_measure_counts() {
    let svc = SocialValueService::new();
    let themes = svc.list_themes();
    let total: usize = themes.iter().map(|t| t.measure_count).sum();
    assert_eq!(total, svc.measure_count());
}

#[test]
fn test_get_by_theme() {
    let svc = SocialValueService::new();
    let measures = svc.get_by_theme("environment");
    assert!(!measures.is_empty());
    assert!(measures.iter().all(|m| m.theme == "environment"));
}

#[test]
fn test_get_by_theme_unknown() {
    let svc = SocialValueService::new();
    let measures = svc.get_by_theme("nonexistent");
    assert!(measures.is_empty());
}

#[test]
fn test_calculate_social_value() {
    let svc = SocialValueService::new();
    let commitments = vec![
        ("NT29", 100.0), // Volunteering: 100 hours x GBP 16.93
        ("NT31", 50.0),  // CO2: 50 tCO2e x GBP 244.63
    ];
    let result = svc.calculate(&commitments);
    assert!(result.total > 0.0);
    assert_eq!(result.items.len(), 2);
    // NT29: 100 * 16.93 = 1693.0
    // NT31: 50 * 244.63 = 12231.5
    let expected_total = 100.0 * 16.93 + 50.0 * 244.63;
    assert!((result.total - expected_total).abs() < 0.01);
}

#[test]
fn test_calculate_skips_localised() {
    let svc = SocialValueService::new();
    let commitments = vec![
        ("NT1", 10.0), // NT1 is localised -- should be skipped in total
    ];
    let result = svc.calculate(&commitments);
    assert_eq!(result.total, 0.0);
    // Item should still be present but with 0 social_value_gbp
    assert_eq!(result.items.len(), 1);
}

#[test]
fn test_calculate_skips_no_proxy() {
    let svc = SocialValueService::new();
    let commitments = vec![
        ("NT71", 500.0), // NT71 has proxy_value=null
    ];
    let result = svc.calculate(&commitments);
    assert_eq!(result.total, 0.0);
    assert_eq!(result.items.len(), 1);
}

#[test]
fn test_calculate_unknown_ref() {
    let svc = SocialValueService::new();
    let commitments = vec![
        ("INVALID", 10.0),
    ];
    let result = svc.calculate(&commitments);
    assert_eq!(result.items.len(), 0);
    assert_eq!(result.total, 0.0);
}

#[test]
fn test_suggest_social_value() {
    let svc = SocialValueService::new();
    let suggestions = svc.suggest("consultancy", None, None);
    assert!(!suggestions.is_empty());
}

#[test]
fn test_suggest_training_contract() {
    let svc = SocialValueService::new();
    let suggestions = svc.suggest("training", None, None);
    // Training contracts should include skills-related measures
    let refs: Vec<&str> = suggestions.iter().map(|s| s.reference.as_str()).collect();
    assert!(refs.contains(&"HE1") || refs.contains(&"NT9") || refs.contains(&"NT10"));
}

#[test]
fn test_suggest_always_includes_universal() {
    let svc = SocialValueService::new();
    let suggestions = svc.suggest("random unknown type", None, None);
    let refs: Vec<&str> = suggestions.iter().map(|s| s.reference.as_str()).collect();
    // Universal measures should always be present
    assert!(refs.contains(&"NT29")); // volunteering
    assert!(refs.contains(&"NT20")); // wellbeing
}

#[test]
fn test_draft_response() {
    let svc = SocialValueService::new();
    let result = svc.draft_response(&["NT1", "NT29"], "AI training for SMEs, 12 months");
    assert_eq!(result.sections.len(), 2);
    assert_eq!(result.sections[0].reference, "NT1");
    assert!(!result.writing_tips.is_empty());
}

#[test]
fn test_draft_response_unknown_ref() {
    let svc = SocialValueService::new();
    let result = svc.draft_response(&["INVALID", "NT29"], "test contract");
    // Should still have 1 valid section (NT29), skip INVALID
    assert_eq!(result.sections.len(), 1);
}

#[test]
fn test_measure_count() {
    let svc = SocialValueService::new();
    assert_eq!(svc.measure_count(), 24);
}
