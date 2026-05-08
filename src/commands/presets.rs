//! Presets command - browse available presets

use crate::config::presets;
use console::style;

pub fn show_presets(category: Option<&str>) -> anyhow::Result<()> {
    let presets_list = if let Some(cat) = category {
        presets::get_presets_by_category(cat)
    } else {
        presets::get_all_presets()
    };

    if presets_list.is_empty() {
        println!("  No presets found.");
        return Ok(());
    }

    println!("\n  {} Available Presets\n", style("📦").cyan());

    let mut current_category = String::new();
    for preset in &presets_list {
        if preset.category != current_category {
            current_category = preset.category.to_string();
            println!(
                "\n  {} {}",
                style("▶").cyan(),
                style(presets::get_category_display_name(&current_category))
                    .cyan()
                    .bold()
            );
            println!("  {}", style("─".repeat(40)).dim());
        }
        println!(
            "    {:<20} {:<25} {}",
            style(preset.id).green(),
            preset.name,
            preset.display_name
        );
    }

    println!(
        "\n  {} Total: {} presets\n",
        style("ℹ").blue(),
        style(presets_list.len()).cyan()
    );

    Ok(())
}
