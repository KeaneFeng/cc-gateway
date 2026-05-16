//! Provider balance/quota query service
//!
//! Supports: DeepSeek, StepFun, SiliconFlow, OpenRouter, Novita AI, Zhipu

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Shared blocking HTTP client for balance queries.
/// Configuration mirrors cc-switch's global client:
/// - Follows system proxy (HTTPS_PROXY, HTTP_PROXY, ALL_PROXY env vars)
/// - Disables auto-decompression (prevents encoding issues)
/// - 30s connect timeout, 60s total timeout (balance APIs are fast)
static BALANCE_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to build balance HTTP client")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceData {
    pub plan_name: String,
    pub remaining: Option<f64>,
    pub total: Option<f64>,
    pub used: Option<f64>,
    pub unit: String,
    pub is_valid: bool,
    pub invalid_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResult {
    pub success: bool,
    pub data: Vec<BalanceData>,
    pub error: Option<String>,
}

enum BalanceProvider {
    DeepSeek,
    StepFun,
    SiliconFlow,
    SiliconFlowEn,
    OpenRouter,
    NovitaAI,
    Zhipu,
}

fn detect_provider(base_url: &str) -> Option<BalanceProvider> {
    let url = base_url.to_lowercase();
    if url.contains("api.deepseek.com") {
        Some(BalanceProvider::DeepSeek)
    } else if url.contains("api.stepfun.ai") || url.contains("api.stepfun.com") {
        Some(BalanceProvider::StepFun)
    } else if url.contains("api.siliconflow.cn") {
        Some(BalanceProvider::SiliconFlow)
    } else if url.contains("api.siliconflow.com") {
        Some(BalanceProvider::SiliconFlowEn)
    } else if url.contains("openrouter.ai") {
        Some(BalanceProvider::OpenRouter)
    } else if url.contains("api.novita.ai") {
        Some(BalanceProvider::NovitaAI)
    } else if url.contains("bigmodel.cn") || url.contains("z.ai") {
        Some(BalanceProvider::Zhipu)
    } else {
        None
    }
}

fn parse_f64_field(obj: &serde_json::Value, field: &str) -> Option<f64> {
    obj.get(field).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn make_error(msg: String) -> BalanceResult {
    BalanceResult {
        success: false,
        data: vec![],
        error: Some(msg),
    }
}

fn make_auth_error(status: reqwest::StatusCode) -> BalanceResult {
    BalanceResult {
        success: false,
        data: vec![BalanceData {
            plan_name: "Auth".to_string(),
            remaining: None,
            total: None,
            used: None,
            unit: String::new(),
            is_valid: false,
            invalid_message: Some(format!("Authentication failed (HTTP {status})")),
        }],
        error: Some(format!("Authentication failed (HTTP {status})")),
    }
}

fn query_deepseek(api_key: &str) -> BalanceResult {
    let resp = match BALANCE_CLIENT
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    let is_available = body
        .get("is_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut data = Vec::new();

    if let Some(infos) = body.get("balance_infos").and_then(|v| v.as_array()) {
        for info in infos {
            let currency = info
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("CNY");
            let total = parse_f64_field(info, "total_balance");

            data.push(BalanceData {
                plan_name: currency.to_string(),
                remaining: total,
                total: None,
                used: None,
                unit: currency.to_string(),
                is_valid: is_available,
                invalid_message: if !is_available {
                    Some("Insufficient balance".to_string())
                } else {
                    None
                },
            });
        }
    }

    BalanceResult {
        success: true,
        data,
        error: None,
    }
}

fn query_stepfun(api_key: &str) -> BalanceResult {
    let resp = match BALANCE_CLIENT
        .get("https://api.stepfun.com/v1/accounts")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    let balance = parse_f64_field(&body, "balance").unwrap_or(0.0);

    BalanceResult {
        success: true,
        data: vec![BalanceData {
            plan_name: "StepFun".to_string(),
            remaining: Some(balance),
            total: None,
            used: None,
            unit: "CNY".to_string(),
            is_valid: true,
            invalid_message: None,
        }],
        error: None,
    }
}

fn query_siliconflow(api_key: &str, is_cn: bool) -> BalanceResult {
    let domain = if is_cn {
        "api.siliconflow.cn"
    } else {
        "api.siliconflow.com"
    };
    let url = format!("https://{domain}/v1/user/info");

    let resp = match BALANCE_CLIENT
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    let data = match body.get("data") {
        Some(d) => d,
        None => return make_error("Missing 'data' field in response".to_string()),
    };

    let total_balance = parse_f64_field(data, "totalBalance").unwrap_or(0.0);
    let unit = if is_cn { "CNY" } else { "USD" };
    let plan_name = if is_cn {
        "SiliconFlow"
    } else {
        "SiliconFlow (EN)"
    };

    BalanceResult {
        success: true,
        data: vec![BalanceData {
            plan_name: plan_name.to_string(),
            remaining: Some(total_balance),
            total: None,
            used: None,
            unit: unit.to_string(),
            is_valid: true,
            invalid_message: None,
        }],
        error: None,
    }
}

fn query_openrouter(api_key: &str) -> BalanceResult {
    let resp = match BALANCE_CLIENT
        .get("https://openrouter.ai/api/v1/credits")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    let data = body.get("data").unwrap_or(&body);
    let total_credits = parse_f64_field(data, "total_credits").unwrap_or(0.0);
    let total_usage = parse_f64_field(data, "total_usage").unwrap_or(0.0);
    let remaining = total_credits - total_usage;

    BalanceResult {
        success: true,
        data: vec![BalanceData {
            plan_name: "OpenRouter".to_string(),
            remaining: Some(remaining),
            total: Some(total_credits),
            used: Some(total_usage),
            unit: "USD".to_string(),
            is_valid: remaining > 0.0,
            invalid_message: if remaining <= 0.0 {
                Some("No credits remaining".to_string())
            } else {
                None
            },
        }],
        error: None,
    }
}

fn query_novita(api_key: &str) -> BalanceResult {
    let resp = match BALANCE_CLIENT
        .get("https://api.novita.ai/v3/user/balance")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    // Novita amounts are in 0.0001 USD units
    let available = parse_f64_field(&body, "availableBalance").unwrap_or(0.0) / 10000.0;

    BalanceResult {
        success: true,
        data: vec![BalanceData {
            plan_name: "Novita AI".to_string(),
            remaining: Some(available),
            total: None,
            used: None,
            unit: "USD".to_string(),
            is_valid: available > 0.0,
            invalid_message: if available <= 0.0 {
                Some("No balance remaining".to_string())
            } else {
                None
            },
        }],
        error: None,
    }
}

fn query_zhipu(api_key: &str) -> BalanceResult {
    let resp = match BALANCE_CLIENT
        .get("https://api.z.ai/api/monitor/usage/quota/limit")
        .header("Authorization", api_key) // Zhipu: no Bearer prefix
        .header("Content-Type", "application/json")
        .header("Accept-Language", "en-US,en")
        .timeout(std::time::Duration::from_secs(10))
        .send()
    {
        Ok(r) => r,
        Err(e) => return make_error(format!("Network error: {e}")),
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return make_auth_error(status);
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return make_error(format!("API error (HTTP {status}): {body}"));
    }

    let body: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => return make_error(format!("Failed to parse response: {e}")),
    };

    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return make_error(format!("API error: {msg}"));
    }

    let data = match body.get("data") {
        Some(d) => d,
        None => return make_error("Missing 'data' field in response".to_string()),
    };

    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Parse limits[] array — show used% (matching Zhipu console display)
    // Types: TOKENS_LIMIT (5h + weekly), MCP_LIMIT (monthly)
    let mut items = Vec::new();
    if let Some(limits) = data.get("limits").and_then(|v| v.as_array()) {
        let mut token_entries: Vec<(i64, f64, String)> = Vec::new(); // (reset_ms, used%, reset_label)
        let mut mcp_entries: Vec<(i64, f64, String)> = Vec::new();

        for item in limits {
            let limit_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let percentage = item
                .get("percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let reset_ms = item
                .get("nextResetTime")
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MAX);

            let reset_label = if reset_ms == i64::MAX {
                String::new()
            } else {
                let secs = reset_ms / 1000;
                use chrono::{TimeZone, Utc};
                Utc.timestamp_opt(secs, 0)
                    .single()
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_default()
            };

            if limit_type.eq_ignore_ascii_case("TOKENS_LIMIT") {
                token_entries.push((reset_ms, percentage, reset_label));
            } else if limit_type.to_lowercase().contains("mcp") {
                mcp_entries.push((reset_ms, percentage, reset_label));
            }
        }

        // TOKENS_LIMIT: sort by reset time — first is 5h, second is weekly
        token_entries.sort_by_key(|e| e.0);
        let tier_labels: [&str; 2] = ["5h quota", "Weekly"];
        for (idx, (_reset_ms, used_pct, reset_label)) in token_entries.into_iter().enumerate() {
            if idx >= 2 {
                break;
            }
            let used_rounded = used_pct.round() as i64;
            let reset_str = if reset_label.is_empty() {
                String::new()
            } else {
                format!(" (reset: {})", reset_label)
            };
            items.push(BalanceData {
                plan_name: format!("{}{}: {}% used", tier_labels[idx], reset_str, used_rounded),
                remaining: Some(100.0 - used_pct),
                total: Some(100.0),
                used: Some(used_pct),
                unit: "%".to_string(),
                is_valid: used_pct < 100.0,
                invalid_message: if used_pct >= 100.0 {
                    Some("Quota exhausted".to_string())
                } else {
                    None
                },
            });
        }

        // MCP_LIMIT: show as monthly
        for (_reset_ms, used_pct, reset_label) in &mcp_entries {
            let used_rounded = used_pct.round() as i64;
            let reset_str = if reset_label.is_empty() {
                String::new()
            } else {
                format!(" (reset: {})", reset_label)
            };
            items.push(BalanceData {
                plan_name: format!("MCP Monthly{}: {}% used", reset_str, used_rounded),
                remaining: Some(100.0 - used_pct),
                total: Some(100.0),
                used: Some(*used_pct),
                unit: "%".to_string(),
                is_valid: *used_pct < 100.0,
                invalid_message: if *used_pct >= 100.0 {
                    Some("Quota exhausted".to_string())
                } else {
                    None
                },
            });
        }
    }

    if items.is_empty() {
        // Fallback: just show level
        items.push(BalanceData {
            plan_name: format!("Zhipu (level: {})", level),
            remaining: None,
            total: None,
            used: None,
            unit: String::new(),
            is_valid: true,
            invalid_message: None,
        });
    }

    BalanceResult {
        success: true,
        data: items,
        error: None,
    }
}

/// Query provider balance synchronously. Returns None if provider not supported.
pub fn query_balance(base_url: &str, api_key: &str) -> Option<BalanceResult> {
    if api_key.trim().is_empty() {
        return Some(BalanceResult {
            success: false,
            data: vec![],
            error: Some("API key is empty".to_string()),
        });
    }

    let provider = detect_provider(base_url)?;
    Some(match provider {
        BalanceProvider::DeepSeek => query_deepseek(api_key),
        BalanceProvider::StepFun => query_stepfun(api_key),
        BalanceProvider::SiliconFlow => query_siliconflow(api_key, true),
        BalanceProvider::SiliconFlowEn => query_siliconflow(api_key, false),
        BalanceProvider::OpenRouter => query_openrouter(api_key),
        BalanceProvider::NovitaAI => query_novita(api_key),
        BalanceProvider::Zhipu => query_zhipu(api_key),
    })
}

/// Check if a base_url belongs to a supported balance provider.
pub fn is_balance_supported(base_url: &str) -> bool {
    detect_provider(base_url).is_some()
}
