mod config;
mod error;
mod provider;
mod proxy;

use axum::{
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// CC-Switch-Pro: Lightweight multi-provider aggregation proxy for Claude Code
#[derive(Parser)]
#[command(name = "cc-switch-pro")]
#[command(version = "0.1.0")]
#[command(about = "Lightweight multi-provider aggregation proxy for Claude Code", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the proxy server
    Serve {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Server port (overrides config)
        #[arg(short, long)]
        port: Option<u16>,

        /// Server host (overrides config)
        #[arg(long)]
        host: Option<String>,
    },

    /// Generate example config file
    Init {
        /// Output path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        output: String,
    },

    /// List configured providers
    List {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,
    },

    /// Add a new provider
    Add {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Provider ID (e.g., mimo, kimi, glm)
        #[arg(short, long)]
        id: String,

        /// Provider name
        #[arg(short, long)]
        name: String,

        /// Base URL
        #[arg(short, long)]
        url: String,

        /// API key
        #[arg(short, long)]
        key: String,

        /// Model name
        #[arg(short, long)]
        model: String,

        /// Display name (optional)
        #[arg(long)]
        display_name: Option<String>,

        /// Set as default provider
        #[arg(long)]
        default: bool,
    },

    /// Remove a provider
    Remove {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Provider ID to remove
        #[arg(short, long)]
        id: String,
    },

    /// Set default provider
    SetDefault {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Provider ID to set as default
        #[arg(short, long)]
        id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, port, host } => {
            serve(config, port, host).await?;
        }
        Commands::Init { output } => {
            init(output)?;
        }
        Commands::List { config } => {
            list(config)?;
        }
        Commands::Add {
            config,
            id,
            name,
            url,
            key,
            model,
            display_name,
            default,
        } => {
            add(config, id, name, url, key, model, display_name, default)?;
        }
        Commands::Remove { config, id } => {
            remove(config, id)?;
        }
        Commands::SetDefault { config, id } => {
            set_default(config, id)?;
        }
    }

    Ok(())
}

async fn serve(config_path: String, port: Option<u16>, host: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);

    // Load config
    let mut config = if config_path.exists() {
        config::AppConfig::load(&config_path)?
    } else {
        tracing::warn!("Config file not found, using default config");
        config::AppConfig::default()
    };

    // Override with CLI args
    if let Some(port) = port {
        config.port = port;
    }
    if let Some(host) = host {
        config.host = host;
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .init();

    tracing::info!("Starting CC-Switch-Pro proxy...");
    tracing::info!("Config: {:?}", config_path);
    tracing::info!("Providers: {}", config.providers.len());

    // Create app state
    let state = proxy::handlers::AppState::new(config.clone())?;

    // Build router
    let app = Router::new()
        .route("/v1/models", get(proxy::handlers::list_models))
        .route("/v1/messages", post(proxy::handlers::handle_messages))
        .route("/health", get(proxy::handlers::health_check))
        .route("/status", get(proxy::handlers::get_status))
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init(output: String) -> anyhow::Result<()> {
    let output = expand_path(&output);
    let config = config::generate_example_config();
    config.save(&output)?;
    println!("Example config saved to: {}", output.display());
    println!("\nEdit the config file to add your API keys, then run:");
    println!("  cc-switch-pro serve");
    Ok(())
}

fn list(config_path: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let config = config::AppConfig::load(&config_path)?;

    println!("Configured providers:");
    println!("{:-<60}", "");

    for provider in &config.providers {
        let default_marker = if provider.is_default { " (default)" } else { "" };
        println!("ID:       {}", provider.id);
        println!("Name:     {}", provider.name);
        println!("Model ID: claude-{}", provider.id);
        println!("Model:    {}", provider.model);
        println!("URL:      {}", provider.base_url);
        println!("Default:  {}{}", provider.is_default, default_marker);
        println!("{:-<60}", "");
    }

    println!("\nTotal: {} providers", config.providers.len());
    println!("\nUsage with Claude Code:");
    println!("  ANTHROPIC_BASE_URL=http://127.0.0.1:{} claude", config.port);
    println!("  /model → select a model from the list");

    Ok(())
}

fn add(
    config_path: String,
    id: String,
    name: String,
    url: String,
    key: String,
    model: String,
    display_name: Option<String>,
    default: bool,
) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = if config_path.exists() {
        config::AppConfig::load(&config_path)?
    } else {
        config::AppConfig::default()
    };

    // Check if ID already exists
    if config.providers.iter().any(|p| p.id == id) {
        anyhow::bail!("Provider with ID '{}' already exists", id);
    }

    // If setting as default, unset others
    if default {
        for p in &mut config.providers {
            p.is_default = false;
        }
    }

    let provider = config::ProviderConfig {
        id: id.clone(),
        name,
        api_type: "openai".to_string(),
        base_url: url,
        api_key: key,
        model,
        display_name,
        is_default: default || config.providers.is_empty(),
    };

    config.providers.push(provider);
    config.save(&config_path)?;

    println!("Provider '{}' added successfully", id);
    Ok(())
}

fn remove(config_path: String, id: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    let len_before = config.providers.len();
    config.providers.retain(|p| p.id != id);

    if config.providers.len() == len_before {
        anyhow::bail!("Provider with ID '{}' not found", id);
    }

    // If we removed the default, set the first one as default
    if !config.providers.iter().any(|p| p.is_default) {
        if let Some(first) = config.providers.first_mut() {
            first.is_default = true;
        }
    }

    config.save(&config_path)?;
    println!("Provider '{}' removed", id);
    Ok(())
}

fn set_default(config_path: String, id: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    let mut found = false;
    for p in &mut config.providers {
        if p.id == id {
            p.is_default = true;
            found = true;
        } else {
            p.is_default = false;
        }
    }

    if !found {
        anyhow::bail!("Provider with ID '{}' not found", id);
    }

    config.save(&config_path)?;
    println!("Default provider set to '{}'", id);
    Ok(())
}

fn expand_path(path: &str) -> std::path::PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    std::path::PathBuf::from(path)
}
