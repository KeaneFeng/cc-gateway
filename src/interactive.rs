//! Interactive TUI module
//!
//! Provides interactive terminal UI with arrow keys, TAB, space, enter navigation

use crate::config::{AppConfig, ProviderConfig, ProviderUpdate, presets};
use console::{style, Term};
use dialoguer::{Select, Input, Confirm};
use std::path::PathBuf;

/// Run interactive mode
pub fn run_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();

    loop {
        term.clear_screen()?;
        print_header(&term)?;

        let options = vec![
            "📋 List providers",
            "➕ Add provider (from preset)",
            "➕ Add provider (custom)",
            "✏️  Edit provider",
            "📋 Copy provider",
            "🗑️  Remove provider",
            "⭐ Set default provider",
            "📦 List presets",
            "📥 Import from cc-switch",
            "❌ Exit",
        ];

        let selection = Select::new()
            .with_prompt("What do you want to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => list_providers_interactive(config_path)?,
            1 => add_from_preset_interactive(config_path)?,
            2 => add_custom_interactive(config_path)?,
            3 => edit_provider_interactive(config_path)?,
            4 => copy_provider_interactive(config_path)?,
            5 => remove_provider_interactive(config_path)?,
            6 => set_default_interactive(config_path)?,
            7 => list_presets_interactive()?,
            8 => import_interactive(config_path)?,
            9 => {
                println!("Goodbye!");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn print_header(term: &Term) -> anyhow::Result<()> {
    term.write_line("")?;
    term.write_line(&format!("{}", style("╔═══════════════════════════════════════════════════════════╗").cyan()))?;
    term.write_line(&format!("{}", style("║           CC-Switch-Pro - Provider Manager               ║").cyan()))?;
    term.write_line(&format!("{}", style("╚═══════════════════════════════════════════════════════════╝").cyan()))?;
    term.write_line("")?;
    Ok(())
}

fn list_providers_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    if config.providers.is_empty() {
        term.write_line("\n  No providers configured.\n")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    term.write_line("\n  Configured Providers:")?;
    term.write_line("  ────────────────────────────────────────────────────────────────────────")?;
    term.write_line(&format!("  {:<5} {:<15} {:<25} {:<15} {:<8}", "#", "ID", "Name", "Model ID", "Default"))?;
    term.write_line("  ────────────────────────────────────────────────────────────────────────")?;

    for (i, p) in config.providers.iter().enumerate() {
        let default = if p.is_default { "  ⭐" } else { "" };
        let notes = p.notes.as_deref().unwrap_or("");
        term.write_line(&format!("  {:<5} {:<15} {:<25} {:<15} {:<8}", i + 1, truncate(&p.id, 14), truncate(&p.name, 24), truncate(notes, 14), default))?;
    }

    term.write_line("  ────────────────────────────────────────────────────────────────────────")?;
    term.write_line(&format!("  Total: {} providers", config.providers.len()))?;
    term.write_line("")?;

    let mut items: Vec<String> = config.providers.iter().map(|p| {
        let default = if p.is_default { " ⭐" } else { "" };
        let notes = p.notes.as_deref().unwrap_or("");
        format!("{} ({}){} - {}", p.id, p.name, default, notes)
    }).collect();
    items.push("← Back".to_string());

    let selection = Select::new().with_prompt("Select provider to view details").items(&items).default(0).interact()?;

    if selection < config.providers.len() {
        let p = &config.providers[selection];
        term.clear_screen()?;
        term.write_line("\n  Provider Details:")?;
        term.write_line("  ─────────────────────────────────────────────")?;
        term.write_line(&format!("  ID:          {}", p.id))?;
        term.write_line(&format!("  Name:        {}", p.name))?;
        term.write_line(&format!("  Model ID:    claude-{}", p.id))?;
        term.write_line(&format!("  Model:       {}", p.model))?;
        term.write_line(&format!("  URL:         {}", p.base_url))?;
        if !p.api_key.is_empty() { term.write_line(&format!("  API Key:     {}...", &p.api_key[..std::cmp::min(8, p.api_key.len())]))?; }
        if let Some(ref dn) = p.display_name { term.write_line(&format!("  Display:     {}", dn))?; }
        if let Some(ref n) = p.notes { term.write_line(&format!("  Notes:       {}", n))?; }
        term.write_line(&format!("  Default:     {}", if p.is_default { "Yes ⭐" } else { "No" }))?;
        term.write_line("  ─────────────────────────────────────────────")?;
        term.write_line("\n  Press Enter to continue...")?;
        term.read_line()?;
    }

    Ok(())
}

fn add_from_preset_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    let categories = presets::get_categories();
    let category_items: Vec<String> = categories.iter().map(|c| format!("{} ({})", presets::get_category_display_name(c), presets::get_presets_by_category(c).len())).collect();

    term.write_line("\n  Select category:")?;
    let cat_selection = Select::new().with_prompt("Category").items(&category_items).default(0).interact()?;

    let selected_category = categories[cat_selection];
    let presets_list = presets::get_presets_by_category(selected_category);
    let preset_items: Vec<String> = presets_list.iter().map(|p| format!("{} - {}", p.id, p.display_name)).collect();

    let preset_selection = Select::new().with_prompt("Select preset").items(&preset_items).default(0).interact()?;
    let selected_preset = &presets_list[preset_selection];

    let api_key: String = Input::new().with_prompt("API Key").interact_text()?;
    let custom_id: String = Input::new().with_prompt("Custom ID (leave empty for default)").default(selected_preset.id.to_string()).interact_text()?;
    let is_default = Confirm::new().with_prompt("Set as default provider?").default(false).interact()?;

    let provider = ProviderConfig {
        id: custom_id,
        name: selected_preset.name.to_string(),
        api_type: "openai".to_string(),
        base_url: selected_preset.base_url.to_string(),
        api_key,
        model: selected_preset.model.to_string(),
        display_name: Some(selected_preset.display_name.to_string()),
        is_default,
        preset_id: Some(selected_preset.id.to_string()),
        notes: None,
    };

    config.add_provider(provider)?;
    config.save(config_path)?;

    term.write_line(&format!("\n  {} Provider '{}' added successfully!", style("✓").green(), selected_preset.id))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn add_custom_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    term.write_line("\n  Add Custom Provider:")?;
    term.write_line("  ─────────────────────────────────────────────")?;

    let id: String = Input::new().with_prompt("Provider ID").interact_text()?;
    let name: String = Input::new().with_prompt("Provider Name").interact_text()?;
    let base_url: String = Input::new().with_prompt("Base URL").interact_text()?;
    let api_key: String = Input::new().with_prompt("API Key").interact_text()?;
    let model: String = Input::new().with_prompt("Model Name").interact_text()?;
    let notes: String = Input::new().with_prompt("Notes (optional)").allow_empty(true).interact_text()?;
    let is_default = Confirm::new().with_prompt("Set as default provider?").default(false).interact()?;

    let provider = ProviderConfig {
        id, name, api_type: "openai".to_string(), base_url, api_key, model,
        display_name: None, is_default, preset_id: None,
        notes: if notes.is_empty() { None } else { Some(notes) },
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;

    term.write_line(&format!("\n  {} Provider '{}' added successfully!", style("✓").green(), provider_id))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn edit_provider_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    if config.providers.is_empty() {
        term.write_line("\n  No providers to edit.\n")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let items: Vec<String> = config.providers.iter().map(|p| format!("{} ({})", p.id, p.name)).collect();
    let selection = Select::new().with_prompt("Select provider to edit").items(&items).default(0).interact()?;
    let provider = &config.providers[selection];

    term.write_line("\n  Edit Provider (press Enter to keep current value):")?;
    term.write_line("  ─────────────────────────────────────────────")?;

    let name: String = Input::new().with_prompt("Name").default(provider.name.clone()).interact_text()?;
    let base_url: String = Input::new().with_prompt("Base URL").default(provider.base_url.clone()).interact_text()?;
    let api_key: String = Input::new().with_prompt("API Key").default(provider.api_key.clone()).interact_text()?;
    let model: String = Input::new().with_prompt("Model").default(provider.model.clone()).interact_text()?;
    let notes: String = Input::new().with_prompt("Notes").default(provider.notes.clone().unwrap_or_default()).interact_text()?;
    let is_default = Confirm::new().with_prompt("Set as default provider?").default(provider.is_default).interact()?;

    let provider_id = provider.id.clone();
    let updates = ProviderUpdate {
        name: Some(name), base_url: Some(base_url), api_key: Some(api_key), model: Some(model),
        display_name: None, is_default: Some(is_default),
        notes: if notes.is_empty() { None } else { Some(notes) },
    };

    config.update_provider(&provider_id, updates)?;
    config.save(config_path)?;

    term.write_line(&format!("\n  {} Provider '{}' updated!", style("✓").green(), provider_id))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn copy_provider_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    if config.providers.is_empty() {
        term.write_line("\n  No providers to copy.\n")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let items: Vec<String> = config.providers.iter().map(|p| format!("{} ({})", p.id, p.name)).collect();
    let selection = Select::new().with_prompt("Select provider to copy").items(&items).default(0).interact()?;
    let source_id = config.providers[selection].id.clone();

    let new_id: String = Input::new().with_prompt("New provider ID").default(format!("{}-copy", source_id)).interact_text()?;

    config.copy_provider(&source_id, &new_id)?;
    config.save(config_path)?;

    term.write_line(&format!("\n  {} Provider '{}' copied to '{}'!", style("✓").green(), source_id, new_id))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn remove_provider_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    if config.providers.is_empty() {
        term.write_line("\n  No providers to remove.\n")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let items: Vec<String> = config.providers.iter().map(|p| {
        let default = if p.is_default { " ⭐" } else { "" };
        format!("{} ({}){}", p.id, p.name, default)
    }).collect();

    let selection = Select::new().with_prompt("Select provider to remove").items(&items).default(0).interact()?;
    let provider_id = config.providers[selection].id.clone();

    let confirm = Confirm::new().with_prompt(format!("Remove provider '{}'?", provider_id)).default(false).interact()?;
    if confirm {
        config.remove_provider(&provider_id)?;
        config.save(config_path)?;
        term.write_line(&format!("\n  {} Provider '{}' removed!", style("✓").green(), provider_id))?;
    } else {
        term.write_line("\n  Cancelled.")?;
    }

    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn set_default_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    if config.providers.is_empty() {
        term.write_line("\n  No providers configured.\n")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let items: Vec<String> = config.providers.iter().map(|p| {
        let default = if p.is_default { " ⭐ (current)" } else { "" };
        format!("{} ({}){}", p.id, p.name, default)
    }).collect();

    let selection = Select::new().with_prompt("Select default provider").items(&items).default(0).interact()?;
    let provider_id = config.providers[selection].id.clone();

    config.set_default_provider(&provider_id)?;
    config.save(config_path)?;

    term.write_line(&format!("\n  {} Default provider set to '{}'!", style("✓").green(), provider_id))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn list_presets_interactive() -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let categories = presets::get_categories();
    let category_items: Vec<String> = categories.iter().map(|c| format!("{} ({})", presets::get_category_display_name(c), presets::get_presets_by_category(c).len())).collect();

    term.write_line("\n  Available Presets:")?;

    loop {
        let cat_selection = Select::new().with_prompt("Select category").items(&category_items).default(0).interact()?;
        let selected_category = categories[cat_selection];
        let presets_list = presets::get_presets_by_category(selected_category);

        term.write_line(&format!("\n  {} presets:", presets::get_category_display_name(selected_category)))?;
        term.write_line("  ────────────────────────────────────────────────────────────────────────")?;
        term.write_line(&format!("  {:<20} {:<30} {}", "ID", "Name", "Display Name"))?;
        term.write_line("  ────────────────────────────────────────────────────────────────────────")?;

        for p in &presets_list {
            term.write_line(&format!("  {:<20} {:<30} {}", p.id, p.name, p.display_name))?;
        }

        term.write_line("  ────────────────────────────────────────────────────────────────────────")?;
        term.write_line("\n  Press Enter to go back...")?;
        term.read_line()?;
        term.clear_screen()?;
    }
}

fn import_interactive(config_path: &PathBuf) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line("\n  Import from cc-switch:")?;
    term.write_line("  ─────────────────────────────────────────────")?;

    let cc_switch_db = dirs::home_dir().unwrap_or_default().join(".cc-switch").join("cc-switch.db");

    if !cc_switch_db.exists() {
        term.write_line(&format!("\n  {} cc-switch database not found at: {}", style("✗").red(), cc_switch_db.display()))?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    term.write_line(&format!("  Found cc-switch database: {}", cc_switch_db.display()))?;

    let confirm = Confirm::new().with_prompt("Import providers from cc-switch?").default(true).interact()?;
    if !confirm {
        term.write_line("  Cancelled.")?;
        term.write_line("  Press Enter to continue...")?;
        term.read_line()?;
        return Ok(());
    }

    let imported_config = crate::config::import_from_cc_switch()?;
    let mut config = if config_path.exists() { AppConfig::load(config_path)? } else { AppConfig::default() };

    let mut imported_count = 0;
    for provider in imported_config.providers {
        if !config.providers.iter().any(|p| p.id == provider.id) {
            config.providers.push(provider);
            imported_count += 1;
        }
    }

    if !config.providers.iter().any(|p| p.is_default) {
        if let Some(first) = config.providers.first_mut() { first.is_default = true; }
    }

    config.save(config_path)?;

    term.write_line(&format!("\n  {} Imported {} providers from cc-switch!", style("✓").green(), imported_count))?;
    term.write_line("  Press Enter to continue...")?;
    term.read_line()?;
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len { s.to_string() } else { format!("{}...", &s[..max_len - 3]) }
}
