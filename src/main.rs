use clap::{Parser, Subcommand};
use rmcp::ServiceExt;

use opencheir::config::{expand_tilde, Config};
use opencheir::gateway::server::OpenCheirServer;
use opencheir::store::state::StateDb;
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::watch;

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
        Commands::Serve { config: config_path } => {
            use opencheir::orchestration::enforcer::Enforcer;

            let config_path = expand_tilde(&config_path);
            let cfg = match Config::load(std::path::Path::new(&config_path)) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("failed to read") {
                        // Config file doesn't exist yet — use defaults (fresh install)
                        Config::default()
                    } else {
                        // Parse error — fail loudly so operators don't silently lose custom rules
                        return Err(e);
                    }
                }
            };
            let data_dir = expand_tilde(&cfg.general.data_dir);
            let db_path = std::path::Path::new(&data_dir).join("opencheir.db");
            let db = StateDb::open(&db_path)?;

            // Seed built-in rules (INSERT OR IGNORE), then TOML overrides (INSERT OR REPLACE)
            Enforcer::seed_builtins_to_db(&db)?;
            if !cfg.enforcer.rules.is_empty() {
                Enforcer::seed_config_rules_to_db(&db, &cfg.enforcer.rules)?;
            }

            // Load initial rule-set from DB into in-memory enforcer
            let enforcer = {
                let mut e = Enforcer::new();
                e.reload_from_db(&db)?;
                std::sync::Arc::new(std::sync::Mutex::new(e))
            };

            // ── File watcher for config hot-reload ───────────────────────────────────
            let (reload_tx, reload_rx) = watch::channel(());

            let mut watcher = {
                let tx = reload_tx.clone();
                recommended_watcher(move |res: notify::Result<Event>| {
                    if res.map(|e| e.kind.is_modify() || e.kind.is_create()).unwrap_or(false) {
                        let _ = tx.send(());
                    }
                })?
            };
            watcher.watch(
                std::path::Path::new(&config_path),
                RecursiveMode::NonRecursive,
            )?;

            // Background task: re-seed DB and reload enforcer in-memory on config change
            {
                let enforcer_arc = std::sync::Arc::clone(&enforcer);
                let db_watch = db.clone();
                let path_watch = config_path.clone();
                tokio::spawn(async move {
                    let mut rx = reload_rx;
                    loop {
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let new_cfg = match Config::load(std::path::Path::new(&path_watch)) {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::warn!("config reload failed: {e}");
                                continue;
                            }
                        };
                        if let Err(e) = Enforcer::seed_config_rules_to_db(&db_watch, &new_cfg.enforcer.rules) {
                            tracing::warn!("seed rules on reload failed: {e}");
                            continue;
                        }
                        let mut e = enforcer_arc.lock().unwrap();
                        if let Err(e) = e.reload_from_db(&db_watch) {
                            tracing::warn!("reload_from_db failed: {e}");
                        } else {
                            tracing::info!("enforcer rules hot-reloaded from {path_watch}");
                        }
                    }
                });
            }

            // Keep watcher alive until server exits
            let _watcher = watcher;

            let server = OpenCheirServer::new(db, enforcer);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
    }

    Ok(())
}
