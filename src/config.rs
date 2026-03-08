use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level configuration for OpenCheir.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub supervisor: SupervisorConfig,
    pub enforcer: EnforcerConfig,
    pub hive: HiveConfig,
    pub eyes: EyesConfig,
    pub search: SearchConfig,
    pub lineage: LineageConfig,
    #[serde(default)]
    pub external_servers: HashMap<String, ExternalServerConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            supervisor: SupervisorConfig::default(),
            enforcer: EnforcerConfig::default(),
            hive: HiveConfig::default(),
            eyes: EyesConfig::default(),
            search: SearchConfig::default(),
            lineage: LineageConfig::default(),
            external_servers: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// Missing sections/fields fall back to defaults via `#[serde(default)]`.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}

/// General paths and directories.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub data_dir: String,
    pub skills_dir: String,
    pub personal_skills_dir: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.opencheir".into(),
            skills_dir: "~/.opencheir/skills".into(),
            personal_skills_dir: "~/.claude/skills".into(),
        }
    }
}

/// Supervisor process management settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SupervisorConfig {
    pub health_check_interval: String,
    pub max_restart_attempts: u32,
    pub restart_cooldown: String,
    pub pattern_analysis_interval: u32,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            health_check_interval: "5s".into(),
            max_restart_attempts: 3,
            restart_cooldown: "60s".into(),
            pattern_analysis_interval: 100,
        }
    }
}

/// Policy enforcer settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnforcerConfig {
    pub enabled: bool,
    pub default_action: String,
}

impl Default for EnforcerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_action: "block".into(),
        }
    }
}

/// Hive agent orchestration settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct HiveConfig {
    pub max_agents: u32,
    pub claude_path: String,
    pub default_model: String,
    pub agent_timeout: String,
}

impl Default for HiveConfig {
    fn default() -> Self {
        Self {
            max_agents: 5,
            claude_path: "claude".into(),
            default_model: "claude-sonnet-4-6".into(),
            agent_timeout: "300s".into(),
        }
    }
}

/// Eyes (visual inspection) server settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EyesConfig {
    pub port: u16,
    pub max_image_width: u32,
}

impl Default for EyesConfig {
    fn default() -> Self {
        Self {
            port: 0,
            max_image_width: 800,
        }
    }
}

/// TF-IDF search settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    pub max_features: u32,
    pub ngram_range: [u32; 2],
    pub min_df: u32,
    pub max_df: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_features: 20_000,
            ngram_range: [1, 2],
            min_df: 2,
            max_df: 0.9,
        }
    }
}

/// Lineage tracking HTTP API settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LineageConfig {
    pub http_port: u16,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self { http_port: 0 }
    }
}

/// Configuration for an external MCP server managed by the supervisor.
#[derive(Debug, Clone, Deserialize)]
pub struct ExternalServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Expand a leading `~` or `~/` in a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}
