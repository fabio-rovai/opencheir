use std::collections::HashSet;

use opencheir::orchestration::hive::{
    coordinator::*, memory::*, planner::*, spawner::*,
};
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

// ── Memory tests ──────────────────────────────────────────────────────

#[test]
fn test_store_and_recall() {
    let (_dir, db) = setup_db();

    let id = MemoryService::store(&db, "rust", "Always handle errors with Result", None, &["error-handling", "best-practice"]).unwrap();
    assert!(id > 0);

    let results = MemoryService::recall(&db, "errors Result", 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].domain, "rust");
    assert!(results[0].lesson.contains("errors"));
}

#[test]
fn test_recall_no_results() {
    let (_dir, db) = setup_db();

    let results = MemoryService::recall(&db, "nonexistent_xyzzy_query", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_by_domain() {
    let (_dir, db) = setup_db();

    MemoryService::store(&db, "rust", "Use clippy for linting", None, &[]).unwrap();
    MemoryService::store(&db, "python", "Use black for formatting", None, &[]).unwrap();
    MemoryService::store(&db, "rust", "Prefer &str over String in params", None, &[]).unwrap();

    let rust_learnings = MemoryService::by_domain(&db, "rust").unwrap();
    assert_eq!(rust_learnings.len(), 2);
    for l in &rust_learnings {
        assert_eq!(l.domain, "rust");
    }

    let python_learnings = MemoryService::by_domain(&db, "python").unwrap();
    assert_eq!(python_learnings.len(), 1);
    assert_eq!(python_learnings[0].domain, "python");
}

#[test]
fn test_count() {
    let (_dir, db) = setup_db();

    assert_eq!(MemoryService::count(&db).unwrap(), 0);

    MemoryService::store(&db, "rust", "Lesson one", None, &[]).unwrap();
    assert_eq!(MemoryService::count(&db).unwrap(), 1);

    MemoryService::store(&db, "rust", "Lesson two", None, &[]).unwrap();
    assert_eq!(MemoryService::count(&db).unwrap(), 2);
}

#[test]
fn test_tags_roundtrip() {
    let (_dir, db) = setup_db();

    let tags = &["async", "tokio", "performance"];
    MemoryService::store(&db, "rust", "Use buffered channels for throughput", None, tags).unwrap();

    let results = MemoryService::by_domain(&db, "rust").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tags, vec!["async", "tokio", "performance"]);
}

// ── Planner tests ─────────────────────────────────────────────────────

#[test]
fn test_create_plan() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-1";
    ensure_session(&db, session_id);

    let plan = Planner::create_plan(&db, session_id, "Build a REST API").unwrap();

    assert!(!plan.goal_id.is_empty());
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(plan.tasks[0].description, "Build a REST API");
    assert!(plan.tasks[0].depends_on.is_empty());

    // Verify persisted in DB
    let conn = db.conn();
    let goal_desc: String = conn
        .query_row(
            "SELECT description FROM goals WHERE id = ?1",
            rusqlite::params![plan.goal_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(goal_desc, "Build a REST API");

    let task_desc: String = conn
        .query_row(
            "SELECT description FROM tasks WHERE id = ?1",
            rusqlite::params![plan.tasks[0].id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(task_desc, "Build a REST API");
}

#[test]
fn test_load_plan() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-2";
    ensure_session(&db, session_id);

    let created = Planner::create_plan(&db, session_id, "Write unit tests").unwrap();
    let loaded = Planner::load_plan(&db, &created.goal_id).unwrap();

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.goal_id, created.goal_id);
    assert_eq!(loaded.tasks.len(), 1);
    assert_eq!(loaded.tasks[0].id, created.tasks[0].id);
    assert_eq!(loaded.tasks[0].description, "Write unit tests");
}

#[test]
fn test_load_nonexistent() {
    let (_dir, db) = setup_db();

    let loaded = Planner::load_plan(&db, "nonexistent-goal-id").unwrap();
    assert!(loaded.is_none());
}

#[test]
fn test_list_goals() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-3";
    ensure_session(&db, session_id);

    Planner::create_plan(&db, session_id, "Goal A").unwrap();
    Planner::create_plan(&db, session_id, "Goal B").unwrap();
    Planner::create_plan(&db, session_id, "Goal C").unwrap();

    let goals = Planner::list_goals(&db, session_id).unwrap();
    assert_eq!(goals.len(), 3);

    let descriptions: Vec<&str> = goals.iter().map(|g| g.description.as_str()).collect();
    assert!(descriptions.contains(&"Goal A"));
    assert!(descriptions.contains(&"Goal B"));
    assert!(descriptions.contains(&"Goal C"));

    for g in &goals {
        assert_eq!(g.status, "pending");
        assert_eq!(g.task_count, 1);
    }
}

// ── Coordinator tests ─────────────────────────────────────────────────

#[test]
fn test_ready_tasks_no_deps() {
    let plan = Plan {
        goal_id: "g1".to_string(),
        tasks: vec![
            TaskNode {
                id: "t1".to_string(),
                description: "Task 1".to_string(),
                depends_on: vec![],
            },
            TaskNode {
                id: "t2".to_string(),
                description: "Task 2".to_string(),
                depends_on: vec![],
            },
            TaskNode {
                id: "t3".to_string(),
                description: "Task 3".to_string(),
                depends_on: vec![],
            },
        ],
    };

    let completed = HashSet::new();
    let ready = Coordinator::ready_tasks(&plan, &completed);
    assert_eq!(ready.len(), 3);
}

#[test]
fn test_ready_tasks_with_deps() {
    let plan = Plan {
        goal_id: "g1".to_string(),
        tasks: vec![
            TaskNode {
                id: "t1".to_string(),
                description: "Setup".to_string(),
                depends_on: vec![],
            },
            TaskNode {
                id: "t2".to_string(),
                description: "Build".to_string(),
                depends_on: vec!["t1".to_string()],
            },
            TaskNode {
                id: "t3".to_string(),
                description: "Test".to_string(),
                depends_on: vec!["t1".to_string(), "t2".to_string()],
            },
        ],
    };

    // Nothing completed: only t1 is ready
    let completed = HashSet::new();
    let ready = Coordinator::ready_tasks(&plan, &completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "t1");

    // t1 completed: t2 becomes ready, t3 still blocked
    let mut completed = HashSet::new();
    completed.insert("t1".to_string());
    let ready = Coordinator::ready_tasks(&plan, &completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "t2");

    // t1 and t2 completed: t3 becomes ready
    completed.insert("t2".to_string());
    let ready = Coordinator::ready_tasks(&plan, &completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "t3");

    // All completed: nothing ready
    completed.insert("t3".to_string());
    let ready = Coordinator::ready_tasks(&plan, &completed);
    assert!(ready.is_empty());
}

#[test]
fn test_update_task_status() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-4";
    ensure_session(&db, session_id);

    let plan = Planner::create_plan(&db, session_id, "Some task").unwrap();
    let task_id = &plan.tasks[0].id;

    Coordinator::update_task_status(&db, task_id, "running", None, None).unwrap();

    {
        let conn = db.conn();
        let status: String = conn
            .query_row(
                "SELECT status FROM tasks WHERE id = ?1",
                rusqlite::params![task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "running");
    }

    Coordinator::update_task_status(&db, task_id, "completed", Some("output here"), Some("warn")).unwrap();

    {
        let conn = db.conn();
        let (status, stdout, stderr): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT status, stdout, stderr FROM tasks WHERE id = ?1",
                rusqlite::params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "completed");
        assert_eq!(stdout.unwrap(), "output here");
        assert_eq!(stderr.unwrap(), "warn");
    }
}

#[test]
fn test_update_goal_status() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-5";
    ensure_session(&db, session_id);

    let plan = Planner::create_plan(&db, session_id, "A goal").unwrap();

    Coordinator::update_goal_status(&db, &plan.goal_id, "running").unwrap();

    {
        let conn = db.conn();
        let status: String = conn
            .query_row(
                "SELECT status FROM goals WHERE id = ?1",
                rusqlite::params![plan.goal_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "running");
    }

    Coordinator::update_goal_status(&db, &plan.goal_id, "completed").unwrap();

    {
        let conn = db.conn();
        let status: String = conn
            .query_row(
                "SELECT status FROM goals WHERE id = ?1",
                rusqlite::params![plan.goal_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "completed");
    }
}

#[test]
fn test_goal_status() {
    let (_dir, db) = setup_db();
    let session_id = "test-session-6";
    ensure_session(&db, session_id);

    let plan = Planner::create_plan(&db, session_id, "Deploy application").unwrap();
    let task_id = &plan.tasks[0].id;

    Coordinator::update_task_status(&db, task_id, "completed", Some("done"), None).unwrap();
    Coordinator::update_goal_status(&db, &plan.goal_id, "completed").unwrap();

    let status = Coordinator::goal_status(&db, &plan.goal_id).unwrap();
    assert_eq!(status.goal_id, plan.goal_id);
    assert_eq!(status.description, "Deploy application");
    assert_eq!(status.status, "completed");
    assert_eq!(status.tasks.len(), 1);
    assert_eq!(status.tasks[0].id, *task_id);
    assert_eq!(status.tasks[0].status, "completed");
}

// ── Spawner tests ─────────────────────────────────────────────────────

#[test]
fn test_build_command() {
    let spawner = Spawner::new("/usr/local/bin/claude", "opus");
    let dir = TempDir::new().unwrap();

    let cmd = spawner.build_command("Hello world", dir.path(), None);
    let as_std = cmd.as_std();

    assert_eq!(as_std.get_program(), "/usr/local/bin/claude");

    let args: Vec<&std::ffi::OsStr> = as_std.get_args().collect();
    assert!(args.contains(&std::ffi::OsStr::new("--print")));
    assert!(args.contains(&std::ffi::OsStr::new("--output-format")));
    assert!(args.contains(&std::ffi::OsStr::new("json")));
    assert!(args.contains(&std::ffi::OsStr::new("--model")));
    assert!(args.contains(&std::ffi::OsStr::new("opus")));
    assert!(args.contains(&std::ffi::OsStr::new("--dangerously-skip-permissions")));
    assert!(args.contains(&std::ffi::OsStr::new("--prompt")));
    assert!(args.contains(&std::ffi::OsStr::new("Hello world")));
    // No system prompt args
    assert!(!args.contains(&std::ffi::OsStr::new("--system-prompt")));
}

#[test]
fn test_build_command_with_system_prompt() {
    let spawner = Spawner::new("/usr/local/bin/claude", "sonnet");
    let dir = TempDir::new().unwrap();

    let cmd = spawner.build_command("Do something", dir.path(), Some("You are a helpful agent"));
    let as_std = cmd.as_std();

    let args: Vec<&std::ffi::OsStr> = as_std.get_args().collect();
    assert!(args.contains(&std::ffi::OsStr::new("--system-prompt")));
    assert!(args.contains(&std::ffi::OsStr::new("You are a helpful agent")));
    assert!(args.contains(&std::ffi::OsStr::new("--model")));
    assert!(args.contains(&std::ffi::OsStr::new("sonnet")));
}
