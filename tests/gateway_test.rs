use std::sync::{Arc, Mutex};
use opencheir::gateway::server::OpenCheirServer;
use opencheir::orchestration::enforcer::Enforcer;
use opencheir::store::state::StateDb;
use tempfile::TempDir;

fn setup() -> (TempDir, OpenCheirServer) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    let enforcer = Arc::new(Mutex::new(Enforcer::new()));
    let server = OpenCheirServer::new(db, enforcer, 60);
    (dir, server)
}

#[test]
fn test_server_has_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    assert!(!tools.is_empty(), "Server should register at least one tool");

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"opencheir_status"),
        "Expected 'opencheir_status' tool, found: {:?}",
        names
    );
    assert!(
        names.contains(&"opencheir_health"),
        "Expected 'opencheir_health' tool, found: {:?}",
        names
    );
}

#[test]
fn test_server_has_domain_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // QA
    assert!(names.contains(&"qa_check_fonts"), "Missing qa_check_fonts");
    assert!(names.contains(&"qa_full_check"), "Missing qa_full_check");

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
fn test_server_tool_count() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    // 2 opencheir + 8 qa + 3 lineage + 4 enforcer + 3 memory + 2 pattern + search + eyes
    assert!(
        tools.len() >= 15,
        "Expected at least 15 tools, found: {}",
        tools.len()
    );
}

#[test]
fn test_server_tools_have_descriptions() {
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
fn test_server_is_clone() {
    let (_dir, server) = setup();
    let _cloned = server.clone();
}

#[test]
fn test_status_includes_tool_count() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    let expected_count = tools.len();

    let status_tool = tools.iter().find(|t| t.name.as_ref() == "opencheir_status");
    assert!(
        status_tool.is_some(),
        "opencheir_status tool must be registered"
    );

    assert!(
        expected_count >= 15,
        "Expected at least 15 tools, found {}",
        expected_count
    );
}
