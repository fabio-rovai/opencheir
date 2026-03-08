use tempfile::TempDir;

#[test]
fn test_db_opens_and_creates_tables() {
    let dir = TempDir::new().unwrap();
    let db = opencheir::store::state::StateDb::open(&dir.path().join("test.db")).unwrap();

    let tables = db.list_tables().unwrap();
    // Verify core tables exist
    assert!(tables.contains(&"sessions".to_string()));
    assert!(tables.contains(&"documents".to_string()));
    assert!(tables.contains(&"learnings".to_string()));
}

#[test]
fn test_db_open_idempotent() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let _db1 = opencheir::store::state::StateDb::open(&db_path).unwrap();
    let _db2 = opencheir::store::state::StateDb::open(&db_path).unwrap(); // second open should not error
}
