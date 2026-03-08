use sentinel::gateway::router::route_tool;

#[test]
fn test_route_by_prefix() {
    assert_eq!(route_tool("tender_parse"), "domain::tender");
    assert_eq!(route_tool("qa_check_fonts"), "domain::qa");
    assert_eq!(route_tool("sv_suggest"), "domain::social_value");
    assert_eq!(route_tool("bid_score"), "domain::bid");
    assert_eq!(route_tool("eyes_capture"), "domain::eyes");
    assert_eq!(route_tool("lineage_track"), "orchestration::lineage");
    assert_eq!(route_tool("hive_orchestrate"), "orchestration::hive");
    assert_eq!(route_tool("skill_list"), "orchestration::skills");
    assert_eq!(route_tool("sentinel_status"), "orchestration::supervisor");
    assert_eq!(route_tool("word_add_paragraph"), "proxy::word-document-server");
    assert_eq!(route_tool("mermaid_render"), "proxy::mermaid-kroki");
    assert_eq!(route_tool("puppeteer_click"), "proxy::puppeteer");
    assert_eq!(route_tool("threejs_show"), "proxy::threejs");
    assert_eq!(route_tool("unknown_tool"), "unknown");
}

#[test]
fn test_route_empty_string() {
    assert_eq!(route_tool(""), "unknown");
}

#[test]
fn test_route_prefix_only() {
    // A bare prefix with underscore should still match
    assert_eq!(route_tool("tender_"), "domain::tender");
    assert_eq!(route_tool("qa_"), "domain::qa");
}
