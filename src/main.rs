use clap::{Parser, Subcommand};
use rmcp::ServiceExt;

use opencheir::config::expand_tilde;
use opencheir::gateway::server::OpenCheirServer;
use opencheir::sentinel_core::state::StateDb;

const DEFAULT_CONFIG: &str = r#"[general]
data_dir = "~/.opencheir"
skills_dir = "~/.opencheir/skills"
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
#[command(name = "opencheir", about = "One brain to rule them all")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize opencheir: create DB and default config
    Init {
        /// Path to opencheir data directory
        #[arg(long, default_value = "~/.opencheir")]
        data_dir: String,
    },
    /// Start the MCP server
    Serve {
        /// Path to config file
        #[arg(long, default_value = "~/.opencheir/config.toml")]
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
            let db_path = data_path.join("opencheir.db");
            let db = StateDb::open(&db_path)?;
            println!("Initialized database: {}", db_path.display());

            // Create default config
            let config_path = data_path.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, DEFAULT_CONFIG)?;
                println!("Created default config: {}", config_path.display());
            } else {
                println!("Config already exists: {}", config_path.display());
            }

            println!("\nOpenCheir initialized successfully!");
        }
        Commands::Serve { config: _config } => {
            // TODO: load config and use data_dir from it
            let data_dir = expand_tilde("~/.opencheir");
            let db_path = std::path::Path::new(&data_dir).join("opencheir.db");
            let db = StateDb::open(&db_path)?;
            let server = OpenCheirServer::new(db);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
    }

    Ok(())
}
