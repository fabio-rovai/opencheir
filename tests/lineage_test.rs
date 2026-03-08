use opencheir::orchestration::lineage::{LineageEvent, LineageService};
use opencheir::store::state::StateDb;
use tempfile::TempDir;

fn setup() -> (TempDir, StateDb) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

/// Create a session in the DB so foreign key constraints are satisfied.
fn ensure_session(db: &StateDb, session_id: &str) {
    let conn = db.conn();
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, project, started_at) VALUES (?1, NULL, datetime('now'))",
        rusqlite::params![session_id],
    )
    .unwrap();
}

fn make_event(session_id: &str, event_type: &str, tool: Option<&str>, ts: i64) -> LineageEvent {
    LineageEvent {
        seq: None,
        session_id: Some(session_id.into()),
        timestamp: ts,
        event_type: event_type.into(),
        path: None,
        tool: tool.map(String::from),
        meta: None,
    }
}

// ---------------------------------------------------------------------------
// record_event
// ---------------------------------------------------------------------------

#[test]
fn test_record_event() {
    let (_dir, db) = setup();
    ensure_session(&db, "test-session");
    let event = LineageEvent {
        seq: None,
        session_id: Some("test-session".into()),
        timestamp: 1000,
        event_type: "tool_call".into(),
        path: None,
        tool: Some("qa_check_fonts".into()),
        meta: None,
    };
    let seq = LineageService::record_event(&db, &event).unwrap();
    assert!(seq > 0);
}

#[test]
fn test_record_event_with_meta() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    let meta = serde_json::json!({"input": "hello", "duration_ms": 42});
    let event = LineageEvent {
        seq: None,
        session_id: Some("s1".into()),
        timestamp: 2000,
        event_type: "tool_call".into(),
        path: Some("/tmp/file.docx".into()),
        tool: Some("render_document".into()),
        meta: Some(meta.clone()),
    };
    let seq = LineageService::record_event(&db, &event).unwrap();
    assert!(seq > 0);

    // Verify meta survives round-trip
    let events = LineageService::get_events(&db, Some("s1"), None, 10).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].meta, Some(meta));
}

#[test]
fn test_record_event_sequential_seq() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    let seq1 = LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 1)).unwrap();
    let seq2 = LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 2)).unwrap();
    assert!(seq2 > seq1);
}

// ---------------------------------------------------------------------------
// get_events
// ---------------------------------------------------------------------------

#[test]
fn test_get_events_empty() {
    let (_dir, db) = setup();
    let events = LineageService::get_events(&db, Some("nonexistent"), None, 100).unwrap();
    assert!(events.is_empty());
}

#[test]
fn test_get_events_returns_all_for_session() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    ensure_session(&db, "s2");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_read", None, 200)).unwrap();
    LineageService::record_event(&db, &make_event("s2", "tool_call", Some("b"), 300)).unwrap();

    let events = LineageService::get_events(&db, Some("s1"), None, 100).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.session_id.as_deref() == Some("s1")));
}

#[test]
fn test_get_events_with_type_filter() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_read", None, 200)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 300)).unwrap();

    let events = LineageService::get_events(&db, Some("s1"), Some("tool_call"), 100).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.event_type == "tool_call"));

    let events = LineageService::get_events(&db, Some("s1"), Some("file_read"), 100).unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn test_get_events_respects_limit() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    for i in 0..10 {
        LineageService::record_event(&db, &make_event("s1", "tool_call", Some("x"), i)).unwrap();
    }
    let events = LineageService::get_events(&db, Some("s1"), None, 3).unwrap();
    assert_eq!(events.len(), 3);
}

#[test]
fn test_get_events_no_filters() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    ensure_session(&db, "s2");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s2", "file_read", None, 200)).unwrap();

    let events = LineageService::get_events(&db, None, None, 100).unwrap();
    assert_eq!(events.len(), 2);
}

// ---------------------------------------------------------------------------
// timeline
// ---------------------------------------------------------------------------

#[test]
fn test_timeline() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_write", None, 200)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_result", Some("a"), 300)).unwrap();

    let timeline = LineageService::get_timeline(&db, "s1").unwrap();
    assert_eq!(timeline.total, 3);
    assert_eq!(timeline.events.len(), 3);
    // Events should be in seq order
    assert!(timeline.events[0].timestamp <= timeline.events[1].timestamp);
    assert!(timeline.events[1].timestamp <= timeline.events[2].timestamp);
}

#[test]
fn test_timeline_empty_session() {
    let (_dir, db) = setup();
    let timeline = LineageService::get_timeline(&db, "nonexistent").unwrap();
    assert_eq!(timeline.total, 0);
    assert!(timeline.events.is_empty());
}

// ---------------------------------------------------------------------------
// build_graph
// ---------------------------------------------------------------------------

#[test]
fn test_build_graph() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    // Simulate a sequence: a -> b -> c -> a -> b
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 200)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("c"), 300)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 400)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 500)).unwrap();

    let graph = LineageService::build_graph(&db, Some("s1")).unwrap();

    // Nodes: a, b, c
    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.nodes.contains(&"a".to_string()));
    assert!(graph.nodes.contains(&"b".to_string()));
    assert!(graph.nodes.contains(&"c".to_string()));

    // Edges: a->b (x2), b->c (x1), c->a (x1), a->b counted twice
    assert!(!graph.edges.is_empty());

    let ab_edge = graph
        .edges
        .iter()
        .find(|e| e.from_tool == "a" && e.to_tool == "b")
        .unwrap();
    assert_eq!(ab_edge.count, 2);

    let bc_edge = graph
        .edges
        .iter()
        .find(|e| e.from_tool == "b" && e.to_tool == "c")
        .unwrap();
    assert_eq!(bc_edge.count, 1);

    let ca_edge = graph
        .edges
        .iter()
        .find(|e| e.from_tool == "c" && e.to_tool == "a")
        .unwrap();
    assert_eq!(ca_edge.count, 1);
}

#[test]
fn test_build_graph_ignores_non_tool_call_events() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_read", None, 150)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 200)).unwrap();

    // The graph query filters by tool_call, so file_read is excluded.
    // Only tool_call events are considered: a -> b.
    let graph = LineageService::build_graph(&db, Some("s1")).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from_tool, "a");
    assert_eq!(graph.edges[0].to_tool, "b");
}

#[test]
fn test_build_graph_empty() {
    let (_dir, db) = setup();
    let graph = LineageService::build_graph(&db, Some("empty")).unwrap();
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

#[test]
fn test_build_graph_single_event() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    let graph = LineageService::build_graph(&db, Some("s1")).unwrap();
    // One event means no windows of 2, so no edges and no nodes
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

// ---------------------------------------------------------------------------
// event_counts
// ---------------------------------------------------------------------------

#[test]
fn test_event_counts() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("a"), 100)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("b"), 200)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_read", None, 300)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_write", None, 400)).unwrap();
    LineageService::record_event(&db, &make_event("s1", "file_write", None, 500)).unwrap();

    let counts = LineageService::event_counts(&db, "s1").unwrap();
    // Should have 3 types: file_read(1), file_write(2), tool_call(2)
    assert_eq!(counts.len(), 3);

    let map: std::collections::HashMap<String, usize> = counts.into_iter().collect();
    assert_eq!(map["tool_call"], 2);
    assert_eq!(map["file_read"], 1);
    assert_eq!(map["file_write"], 2);
}

#[test]
fn test_event_counts_empty_session() {
    let (_dir, db) = setup();
    let counts = LineageService::event_counts(&db, "nonexistent").unwrap();
    assert!(counts.is_empty());
}

// ---------------------------------------------------------------------------
// cleanup
// ---------------------------------------------------------------------------

#[test]
fn test_cleanup_old_events() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");

    let now_ms = chrono::Utc::now().timestamp_millis();
    let old_ms = now_ms - (100 * 86_400_000); // 100 days ago

    // Record old events
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("old"), old_ms)).unwrap();
    LineageService::record_event(
        &db,
        &make_event("s1", "file_read", None, old_ms + 1000),
    )
    .unwrap();

    // Record recent event
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("new"), now_ms)).unwrap();

    // Clean up events older than 30 days
    let deleted = LineageService::cleanup(&db, 30).unwrap();
    assert_eq!(deleted, 2);

    // Only the recent event should remain
    let remaining = LineageService::get_events(&db, Some("s1"), None, 100).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].tool.as_deref(), Some("new"));
}

#[test]
fn test_cleanup_no_old_events() {
    let (_dir, db) = setup();
    ensure_session(&db, "s1");
    let now_ms = chrono::Utc::now().timestamp_millis();
    LineageService::record_event(&db, &make_event("s1", "tool_call", Some("x"), now_ms)).unwrap();

    let deleted = LineageService::cleanup(&db, 30).unwrap();
    assert_eq!(deleted, 0);

    let remaining = LineageService::get_events(&db, Some("s1"), None, 100).unwrap();
    assert_eq!(remaining.len(), 1);
}

// ---------------------------------------------------------------------------
// router (smoke test -- just verify it builds without panic)
// ---------------------------------------------------------------------------

#[test]
fn test_lineage_router_builds() {
    let (_dir, db) = setup();
    let _router = opencheir::orchestration::lineage::lineage_router(db);
}
