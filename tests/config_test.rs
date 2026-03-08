use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_load_config_from_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"
[general]
data_dir = "/tmp/sentinel-test"
tenders_root = "/tmp/tenders"

[supervisor]
health_check_interval = "5s"
max_restart_attempts = 3

[hive]
max_agents = 5
claude_path = "claude"
"#).unwrap();

    let config = sentinel::config::Config::load(f.path()).unwrap();
    assert_eq!(config.general.data_dir, "/tmp/sentinel-test");
    assert_eq!(config.hive.max_agents, 5);
}

#[test]
fn test_config_defaults() {
    let config = sentinel::config::Config::default();
    assert_eq!(config.hive.max_agents, 5);
    assert_eq!(config.supervisor.max_restart_attempts, 3);
}
