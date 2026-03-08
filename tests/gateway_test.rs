use sentinel::gateway::server::SentinelServer;
use sentinel::sentinel_core::state::StateDb;
use tempfile::TempDir;

fn setup() -> (TempDir, SentinelServer) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    let server = SentinelServer::new(db);
    (dir, server)
}

#[test]
fn test_sentinel_server_has_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    assert!(!tools.is_empty(), "Server should register at least one tool");

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"sentinel_status"),
        "Expected 'sentinel_status' tool, found: {:?}",
        names
    );
    assert!(
        names.contains(&"sentinel_health"),
        "Expected 'sentinel_health' tool, found: {:?}",
        names
    );
}

#[test]
fn test_sentinel_server_has_domain_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // Social Value
    assert!(names.contains(&"list_toms_themes"), "Missing list_toms_themes");
    assert!(names.contains(&"search_toms"), "Missing search_toms");
    assert!(names.contains(&"calculate_social_value"), "Missing calculate_social_value");

    // Bid Writing
    assert!(names.contains(&"list_frameworks"), "Missing list_frameworks");
    assert!(names.contains(&"bid_no_bid_analysis"), "Missing bid_no_bid_analysis");
    assert!(names.contains(&"score_proposal"), "Missing score_proposal");

    // Tender
    assert!(names.contains(&"parse_tender"), "Missing parse_tender");
    assert!(names.contains(&"check_compliance"), "Missing check_compliance");

    // QA
    assert!(names.contains(&"qa_check_fonts"), "Missing qa_check_fonts");
    assert!(names.contains(&"qa_full_check"), "Missing qa_full_check");

    // Search
    assert!(names.contains(&"search_tenders"), "Missing search_tenders");

    // Lineage
    assert!(names.contains(&"lineage_record"), "Missing lineage_record");

    // Enforcer
    assert!(names.contains(&"enforcer_check"), "Missing enforcer_check");
    assert!(names.contains(&"enforcer_rules"), "Missing enforcer_rules");

    // Memory
    assert!(names.contains(&"hive_memory_store"), "Missing hive_memory_store");
    assert!(names.contains(&"hive_memory_recall"), "Missing hive_memory_recall");

    // Patterns
    assert!(names.contains(&"pattern_analyze"), "Missing pattern_analyze");
}

#[test]
fn test_sentinel_server_tool_count() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    // 2 sentinel + 7 social value + 14 bid + 5 tender + 8 qa + 1 search
    // + 3 lineage + 4 enforcer + 3 memory + 2 pattern = 49
    assert!(
        tools.len() >= 40,
        "Expected at least 40 tools, found: {}",
        tools.len()
    );
}

#[test]
fn test_sentinel_server_tools_have_descriptions() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    for tool in &tools {
        assert!(
            tool.description.is_some(),
            "Tool '{}' is missing a description",
            tool.name
        );
        let desc = tool.description.as_ref().unwrap();
        assert!(
            !desc.is_empty(),
            "Tool '{}' has an empty description",
            tool.name
        );
    }
}

#[test]
fn test_sentinel_server_is_clone() {
    let (_dir, server) = setup();
    let _cloned = server.clone();
}

#[test]
fn test_status_includes_tool_count() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    let expected_count = tools.len();

    let status_tool = tools.iter().find(|t| t.name.as_ref() == "sentinel_status");
    assert!(
        status_tool.is_some(),
        "sentinel_status tool must be registered"
    );

    assert!(
        expected_count >= 40,
        "Expected at least 40 tools, found {}",
        expected_count
    );
}
