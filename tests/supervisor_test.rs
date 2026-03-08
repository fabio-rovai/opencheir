use opencheir::orchestration::supervisor::{ExternalProcess, RestartTracker, Supervisor};
use std::collections::HashMap;
use opencheir::config::ExternalServerConfig;

#[tokio::test]
async fn test_spawn_process() {
    // Use 'cat' as a mock MCP server -- it reads stdin, writes to stdout
    let proc = ExternalProcess::spawn("test", "cat", &[]).await.unwrap();
    assert!(proc.pid().is_some());
}

#[tokio::test]
async fn test_spawn_and_kill() {
    let mut proc = ExternalProcess::spawn("test", "cat", &[]).await.unwrap();
    assert!(proc.is_running());
    proc.kill().await.unwrap();
    // After kill, process should no longer be running
    assert!(!proc.is_running());
}

#[tokio::test]
async fn test_spawn_invalid_command() {
    let result = ExternalProcess::spawn("bad", "nonexistent_command_xyz", &[]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_process_name() {
    let proc = ExternalProcess::spawn("my-server", "cat", &[]).await.unwrap();
    assert_eq!(proc.name, "my-server");
}

#[tokio::test]
async fn test_supervisor_spawn_all() {
    let mut servers = HashMap::new();
    servers.insert("test1".to_string(), ExternalServerConfig {
        command: "cat".to_string(),
        args: vec![],
        enabled: true,
    });
    servers.insert("test2".to_string(), ExternalServerConfig {
        command: "cat".to_string(),
        args: vec![],
        enabled: false, // disabled -- should not spawn
    });

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    assert_eq!(supervisor.processes.len(), 1);
    assert!(supervisor.processes.contains_key("test1"));
    assert!(!supervisor.processes.contains_key("test2"));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_supervisor_get_status() {
    let mut servers = HashMap::new();
    servers.insert("running".to_string(), ExternalServerConfig {
        command: "cat".to_string(),
        args: vec![],
        enabled: true,
    });

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    let status = supervisor.get_status();
    assert_eq!(status.get("running"), Some(&true));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_supervisor_shutdown_clears_processes() {
    let mut servers = HashMap::new();
    servers.insert("srv".to_string(), ExternalServerConfig {
        command: "cat".to_string(),
        args: vec![],
        enabled: true,
    });

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();
    assert_eq!(supervisor.processes.len(), 1);

    supervisor.shutdown().await.unwrap();
    assert!(supervisor.processes.is_empty());
}

#[tokio::test]
async fn test_supervisor_empty_config() {
    let servers: HashMap<String, ExternalServerConfig> = HashMap::new();
    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();
    assert!(supervisor.processes.is_empty());
}

#[tokio::test]
async fn test_supervisor_spawn_invalid_command_continues() {
    let mut servers = HashMap::new();
    servers.insert("bad".to_string(), ExternalServerConfig {
        command: "nonexistent_command_xyz".to_string(),
        args: vec![],
        enabled: true,
    });
    servers.insert("good".to_string(), ExternalServerConfig {
        command: "cat".to_string(),
        args: vec![],
        enabled: true,
    });

    let mut supervisor = Supervisor::new();
    // Should not error -- bad process is logged, good one succeeds
    supervisor.spawn_all(&servers).await.unwrap();

    // Only the good process should be present
    assert!(supervisor.processes.contains_key("good"));
    assert!(!supervisor.processes.contains_key("bad"));

    supervisor.shutdown().await.unwrap();
}

// -- Task 3.2: Health Check Tests --

#[tokio::test]
async fn test_health_check_running() {
    let mut servers = HashMap::new();
    servers.insert(
        "test".to_string(),
        ExternalServerConfig {
            command: "cat".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    let health = supervisor.check_health();
    assert_eq!(health.len(), 1);
    assert_eq!(health[0], ("test".to_string(), true));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_health_check_dead_process() {
    // 'true' exits immediately, so the process will be dead by health check time
    let mut servers = HashMap::new();
    servers.insert(
        "ephemeral".to_string(),
        ExternalServerConfig {
            command: "true".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    // Give the process time to exit.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let health = supervisor.check_health();
    assert_eq!(health.len(), 1);
    assert_eq!(health[0], ("ephemeral".to_string(), false));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_health_check_multiple_processes() {
    let mut servers = HashMap::new();
    servers.insert(
        "alive".to_string(),
        ExternalServerConfig {
            command: "cat".to_string(),
            args: vec![],
            enabled: true,
        },
    );
    servers.insert(
        "dead".to_string(),
        ExternalServerConfig {
            command: "true".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let health = supervisor.check_health();
    assert_eq!(health.len(), 2);

    let health_map: HashMap<String, bool> = health.into_iter().collect();
    assert_eq!(health_map.get("alive"), Some(&true));
    assert_eq!(health_map.get("dead"), Some(&false));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_record_health_in_db() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();

    db.record_health("mcp-server-1", "process", "healthy", None)
        .unwrap();

    let result = db.get_health("mcp-server-1").unwrap().unwrap();
    assert_eq!(result.0, "process"); // kind
    assert_eq!(result.1, "healthy"); // status
    assert_eq!(result.2, None); // error
    assert_eq!(result.3, 0); // restart_count
}

#[tokio::test]
async fn test_record_health_with_error() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();

    db.record_health("mcp-server-2", "process", "down", Some("process exited"))
        .unwrap();

    let result = db.get_health("mcp-server-2").unwrap().unwrap();
    assert_eq!(result.1, "down");
    assert_eq!(result.2, Some("process exited".to_string()));
}

#[tokio::test]
async fn test_record_health_upsert() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();

    // Insert initial healthy status.
    db.record_health("srv", "process", "healthy", None).unwrap();
    let r1 = db.get_health("srv").unwrap().unwrap();
    assert_eq!(r1.1, "healthy");

    // Update to down -- should upsert, not insert a second row.
    db.record_health("srv", "process", "down", Some("crashed"))
        .unwrap();
    let r2 = db.get_health("srv").unwrap().unwrap();
    assert_eq!(r2.1, "down");
    assert_eq!(r2.2, Some("crashed".to_string()));
}

#[tokio::test]
async fn test_increment_restart_count() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = opencheir::store::state::StateDb::open(tmp.path()).unwrap();

    db.record_health("srv", "process", "restarting", None)
        .unwrap();
    db.increment_restart_count("srv").unwrap();
    db.increment_restart_count("srv").unwrap();

    let result = db.get_health("srv").unwrap().unwrap();
    assert_eq!(result.3, 2); // restart_count
}

// -- Task 3.3: Auto-Restart Tests --

#[tokio::test]
async fn test_restart_tracker_allows_retries() {
    let mut tracker = RestartTracker::new(3, 60);
    assert!(tracker.record_failure());  // 1st
    assert!(tracker.record_failure());  // 2nd
    assert!(tracker.record_failure());  // 3rd
    assert!(!tracker.record_failure()); // 4th -- exceeded
    assert!(tracker.is_exhausted());
}

#[tokio::test]
async fn test_restart_tracker_not_exhausted_initially() {
    let tracker = RestartTracker::new(3, 60);
    assert!(!tracker.is_exhausted());
}

#[tokio::test]
async fn test_restart_tracker_single_attempt() {
    let mut tracker = RestartTracker::new(1, 60);
    assert!(tracker.record_failure());  // 1st -- allowed
    assert!(!tracker.record_failure()); // 2nd -- exceeded
    assert!(tracker.is_exhausted());
}

#[tokio::test]
async fn test_check_and_restart_healthy_processes() {
    // All processes alive -- check_and_restart should report "healthy"
    let mut servers = HashMap::new();
    servers.insert(
        "alive".to_string(),
        ExternalServerConfig {
            command: "cat".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    let results = supervisor.check_and_restart().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], ("alive".to_string(), "healthy".to_string()));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_auto_restart_dead_process() {
    // Use 'cat' as the command -- it stays alive.
    // But we use 'true' which exits immediately, so it dies, then
    // check_and_restart should respawn it (with cat this time via config).
    // Instead: use 'cat' in config, manually kill it, then check_and_restart.
    let mut servers = HashMap::new();
    servers.insert(
        "srv".to_string(),
        ExternalServerConfig {
            command: "cat".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    // Kill the process to simulate a crash.
    supervisor
        .processes
        .get_mut("srv")
        .unwrap()
        .kill()
        .await
        .unwrap();

    // Small delay for the OS to register the kill.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let results = supervisor.check_and_restart().await;
    let result_map: HashMap<String, String> = results.into_iter().collect();
    assert_eq!(result_map.get("srv").map(|s| s.as_str()), Some("restarted"));

    // The restarted process should now be running.
    assert!(supervisor.processes.get_mut("srv").unwrap().is_running());

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_auto_restart_exhausts_retries() {
    // Use 'true' which exits immediately. After max_restarts (2), it should
    // be marked "down".
    let mut servers = HashMap::new();
    servers.insert(
        "flaky".to_string(),
        ExternalServerConfig {
            command: "true".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::with_config(2, 60);
    supervisor.spawn_all(&servers).await.unwrap();

    // Each round: 'true' exits immediately, check_and_restart restarts it.
    // After 2 restarts, the 3rd check should mark it "down".
    for _ in 0..2 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let results = supervisor.check_and_restart().await;
        let result_map: HashMap<String, String> = results.into_iter().collect();
        assert_eq!(
            result_map.get("flaky").map(|s| s.as_str()),
            Some("restarted")
        );
    }

    // Third check -- process has died again but restarts exhausted.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let results = supervisor.check_and_restart().await;
    let result_map: HashMap<String, String> = results.into_iter().collect();
    assert_eq!(result_map.get("flaky").map(|s| s.as_str()), Some("down"));

    // Process should have been removed.
    assert!(!supervisor.processes.contains_key("flaky"));

    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_with_config_constructor() {
    let supervisor = Supervisor::with_config(5, 120);
    assert!(supervisor.processes.is_empty());
    // Just verify it constructs without panic.
}

#[tokio::test]
async fn test_mixed_healthy_and_dead() {
    // One alive (cat), one dead (true). check_and_restart should report
    // "healthy" for cat and "restarted" for true.
    let mut servers = HashMap::new();
    servers.insert(
        "stable".to_string(),
        ExternalServerConfig {
            command: "cat".to_string(),
            args: vec![],
            enabled: true,
        },
    );
    servers.insert(
        "crasher".to_string(),
        ExternalServerConfig {
            command: "true".to_string(),
            args: vec![],
            enabled: true,
        },
    );

    let mut supervisor = Supervisor::new();
    supervisor.spawn_all(&servers).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let results = supervisor.check_and_restart().await;
    let result_map: HashMap<String, String> = results.into_iter().collect();

    assert_eq!(
        result_map.get("stable").map(|s| s.as_str()),
        Some("healthy")
    );
    assert_eq!(
        result_map.get("crasher").map(|s| s.as_str()),
        Some("restarted")
    );

    supervisor.shutdown().await.unwrap();
}
