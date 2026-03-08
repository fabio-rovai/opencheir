use tempfile::TempDir;

#[test]
fn test_seed_company_data() {
    let dir = TempDir::new().unwrap();
    let db = sentinel::sentinel_core::state::StateDb::open(&dir.path().join("test.db")).unwrap();
    db.seed_company().unwrap();

    let conn = db.conn();
    let vat: String = conn.query_row(
        "SELECT value FROM company WHERE key = 'vat_number'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(!vat.is_empty());

    // Verify total record count (all sections loaded)
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM company",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(total >= 20); // company + psc + insurance + turnover + references
}

#[test]
fn test_seed_company_idempotent() {
    let dir = TempDir::new().unwrap();
    let db = sentinel::sentinel_core::state::StateDb::open(&dir.path().join("test.db")).unwrap();
    db.seed_company().unwrap();
    db.seed_company().unwrap(); // second call should not error

    let conn = db.conn();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM company", [], |row| row.get(0)).unwrap();
    assert!(count >= 20);
}
