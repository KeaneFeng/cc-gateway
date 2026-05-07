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

        /// Show as table
        #[arg(long)]
        table: bool,
    },

    /// Add a new provider (interactive or from preset)
    Add {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Provider ID (e.g., mimo, kimi, glm)
        #[arg(short, long)]
        id: Option<String>,

        /// Add from preset ID
        #[arg(long)]
        preset: Option<String>,

        /// Provider name
        #[arg(long)]
        name: Option<String>,

        /// Base URL
        #[arg(long)]
        url: Option<String>,

        /// API key
        #[arg(long)]
        key: Option<String>,

        /// Model name
        #[arg(long)]
        model: Option<String>,

        /// Display name (optional)
        #[arg(long)]
        display_name: Option<String>,

        /// Set as default provider
        #[arg(long)]
        default: bool,
    },

    /// Edit a provider
    Edit {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Provider ID to edit
        #[arg(short, long)]
        id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New base URL
        #[arg(long)]
        url: Option<String>,

        /// New API key
        #[arg(long)]
        key: Option<String>,

        /// New model
        #[arg(long)]
        model: Option<String>,

        /// New display name
        #[arg(long)]
        display_name: Option<String>,

        /// Set as default
        #[arg(long)]
        default: Option<bool>,
    },

    /// Copy a provider
    Copy {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Source provider ID
        #[arg(short, long)]
        from: String,

        /// New provider ID
        #[arg(short, long)]
        to: String,
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

    /// List available presets
    Presets {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Show preset details
        #[arg(long)]
        detail: bool,
    },

    /// Import providers from cc-switch
    Import {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,

        /// Path to cc-switch database (optional)
        #[arg(long)]
        db: Option<String>,
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
        Commands::List { config, table } => {
            list(config, table)?;
        }
        Commands::Add {
            config,
            id,
            preset,
            name,
            url,
            key,
            model,
            display_name,
            default,
        } => {
            add(config, id, preset, name, url, key, model, display_name, default)?;
        }
        Commands::Edit {
            config,
            id,
            name,
            url,
            key,
            model,
            display_name,
            default,
        } => {
            edit(config, id, name, url, key, model, display_name, default)?;
        }
        Commands::Copy { config, from, to } => {
            copy(config, from, to)?;
        }
        Commands::Remove { config, id } => {
            remove(config, id)?;
        }
        Commands::SetDefault { config, id } => {
            set_default(config, id)?;
        }
        Commands::Presets { category, detail } => {
            presets(category, detail)?;
        }
        Commands::Import { config, db } => {
            import(config, db)?;
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

fn list(config_path: String, table: bool) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let config = config::AppConfig::load(&config_path)?;

    if config.providers.is_empty() {
        println!("No providers configured.");
        println!("\nAdd a provider:");
        println!("  cc-switch-pro add --preset mimo --key YOUR_API_KEY");
        println!("  cc-switch-pro add --id myprovider --name \"My Provider\" --url https://api.example.com/v1 --key YOUR_KEY --model model-name");
        println!("\nImport from cc-switch:");
        println!("  cc-switch-pro import");
        return Ok(());
    }

    if table {
        // Table format
        println!("{:<15} {:<25} {:<15} {:<40} {:<8}", "ID", "Name", "Model ID", "URL", "Default");
        println!("{}", "-".repeat(103));
        for p in &config.providers {
            let default = if p.is_default { "✓" } else { "" };
            println!(
                "{:<15} {:<25} {:<15} {:<40} {:<8}",
                p.id,
                truncate(&p.name, 24),
                format!("claude-{}", p.id),
                truncate(&p.base_url, 39),
                default
            );
        }
    } else {
        // Detailed format
        println!("Configured providers:");
        println!("{:-<60}", "");
        for p in &config.providers {
            let default_marker = if p.is_default { " (default)" } else { "" };
            println!("ID:          {}", p.id);
            println!("Name:        {}", p.name);
            println!("Model ID:    claude-{}", p.id);
            println!("Model:       {}", p.model);
            println!("URL:         {}", p.base_url);
            if let Some(ref dn) = p.display_name {
                println!("Display:     {}", dn);
            }
            println!("Default:     {}{}", p.is_default, default_marker);
            println!("{:-<60}", "");
        }
    }

    println!("\nTotal: {} providers", config.providers.len());
    println!("\nUsage with Claude Code:");
    println!("  ANTHROPIC_BASE_URL=http://127.0.0.1:{} claude", config.port);
    println!("  /model → select a model from the list");

    Ok(())
}

fn add(
    config_path: String,
    id: Option<String>,
    preset: Option<String>,
    name: Option<String>,
    url: Option<String>,
    key: Option<String>,
    model: Option<String>,
    display_name: Option<String>,
    default: bool,
) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = if config_path.exists() {
        config::AppConfig::load(&config_path)?
    } else {
        config::AppConfig::default()
    };

    let provider = if let Some(preset_id) = preset {
        // Add from preset
        let preset = config::presets::get_preset_by_id(&preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_id))?;

        let id = id.unwrap_or_else(|| preset.id.to_string());
        let api_key = key.ok_or_else(|| anyhow::anyhow!("API key is required (--key)"))?;

        config::ProviderConfig {
            id,
            name: name.unwrap_or_else(|| preset.name.to_string()),
            api_type: "openai".to_string(),
            base_url: url.unwrap_or_else(|| preset.base_url.to_string()),
            api_key,
            model: model.unwrap_or_else(|| preset.model.to_string()),
            display_name: display_name.or(Some(preset.display_name.to_string())),
            is_default: default,
            preset_id: Some(preset.id.to_string()),
        }
    } else {
        // Add custom provider
        let id = id.ok_or_else(|| anyhow::anyhow!("Provider ID is required (--id)"))?;
        let name = name.ok_or_else(|| anyhow::anyhow!("Provider name is required (--name)"))?;
        let url = url.ok_or_else(|| anyhow::anyhow!("Base URL is required (--url)"))?;
        let key = key.ok_or_else(|| anyhow::anyhow!("API key is required (--key)"))?;
        let model = model.ok_or_else(|| anyhow::anyhow!("Model is required (--model)"))?;

        config::ProviderConfig {
            id,
            name,
            api_type: "openai".to_string(),
            base_url: url,
            api_key: key,
            model,
            display_name,
            is_default: default,
            preset_id: None,
        }
    };

    config.add_provider(provider.clone())?;
    config.save(&config_path)?;

    println!("✓ Provider '{}' added successfully", provider.id);
    if let Some(ref preset_id) = provider.preset_id {
        println!("  Preset: {}", preset_id);
    }
    println!("  Model ID: claude-{}", provider.id);

    Ok(())
}

fn edit(
    config_path: String,
    id: String,
    name: Option<String>,
    url: Option<String>,
    key: Option<String>,
    model: Option<String>,
    display_name: Option<String>,
    default: Option<bool>,
) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    let updates = config::ProviderUpdate {
        name,
        base_url: url,
        api_key: key,
        model,
        display_name,
        is_default: default,
    };

    config.update_provider(&id, updates)?;
    config.save(&config_path)?;

    println!("✓ Provider '{}' updated", id);
    Ok(())
}

fn copy(config_path: String, from: String, to: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    config.copy_provider(&from, &to)?;
    config.save(&config_path)?;

    println!("✓ Provider '{}' copied to '{}'", from, to);
    Ok(())
}

fn remove(config_path: String, id: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    config.remove_provider(&id)?;
    config.save(&config_path)?;

    println!("✓ Provider '{}' removed", id);
    Ok(())
}

fn set_default(config_path: String, id: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;

    config.set_default_provider(&id)?;
    config.save(&config_path)?;

    println!("✓ Default provider set to '{}'", id);
    Ok(())
}

fn presets(category: Option<String>, detail: bool) -> anyhow::Result<()> {
    let presets = if let Some(cat) = &category {
        config::presets::get_presets_by_category(cat)
    } else {
        config::presets::get_all_presets()
    };

    if presets.is_empty() {
        println!("No presets found.");
        return Ok(());
    }

    println!("Available presets:");
    println!("{:-<80}", "");

    let mut current_category = String::new();
    for preset in &presets {
        if preset.category != current_category {
            current_category = preset.category.to_string();
            let cat_name = config::presets::get_category_display_name(&current_category);
            println!("\n📦 {}", cat_name);
            println!("{:-<40}", "");
        }

        if detail {
            println!("  ID:          {}", preset.id);
            println!("  Name:        {}", preset.name);
            println!("  URL:         {}", preset.base_url);
            println!("  Model:       {}", preset.model);
            println!("  Display:     {}", preset.display_name);
            if let Some(api_key_url) = preset.api_key_url {
                println!("  Get API Key: {}", api_key_url);
            }
            println!();
        } else {
            println!("  {:<20} {:<30} {}", preset.id, preset.name, preset.display_name);
        }
    }

    println!("\n{:-<80}", "");
    println!("Total: {} presets", presets.len());
    println!("\nUsage:");
    println!("  cc-switch-pro add --preset <PRESET_ID> --key YOUR_API_KEY");
    println!("  cc-switch-pro add --preset mimo --key sk-xxx");

    Ok(())
}

fn import(config_path: String, db: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);

    // Try to import from cc-switch
    let imported_config = if let Some(db_path) = db {
        // Use custom db path
        let db_path = expand_path(&db_path);
        if !db_path.exists() {
            anyhow::bail!("Database file not found: {}", db_path.display());
        }
        // TODO: implement custom db import
        anyhow::bail!("Custom database import not yet implemented");
    } else {
        config::import_from_cc_switch()?
    };

    // Load existing config or create new
    let mut config = if config_path.exists() {
        config::AppConfig::load(&config_path)?
    } else {
        config::AppConfig::default()
    };

    // Merge providers (skip duplicates)
    let mut imported_count = 0;
    for provider in imported_config.providers {
        if !config.providers.iter().any(|p| p.id == provider.id) {
            config.providers.push(provider);
            imported_count += 1;
        }
    }

    // Set first provider as default if no default exists
    if !config.providers.iter().any(|p| p.is_default) {
        if let Some(first) = config.providers.first_mut() {
            first.is_default = true;
        }
    }

    config.save(&config_path)?;

    println!("✓ Imported {} providers from cc-switch", imported_count);
    if imported_count > 0 {
        println!("\nImported providers:");
        for p in &config.providers {
            println!("  - {} ({})", p.id, p.name);
        }
    }
    println!("\nConfig saved to: {}", config_path.display());

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

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
