//! Status command - show provider overview

use crate::config::AppConfig;
use console::style;
use std::path::Path;

pub fn show_status(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let config = if path.exists() {
        AppConfig::load(path)?
    } else {
        AppConfig::default()
    };

    println!("\n  {} Provider Status\n", style("📊").cyan());

    if config.providers.is_empty() {
        println!("  {} No providers configured.\n", style("⚠").yellow());
        return Ok(());
    }

    println!(
        "  {:<4} {:<18} {:<25} {:<35} {:<8}",
        style("#").dim(),
        style("ID").dim(),
        style("Model").dim(),
        style("URL").dim(),
        style("Default").dim()
    );
    println!("  {}", style("─".repeat(93)).dim());

    for (i, p) in config.providers.iter().enumerate() {
        let default = if p.is_default {
            style("⭐").yellow().to_string()
        } else {
            String::new()
        };
        println!(
            "  {:<4} {:<18} {:<25} {:<35} {}",
            i + 1,
            style(&p.id).green(),
            format!("claude-{}", p.id),
            truncate(&p.base_url, 34),
            default
        );
    }

    println!(
        "\n  {} Total: {} providers\n",
        style("ℹ").blue(),
        style(config.providers.len()).cyan()
    );

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
