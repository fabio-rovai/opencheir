use opencheir::store::{
    search::SearchService,
    state::StateDb,
};
use tempfile::TempDir;

fn setup() -> (TempDir, StateDb) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

// ---------------------------------------------------------------------------
// Tests: index and search
// ---------------------------------------------------------------------------

#[test]
fn test_index_and_search() {
    let (_dir, db) = setup();
    SearchService::index(
        &db,
        "project",
        "1",
        "NHS Data Platform",
        "Building a data platform for the NHS",
        "health,data",
    )
    .unwrap();
    SearchService::index(
        &db,
        "project",
        "2",
        "Council Website",
        "Redesigning the council website",
        "web,government",
    )
    .unwrap();

    let results = SearchService::search(&db, "data platform", None, 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].source_id, "1");
}

#[test]
fn test_search_returns_correct_fields() {
    let (_dir, db) = setup();
    SearchService::index(
        &db,
        "project",
        "42",
        "My Title",
        "Some searchable content here",
        "tag1,tag2",
    )
    .unwrap();

    let results = SearchService::search(&db, "searchable content", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, "project");
    assert_eq!(results[0].source_id, "42");
    assert_eq!(results[0].title, "My Title");
    // snippet should contain content (may include highlight markers)
    assert!(!results[0].snippet.is_empty());
    // rank is a negative number from BM25 (more negative = more relevant)
    assert!(results[0].rank < 0.0);
}

// ---------------------------------------------------------------------------
// Tests: source filter
// ---------------------------------------------------------------------------

#[test]
fn test_search_with_source_filter() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "Project A", "Some content about data", "")
        .unwrap();
    SearchService::index(
        &db,
        "case_study",
        "1",
        "Case Study A",
        "Some content about data",
        "",
    )
    .unwrap();

    let results = SearchService::search(&db, "content", Some("project"), 10).unwrap();
    assert!(results.iter().all(|r| r.source == "project"));
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_without_filter_returns_all_sources() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "Project A", "Some content about data", "")
        .unwrap();
    SearchService::index(
        &db,
        "case_study",
        "1",
        "Case Study A",
        "Some content about data",
        "",
    )
    .unwrap();

    let results = SearchService::search(&db, "content", None, 10).unwrap();
    assert_eq!(results.len(), 2);
}

// ---------------------------------------------------------------------------
// Tests: empty / no results
// ---------------------------------------------------------------------------

#[test]
fn test_search_no_results() {
    let (_dir, db) = setup();
    let results = SearchService::search(&db, "nonexistent", None, 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_search_empty_index() {
    let (_dir, db) = setup();
    let results = SearchService::search(&db, "anything", None, 10).unwrap();
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: clear_source
// ---------------------------------------------------------------------------

#[test]
fn test_clear_source() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "T1", "content one", "").unwrap();
    SearchService::index(&db, "case_study", "1", "CS1", "content two", "").unwrap();

    let cleared = SearchService::clear_source(&db, "project").unwrap();
    assert_eq!(cleared, 1);
    assert_eq!(SearchService::count(&db).unwrap(), 1);
}

#[test]
fn test_clear_source_leaves_other_sources() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "T1", "content", "").unwrap();
    SearchService::index(&db, "project", "2", "T2", "content", "").unwrap();
    SearchService::index(&db, "case_study", "1", "CS1", "content", "").unwrap();

    SearchService::clear_source(&db, "project").unwrap();

    // Only case_study should remain
    let results = SearchService::search(&db, "content", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, "case_study");
}

#[test]
fn test_clear_nonexistent_source() {
    let (_dir, db) = setup();
    let cleared = SearchService::clear_source(&db, "nonexistent").unwrap();
    assert_eq!(cleared, 0);
}

// ---------------------------------------------------------------------------
// Tests: count
// ---------------------------------------------------------------------------

#[test]
fn test_count_empty() {
    let (_dir, db) = setup();
    assert_eq!(SearchService::count(&db).unwrap(), 0);
}

#[test]
fn test_count_after_inserts() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "T1", "c", "").unwrap();
    assert_eq!(SearchService::count(&db).unwrap(), 1);

    SearchService::index(&db, "project", "2", "T2", "c", "").unwrap();
    assert_eq!(SearchService::count(&db).unwrap(), 2);
}

#[test]
fn test_count_after_clear() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "T1", "c", "").unwrap();
    SearchService::index(&db, "project", "2", "T2", "c", "").unwrap();
    SearchService::clear_source(&db, "project").unwrap();
    assert_eq!(SearchService::count(&db).unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Tests: find_similar
// ---------------------------------------------------------------------------

#[test]
fn test_find_similar() {
    let (_dir, db) = setup();
    SearchService::index(
        &db,
        "project",
        "1",
        "Digital Transformation",
        "cloud computing migration strategy",
        "",
    )
    .unwrap();
    SearchService::index(
        &db,
        "project",
        "2",
        "Paper Filing System",
        "traditional paper based filing",
        "",
    )
    .unwrap();

    let results = SearchService::find_similar(
        &db,
        "cloud computing infrastructure migration",
        None,
        10,
    )
    .unwrap();
    assert!(!results.is_empty());
    // The cloud computing result should rank first
    assert_eq!(results[0].source_id, "1");
}

#[test]
fn test_find_similar_with_source_filter() {
    let (_dir, db) = setup();
    SearchService::index(
        &db,
        "project",
        "1",
        "Cloud Project",
        "cloud computing migration",
        "",
    )
    .unwrap();
    SearchService::index(
        &db,
        "case_study",
        "1",
        "Cloud Case Study",
        "cloud computing example",
        "",
    )
    .unwrap();

    let results = SearchService::find_similar(
        &db,
        "cloud computing infrastructure",
        Some("case_study"),
        10,
    )
    .unwrap();
    assert!(results.iter().all(|r| r.source == "case_study"));
}

#[test]
fn test_find_similar_short_words_skipped() {
    let (_dir, db) = setup();
    SearchService::index(&db, "project", "1", "Test", "some content here", "").unwrap();

    // All words are 3 chars or fewer, should return empty
    let results = SearchService::find_similar(&db, "a to be or", None, 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_find_similar_empty_text() {
    let (_dir, db) = setup();
    let results = SearchService::find_similar(&db, "", None, 10).unwrap();
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: search limit
// ---------------------------------------------------------------------------

#[test]
fn test_search_respects_limit() {
    let (_dir, db) = setup();
    for i in 0..10 {
        SearchService::index(
            &db,
            "project",
            &i.to_string(),
            &format!("Project {}", i),
            "cloud computing digital transformation",
            "",
        )
        .unwrap();
    }

    let results = SearchService::search(&db, "cloud computing", None, 3).unwrap();
    assert_eq!(results.len(), 3);
}

// ---------------------------------------------------------------------------
// Tests: search by tags
// ---------------------------------------------------------------------------

#[test]
fn test_search_matches_tags() {
    let (_dir, db) = setup();
    SearchService::index(
        &db,
        "project",
        "1",
        "Generic Title",
        "Generic content",
        "healthcare,nhs,digital",
    )
    .unwrap();

    let results = SearchService::search(&db, "healthcare", None, 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].source_id, "1");
}
