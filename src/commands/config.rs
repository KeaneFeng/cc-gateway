//! Config command - show or edit configuration

use crate::config::AppConfig;
use console::style;
use std::path::Path;

pub fn show_config(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let config = if path.exists() {
        AppConfig::load(path)?
    } else {
        AppConfig::default()
    };

    println!("\n  {} Configuration\n", style("⚙️").cyan());
    println!("  Config: {}", style(path.display()).dim());
    println!("  Port:   {}", style(config.port).green());
    println!("  Host:   {}", style(&config.host).green());
    println!("  Log:    {}", style(&config.log_level).green());
    println!("  Providers: {}", style(config.providers.len()).cyan());
    println!();

    Ok(())
}

pub fn set_config(config_path: &str, key: &str, value: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let mut config = if path.exists() {
        AppConfig::load(path)?
    } else {
        AppConfig::default()
    };

    match key {
        "port" => {
            config.port = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid port number"))?;
        }
        "host" => {
            config.host = value.to_string();
        }
        "log_level" | "log" => {
            config.log_level = value.to_string();
        }
        _ => {
            anyhow::bail!(
                "Unknown config key: {}. Valid keys: port, host, log_level",
                key
            );
        }
    }

    config.save(path)?;

    println!(
        "\n  {} Config updated: {} = {}",
        style("✓").green(),
        style(key).cyan(),
        style(value).green()
    );

    Ok(())
}
