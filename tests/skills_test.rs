use opencheir::orchestration::skills::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: create a skill directory with a SKILL.md file.
fn create_skill(base: &std::path::Path, dir_name: &str, content: &str) -> PathBuf {
    let skill_dir = base.join(dir_name);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    skill_dir
}

/// Helper: build standard frontmatter content.
fn skill_content(name: &str, description: &str, command: Option<&str>) -> String {
    let mut s = String::from("---\n");
    s.push_str(&format!("name: {}\n", name));
    s.push_str(&format!("description: {}\n", description));
    if let Some(cmd) = command {
        s.push_str(&format!("command: {}\n", cmd));
    }
    s.push_str("---\n\n# Content\n\nSome body text.\n");
    s
}

#[test]
fn test_scan_finds_skills() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Help brainstorm ideas", Some("brainstorm")),
    );
    create_skill(
        builtin.path(),
        "debugging",
        &skill_content("debugging", "Debug issues", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skills = engine.list();
    assert_eq!(skills.len(), 2);
}

#[test]
fn test_list_returns_sorted() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "zebra",
        &skill_content("zebra", "Z skill", None),
    );
    create_skill(
        builtin.path(),
        "alpha",
        &skill_content("alpha", "A skill", None),
    );
    create_skill(
        builtin.path(),
        "mid",
        &skill_content("mid", "M skill", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skills = engine.list();
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mid", "zebra"]);
}

#[test]
fn test_get_existing_skill() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Help brainstorm ideas", Some("brainstorm")),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skill = engine.get("brainstorming").unwrap();
    assert_eq!(skill.name, "brainstorming");
    assert_eq!(skill.description, "Help brainstorm ideas");
    assert_eq!(skill.command.as_deref(), Some("brainstorm"));
    assert_eq!(skill.source, SkillSource::Builtin);
}

#[test]
fn test_get_nonexistent() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    assert!(engine.get("nonexistent").is_none());
}

#[test]
fn test_get_content() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    let content = skill_content("brainstorming", "Help brainstorm ideas", Some("brainstorm"));
    create_skill(builtin.path(), "brainstorming", &content);

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let retrieved = engine.get_content("brainstorming").unwrap();
    assert_eq!(retrieved, content);
}

#[test]
fn test_sub_documents() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    let skill_dir = create_skill(
        builtin.path(),
        "debugging",
        &skill_content("debugging", "Debug issues", None),
    );
    fs::write(skill_dir.join("root-cause-tracing.md"), "# Root Cause\n").unwrap();
    fs::write(skill_dir.join("common-errors.md"), "# Common Errors\n").unwrap();

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skill = engine.get("debugging").unwrap();
    let mut sub_docs = skill.sub_documents.clone();
    sub_docs.sort();
    assert_eq!(sub_docs, vec!["common-errors.md", "root-cause-tracing.md"]);
}

#[test]
fn test_get_sub_document() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    let skill_dir = create_skill(
        builtin.path(),
        "debugging",
        &skill_content("debugging", "Debug issues", None),
    );
    let sub_content = "# Root Cause Tracing\n\nDetailed guide here.\n";
    fs::write(skill_dir.join("root-cause-tracing.md"), sub_content).unwrap();

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let retrieved = engine.get_sub_document("debugging", "root-cause-tracing.md").unwrap();
    assert_eq!(retrieved, sub_content);
}

#[test]
fn test_personal_shadows_builtin() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Builtin version", Some("brainstorm")),
    );
    create_skill(
        personal.path(),
        "brainstorming",
        &skill_content("brainstorming", "Personal version", Some("brainstorm")),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skill = engine.get("brainstorming").unwrap();
    assert_eq!(skill.description, "Personal version");
    assert_eq!(skill.source, SkillSource::Personal);

    // Only one skill with that name should exist
    let skills = engine.list();
    let brainstorm_count = skills.iter().filter(|s| s.name == "brainstorming").count();
    assert_eq!(brainstorm_count, 1);
}

#[test]
fn test_health_healthy() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Help brainstorm", None),
    );
    create_skill(
        builtin.path(),
        "debugging",
        &skill_content("debugging", "Debug issues", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let health = engine.health();
    assert_eq!(health.len(), 2);
    assert!(health.iter().all(|h| h.healthy));
    assert!(health.iter().all(|h| h.error.is_none()));
}

#[test]
fn test_health_deleted() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Help brainstorm", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());

    // Delete the SKILL.md after scan
    fs::remove_file(builtin.path().join("brainstorming").join("SKILL.md")).unwrap();

    let health = engine.health();
    assert_eq!(health.len(), 1);
    assert!(!health[0].healthy);
    assert!(health[0].error.is_some());
}

#[test]
fn test_resolve_is_same_as_get() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "brainstorming",
        &skill_content("brainstorming", "Help brainstorm", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let by_get = engine.get("brainstorming");
    let by_resolve = engine.resolve("brainstorming");
    assert!(by_get.is_some());
    assert!(by_resolve.is_some());
    assert_eq!(by_get.unwrap().name, by_resolve.unwrap().name);
    assert_eq!(by_get.unwrap().description, by_resolve.unwrap().description);

    // Both return None for nonexistent
    assert!(engine.get("nope").is_none());
    assert!(engine.resolve("nope").is_none());
}

#[test]
fn test_empty_directories() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skills = engine.list();
    assert!(skills.is_empty());
}

#[test]
fn test_frontmatter_with_command() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    create_skill(
        builtin.path(),
        "review",
        &skill_content("review", "Review code changes", Some("review-pr")),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skill = engine.get("review").unwrap();
    assert_eq!(skill.command.as_deref(), Some("review-pr"));
}

#[test]
fn test_scan_ignores_non_skill_dirs() {
    let builtin = TempDir::new().unwrap();
    let personal = TempDir::new().unwrap();

    // Create a regular file at top level (not a directory)
    fs::write(builtin.path().join("not-a-skill.txt"), "just a file").unwrap();
    // Create a directory without SKILL.md
    fs::create_dir_all(builtin.path().join("no-skill-md")).unwrap();
    fs::write(
        builtin.path().join("no-skill-md").join("README.md"),
        "not a skill",
    )
    .unwrap();
    // Create a valid skill
    create_skill(
        builtin.path(),
        "real-skill",
        &skill_content("real-skill", "A real skill", None),
    );

    let engine = SkillsEngine::new(builtin.path().to_path_buf(), personal.path().to_path_buf());
    let skills = engine.list();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "real-skill");
}
