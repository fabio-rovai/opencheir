use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug)]
pub struct AgentResult {
    pub task_id: String,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub struct Spawner {
    pub claude_path: String,
    pub model: String,
}

impl Spawner {
    pub fn new(claude_path: &str, model: &str) -> Self {
        Self {
            claude_path: claude_path.to_string(),
            model: model.to_string(),
        }
    }

    /// Spawn a Claude CLI agent for a task. Returns the result.
    /// Uses --print --output-format json --dangerously-skip-permissions
    pub async fn spawn_agent(
        &self,
        task_id: &str,
        prompt: &str,
        working_dir: &Path,
        system_prompt: Option<&str>,
    ) -> Result<AgentResult> {
        let mut cmd = self.build_command(prompt, working_dir, system_prompt);

        let output = cmd.output().await?;

        Ok(AgentResult {
            task_id: task_id.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }

    /// Build the command (factored out for testing).
    pub fn build_command(
        &self,
        prompt: &str,
        working_dir: &Path,
        system_prompt: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new(&self.claude_path);
        cmd.arg("--print")
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(&self.model)
            .arg("--dangerously-skip-permissions");

        if let Some(sp) = system_prompt {
            cmd.arg("--system-prompt").arg(sp);
        }

        cmd.arg("--prompt").arg(prompt);
        cmd.current_dir(working_dir);

        cmd
    }
}
