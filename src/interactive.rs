//! Interactive TUI Dashboard
//!
//! Default mode: shows provider overview with interactive commands

use crate::config::{AppConfig, ProviderConfig, ProviderUpdate, ApiFormat, presets};
use console::{style, Term};
use dialoguer::{Select, Input, Confirm, theme::ColorfulTheme};
use std::path::Path;

/// Main dashboard - default mode when no subcommand given
pub fn run_dashboard(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let term = Term::stdout();
    let theme = ColorfulTheme::default();

    loop {
        term.clear_screen()?;

        // Header
        term.write_line(&format!(
            "\n  {} {}",
            style("cc-gateway").cyan().bold(),
            style("v0.3.0").dim()
        ))?;
        term.write_line(&format!(
            "  {}",
            style("Multi-provider aggregation gateway for Claude Code").dim()
        ))?;
        term.write_line("")?;

        // Load config
        let config = if path.exists() {
            AppConfig::load(path)?
        } else {
            AppConfig::default()
        };

        // Provider table
        if config.providers.is_empty() {
            term.write_line(&format!(
                "  {} No providers configured.",
                style("⚠").yellow()
            ))?;
            term.write_line(&format!(
                "  Run {} to add your first provider.\n",
                style("cc-gateway add").green()
            ))?;
        } else {
            term.write_line(&format!(
                "  {:<4} {:<18} {:<25} {:<8}",
                style("#").dim(),
                style("ID").dim(),
                style("Model").dim(),
                style("Default").dim()
            ))?;
            term.write_line(&format!("  {}", style("─".repeat(58)).dim()))?;

            for (i, p) in config.providers.iter().enumerate() {
                let default = if p.is_default {
                    style("⭐").yellow().to_string()
                } else {
                    String::new()
                };
                term.write_line(&format!(
                    "  {:<4} {:<18} {:<25} {}",
                    i + 1,
                    style(&p.id).green(),
                    format!("claude-{}", p.id),
                    default
                ))?;
            }
            term.write_line("")?;
        }

        // Menu
        let options = vec![
            "➕  Add provider",
            "✏️   Edit provider",
            "🗑️   Remove provider",
            "⭐  Set default",
            "🔌  Test connections",
            "📊  Show status",
            "📥  Import from cc-switch",
            "📦  Browse presets",
            "❌  Exit",
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("What do you want to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => add_provider(path, None)?,
            1 => edit_provider(path, None)?,
            2 => remove_provider(path, None)?,
            3 => set_default(path, None)?,
            4 => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::commands::test::run_test(
                    &path.to_string_lossy(),
                    None,
                ))?;
                term.write_line("\n  Press Enter to continue...")?;
                term.read_line()?;
            }
            5 => {
                crate::commands::status::show_status(&path.to_string_lossy())?;
                term.write_line("\n  Press Enter to continue...")?;
                term.read_line()?;
            }
            6 => import_providers(path, None)?,
            7 => {
                crate::commands::presets::show_presets(None)?;
                term.write_line("\n  Press Enter to continue...")?;
                term.read_line()?;
            }
            8 => {
                term.write_line("\n  Goodbye! 👋")?;
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Add a provider (interactive or from preset)
pub fn add_provider(config_path: &Path, preset_id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    let mut config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    let provider = if let Some(preset_id) = preset_id {
        // Add from preset
        let preset = presets::get_preset_by_id(preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_id))?;

        term.write_line(&format!(
            "\n  {} Adding from preset: {}",
            style("➕").green(),
            style(&preset.name).cyan()
        ))?;
        term.write_line(&format!(
            "  {}",
            style(&preset.display_name).dim()
        ))?;
        term.write_line("")?;

        let api_key: String = Input::with_theme(&theme)
            .with_prompt("  API Key")
            .interact_text()?;

        let custom_id: String = Input::with_theme(&theme)
            .with_prompt("  Custom ID (Enter for default)")
            .default(preset.id.to_string())
            .interact_text()?;

        let is_default = Confirm::with_theme(&theme)
            .with_prompt("  Set as default?")
            .default(false)
            .interact()?;

        ProviderConfig {
            id: custom_id,
            name: preset.name.to_string(),
            api_format: preset.api_format,
            base_url: preset.base_url.to_string(),
            api_key,
            model: preset.model.to_string(),
            display_name: Some(preset.display_name.to_string()),
            is_default,
            preset_id: Some(preset.id.to_string()),
            notes: None,
        }
    } else {
        // Interactive preset selection or custom
        let categories = presets::get_categories();
        let mut category_items: Vec<String> = categories
            .iter()
            .map(|c| {
                format!(
                    "{} ({})",
                    presets::get_category_display_name(c),
                    presets::get_presets_by_category(c).len()
                )
            })
            .collect();
        category_items.push("Custom (manual)".to_string());

        term.write_line(&format!(
            "\n  {} Add provider",
            style("➕").green()
        ))?;
        term.write_line("")?;

        let cat_selection = Select::with_theme(&theme)
            .with_prompt("  Select category")
            .items(&category_items)
            .default(0)
            .interact()?;

        if cat_selection < categories.len() {
            // From preset
            let selected_category = categories[cat_selection];
            let presets_list = presets::get_presets_by_category(selected_category);
            let preset_items: Vec<String> = presets_list
                .iter()
                .map(|p| format!("{} - {}", p.id, p.display_name))
                .collect();

            let preset_selection = Select::with_theme(&theme)
                .with_prompt("  Select preset")
                .items(&preset_items)
                .default(0)
                .interact()?;
            let selected_preset = &presets_list[preset_selection];

            let api_key: String = Input::with_theme(&theme)
                .with_prompt("  API Key")
                .interact_text()?;

            let custom_id: String = Input::with_theme(&theme)
                .with_prompt("  Custom ID (Enter for default)")
                .default(selected_preset.id.to_string())
                .interact_text()?;

            let is_default = Confirm::with_theme(&theme)
                .with_prompt("  Set as default?")
                .default(false)
                .interact()?;

            ProviderConfig {
                id: custom_id,
                name: selected_preset.name.to_string(),
                api_format: selected_preset.api_format.clone(),
                base_url: selected_preset.base_url.to_string(),
                api_key,
                model: selected_preset.model.to_string(),
                display_name: Some(selected_preset.display_name.to_string()),
                is_default,
                preset_id: Some(selected_preset.id.to_string()),
                notes: None,
            }
        } else {
            // Custom provider
            let id: String = Input::with_theme(&theme)
                .with_prompt("  Provider ID")
                .interact_text()?;
            let name: String = Input::with_theme(&theme)
                .with_prompt("  Provider Name")
                .interact_text()?;
            let base_url: String = Input::with_theme(&theme)
                .with_prompt("  Base URL")
                .interact_text()?;
            let api_key: String = Input::with_theme(&theme)
                .with_prompt("  API Key")
                .interact_text()?;
            let model: String = Input::with_theme(&theme)
                .with_prompt("  Model Name")
                .interact_text()?;

            // API format selection
            let api_format_options = vec![
                ("Anthropic Messages (direct passthrough)", ApiFormat::Anthropic),
                ("OpenAI Chat Completions (needs conversion)", ApiFormat::OpenAiChat),
                ("OpenAI Responses API (needs conversion)", ApiFormat::OpenAiResponses),
                ("Gemini Native (needs conversion)", ApiFormat::GeminiNative),
            ];
            let api_format_items: Vec<String> = api_format_options.iter().map(|(desc, _)| desc.to_string()).collect();
            let api_format_selection = Select::with_theme(&theme)
                .with_prompt("  API Format")
                .items(&api_format_items)
                .default(1)
                .interact()?;
            let api_format = api_format_options[api_format_selection].1.clone();

            let is_default = Confirm::with_theme(&theme)
                .with_prompt("  Set as default?")
                .default(false)
                .interact()?;

            ProviderConfig {
                id,
                name,
                api_format,
                base_url,
                api_key,
                model,
                display_name: None,
                is_default,
                preset_id: None,
                notes: None,
            }
        }
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;

    term.write_line(&format!(
        "\n  {} Provider '{}' added!",
        style("✓").green(),
        style(&provider_id).cyan()
    ))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

/// Edit a provider
pub fn edit_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  No providers to edit.")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let provider_idx = if let Some(id) = id {
        config
            .providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", id))?
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default { " ⭐" } else { "" };
                format!("{} ({}){}", p.id, p.name, default)
            })
            .collect();
        Select::with_theme(&theme)
            .with_prompt("  Select provider to edit")
            .items(&items)
            .default(0)
            .interact()?
    };

    let provider = &config.providers[provider_idx];
    let provider_id = provider.id.clone();

    term.write_line(&format!(
        "\n  {} Edit provider '{}'",
        style("✏️").cyan(),
        style(&provider_id).green()
    ))?;
    term.write_line("  (Press Enter to keep current value)\n")?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("  Name")
        .default(provider.name.clone())
        .interact_text()?;
    let base_url: String = Input::with_theme(&theme)
        .with_prompt("  Base URL")
        .default(provider.base_url.clone())
        .interact_text()?;
    let api_key: String = Input::with_theme(&theme)
        .with_prompt("  API Key")
        .default(provider.api_key.clone())
        .interact_text()?;
    let model: String = Input::with_theme(&theme)
        .with_prompt("  Model")
        .default(provider.model.clone())
        .interact_text()?;
    let is_default = Confirm::with_theme(&theme)
        .with_prompt("  Set as default?")
        .default(provider.is_default)
        .interact()?;

    let updates = ProviderUpdate {
        name: Some(name),
        base_url: Some(base_url),
        api_key: Some(api_key),
        model: Some(model),
        display_name: None,
        is_default: Some(is_default),
        notes: None,
    };

    config.update_provider(&provider_id, updates)?;
    config.save(config_path)?;

    term.write_line(&format!(
        "\n  {} Provider '{}' updated!",
        style("✓").green(),
        style(&provider_id).cyan()
    ))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

/// Remove a provider
pub fn remove_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  No providers to remove.")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let provider_id = if let Some(id) = id {
        id.to_string()
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default { " ⭐" } else { "" };
                format!("{} ({}){}", p.id, p.name, default)
            })
            .collect();
        let selection = Select::with_theme(&theme)
            .with_prompt("  Select provider to remove")
            .items(&items)
            .default(0)
            .interact()?;
        config.providers[selection].id.clone()
    };

    let confirm = Confirm::with_theme(&theme)
        .with_prompt(format!("  Remove provider '{}'?", provider_id))
        .default(false)
        .interact()?;

    if confirm {
        config.remove_provider(&provider_id)?;
        config.save(config_path)?;
        term.write_line(&format!(
            "\n  {} Provider '{}' removed!",
            style("✓").green(),
            style(&provider_id).cyan()
        ))?;
    } else {
        term.write_line("\n  Cancelled.")?;
    }
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

/// Set default provider
pub fn set_default(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  No providers configured.")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let provider_id = if let Some(id) = id {
        id.to_string()
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default { " ⭐ (current)" } else { "" };
                format!("{} ({}){}", p.id, p.name, default)
            })
            .collect();
        let selection = Select::with_theme(&theme)
            .with_prompt("  Select default provider")
            .items(&items)
            .default(0)
            .interact()?;
        config.providers[selection].id.clone()
    };

    config.set_default_provider(&provider_id)?;
    config.save(config_path)?;

    term.write_line(&format!(
        "\n  {} Default set to '{}'!",
        style("⭐").yellow(),
        style(&provider_id).cyan()
    ))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

/// Import providers from cc-switch
pub fn import_providers(config_path: &Path, db_path: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    term.write_line(&format!(
        "\n  {} Import from cc-switch",
        style("📥").cyan()
    ))?;

    let cc_switch_db = if let Some(db) = db_path {
        std::path::PathBuf::from(db)
    } else {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cc-switch")
            .join("cc-switch.db")
    };

    if !cc_switch_db.exists() {
        term.write_line(&format!(
            "  {} cc-switch database not found at: {}",
            style("✗").red(),
            cc_switch_db.display()
        ))?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    term.write_line(&format!(
        "  Found: {}",
        style(cc_switch_db.display()).dim()
    ))?;

    let confirm = Confirm::with_theme(&theme)
        .with_prompt("  Import providers?")
        .default(true)
        .interact()?;

    if !confirm {
        term.write_line("  Cancelled.")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let imported_config = crate::config::import_from_cc_switch()?;
    let mut config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    let mut count = 0;
    for provider in imported_config.providers {
        if !config.providers.iter().any(|p| p.id == provider.id) {
            config.providers.push(provider);
            count += 1;
        }
    }

    if !config.providers.iter().any(|p| p.is_default) {
        if let Some(first) = config.providers.first_mut() {
            first.is_default = true;
        }
    }

    config.save(config_path)?;

    term.write_line(&format!(
        "\n  {} Imported {} providers!",
        style("✓").green(),
        style(count).cyan()
    ))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}
