//! Usage statistics module

use crate::database::Database;
use chrono::{Duration, Utc};
use console::style;

/// Show usage statistics
pub fn show_usage(db: &Database, days: i32) -> anyhow::Result<()> {
    let app_type = "claude";

    // Get summary
    let summary = db.get_usage_summary(app_type, days)?;

    println!(
        "\n  {} Usage Statistics (Last {} Days)",
        style("📊").cyan(),
        days
    );
    println!("  ────────────────────────────────────────────────────────────────────────");
    println!("  {:<25} {:>15}", "Total Requests", summary.total_requests);
    println!("  {:<25} {:>15}", "Successful", summary.total_success);
    println!(
        "  {:<25} {:>15}%",
        "Success Rate",
        if summary.total_requests > 0 {
            (summary.total_success as f64 / summary.total_requests as f64 * 100.0) as i64
        } else {
            0
        }
    );
    println!(
        "  {:<25} {:>15}",
        "Input Tokens",
        format_number(summary.total_input_tokens)
    );
    println!(
        "  {:<25} {:>15}",
        "Output Tokens",
        format_number(summary.total_output_tokens)
    );
    println!(
        "  {:<25} {:>15}",
        "Total Cost",
        format!("${:.4}", summary.total_cost_usd)
    );
    println!("  {:<25} {:>15}ms", "Avg Latency", summary.avg_latency_ms);
    println!("  ────────────────────────────────────────────────────────────────────────");

    // Get daily breakdown
    let end_date = Utc::now().format("%Y-%m-%d").to_string();
    let start_date = (Utc::now() - Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let stats = db.get_usage_stats(app_type, &start_date, &end_date)?;

    if !stats.is_empty() {
        println!("\n  Daily Breakdown:");
        println!("  ────────────────────────────────────────────────────────────────────────");
        println!(
            "  {:<12} {:<10} {:>12} {:>12} {:>12} {:>10}",
            "Date", "Provider", "Requests", "Input Tok", "Output Tok", "Cost"
        );
        println!("  ────────────────────────────────────────────────────────────────────────");

        for stat in &stats {
            println!(
                "  {:<12} {:<10} {:>12} {:>12} {:>12} {:>10}",
                stat.date,
                truncate(&stat.provider_id, 9),
                stat.request_count,
                format_number(stat.input_tokens),
                format_number(stat.output_tokens),
                format!("${:.4}", stat.total_cost_usd),
            );
        }
        println!("  ────────────────────────────────────────────────────────────────────────");
    }

    Ok(())
}

/// Show usage for a specific provider
pub fn show_provider_usage(db: &Database, provider_id: &str, days: i32) -> anyhow::Result<()> {
    let app_type = "claude";

    let end_date = Utc::now().format("%Y-%m-%d").to_string();
    let start_date = (Utc::now() - Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let stats = db.get_usage_stats(app_type, &start_date, &end_date)?;
    let provider_stats: Vec<_> = stats
        .iter()
        .filter(|s| s.provider_id == provider_id)
        .collect();

    if provider_stats.is_empty() {
        println!("\n  No usage data found for provider '{}'", provider_id);
        return Ok(());
    }

    let total_requests: i64 = provider_stats.iter().map(|s| s.request_count).sum();
    let total_input: i64 = provider_stats.iter().map(|s| s.input_tokens).sum();
    let total_output: i64 = provider_stats.iter().map(|s| s.output_tokens).sum();
    let total_cost: f64 = provider_stats.iter().map(|s| s.total_cost_usd).sum();

    println!(
        "\n  {} Usage for Provider: {}",
        style("📊").cyan(),
        style(provider_id).green()
    );
    println!("  ────────────────────────────────────────────────────────────────────────");
    println!("  {:<25} {:>15}", "Total Requests", total_requests);
    println!(
        "  {:<25} {:>15}",
        "Input Tokens",
        format_number(total_input)
    );
    println!(
        "  {:<25} {:>15}",
        "Output Tokens",
        format_number(total_output)
    );
    println!(
        "  {:<25} {:>15}",
        "Total Cost",
        format!("${:.4}", total_cost)
    );
    println!("  ────────────────────────────────────────────────────────────────────────");

    println!("\n  Daily Breakdown:");
    println!("  ────────────────────────────────────────────────────────────────────────");
    println!(
        "  {:<12} {:>12} {:>12} {:>12} {:>10}",
        "Date", "Requests", "Input Tok", "Output Tok", "Cost"
    );
    println!("  ────────────────────────────────────────────────────────────────────────");

    for stat in &provider_stats {
        println!(
            "  {:<12} {:>12} {:>12} {:>12} {:>10}",
            stat.date,
            stat.request_count,
            format_number(stat.input_tokens),
            format_number(stat.output_tokens),
            format!("${:.4}", stat.total_cost_usd),
        );
    }
    println!("  ────────────────────────────────────────────────────────────────────────");

    Ok(())
}

fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
