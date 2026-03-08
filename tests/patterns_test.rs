use opencheir::orchestration::patterns::{Pattern, PatternService};
use opencheir::store::state::StateDb;
use tempfile::TempDir;

fn setup_db() -> (TempDir, StateDb) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

fn ensure_session(db: &StateDb, session_id: &str) {
    let conn = db.conn();
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, project, started_at) VALUES (?1, NULL, datetime('now'))",
        rusqlite::params![session_id],
    )
    .unwrap();
}

#[test]
fn test_store_and_list() {
    let (_dir, db) = setup_db();

    let pattern = Pattern {
        id: None,
        category: "frequent_block".into(),
        description: "Rule 'parse_before_write' has blocked 5 times".into(),
        evidence: Some("enforcement log: 5 blocks".into()),
        confidence: 0.8,
        occurrences: 5,
        actionable: true,
    };

    let id = PatternService::store(&db, &pattern).unwrap();
    assert!(id > 0);

    let listed = PatternService::list(&db, None).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, Some(id));
    assert_eq!(listed[0].category, "frequent_block");
    assert_eq!(listed[0].occurrences, 5);
    assert!(listed[0].actionable);
}

#[test]
fn test_list_by_category() {
    let (_dir, db) = setup_db();

    let p1 = Pattern {
        id: None,
        category: "frequent_block".into(),
        description: "Block pattern".into(),
        evidence: None,
        confidence: 0.8,
        occurrences: 3,
        actionable: true,
    };
    let p2 = Pattern {
        id: None,
        category: "friction_point".into(),
        description: "Friction pattern".into(),
        evidence: Some("evidence".into()),
        confidence: 0.7,
        occurrences: 4,
        actionable: true,
    };
    let p3 = Pattern {
        id: None,
        category: "frequent_block".into(),
        description: "Another block pattern".into(),
        evidence: None,
        confidence: 0.9,
        occurrences: 10,
        actionable: false,
    };

    PatternService::store(&db, &p1).unwrap();
    PatternService::store(&db, &p2).unwrap();
    PatternService::store(&db, &p3).unwrap();

    // Filter by frequent_block
    let blocks = PatternService::list(&db, Some("frequent_block")).unwrap();
    assert_eq!(blocks.len(), 2);
    for p in &blocks {
        assert_eq!(p.category, "frequent_block");
    }

    // Filter by friction_point
    let friction = PatternService::list(&db, Some("friction_point")).unwrap();
    assert_eq!(friction.len(), 1);
    assert_eq!(friction[0].category, "friction_point");

    // No filter returns all
    let all = PatternService::list(&db, None).unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn test_count() {
    let (_dir, db) = setup_db();

    assert_eq!(PatternService::count(&db).unwrap(), 0);

    let p = Pattern {
        id: None,
        category: "test".into(),
        description: "Test pattern".into(),
        evidence: None,
        confidence: 0.5,
        occurrences: 1,
        actionable: false,
    };

    PatternService::store(&db, &p).unwrap();
    assert_eq!(PatternService::count(&db).unwrap(), 1);

    PatternService::store(&db, &p).unwrap();
    assert_eq!(PatternService::count(&db).unwrap(), 2);
}

#[test]
fn test_analyze_enforcement_empty() {
    let (_dir, db) = setup_db();

    // No enforcement entries -> no patterns
    let patterns = PatternService::analyze_enforcement(&db).unwrap();
    assert!(patterns.is_empty());
}

#[test]
fn test_analyze_frequent_blocks() {
    let (_dir, db) = setup_db();
    let session_id = "test-session";
    ensure_session(&db, session_id);

    // Insert 4 blocks for the same rule
    let conn = db.conn();
    for _ in 0..4 {
        conn.execute(
            "INSERT INTO enforcement (session_id, rule, action, tool_call, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session_id,
                "parse_before_write",
                "block",
                "write_document_cell",
                "no parse in history"
            ],
        )
        .unwrap();
    }
    drop(conn);

    let patterns = PatternService::analyze_enforcement(&db).unwrap();

    // Should find at least a frequent_block pattern
    let freq: Vec<_> = patterns
        .iter()
        .filter(|p| p.category == "frequent_block")
        .collect();
    assert_eq!(freq.len(), 1);
    assert!(freq[0].description.contains("parse_before_write"));
    assert_eq!(freq[0].occurrences, 4);
    assert!((freq[0].confidence - 0.8).abs() < f64::EPSILON);
}

#[test]
fn test_analyze_friction_point() {
    let (_dir, db) = setup_db();
    let session_id = "test-session";
    ensure_session(&db, session_id);

    // Insert 5 blocks on the same tool_call
    let conn = db.conn();
    for i in 0..5 {
        conn.execute(
            "INSERT INTO enforcement (session_id, rule, action, tool_call, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session_id,
                format!("rule_{}", i % 2), // different rules
                "block",
                "write_document_cell",     // same tool
                "some reason"
            ],
        )
        .unwrap();
    }
    drop(conn);

    let patterns = PatternService::analyze_enforcement(&db).unwrap();

    // Should find a friction_point pattern for write_document_cell
    let friction: Vec<_> = patterns
        .iter()
        .filter(|p| p.category == "friction_point")
        .collect();
    assert_eq!(friction.len(), 1);
    assert!(friction[0].description.contains("write_document_cell"));
    assert_eq!(friction[0].occurrences, 5);
    assert!((friction[0].confidence - 0.7).abs() < f64::EPSILON);
}
