use clap::{Parser, Subcommand};
use rmcp::ServiceExt;

use sentinel::config::expand_tilde;
use sentinel::gateway::server::SentinelServer;
use sentinel::sentinel_core::state::StateDb;

const DEFAULT_CONFIG: &str = r#"[general]
data_dir = "~/.sentinel"
tenders_root = "~/Desktop/Tenders"
skills_dir = "~/.sentinel/skills"
personal_skills_dir = "~/.claude/skills"

[supervisor]
health_check_interval = "5s"
max_restart_attempts = 3
restart_cooldown = "60s"

[enforcer]
enabled = true
default_action = "block"

[hive]
max_agents = 5
claude_path = "claude"
default_model = "claude-sonnet-4-6"
agent_timeout = "300s"

[eyes]
port = 0
max_image_width = 800
"#;

#[derive(Parser)]
#[command(name = "sentinel", about = "One brain to rule them all")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize sentinel: create DB, seed data, update settings
    Init {
        /// Path to sentinel data directory
        #[arg(long, default_value = "~/.sentinel")]
        data_dir: String,
    },
    /// Start the MCP server
    Serve {
        /// Path to config file
        #[arg(long, default_value = "~/.sentinel/config.toml")]
        config: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { data_dir } => {
            let data_dir = expand_tilde(&data_dir);
            let data_path = std::path::Path::new(&data_dir);

            // Create directory
            std::fs::create_dir_all(data_path)?;
            println!("Created data directory: {data_dir}");

            // Create DB
            let db_path = data_path.join("sentinel.db");
            let db = StateDb::open(&db_path)?;
            println!("Initialized database: {}", db_path.display());

            // Seed company data
            db.seed_company()?;
            println!("Seeded company data");

            // Create default config
            let config_path = data_path.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, DEFAULT_CONFIG)?;
                println!("Created default config: {}", config_path.display());
            } else {
                println!("Config already exists: {}", config_path.display());
            }

            println!("\nSentinel initialized successfully!");
        }
        Commands::Serve { config: _config } => {
            // TODO: load config and use data_dir from it
            let data_dir = expand_tilde("~/.sentinel");
            let db_path = std::path::Path::new(&data_dir).join("sentinel.db");
            let db = StateDb::open(&db_path)?;
            let server = SentinelServer::new(db);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
    }

    Ok(())
}
