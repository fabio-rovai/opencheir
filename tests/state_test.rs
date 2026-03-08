use tempfile::TempDir;

#[test]
fn test_create_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("opencheir.db");
    let _db = opencheir::store::state::StateDb::open(&db_path).unwrap();
    assert!(db_path.exists());
}

#[test]
fn test_schema_tables_exist() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("opencheir.db");
    let db = opencheir::store::state::StateDb::open(&db_path).unwrap();

    let tables = db.list_tables().unwrap();
    assert!(tables.contains(&"sessions".to_string()));
    assert!(tables.contains(&"documents".to_string()));
    assert!(tables.contains(&"events".to_string()));
    assert!(tables.contains(&"goals".to_string()));
    assert!(tables.contains(&"tasks".to_string()));
    assert!(tables.contains(&"learnings".to_string()));
    assert!(tables.contains(&"health".to_string()));
    assert!(tables.contains(&"enforcement".to_string()));
    assert!(tables.contains(&"rules".to_string()));
    assert!(tables.contains(&"patterns".to_string()));
    assert!(tables.contains(&"skills".to_string()));
    assert!(tables.contains(&"qa_results".to_string()));
}

#[test]
fn test_create_session() {
    let dir = TempDir::new().unwrap();
    let db = opencheir::store::state::StateDb::open(&dir.path().join("test.db")).unwrap();
    let session_id = db.create_session(Some("test-project")).unwrap();
    assert!(!session_id.is_empty());
}
