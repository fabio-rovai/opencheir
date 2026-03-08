use std::collections::HashMap;
use std::process::Stdio;
use std::time::Instant;

use tokio::io::{BufReader, BufWriter};
use tokio::process::{Child, Command};

use crate::config::ExternalServerConfig;

/// An external process managed by the supervisor.
///
/// Holds the child process and its stdin/stdout handles for MCP
/// communication over stdio transport.
pub struct ExternalProcess {
    pub name: String,
    pub child: Child,
    pub stdin: BufWriter<tokio::process::ChildStdin>,
    pub stdout: BufReader<tokio::process::ChildStdout>,
}

impl ExternalProcess {
    /// Spawn an external process with the given command and arguments.
    ///
    /// stdin and stdout are piped for MCP communication; stderr is
    /// suppressed so it does not leak into the server's own stderr.
    /// `kill_on_drop(true)` ensures the child is cleaned up if the
    /// handle is dropped.
    pub async fn spawn(name: &str, command: &str, args: &[String]) -> anyhow::Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = BufWriter::new(
            child
                .stdin
                .take()
                .expect("stdin was configured as piped but is missing"),
        );
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .expect("stdout was configured as piped but is missing"),
        );

        Ok(Self {
            name: name.to_string(),
            child,
            stdin,
            stdout,
        })
    }

    /// Return the OS process id, if still available.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Kill the child process.
    pub async fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill().await?;
        Ok(())
    }

    /// Check whether the child process is still running.
    ///
    /// Uses a non-blocking `try_wait` -- if no exit status is
    /// available yet the process is still alive.
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
}

/// Manages a set of external MCP server processes.
pub struct Supervisor {
    pub processes: HashMap<String, ExternalProcess>,
    configs: HashMap<String, ExternalServerConfig>,
    restart_trackers: HashMap<String, RestartTracker>,
    max_restart_attempts: u32,
    restart_cooldown_secs: u64,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            configs: HashMap::new(),
            restart_trackers: HashMap::new(),
            max_restart_attempts: 3,
            restart_cooldown_secs: 60,
        }
    }

    /// Create a supervisor with custom restart limits.
    pub fn with_config(max_restarts: u32, cooldown_secs: u64) -> Self {
        Self {
            processes: HashMap::new(),
            configs: HashMap::new(),
            restart_trackers: HashMap::new(),
            max_restart_attempts: max_restarts,
            restart_cooldown_secs: cooldown_secs,
        }
    }

    /// Spawn all enabled external servers from the given configuration.
    ///
    /// Servers that fail to spawn are logged but do not cause the
    /// entire operation to fail -- other servers are still started.
    pub async fn spawn_all(
        &mut self,
        servers: &HashMap<String, ExternalServerConfig>,
    ) -> anyhow::Result<()> {
        for (name, config) in servers {
            if !config.enabled {
                continue;
            }
            self.configs.insert(name.clone(), config.clone());
            match ExternalProcess::spawn(name, &config.command, &config.args).await {
                Ok(proc) => {
                    self.processes.insert(name.clone(), proc);
                }
                Err(e) => {
                    tracing::error!("Failed to spawn {name}: {e}");
                }
            }
        }
        Ok(())
    }

    /// Return a snapshot of which managed processes are still running.
    pub fn get_status(&mut self) -> HashMap<String, bool> {
        let mut status = HashMap::new();
        for (name, proc) in &mut self.processes {
            status.insert(name.clone(), proc.is_running());
        }
        status
    }

    /// Check health of all managed processes.
    /// Returns a list of (name, is_healthy) pairs.
    pub fn check_health(&mut self) -> Vec<(String, bool)> {
        let mut results = vec![];
        for (name, proc) in &mut self.processes {
            results.push((name.clone(), proc.is_running()));
        }
        results
    }

    /// Check all processes and restart any that have died (if under restart limit).
    /// Returns list of (name, action) where action is "healthy", "restarted", or "down".
    pub async fn check_and_restart(&mut self) -> Vec<(String, String)> {
        // First, collect names and their health status.
        let mut status: Vec<(String, bool)> = vec![];
        for (name, proc) in &mut self.processes {
            status.push((name.clone(), proc.is_running()));
        }

        let mut results = vec![];
        let mut to_restart: Vec<String> = vec![];

        for (name, healthy) in &status {
            if *healthy {
                results.push((name.clone(), "healthy".to_string()));
            } else {
                to_restart.push(name.clone());
            }
        }

        // Process restarts for dead processes.
        for name in to_restart {
            // Remove the dead process entry.
            self.processes.remove(&name);

            // Get or create a restart tracker for this process.
            let tracker = self
                .restart_trackers
                .entry(name.clone())
                .or_insert_with(|| {
                    RestartTracker::new(self.max_restart_attempts, self.restart_cooldown_secs)
                });

            if !tracker.record_failure() {
                // Exceeded restart limit.
                results.push((name, "down".to_string()));
                continue;
            }

            // Attempt respawn using the stored config.
            if let Some(config) = self.configs.get(&name) {
                match ExternalProcess::spawn(&name, &config.command, &config.args).await {
                    Ok(proc) => {
                        self.processes.insert(name.clone(), proc);
                        results.push((name, "restarted".to_string()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to restart {name}: {e}");
                        results.push((name, "down".to_string()));
                    }
                }
            } else {
                tracing::error!("No config stored for {name}, cannot restart");
                results.push((name, "down".to_string()));
            }
        }

        results
    }

    /// Kill all managed processes and clear the process map.
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        for (name, proc) in &mut self.processes {
            if let Err(e) = proc.kill().await {
                tracing::warn!("Failed to kill {name}: {e}");
            }
        }
        self.processes.clear();
        Ok(())
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks restart attempts for a single process with cooldown-based reset.
pub struct RestartTracker {
    pub count: u32,
    pub first_failure: Instant,
    pub max_attempts: u32,
    pub cooldown_secs: u64,
}

impl RestartTracker {
    pub fn new(max_attempts: u32, cooldown_secs: u64) -> Self {
        Self {
            count: 0,
            first_failure: Instant::now(),
            max_attempts,
            cooldown_secs,
        }
    }

    /// Record a failure. Returns true if we should retry, false if we've exceeded limits.
    pub fn record_failure(&mut self) -> bool {
        let now = Instant::now();
        // Reset counter if cooldown has elapsed.
        if now.duration_since(self.first_failure).as_secs() > self.cooldown_secs {
            self.count = 0;
            self.first_failure = now;
        }
        if self.count == 0 {
            self.first_failure = now;
        }
        self.count += 1;
        self.count <= self.max_attempts
    }

    pub fn is_exhausted(&self) -> bool {
        self.count > self.max_attempts
    }
}
