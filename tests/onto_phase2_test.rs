use opencheir::store::graph::GraphStore;
use opencheir::store::state::StateDb;
use opencheir::domain::ontology::OntologyService;
use opencheir::orchestration::enforcer::{Action, Enforcer};
use std::sync::Arc;

fn test_db() -> StateDb {
    StateDb::open(std::path::Path::new(":memory:")).unwrap()
}

#[test]
fn test_version_save_list_rollback_workflow() {
    let db = test_db();
    let store = Arc::new(GraphStore::new());

    store.load_turtle(r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#, None).unwrap();

    let result = OntologyService::save_version(&db, &store, "v1.0").unwrap();
    assert!(result.contains("v1.0"));

    store.clear().unwrap();
    store.load_turtle(r#"
        @prefix ex: <http://example.org/> .
        ex:Bob a ex:Person .
    "#, None).unwrap();
    OntologyService::save_version(&db, &store, "v2.0").unwrap();

    let history = OntologyService::list_versions(&db).unwrap();
    assert!(history.contains("v1.0"));
    assert!(history.contains("v2.0"));

    let result = OntologyService::rollback_version(&db, &store, "v1.0").unwrap();
    assert!(result.contains("triples_restored"));

    let query = store.sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }").unwrap();
    assert!(query.contains("Alice"));
}

#[test]
fn test_snapshot_ntriples_roundtrip() {
    let store = GraphStore::new();
    store.load_turtle(r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Bob a ex:Person .
    "#, None).unwrap();

    let snapshot = store.snapshot("ntriples").unwrap();
    store.clear().unwrap();
    assert_eq!(store.triple_count(), 0);

    store.load_ntriples(&snapshot).unwrap();
    assert_eq!(store.triple_count(), 2);
}

#[test]
fn test_version_before_push_enforcer() {
    let mut enforcer = Enforcer::new();
    // Push without versioning -- should warn (no version in window)
    let verdict = enforcer.pre_check("onto_push");
    // First call, empty history, so MissingInWindow fires
    assert!(matches!(verdict.action, Action::Warn));

    // Now with version in window
    enforcer.post_check("onto_version");
    let verdict = enforcer.pre_check("onto_push");
    assert!(matches!(verdict.action, Action::Allow));
}
