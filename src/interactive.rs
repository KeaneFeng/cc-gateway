//! Interactive TUI Dashboard
//!
//! cc-gateway TUI - full-featured interactive management
//! Supports ESC to go back at any point

use crate::config::{presets, ApiFormat, AppConfig, ProviderConfig, ProviderUpdate};
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Helper: show after-action menu (return to main menu / exit)
/// Returns true if user wants to return to main menu, false to exit
fn after_action_menu() -> bool {
    let options = vec!["返回主菜单".to_string(), "退出".to_string()];
    match select_with_esc("  操作完成", &options, 0) {
        Some(0) => true, // 返回主菜单
        _ => false,      // ESC or 退出
    }
}

/// Helper: notify proxy to reload config
fn reload_proxy_config() {
    let client = reqwest::blocking::Client::new();
    let _ = client
        .post("http://127.0.0.1:16789/api/reload")
        .timeout(std::time::Duration::from_secs(2))
        .send();
}

/// Helper: run a Select with ESC support, returns None if ESC pressed
fn select_with_esc(prompt: &str, items: &[String], default: usize) -> Option<usize> {
    let theme = ColorfulTheme::default();
    match Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact_opt()
    {
        Ok(Some(idx)) => Some(idx),
        _ => None, // ESC or error
    }
}

/// Helper: run an Input with ESC support, returns None if ESC pressed
fn input_with_esc(prompt: &str, default: Option<&str>) -> Option<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::with_theme(&theme).with_prompt(prompt);
    if let Some(d) = default {
        input = input.default(d.to_string());
    }
    match input.interact_text() {
        Ok(val) => Some(val),
        _ => None,
    }
}

/// Helper: run a Confirm with ESC support, returns None if ESC pressed
fn confirm_with_esc(prompt: &str, default: bool) -> Option<bool> {
    let theme = ColorfulTheme::default();
    match Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .interact_opt()
    {
        Ok(Some(val)) => Some(val),
        _ => None,
    }
}

/// Main dashboard
pub fn run_dashboard(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let term = Term::stdout();

    loop {
        term.clear_screen()?;

        // Header
        term.write_line(&format!(
            "\n  {} {}",
            style("cc-gateway").cyan().bold(),
            style(format!("v{}", VERSION)).dim()
        ))?;
        term.write_line(&format!(
            "  {}",
            style("Per-project provider routing for Claude Code").dim()
        ))?;

        // Load config
        let config = if path.exists() {
            AppConfig::load(path)?
        } else {
            AppConfig::default()
        };

        // Get current active provider from proxy
        let current_provider_id = get_current_provider_id();

        // Provider table
        term.write_line("")?;
        if config.providers.is_empty() {
            term.write_line(&format!(
                "  {} 没有配置 Provider，添加一个开始使用。\n",
                style("⚠").yellow()
            ))?;
        } else {
            print_provider_table_with_active(&term, &config, current_provider_id.as_deref())?;
        }

        // Menu
        let options: Vec<String> = vec![
            "📋  查看 Provider 详情",
            "➕  添加 Provider",
            "✏️   编辑 Provider",
            "🗑️   删除 Provider",
            "⭐  设置默认 Provider",
            "📁  项目管理",
            "🔌  测试连接",
            "📊  查看统计",
            "📥  从 cc-switch 导入",
            "📦  浏览 Presets",
            "📋  复制配置",
            "❌  退出",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let selection = match select_with_esc("  选择操作 (ESC 退出)", &options, 0) {
            Some(s) => s,
            None => break, // ESC on main menu = exit
        };

        match selection {
            0 => list_providers(path)?,
            1 => add_provider(path, None)?,
            2 => edit_provider(path, None)?,
            3 => remove_provider(path, None)?,
            4 => set_default(path, None)?,
            5 => manage_projects(path)?,
            6 => test_connections(path)?,
            7 => show_usage(path)?,
            8 => import_providers(path, None)?,
            9 => browse_presets()?,
            10 => copy_config(path)?,
            11 => {
                term.write_line(&format!("\n  {} 再见!", style("👋").cyan()))?;
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Print provider table
fn print_provider_table(term: &Term, config: &AppConfig) -> anyhow::Result<()> {
    print_provider_table_with_active(term, config, None)
}

/// Print provider table with active provider highlight
fn print_provider_table_with_active(term: &Term, config: &AppConfig, active_provider_id: Option<&str>) -> anyhow::Result<()> {
    term.write_line(&format!(
        "  {:<4} {:<20} {:<20} {:<15} {}",
        style("#").dim(),
        style("名称").dim(),
        style("模型").dim(),
        style("图像模型").dim(),
        style("状态").dim()
    ))?;
    term.write_line(&format!("  {}", style("─".repeat(75)).dim()))?;

    for (i, p) in config.providers.iter().enumerate() {
        let is_active = active_provider_id.map(|id| id == p.id).unwrap_or(false);
        let status = if is_active {
            style("🟢 当前").green().bold().to_string()
        } else if p.is_default {
            style("⭐ 默认").yellow().to_string()
        } else {
            style("可用").dim().to_string()
        };
        let vision = p.vision_model.as_deref().unwrap_or("-");
        let display_name = p.display_name.as_deref().unwrap_or(&p.name);

        term.write_line(&format!(
            "  {:<4} {:<20} {:<20} {:<15} {}",
            i + 1,
            style(display_name).green(),
            &p.model,
            style(vision).cyan(),
            status
        ))?;
    }
    term.write_line("")?;
    Ok(())
}

/// Get current active provider ID from proxy
fn get_current_provider_id() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;

    let resp = client
        .get("http://127.0.0.1:16789/api/current-provider")
        .send()
        .ok()?;

    let json: serde_json::Value = resp.json().ok()?;
    json.get("provider_id")?.as_str().map(|s| s.to_string())
}

/// List providers (detailed view)
fn list_providers(config_path: &Path) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let config = AppConfig::load(config_path)?;

    term.write_line(&format!("\n  {} Provider 详情\n", style("📋").cyan()))?;

    if config.providers.is_empty() {
        term.write_line("  没有配置 Provider。\n")?;
    } else {
        for (i, p) in config.providers.iter().enumerate() {
            let default_mark = if p.is_default {
                format!(" {}", style("⭐ 默认").yellow())
            } else {
                String::new()
            };

            let display_name = p.display_name.as_deref().unwrap_or(&p.name);

            term.write_line(&format!(
                "  {}. {}{}",
                i + 1,
                style(display_name).green().bold(),
                default_mark
            ))?;
            term.write_line(&format!(
                "     ID:          {}",
                style(&p.id).dim()
            ))?;
            term.write_line(&format!("     模型:        {}", &p.model))?;
            if let Some(ref vm) = p.vision_model {
                term.write_line(&format!(
                    "     图像模型:    {}",
                    style(vm).cyan()
                ))?;
            }
            term.write_line(&format!("     API 格式:    {:?}", p.api_format))?;
            term.write_line(&format!(
                "     Base URL:    {}",
                style(&p.base_url).dim()
            ))?;
            if let Some(ref notes) = p.notes {
                term.write_line(&format!(
                    "     备注:        {}",
                    style(notes).dim()
                ))?;
            }
            term.write_line("")?;
        }

        term.write_line(&format!("  {} 使用说明:", style("💡").yellow()))?;
        term.write_line("     1. 确保 ~/.claude/settings.json 已配置代理地址:")?;
        term.write_line(&format!(
            "        {}",
            style("ANTHROPIC_BASE_URL=http://127.0.0.1:16789").cyan()
        ))?;
        term.write_line("     2. 在项目管理中设置项目 provider")?;
        term.write_line("     3. 在 Claude Code 中使用 /model 命令切换:")?;
        term.write_line(&format!(
            "        {}",
            style("/model claude-{project-path}").cyan()
        ))?;
        term.write_line("")?;
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Add a provider - cc-switch style: show all presets directly
pub fn add_provider(config_path: &Path, preset_id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();
    term.clear_screen()?;

    let mut config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    // If preset_id provided, use it directly
    if let Some(preset_id) = preset_id {
        let preset = presets::get_preset_by_id(preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_id))?;
        return add_from_preset(&term, &mut config, config_path, &preset);
    }

    // Show all presets in a flat list (like cc-switch)
    let all_presets = presets::get_all_presets();
    let categories = presets::get_categories();

    // Build items: presets grouped by category
    let mut items: Vec<String> = Vec::new();
    let mut preset_refs: Vec<Option<usize>> = Vec::new(); // None = custom, Some(idx) = preset index

    for category in &categories {
        let display_name = presets::get_category_display_name(category);
        let category_presets: Vec<_> = all_presets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.category == *category)
            .collect();

        if !category_presets.is_empty() {
            items.push(format!("── {} ──", display_name));
            preset_refs.push(None); // separator

            for (idx, preset) in &category_presets {
                items.push(format!("  {:<16} {}", preset.id, preset.display_name));
                preset_refs.push(Some(*idx));
            }
        }
    }

    // Add custom option at the end
    items.push(format!("── 其他 ──"));
    preset_refs.push(None);
    items.push(format!("  自定义 (手动输入)"));
    preset_refs.push(None);

    term.write_line(&format!("\n  {} 添加 Provider", style("➕").green()))?;
    term.write_line(&format!("  {}", style("选择 Preset 或自定义配置").dim()))?;
    term.write_line("")?;

    let selection = match select_with_esc("  选择 Provider (ESC 返回)", &items, 0) {
        Some(s) => s,
        None => return Ok(()),
    };

    // Check if selected a preset
    if let Some(Some(preset_idx)) = preset_refs.get(selection) {
        let preset = &all_presets[*preset_idx];
        return add_from_preset(&term, &mut config, config_path, preset);
    }

    // Custom provider
    add_custom_provider(&term, &mut config, config_path)
}

/// Add from a preset - show config and allow modification
fn add_from_preset(
    term: &Term,
    config: &mut AppConfig,
    config_path: &Path,
    preset: &presets::ProviderPreset,
) -> anyhow::Result<()> {
    term.write_line(&format!(
        "\n  {} 添加: {}",
        style("➕").green(),
        style(&preset.display_name).cyan()
    ))?;
    term.write_line(&format!(
        "  {}",
        style(format!("Base URL: {}", preset.base_url)).dim()
    ))?;
    term.write_line(&format!(
        "  {}",
        style(format!("API 格式: {:?}", preset.api_format)).dim()
    ))?;
    term.write_line(&format!(
        "  {}",
        style(format!("默认模型: {}", preset.model)).dim()
    ))?;
    term.write_line("")?;

    // Required: API Key
    let api_key = match input_with_esc("  API Key *", None) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Optional: Custom ID (default from preset)
    let custom_id = match input_with_esc("  Provider ID (可选修改)", Some(&preset.id.to_string())) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Optional: Model (default from preset)
    let model = match input_with_esc("  模型 (可选修改)", Some(&preset.model.to_string())) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Advanced options
    let show_advanced = match confirm_with_esc("  显示高级选项?", false) {
        Some(v) => v,
        None => return Ok(()),
    };

    let (vision_model, is_default, notes) = if show_advanced {
        let vision_model_input = match input_with_esc("  图像模型 (可选)", None) {
            Some(v) => v,
            None => return Ok(()),
        };
        let vision_model = if vision_model_input.is_empty() {
            None
        } else {
            Some(vision_model_input)
        };

        let is_default = match confirm_with_esc("  设为默认?", false) {
            Some(v) => v,
            None => return Ok(()),
        };

        let notes_input = match input_with_esc("  备注 (可选)", None) {
            Some(v) => v,
            None => return Ok(()),
        };
        let notes = if notes_input.is_empty() {
            None
        } else {
            Some(notes_input)
        };

        (vision_model, is_default, notes)
    } else {
        (None, false, None)
    };

    let provider = ProviderConfig {
        id: custom_id,
        name: preset.name.to_string(),
        api_format: preset.api_format.clone(),
        base_url: preset.base_url.to_string(),
        api_key,
        model,
        vision_model,
        display_name: Some(preset.display_name.to_string()),
        is_default,
        preset_id: Some(preset.id.to_string()),
        notes,
        effort_level: None,
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;
    reload_proxy_config();

    term.write_line(&format!(
        "\n  {} Provider '{}' 已添加!",
        style("✓").green(),
        style(&provider_id).cyan()
    ))?;
    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Add custom provider manually
fn add_custom_provider(
    term: &Term,
    config: &mut AppConfig,
    config_path: &Path,
) -> anyhow::Result<()> {
    term.write_line(&format!(
        "\n  {} 自定义 Provider",
        style("➕").green()
    ))?;
    term.write_line("")?;

    let id = match input_with_esc("  Provider ID", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let name = match input_with_esc("  Provider 名称", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let base_url = match input_with_esc("  Base URL", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let api_key = match input_with_esc("  API Key", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let model = match input_with_esc("  模型名称", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let vision_model_input = match input_with_esc("  图像模型 (可选)", None) {
        Some(v) => v,
        None => return Ok(()),
    };
    let vision_model = if vision_model_input.is_empty() {
        None
    } else {
        Some(vision_model_input)
    };

    let api_format_options = vec![
        ("anthropic (Anthropic Messages)", ApiFormat::Anthropic),
        ("openai_chat (OpenAI Chat)", ApiFormat::OpenAiChat),
        ("openai_responses (OpenAI Responses)", ApiFormat::OpenAiResponses),
        ("gemini_native (Gemini Native)", ApiFormat::GeminiNative),
    ];
    let api_format_items: Vec<String> = api_format_options
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    let api_format_selection =
        match select_with_esc("  API 格式", &api_format_items, 0) {
            Some(s) => s,
            None => return Ok(()),
        };
    let api_format = api_format_options[api_format_selection].1.clone();

    let is_default = match confirm_with_esc("  设为默认?", false) {
        Some(v) => v,
        None => return Ok(()),
    };

    let provider = ProviderConfig {
        id,
        name,
        api_format,
        base_url,
        api_key,
        model,
        vision_model,
        display_name: None,
        is_default,
        preset_id: None,
        notes: None,
        effort_level: None,
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;
    reload_proxy_config();

    term.write_line(&format!(
        "\n  {} Provider '{}' 已添加!",
        style("✓").green(),
        style(&provider_id).cyan()
    ))?;

    // Show options
    let options = vec![
        "返回主菜单".to_string(),
        "继续添加其他 Provider".to_string(),
        "退出".to_string(),
    ];

    let selection = match select_with_esc("  添加完成 (ESC 返回主菜单)", &options, 0) {
        Some(s) => s,
        None => return Ok(()), // ESC returns to main menu
    };

    match selection {
        0 => Ok(()),
        1 => {
            // Continue adding - call add_provider again
            add_provider(config_path, None)
        }
        2 => {
            std::process::exit(0);
        }
        _ => Ok(()),
    }
}

/// Edit a provider
pub fn edit_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  没有可编辑的 Provider。")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
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
                let display_name = p.display_name.as_deref().unwrap_or(&p.name);
                format!("{} ({}){}", display_name, p.model, default)
            })
            .collect();
        match select_with_esc("  选择要编辑的 Provider (ESC 返回)", &items, 0) {
            Some(s) => s,
            None => return Ok(()),
        }
    };

    let provider = &config.providers[provider_idx];
    let provider_id = provider.id.clone();

    term.write_line(&format!(
        "\n  {} 编辑 Provider '{}'",
        style("✏️").cyan(),
        style(provider.display_name.as_deref().unwrap_or(&provider.name)).green()
    ))?;
    term.write_line("  (回车保持当前值，ESC 返回)\n")?;

    let name = match input_with_esc("  名称", Some(&provider.name)) {
        Some(v) => v,
        None => return Ok(()),
    };
    let base_url = match input_with_esc("  Base URL", Some(&provider.base_url)) {
        Some(v) => v,
        None => return Ok(()),
    };
    let api_key = match input_with_esc("  API Key", Some(&provider.api_key)) {
        Some(v) => v,
        None => return Ok(()),
    };
    let model = match input_with_esc("  模型", Some(&provider.model)) {
        Some(v) => v,
        None => return Ok(()),
    };
    let vision_model_default = provider.vision_model.clone().unwrap_or_default();
    let vision_model_input =
        match input_with_esc("  图像模型 (空值清除)", Some(&vision_model_default)) {
            Some(v) => v,
            None => return Ok(()),
        };
    let vision_model = if vision_model_input.is_empty() {
        None
    } else {
        Some(vision_model_input)
    };
    let is_default = match confirm_with_esc("  设为默认?", provider.is_default) {
        Some(v) => v,
        None => return Ok(()),
    };

    let updates = ProviderUpdate {
        name: Some(name),
        base_url: Some(base_url),
        api_key: Some(api_key),
        model: Some(model),
        vision_model: Some(vision_model),
        display_name: None,
        is_default: Some(is_default),
        notes: None,
        effort_level: None,
    };

    config.update_provider(&provider_id, updates)?;
    config.save(config_path)?;
    reload_proxy_config();

    term.write_line(&format!(
        "\n  {} Provider '{}' 已更新!",
        style("✓").green(),
        style(&provider_id).cyan()
    ))?;
    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Remove a provider
pub fn remove_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  没有可删除的 Provider。")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
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
                let display_name = p.display_name.as_deref().unwrap_or(&p.name);
                format!("{} ({}){}", display_name, p.model, default)
            })
            .collect();
        let selection =
            match select_with_esc("  选择要删除的 Provider (ESC 返回)", &items, 0) {
                Some(s) => s,
                None => return Ok(()),
            };
        config.providers[selection].id.clone()
    };

    let confirm = match confirm_with_esc(
        &format!("  确认删除 '{}'?", provider_id),
        false,
    ) {
        Some(v) => v,
        None => return Ok(()),
    };

    if confirm {
        config.remove_provider(&provider_id)?;
        config.save(config_path)?;
    reload_proxy_config();
        term.write_line(&format!(
            "\n  {} Provider '{}' 已删除!",
            style("✓").green(),
            style(&provider_id).cyan()
        ))?;
    } else {
        term.write_line("\n  已取消。")?;
    }
    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Set default provider
pub fn set_default(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("\n  没有配置 Provider。")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
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
                let display_name = p.display_name.as_deref().unwrap_or(&p.name);
                let notes = p.notes.as_deref().unwrap_or("");
                if notes.is_empty() {
                    format!("{} ({}){}", display_name, p.model, default)
                } else {
                    format!("{} - {} ({}){}", display_name, notes, p.model, default)
                }
            })
            .collect();
        let selection =
            match select_with_esc("  选择默认 Provider (ESC 返回)", &items, 0) {
                Some(s) => s,
                None => return Ok(()),
            };
        config.providers[selection].id.clone()
    };

    // Update config file
    config.set_default_provider(&provider_id)?;
    config.save(config_path)?;
    reload_proxy_config();

    // Also switch at runtime via API (no restart needed)
    let client = reqwest::blocking::Client::new();
    let switch_result = client
        .post("http://127.0.0.1:16789/api/switch-provider")
        .json(&serde_json::json!({ "provider_id": provider_id }))
        .timeout(std::time::Duration::from_secs(2))
        .send();

    let provider_name = config
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .map(|p| {
            let display_name = p.display_name.as_deref().unwrap_or(&p.name);
            let notes = p.notes.as_deref().unwrap_or("");
            if notes.is_empty() {
                display_name.to_string()
            } else {
                format!("{} - {}", display_name, notes)
            }
        })
        .unwrap_or_else(|| provider_id.clone());

    match switch_result {
        Ok(resp) if resp.status().is_success() => {
            term.write_line(&format!(
                "\n  {} 默认 Provider 已切换为 {} (即时生效)",
                style("✓").green(),
                style(&provider_name).cyan()
            ))?;
        }
        _ => {
            term.write_line(&format!(
                "\n  {} 默认 Provider 已设为 {} (重启 cc-gateway serve 后生效)",
                style("✓").green(),
                style(&provider_name).cyan()
            ))?;
        }
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Test connections
fn test_connections(config_path: &Path) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line(&format!("\n  {} 测试连接...\n", style("🔌").cyan()))?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(crate::commands::test::run_test(
        &config_path.to_string_lossy(),
        None,
    ))?;

    // Show options after test
    let options = vec![
        "返回主菜单".to_string(),
        "退出".to_string(),
    ];

    let selection = match select_with_esc("  测试完成 (ESC 返回主菜单)", &options, 0) {
        Some(s) => s,
        None => return Ok(()), // ESC returns to main menu
    };

    if selection == 1 {
        std::process::exit(0);
    }

    Ok(())
}

/// Show usage statistics
fn show_usage(config_path: &Path) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line(&format!("\n  {} 使用统计\n", style("📊").cyan()))?;

    let config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term.write_line("  没有配置 Provider。\n")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
        return Ok(());
    }

    // Query usage from database
    let db = crate::database::Database::open_cc_switch_compatible()
        .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

    let days_options: Vec<String> = vec!["7 天".to_string(), "30 天".to_string(), "90 天".to_string(), "全部".to_string()]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let days_values = vec![7, 30, 90, 36500]; // 100 years = "all"

    let days_selection = match select_with_esc("  选择时间范围 (ESC 返回)", &days_options, 1)
    {
        Some(s) => s,
        None => return Ok(()),
    };
    let days = days_values[days_selection];

    let time_range = if days == 36500 {
        "全部".to_string()
    } else {
        format!("{} 天", days)
    };

    term.write_line(&format!("  时间范围: 最近 {}\n", time_range))?;

    // Show overall summary
    match db.get_usage_summary("claude", days) {
        Ok(summary) => {
            term.write_line(&format!(
                "  {:<20} {}",
                style("总请求数:").dim(),
                summary.total_requests
            ))?;
            term.write_line(&format!(
                "  {:<20} {}",
                style("成功请求:").dim(),
                summary.total_success
            ))?;
            term.write_line(&format!(
                "  {:<20} {}",
                style("输入 Tokens:").dim(),
                summary.total_input_tokens
            ))?;
            term.write_line(&format!(
                "  {:<20} {}",
                style("输出 Tokens:").dim(),
                summary.total_output_tokens
            ))?;
            term.write_line(&format!(
                "  {:<20} ${:.4}",
                style("总费用:").dim(),
                summary.total_cost_usd
            ))?;
            term.write_line(&format!(
                "  {:<20} {} ms",
                style("平均延迟:").dim(),
                summary.avg_latency_ms
            ))?;
        }
        Err(e) => {
            term.write_line(&format!("  {} 获取统计失败: {}", style("✗").red(), e))?;
        }
    }

    // Show per-provider statistics
    term.write_line(&format!("\n  {} 按 Provider 分组统计\n", style("📈").cyan()))?;

    match db.get_usage_by_provider("claude", days) {
        Ok(provider_stats) => {
            if provider_stats.is_empty() {
                term.write_line("  暂无使用记录。")?;
            } else {
                // Create a map of provider_id -> provider name
                let provider_name_map: std::collections::HashMap<String, String> = config
                    .providers
                    .iter()
                    .map(|p| {
                        let name = p.display_name.as_deref().unwrap_or(&p.name).to_string();
                        (p.id.clone(), name)
                    })
                    .collect();

                // Print table header
                term.write_line(&format!(
                    "  {:<20} {:<10} {:<12} {:<12} {:<10} {:<10}",
                    style("Provider").dim(),
                    style("请求数").dim(),
                    style("输入Tokens").dim(),
                    style("输出Tokens").dim(),
                    style("费用").dim(),
                    style("延迟").dim()
                ))?;
                term.write_line(&format!("  {}", style("─".repeat(80)).dim()))?;

                // Print each provider's stats
                for stats in provider_stats {
                    let provider_name = provider_name_map
                        .get(&stats.provider_id)
                        .cloned()
                        .unwrap_or_else(|| {
                            // Try to extract a short name from the ID
                            if stats.provider_id.len() > 20 {
                                format!("{}...", &stats.provider_id[..17])
                            } else {
                                stats.provider_id.clone()
                            }
                        });

                    let success_rate = if stats.total_requests > 0 {
                        (stats.total_success as f64 / stats.total_requests as f64 * 100.0) as u64
                    } else {
                        0
                    };

                    term.write_line(&format!(
                        "  {:<20} {:<10} {:<12} {:<12} ${:<9.4} {:<10}",
                        style(&provider_name).green(),
                        stats.total_requests,
                        stats.total_input_tokens,
                        stats.total_output_tokens,
                        stats.total_cost_usd,
                        format!("{} ms", stats.avg_latency_ms)
                    ))?;

                    // Show success rate if not 100%
                    if success_rate < 100 {
                        term.write_line(&format!(
                            "  {:<20} {}",
                            "",
                            style(format!("成功率: {}%", success_rate)).yellow()
                        ))?;
                    }
                }
            }
        }
        Err(e) => {
            term.write_line(&format!("  {} 获取分组统计失败: {}", style("✗").red(), e))?;
        }
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Import providers from cc-switch
pub fn import_providers(config_path: &Path, db_path: Option<&str>) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line(&format!(
        "\n  {} 从 cc-switch 导入",
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
            "  {} cc-switch 数据库未找到: {}",
            style("✗").red(),
            cc_switch_db.display()
        ))?;

        // Show options
        let options = vec![
            "返回主菜单".to_string(),
            "退出".to_string(),
        ];

        let selection = match select_with_esc("  错误 (ESC 返回主菜单)", &options, 0) {
            Some(s) => s,
            None => return Ok(()), // ESC returns to main menu
        };

        if selection == 1 {
            std::process::exit(0);
        }

        return Ok(());
    }

    term.write_line(&format!(
        "  找到: {}",
        style(cc_switch_db.display()).dim()
    ))?;

    let confirm = match confirm_with_esc("  导入 Provider?", true) {
        Some(v) => v,
        None => return Ok(()),
    };

    if !confirm {
        term.write_line("  已取消。")?;

        // Show options
        let options = vec![
            "返回主菜单".to_string(),
            "退出".to_string(),
        ];

        let selection = match select_with_esc("  已取消 (ESC 返回主菜单)", &options, 0) {
            Some(s) => s,
            None => return Ok(()), // ESC returns to main menu
        };

        if selection == 1 {
            std::process::exit(0);
        }

        return Ok(());
    }

    let config = crate::config::import_from_cc_switch()?;
    let count = config.providers.len();

    let mut existing_config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    for provider in config.providers {
        if !existing_config.providers.iter().any(|p| p.id == provider.id) {
            existing_config.providers.push(provider);
        }
    }

    if existing_config
        .providers
        .iter()
        .filter(|p| p.is_default)
        .count()
        == 0
    {
        if let Some(first) = existing_config.providers.first_mut() {
            first.is_default = true;
        }
    }

    existing_config.save(config_path)?;
    reload_proxy_config();

    term.write_line(&format!(
        "\n  {} 已导入 {} 个 Provider!",
        style("✓").green(),
        style(count).cyan()
    ))?;

    // Show options
    let options = vec![
        "返回主菜单".to_string(),
        "退出".to_string(),
    ];

    let selection = match select_with_esc("  导入完成 (ESC 返回主菜单)", &options, 0) {
        Some(s) => s,
        None => return Ok(()), // ESC returns to main menu
    };

    if selection == 1 {
        std::process::exit(0);
    }

    Ok(())
}

/// Browse presets
fn browse_presets() -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line(&format!("\n  {} 可用 Presets\n", style("📦").cyan()))?;

    let categories = presets::get_categories();
    for category in &categories {
        let display_name = presets::get_category_display_name(category);
        let presets_list = presets::get_presets_by_category(category);

        term.write_line(&format!(
            "  {}:",
            style(display_name).yellow().bold()
        ))?;
        for preset in &presets_list {
            term.write_line(&format!(
                "    {:<16} {}",
                style(preset.id).green(),
                preset.display_name
            ))?;
        }
        term.write_line("")?;
    }

    // Show options
    let options = vec![
        "返回主菜单".to_string(),
        "退出".to_string(),
    ];

    let selection = match select_with_esc("  浏览完成 (ESC 返回主菜单)", &options, 0) {
        Some(s) => s,
        None => return Ok(()), // ESC returns to main menu
    };

    if selection == 1 {
        std::process::exit(0);
    }

    Ok(())
}

/// Project management
fn manage_projects(config_path: &Path) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let mut config = AppConfig::load(config_path)?;

    term.write_line(&format!("\n  {} 项目管理", style("📁").cyan()))?;
    term.write_line("")?;

    // Scan ~/.claude/projects for JSONL session files
    let claude_projects_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("projects");

    // Map: project_path -> (session_count, last_used)
    let mut project_map: std::collections::HashMap<String, (usize, i64)> = std::collections::HashMap::new();

    if claude_projects_dir.exists() {
        // Recursively find all .jsonl files
        find_jsonl_files(&claude_projects_dir, &mut |path| {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Read first few lines to extract cwd
                for line in content.lines().take(20) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(cwd) = value.get("cwd").and_then(|v| v.as_str()) {
                            let project_path = cwd.to_string();
                            let timestamp = value.get("timestamp")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as i64;

                            let entry = project_map.entry(project_path).or_insert((0, 0));
                            entry.0 += 1; // session count
                            if timestamp > entry.1 {
                                entry.1 = timestamp; // last used
                            }
                            break;
                        }
                    }
                }
            }
        });
    }

    // Convert to sorted vector
    let mut projects: Vec<(String, usize, i64, String)> = project_map
        .into_iter()
        .map(|(path, (count, last_used))| {
            let provider_id = config
                .project_providers
                .get(&path)
                .cloned()
                .unwrap_or_else(|| "未设置".to_string());
            let provider_name = config
                .providers
                .iter()
                .find(|p| p.id == provider_id)
                .map(|p| p.display_name.as_deref().unwrap_or(&p.name).to_string())
                .unwrap_or(provider_id);
            (path, count, last_used, provider_name)
        })
        .collect();

    // Sort by last used (most recent first)
    projects.sort_by(|a, b| b.2.cmp(&a.2));

    if projects.is_empty() {
        term.write_line("  没有找到 Claude Code 使用记录。")?;
        term.write_line("")?;
        term.write_line("  请确保已使用 Claude Code 创建过会话。")?;
    } else {
        term.write_line(&format!("  找到 {} 个项目:\n", projects.len()))?;

        // Display projects
        for (i, (path, count, _, provider_id)) in projects.iter().enumerate() {
            let short_path = path.replace(
                &dirs::home_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "~",
            );

            // Get provider display name
            let provider_display = if provider_id == "未设置" {
                "未设置".to_string()
            } else {
                config
                    .providers
                    .iter()
                    .find(|p| p.id == *provider_id)
                    .map(|p| {
                        let display_name = p.display_name.as_deref().unwrap_or(&p.name);
                        let notes = p.notes.as_deref().unwrap_or("");
                        if notes.is_empty() {
                            display_name.to_string()
                        } else {
                            format!("{} - {}", display_name, notes)
                        }
                    })
                    .unwrap_or_else(|| provider_id.clone())
            };

            term.write_line(&format!(
                "  {}. {} {} {}",
                i + 1,
                style(&short_path).green(),
                style(format!("({} 次会话)", count)).dim(),
                style(format!("[{}]", provider_display)).yellow()
            ))?;
        }

        term.write_line("")?;

        // Options
        let options: Vec<String> = vec![
            "🔄  切换项目 Provider",
            "➕  添加到 cc-gateway 管理",
            "🔙  返回",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let selection = match select_with_esc("  选择操作 (ESC 返回)", &options, 0) {
            Some(s) => s,
            None => return Ok(()),
        };

        match selection {
            0 => {
                // Switch project provider
                let project_items: Vec<String> = projects
                    .iter()
                    .map(|(path, count, _, provider)| {
                        let short_path = path.replace(
                            &dirs::home_dir()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            "~",
                        );
                        format!("{} ({} 次会话) [{}]", short_path, count, provider)
                    })
                    .collect();

                let project_selection =
                    match select_with_esc("  选择项目 (ESC 返回)", &project_items, 0) {
                        Some(s) => s,
                        None => return Ok(()),
                    };

                let selected_path = &projects[project_selection].0;

                // Select provider
                let provider_items: Vec<String> = config
                    .providers
                    .iter()
                    .map(|p| {
                        let default = if p.is_default { " ⭐" } else { "" };
                        let display_name = p.display_name.as_deref().unwrap_or(&p.name);
                        let notes = p.notes.as_deref().unwrap_or("");
                        if notes.is_empty() {
                            format!("{} ({}){}", display_name, p.model, default)
                        } else {
                            format!("{} - {} ({}){}", display_name, notes, p.model, default)
                        }
                    })
                    .collect();

                let provider_selection =
                    match select_with_esc("  选择 Provider (ESC 返回)", &provider_items, 0) {
                        Some(s) => s,
                        None => return Ok(()),
                    };

                let selected_provider = &config.providers[provider_selection];
                let provider_id = selected_provider.id.clone();

                // Save to config
                config
                    .project_providers
                    .insert(selected_path.clone(), provider_id.clone());
                config.save(config_path)?;
    reload_proxy_config();

                let short_path = selected_path.replace(
                    &dirs::home_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    "~",
                );
                let provider_name = selected_provider
                    .display_name
                    .as_deref()
                    .unwrap_or(&selected_provider.name);

                term.write_line(&format!(
                    "\n  {} 已切换 {} → {} (代理层面由 SessionRouter 自动路由)",
                    style("✓").green(),
                    short_path,
                    style(provider_name).cyan()
                ))?;
            }
            1 => {
                // Add all to cc-gateway management
                for (path, _, _, _) in &projects {
                    if !config.project_providers.contains_key(path) {
                        let default_provider = config
                            .providers
                            .iter()
                            .find(|p| p.is_default)
                            .map(|p| p.id.clone())
                            .unwrap_or_default();
                        config
                            .project_providers
                            .insert(path.clone(), default_provider);
                    }
                }
                config.save(config_path)?;
    reload_proxy_config();

                term.write_line(&format!(
                    "\n  {} 已将所有项目添加到 cc-gateway 管理",
                    style("✓").green()
                ))?;
            }
            2 => return Ok(()),
            _ => {}
        }
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Recursively find .jsonl files
fn find_jsonl_files(dir: &Path, callback: &mut dyn FnMut(&Path)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_jsonl_files(&path, callback);
            } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                callback(&path);
            }
        }
    }
}

/// List projects
fn list_projects(term: &Term, config: &AppConfig) -> anyhow::Result<()> {
    term.clear_screen()?;
    term.write_line(&format!("\n  {} 项目列表", style("📋").cyan()))?;
    term.write_line("")?;

    if config.project_providers.is_empty() {
        term.write_line("  没有项目。请先扫描目录。")?;
    } else {
        for (i, (path, provider_id)) in config.project_providers.iter().enumerate() {
            let short_path = path.replace(&dirs::home_dir().unwrap_or_default().to_string_lossy().to_string(), "~");
            let provider_name = config
                .providers
                .iter()
                .find(|p| p.id == *provider_id)
                .map(|p| p.display_name.as_deref().unwrap_or(&p.name).to_string())
                .unwrap_or_else(|| provider_id.clone());

            term.write_line(&format!(
                "  {}. {} → {}",
                i + 1,
                style(&short_path).green(),
                style(&provider_name).cyan()
            ))?;
        }
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Switch project provider
fn switch_project_provider(
    term: &Term,
    config: &mut AppConfig,
    config_path: &Path,
) -> anyhow::Result<()> {
    term.clear_screen()?;
    term.write_line(&format!("\n  {} 切换项目 Provider", style("🔄").cyan()))?;
    term.write_line("")?;

    if config.project_providers.is_empty() {
        term.write_line("  没有项目。请先扫描目录。")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
        return Ok(());
    }

    // Select project
    let project_items: Vec<String> = config
        .project_providers
        .iter()
        .map(|(path, provider_id)| {
            let short_path = path.replace(&dirs::home_dir().unwrap_or_default().to_string_lossy().to_string(), "~");
            let provider_name = config
                .providers
                .iter()
                .find(|p| p.id == *provider_id)
                .map(|p| p.display_name.as_deref().unwrap_or(&p.name).to_string())
                .unwrap_or_else(|| provider_id.clone());
            format!("{} → {}", short_path, provider_name)
        })
        .collect();

    let project_paths: Vec<String> = config.project_providers.keys().cloned().collect();

    let project_selection =
        match select_with_esc("  选择项目 (ESC 返回)", &project_items, 0) {
            Some(s) => s,
            None => return Ok(()),
        };

    let selected_path = &project_paths[project_selection];

    // Select provider
    let provider_items: Vec<String> = config
        .providers
        .iter()
        .map(|p| {
            let default = if p.is_default { " ⭐" } else { "" };
            let display_name = p.display_name.as_deref().unwrap_or(&p.name);
            format!("{} ({}){}", display_name, p.model, default)
        })
        .collect();

    let provider_selection =
        match select_with_esc("  选择 Provider (ESC 返回)", &provider_items, 0) {
            Some(s) => s,
            None => return Ok(()),
        };

    let selected_provider = &config.providers[provider_selection];
    let provider_id = selected_provider.id.clone();

    // Update project provider
    config.project_providers.insert(selected_path.clone(), provider_id.clone());
    config.save(config_path)?;
    reload_proxy_config();

    let short_path = selected_path.replace(&dirs::home_dir().unwrap_or_default().to_string_lossy().to_string(), "~");
    let provider_name = selected_provider.display_name.as_deref().unwrap_or(&selected_provider.name);

    term.write_line(&format!(
        "\n  {} 已切换 {} → {} (代理层面由 SessionRouter 自动路由)",
        style("✓").green(),
        style(&short_path).cyan(),
        style(provider_name).green()
    ))?;
    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Add project directory
fn add_project_dir(
    term: &Term,
    config: &mut AppConfig,
    config_path: &Path,
) -> anyhow::Result<()> {
    term.clear_screen()?;
    term.write_line(&format!("\n  {} 添加项目目录", style("➕").cyan()))?;
    term.write_line("")?;

    let dir = match input_with_esc("  输入目录路径 (ESC 返回)", None) {
        Some(v) => v,
        None => return Ok(()),
    };

    if !std::path::Path::new(&dir).exists() {
        term.write_line(&format!("  {} 目录不存在: {}", style("✗").red(), dir))?;
        if !after_action_menu() {
            std::process::exit(0);
        }
        return Ok(());
    }

    if config.project_dirs.contains(&dir) {
        term.write_line("  目录已存在。")?;
    } else {
        config.project_dirs.push(dir.clone());
        config.save(config_path)?;
    reload_proxy_config();
        term.write_line(&format!("  {} 已添加: {}", style("✓").green(), dir))?;
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}

/// Copy config - generate Claude Code settings.json
fn copy_config(config_path: &Path) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    let config = AppConfig::load(config_path)?;

    term.write_line(&format!(
        "\n  {} 复制配置",
        style("📋").cyan()
    ))?;
    term.write_line("")?;

    if config.providers.is_empty() {
        term.write_line("  没有配置 Provider。")?;
        if !after_action_menu() {
            std::process::exit(0);
        }
        return Ok(());
    }

    // Select provider
    let items: Vec<String> = config
        .providers
        .iter()
        .map(|p| {
            let default = if p.is_default { " ⭐" } else { "" };
            let display_name = p.display_name.as_deref().unwrap_or(&p.name);
            format!("{} ({}){}", display_name, p.model, default)
        })
        .collect();

    let selection = match select_with_esc("  选择 Provider (ESC 返回)", &items, 0) {
        Some(s) => s,
        None => return Ok(()),
    };

    let provider = &config.providers[selection];
    let base_url = format!("http://127.0.0.1:16789/provider/{}", provider.id);

    // Generate settings.json
    let settings = serde_json::json!({
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
            "ANTHROPIC_BASE_URL": base_url,
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
            "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1",
            "ENABLE_TOOL_SEARCH": "true"
        },
        "alwaysThinkingEnabled": true,
        "model": "opus",
        "permissions": {
            "allow": [],
            "deny": []
        }
    });

    let settings_str = serde_json::to_string_pretty(&settings)?;

    term.write_line(&format!(
        "  {} 生成的配置 (Provider: {}):\n",
        style("✓").green(),
        provider.display_name.as_deref().unwrap_or(&provider.name)
    ))?;
    term.write_line(&settings_str)?;
    term.write_line("")?;

    // Copy to clipboard if possible
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::Command;

        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(settings_str.as_bytes())?;
        }
        child.wait()?;

        term.write_line(&format!(
            "  {} 已复制到剪贴板！",
            style("📋").green()
        ))?;
    }

    term.write_line(&format!(
        "  {} 保存到文件？",
        style("💾").cyan()
    ))?;

    let save = match confirm_with_esc("  保存到项目 .claude/settings.json?", false) {
        Some(v) => v,
        None => return Ok(()),
    };

    if save {
        let project_path = match input_with_esc("  项目路径", None) {
            Some(v) => v,
            None => return Ok(()),
        };

        let claude_dir = std::path::Path::new(&project_path).join(".claude");
        std::fs::create_dir_all(&claude_dir)?;
        let settings_path = claude_dir.join("settings.json");
        std::fs::write(&settings_path, &settings_str)?;

        term.write_line(&format!(
            "\n  {} 已保存到 {}",
            style("✓").green(),
            settings_path.display()
        ))?;
    }

    if !after_action_menu() {
        std::process::exit(0);
    }
    Ok(())
}
