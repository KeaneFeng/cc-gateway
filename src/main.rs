mod balance;
mod commands;
mod config;
mod database;
mod error;
mod interactive;
mod provider;
mod proxy;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use clap::{CommandFactory, Parser, Subcommand};
use std::sync::{Arc, OnceLock};
use tracing_subscriber::EnvFilter;

/// Reloadable log filter handle for dynamic log level changes without restart.
/// Stored as a global static, initialized once in `serve()`.
static LOG_FILTER_HANDLE: OnceLock<
    Arc<tracing_subscriber::reload::Handle<EnvFilter, tracing_subscriber::Registry>>,
> = OnceLock::new();

/// Update the log level at runtime. Called from TUI toggle and /api/reload.
pub fn set_log_level(level: &str) {
    if let Some(handle) = LOG_FILTER_HANDLE.get() {
        let filter = EnvFilter::new(level);
        if let Err(e) = handle.modify(|f| *f = filter) {
            tracing::warn!("Failed to update log level: {}", e);
        } else {
            tracing::info!("Log level changed to: {}", level);
        }
    }
}

/// cc-gateway: Multi-provider aggregation gateway for Claude Code
#[derive(Parser)]
#[command(name = "cc-gateway")]
#[command(version)]
#[command(about = "Multi-provider aggregation gateway for Claude Code", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the server (foreground by default, --daemon for background)
    Start {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
        /// Server port (overrides config)
        #[arg(short, long)]
        port: Option<u16>,
        /// Server host (overrides config)
        #[arg(long)]
        host: Option<String>,
        /// Run in background
        #[arg(short, long)]
        daemon: bool,
        /// Force: stop existing server before starting
        #[arg(short, long)]
        force: bool,
    },

    /// Stop the running server
    Stop,

    /// Restart the server (stop + start)
    Restart {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-gateway/config.toml")]
        config: String,
        /// Server port (overrides config)
        #[arg(short, long)]
        port: Option<u16>,
        /// Server host (overrides config)
        #[arg(long)]
        host: Option<String>,
        /// Run in background
        #[arg(short, long)]
        daemon: bool,
    },

    /// Internal: run the proxy server directly (used by start/restart)
    #[command(hide = true)]
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
            Commands::Start {
                config,
                port,
                host,
                daemon,
                force,
            } => {
                commands::serve::run_start(&expand_path(&config), port, host, daemon, force)?;
            }
            Commands::Stop => {
                commands::serve::run_stop()?;
            }
            Commands::Restart {
                config,
                port,
                host,
                daemon,
            } => {
                commands::serve::run_restart(&expand_path(&config), port, host, daemon)?;
            }
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
                let cfg = crate::config::AppConfig::load(&path).unwrap_or_default();
                interactive::set_default(&path, id.as_deref(), cfg.port)?;
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

    // Update ~/.claude/settings.json to point to cc-gateway
    update_global_claude_settings(config.port)?;

    // Use reloadable filter so log level can be changed at runtime without restart
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    let (filter_layer, filter_handle) = tracing_subscriber::reload::Layer::new(filter);
    LOG_FILTER_HANDLE.set(Arc::new(filter_handle)).ok();

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting cc-gateway on {}:{}", config.host, config.port);

    let state =
        proxy::handlers::AppState::new(config.clone(), config_path.to_string_lossy().to_string())?;
    let app = Router::new()
        .route("/v1/models", get(proxy::handlers::list_models))
        .route("/v1/messages", post(proxy::handlers::handle_messages))
        .route("/health", get(proxy::handlers::health_check))
        .route("/status", get(proxy::handlers::get_status))
        // Per-provider routing: /provider/:provider_id/v1/messages
        .route(
            "/provider/:provider_id/v1/messages",
            post(proxy::handlers::handle_messages_for_provider),
        )
        .route(
            "/provider/:provider_id/v1/models",
            get(proxy::handlers::list_models_for_provider),
        )
        // Provider switching API (runtime, no restart needed)
        .route(
            "/api/switch-provider",
            post(proxy::handlers::switch_provider),
        )
        .route(
            "/api/current-provider",
            get(proxy::handlers::get_current_provider),
        )
        .route("/api/reload", post(proxy::handlers::reload_config))
        // Increase body size limit to 200MB (matching cc-switch)
        // Large Claude Code sessions can exceed default 2MB limit
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Update ~/.claude/settings.json to point to cc-gateway
fn update_global_claude_settings(port: u16) -> anyhow::Result<()> {
    let settings_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json");

    if !settings_path.exists() {
        // Create minimal settings file
        let settings = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": format!("http://127.0.0.1:{}", port),
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
            }
        });
        std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        tracing::info!("Created ~/.claude/settings.json with cc-gateway proxy");
        return Ok(());
    }

    // Read existing settings
    let content = std::fs::read_to_string(&settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    // Update ANTHROPIC_BASE_URL
    let base_url = format!("http://127.0.0.1:{}", port);

    if settings.get("env").is_none() {
        settings["env"] = serde_json::json!({});
    }

    if let Some(env) = settings.get_mut("env") {
        if let Some(obj) = env.as_object_mut() {
            obj.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                serde_json::json!(base_url),
            );
            obj.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                serde_json::json!("PROXY_MANAGED"),
            );
        }
    }

    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    tracing::info!("Updated ~/.claude/settings.json → {}", base_url);

    Ok(())
}

fn expand_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
