mod commands;
mod config;
mod database;
mod error;
mod interactive;
mod provider;
mod proxy;

use axum::{
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand, CommandFactory};
use tracing_subscriber::EnvFilter;

/// cc-gateway: Multi-provider aggregation gateway for Claude Code
#[derive(Parser)]
#[command(name = "cc-gateway")]
#[command(version = "0.3.0")]
#[command(about = "Multi-provider aggregation gateway for Claude Code", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the proxy server (foreground)
    Serve {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
        /// Server port (overrides config)
        #[arg(short, long)]
        port: Option<u16>,
        /// Server host (overrides config)
        #[arg(long)]
        host: Option<String>,
    },

    /// Add a provider (interactive or from preset)
    Add {
        /// Preset ID (e.g., mimo, kimi, qwen, glm)
        preset: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Edit a provider
    Edit {
        /// Provider ID to edit
        id: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Remove a provider
    Remove {
        /// Provider ID to remove
        id: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Set default provider
    Default {
        /// Provider ID to set as default
        id: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Test provider connections
    Test {
        /// Test specific provider ID
        id: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Show provider status
    Status {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Import providers from cc-switch
    Import {
        /// Path to cc-switch database
        #[arg(long)]
        db: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Browse available presets
    Presets {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
    },

    /// Show or edit configuration
    Config {
        /// Config key to set
        #[arg(long)]
        set: Option<String>,
        /// Value to set
        #[arg(long)]
        value: Option<String>,
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
    },

    /// Generate shell completions
    Completion {
        /// Shell type
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // No subcommand = interactive dashboard
        None => {
            interactive::run_dashboard(&expand_path("~/.cc-gateway/config.toml"))?;
        }
        Some(cmd) => match cmd {
            Commands::Serve { config, port, host } => {
                serve(config, port, host).await?;
            }
            Commands::Add { preset, config } => {
                let path = std::path::PathBuf::from(expand_path(&config));
                interactive::add_provider(&path, preset.as_deref())?;
            }
            Commands::Edit { id, config } => {
                let path = std::path::PathBuf::from(expand_path(&config));
                interactive::edit_provider(&path, id.as_deref())?;
            }
            Commands::Remove { id, config } => {
                let path = std::path::PathBuf::from(expand_path(&config));
                interactive::remove_provider(&path, id.as_deref())?;
            }
            Commands::Default { id, config } => {
                let path = std::path::PathBuf::from(expand_path(&config));
                interactive::set_default(&path, id.as_deref())?;
            }
            Commands::Test { id, config } => {
                commands::test::run_test(&expand_path(&config), id.as_deref()).await?;
            }
            Commands::Status { config } => {
                commands::status::show_status(&expand_path(&config))?;
            }
            Commands::Import { db, config } => {
                let path = std::path::PathBuf::from(expand_path(&config));
                interactive::import_providers(&path, db.as_deref())?;
            }
            Commands::Presets { category } => {
                commands::presets::show_presets(category.as_deref())?;
            }
            Commands::Config { set, value, config } => {
                if let (Some(key), Some(val)) = (set, value) {
                    commands::config::set_config(&expand_path(&config), &key, &val)?;
                } else {
                    commands::config::show_config(&expand_path(&config))?;
                }
            }
            Commands::Completion { shell } => {
                let mut cmd = Cli::command();
                clap_complete::generate(shell, &mut cmd, "cc-gateway", &mut std::io::stdout());
            }
        },
    }

    Ok(())
}

async fn serve(config_path: String, port: Option<u16>, host: Option<String>) -> anyhow::Result<()> {
    let config_path = std::path::PathBuf::from(expand_path(&config_path));
    let mut config = if config_path.exists() {
        config::AppConfig::load(&config_path)?
    } else {
        tracing::warn!("Config file not found, using default config");
        config::AppConfig::default()
    };

    if let Some(port) = port {
        config.port = port;
    }
    if let Some(host) = host {
        config.host = host;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)))
        .init();

    tracing::info!("Starting cc-gateway on {}:{}", config.host, config.port);

    let state = proxy::handlers::AppState::new(config.clone())?;
    let app = Router::new()
        .route("/v1/models", get(proxy::handlers::list_models))
        .route("/v1/messages", post(proxy::handlers::handle_messages))
        .route("/health", get(proxy::handlers::health_check))
        .route("/status", get(proxy::handlers::get_status))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn expand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
