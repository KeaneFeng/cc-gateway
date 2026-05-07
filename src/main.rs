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
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// CC-Switch-Pro: Lightweight multi-provider aggregation proxy for Claude Code
#[derive(Parser)]
#[command(name = "cc-switch-pro")]
#[command(version = "0.2.0")]
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
        /// Provider ID
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
        /// Notes
        #[arg(long)]
        notes: Option<String>,
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
        /// Notes
        #[arg(long)]
        notes: Option<String>,
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
    /// Interactive mode (TUI with arrow keys, TAB, etc.)
    Interactive {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,
    },
    /// Test connection to providers
    Test {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,
        /// Test specific provider
        #[arg(short, long)]
        id: Option<String>,
        /// Save results to database
        #[arg(long)]
        save: bool,
    },
    /// Show usage statistics
    Usage {
        /// Number of days to show
        #[arg(short, long, default_value = "30")]
        days: i32,
        /// Show usage for specific provider
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Show provider health status
    Health {
        /// Config file path
        #[arg(short, long, default_value = "~/.cc-switch-pro/config.toml")]
        config: String,
    },
    /// Configure proxy settings
    ProxyConfig {
        /// Enable/disable proxy
        #[arg(long)]
        enable: Option<bool>,
        /// Set listen port
        #[arg(long)]
        port: Option<i32>,
        /// Enable/disable auto failover
        #[arg(long)]
        failover: Option<bool>,
        /// Show current config
        #[arg(long)]
        show: bool,
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
        Commands::Add { config, id, preset, name, url, key, model, display_name, default, notes } => {
            add(config, id, preset, name, url, key, model, display_name, default, notes)?;
        }
        Commands::Edit { config, id, name, url, key, model, display_name, default, notes } => {
            edit(config, id, name, url, key, model, display_name, default, notes)?;
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
        Commands::Interactive { config } => {
            interactive::run_interactive(&expand_path(&config))?;
        }
        Commands::Test { config, id, save } => {
            test_providers(config, id, save).await?;
        }
        Commands::Usage { days, provider } => {
            show_usage(days, provider)?;
        }
        Commands::Health { config } => {
            show_health(config)?;
        }
        Commands::ProxyConfig { enable, port, failover, show } => {
            configure_proxy(enable, port, failover, show)?;
        }
    }

    Ok(())
}

async fn serve(config_path: String, port: Option<u16>, host: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
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

    tracing::info!("Starting CC-Switch-Pro proxy on {}:{}", config.host, config.port);

    let state = proxy::handlers::AppState::new(config.clone())?;
    let app = Router::new()
        .route("/v1/models", get(proxy::handlers::list_models))
        .route("/v1/messages", post(proxy::handlers::handle_messages))
        .route("/health", get(proxy::handlers::health_check))
        .route("/status", get(proxy::handlers::get_status))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init(output: String) -> anyhow::Result<()> {
    let output = expand_path(&output);
    let config = config::generate_example_config();
    config.save(&output)?;
    let _db = database::Database::open_cc_switch_compatible()?;
    println!("✓ Config saved to: {}", output.display());
    println!("✓ Database initialized at: ~/.cc-switch-pro/cc-switch-pro.db");
    Ok(())
}

fn list(config_path: String, table: bool) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let config = config::AppConfig::load(&config_path)?;

    if config.providers.is_empty() {
        println!("No providers configured.");
        return Ok(());
    }

    if table {
        println!("{:<15} {:<20} {:<15} {:<35} {:<8}", "ID", "Name", "Model ID", "URL", "Default");
        println!("{}", "-".repeat(93));
        for p in &config.providers {
            let default = if p.is_default { "✓" } else { "" };
            println!("{:<15} {:<20} {:<15} {:<35} {:<8}", p.id, truncate(&p.name, 19), format!("claude-{}", p.id), truncate(&p.base_url, 34), default);
        }
    } else {
        println!("Configured providers:");
        println!("{:-<60}", "");
        for p in &config.providers {
            let default_marker = if p.is_default { " (default)" } else { "" };
            println!("ID:          {}", p.id);
            println!("Name:        {}", p.name);
            println!("Model ID:    claude-{}", p.id);
            println!("Model:       {}", p.model);
            println!("URL:         {}", p.base_url);
            if let Some(ref dn) = p.display_name { println!("Display:     {}", dn); }
            if let Some(ref n) = p.notes { println!("Notes:       {}", n); }
            println!("Default:     {}{}", p.is_default, default_marker);
            println!("{:-<60}", "");
        }
    }

    println!("\nTotal: {} providers", config.providers.len());
    Ok(())
}

fn add(config_path: String, id: Option<String>, preset: Option<String>, name: Option<String>, url: Option<String>, key: Option<String>, model: Option<String>, display_name: Option<String>, default: bool, notes: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = if config_path.exists() { config::AppConfig::load(&config_path)? } else { config::AppConfig::default() };

    let provider = if let Some(preset_id) = preset {
        let preset = config::presets::get_preset_by_id(&preset_id).ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_id))?;
        let id = id.unwrap_or_else(|| preset.id.to_string());
        let api_key = key.ok_or_else(|| anyhow::anyhow!("API key is required (--key)"))?;
        config::ProviderConfig {
            id, name: name.unwrap_or_else(|| preset.name.to_string()), api_type: "openai".to_string(),
            base_url: url.unwrap_or_else(|| preset.base_url.to_string()), api_key,
            model: model.unwrap_or_else(|| preset.model.to_string()),
            display_name: display_name.or(Some(preset.display_name.to_string())),
            is_default: default, preset_id: Some(preset.id.to_string()), notes,
        }
    } else {
        let id = id.ok_or_else(|| anyhow::anyhow!("Provider ID is required (--id)"))?;
        let name = name.ok_or_else(|| anyhow::anyhow!("Provider name is required (--name)"))?;
        let url = url.ok_or_else(|| anyhow::anyhow!("Base URL is required (--url)"))?;
        let key = key.ok_or_else(|| anyhow::anyhow!("API key is required (--key)"))?;
        let model = model.ok_or_else(|| anyhow::anyhow!("Model is required (--model)"))?;
        config::ProviderConfig { id, name, api_type: "openai".to_string(), base_url: url, api_key: key, model, display_name, is_default: default, preset_id: None, notes }
    };

    config.add_provider(provider.clone())?;
    config.save(&config_path)?;

    let db = database::Database::open_cc_switch_compatible()?;
    let settings_config = serde_json::json!({"env": {"ANTHROPIC_BASE_URL": provider.base_url, "ANTHROPIC_AUTH_TOKEN": provider.api_key, "ANTHROPIC_MODEL": provider.model}});
    let db_provider = database::ProviderRow {
        id: provider.id.clone(), name: provider.name.clone(), settings_config: settings_config.to_string(),
        website_url: None, category: provider.preset_id.clone(), is_current: provider.is_default,
        in_failover_queue: false, cost_multiplier: "1.0".to_string(), provider_type: None, notes: provider.notes.clone(),
    };
    db.save_provider(&db_provider)?;

    println!("✓ Provider '{}' added successfully", provider.id);
    Ok(())
}

fn edit(config_path: String, id: String, name: Option<String>, url: Option<String>, key: Option<String>, model: Option<String>, display_name: Option<String>, default: Option<bool>, notes: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;
    let updates = config::ProviderUpdate { name, base_url: url, api_key: key, model, display_name, is_default: default, notes };
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
    let db = database::Database::open_cc_switch_compatible()?;
    db.delete_provider(&id, "claude")?;
    println!("✓ Provider '{}' removed", id);
    Ok(())
}

fn set_default(config_path: String, id: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let mut config = config::AppConfig::load(&config_path)?;
    config.set_default_provider(&id)?;
    config.save(&config_path)?;
    let db = database::Database::open_cc_switch_compatible()?;
    db.set_current_provider(&id, "claude")?;
    println!("✓ Default provider set to '{}'", id);
    Ok(())
}

fn presets(category: Option<String>, detail: bool) -> anyhow::Result<()> {
    let presets = if let Some(cat) = &category { config::presets::get_presets_by_category(cat) } else { config::presets::get_all_presets() };
    if presets.is_empty() { println!("No presets found."); return Ok(()); }

    println!("Available presets:");
    println!("{:-<80}", "");
    let mut current_category = String::new();
    for preset in &presets {
        if preset.category != current_category {
            current_category = preset.category.to_string();
            println!("\n📦 {}", config::presets::get_category_display_name(&current_category));
            println!("{:-<40}", "");
        }
        if detail {
            println!("  ID: {}", preset.id);
            println!("  Name: {}", preset.name);
            println!("  URL: {}", preset.base_url);
            println!("  Model: {}", preset.model);
            println!("  Display: {}", preset.display_name);
            println!();
        } else {
            println!("  {:<20} {:<30} {}", preset.id, preset.name, preset.display_name);
        }
    }
    println!("\nTotal: {} presets", presets.len());
    Ok(())
}

fn import(config_path: String, db: Option<String>) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let target_db = database::Database::open_cc_switch_compatible()?;
    let cc_switch_db = if let Some(db_path) = db { expand_path(&db_path) } else {
        dirs::home_dir().unwrap_or_default().join(".cc-switch").join("cc-switch.db")
    };
    if !cc_switch_db.exists() { anyhow::bail!("cc-switch database not found at: {}", cc_switch_db.display()); }

    let imported = target_db.import_from_cc_switch(&cc_switch_db)?;

    let mut config = if config_path.exists() { config::AppConfig::load(&config_path)? } else { config::AppConfig::default() };
    let providers = target_db.get_providers("claude")?;
    for p in &providers {
        if !config.providers.iter().any(|cp| cp.id == p.id) {
            let settings: serde_json::Value = serde_json::from_str(&p.settings_config).unwrap_or_default();
            let default_env = serde_json::json!({});
            let env = settings.get("env").unwrap_or(&default_env);
            config.providers.push(config::ProviderConfig {
                id: p.id.clone(), name: p.name.clone(), api_type: "openai".to_string(),
                base_url: env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                api_key: env.get("ANTHROPIC_AUTH_TOKEN").or(env.get("ANTHROPIC_API_KEY")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                model: env.get("ANTHROPIC_MODEL").and_then(|v| v.as_str()).unwrap_or("claude-sonnet-4").to_string(),
                display_name: None, is_default: p.is_current, preset_id: None, notes: p.notes.clone(),
            });
        }
    }
    if !config.providers.iter().any(|p| p.is_default) { if let Some(first) = config.providers.first_mut() { first.is_default = true; } }
    config.save(&config_path)?;

    println!("✓ Imported {} providers from cc-switch", imported);
    println!("✓ Config saved to: {}", config_path.display());
    Ok(())
}

async fn test_providers(config_path: String, id: Option<String>, save: bool) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let config = config::AppConfig::load(&config_path)?;
    let providers_to_test: Vec<commands::test::TestProvider> = if let Some(ref id) = id {
        config.providers.iter().filter(|p| p.id == *id).map(|p| commands::test::TestProvider { id: p.id.clone(), name: p.name.clone(), base_url: p.base_url.clone(), api_key: p.api_key.clone(), api_type: p.api_type.clone() }).collect()
    } else {
        config.providers.iter().map(|p| commands::test::TestProvider { id: p.id.clone(), name: p.name.clone(), base_url: p.base_url.clone(), api_key: p.api_key.clone(), api_type: p.api_type.clone() }).collect()
    };

    if providers_to_test.is_empty() { println!("No providers to test."); return Ok(()); }

    println!("\n  Testing connections...");
    println!("  ────────────────────────────────────────────────────────────────────────");
    let results = commands::test::test_all_providers(&providers_to_test).await;
    for result in &results {
        let icon = if result.success { "✓" } else { "✗" };
        let style = if result.success { console::style(icon).green() } else { console::style(icon).red() };
        println!("  {} {:<20} {}", style, result.provider_id, result.message);
    }
    println!("  ────────────────────────────────────────────────────────────────────────");
    let success_count = results.iter().filter(|r| r.success).count();
    println!("  {}/{} providers passed", success_count, results.len());

    if save {
        let db = database::Database::open_cc_switch_compatible()?;
        let names: std::collections::HashMap<String, String> = config.providers.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
        commands::test::save_test_results(&db, &results, &names)?;
        println!("\n  ✓ Results saved to database");
    }
    Ok(())
}

fn show_usage(days: i32, provider: Option<String>) -> anyhow::Result<()> {
    let db = database::Database::open_cc_switch_compatible()?;
    if let Some(provider_id) = provider { commands::usage::show_provider_usage(&db, &provider_id, days)?; }
    else { commands::usage::show_usage(&db, days)?; }
    Ok(())
}

fn show_health(config_path: String) -> anyhow::Result<()> {
    let config_path = expand_path(&config_path);
    let config = config::AppConfig::load(&config_path)?;
    let db = database::Database::open_cc_switch_compatible()?;

    println!("\n  {} Provider Health Status", console::style("🏥").cyan());
    println!("  ────────────────────────────────────────────────────────────────────────");
    println!("  {:<20} {:<10} {:<15} {:<20}", "Provider", "Status", "Last Check", "Error");
    println!("  ────────────────────────────────────────────────────────────────────────");

    for p in &config.providers {
        let health = db.get_health(&p.id, "claude")?;
        let (status, last_check, error) = if let Some(h) = health {
            let status = if h.is_healthy { console::style("✓ Healthy").green() } else { console::style("✗ Unhealthy").red() };
            let last_check = h.last_success_at.or(h.last_failure_at).unwrap_or_else(|| "Never".to_string());
            let error = h.last_error.unwrap_or_default();
            (status, last_check, error)
        } else {
            (console::style("? Unknown").yellow(), "Never".to_string(), String::new())
        };
        println!("  {:<20} {:<10} {:<15} {:<20}", truncate(&p.id, 19), status, truncate(&last_check, 14), truncate(&error, 19));
    }
    println!("  ────────────────────────────────────────────────────────────────────────");
    Ok(())
}

fn configure_proxy(enable: Option<bool>, port: Option<i32>, failover: Option<bool>, show: bool) -> anyhow::Result<()> {
    let db = database::Database::open_cc_switch_compatible()?;
    let mut config = db.get_proxy_config("claude")?;

    if show {
        println!("\n  {} Proxy Configuration", console::style("⚙️").cyan());
        println!("  ────────────────────────────────────────────────────────────────────────");
        println!("  {:<25} {}", "Enabled", if config.proxy_enabled { "Yes" } else { "No" });
        println!("  {:<25} {}:{}", "Listen Address", config.listen_address, config.listen_port);
        println!("  {:<25} {}", "Auto Failover", if config.auto_failover_enabled { "Yes" } else { "No" });
        println!("  {:<25} {}", "Max Retries", config.max_retries);
        println!("  ────────────────────────────────────────────────────────────────────────");
        return Ok(());
    }

    if let Some(enable) = enable { config.proxy_enabled = enable; }
    if let Some(port) = port { config.listen_port = port; }
    if let Some(failover) = failover { config.auto_failover_enabled = failover; }
    db.update_proxy_config(&config)?;

    println!("✓ Proxy configuration updated");
    Ok(())
}

fn expand_path(path: &str) -> std::path::PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() { return home.join(&path[2..]); }
    }
    std::path::PathBuf::from(path)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len { s.to_string() } else { format!("{}...", &s[..max_len - 3]) }
}
