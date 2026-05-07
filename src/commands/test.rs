//! Connection test module
//!
//! Test connectivity to providers

use crate::database::{Database, StreamCheckLog};
use reqwest::Client;
use std::time::{Duration, Instant};

/// Test result
#[derive(Debug)]
pub struct TestResult {
    pub provider_id: String,
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<u64>,
    pub http_status: Option<u16>,
}

/// Provider info for testing
#[derive(Debug, Clone)]
pub struct TestProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_type: String,
}

/// Test connection to a provider
pub async fn test_provider(provider: &TestProvider) -> TestResult {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let start = Instant::now();
    
    // Try to list models endpoint
    let url = format!("{}/v1/models", provider.base_url.trim_end_matches('/'));
    
    let mut request = client.get(&url);
    
    if provider.api_type == "anthropic" {
        request = request
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        request = request.header("Authorization", format!("Bearer {}", provider.api_key));
    }

    match request.send().await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = response.status().as_u16();
            
            if status == 200 || status == 201 {
                TestResult {
                    provider_id: provider.id.clone(),
                    success: true,
                    message: format!("OK ({}ms)", elapsed),
                    response_time_ms: Some(elapsed),
                    http_status: Some(status),
                }
            } else if status == 401 || status == 403 {
                TestResult {
                    provider_id: provider.id.clone(),
                    success: false,
                    message: format!("Authentication failed (HTTP {})", status),
                    response_time_ms: Some(elapsed),
                    http_status: Some(status),
                }
            } else {
                TestResult {
                    provider_id: provider.id.clone(),
                    success: false,
                    message: format!("Unexpected status: HTTP {}", status),
                    response_time_ms: Some(elapsed),
                    http_status: Some(status),
                }
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            TestResult {
                provider_id: provider.id.clone(),
                success: false,
                message: format!("Connection failed: {}", e),
                response_time_ms: Some(elapsed),
                http_status: None,
            }
        }
    }
}

/// Test all providers
pub async fn test_all_providers(providers: &[TestProvider]) -> Vec<TestResult> {
    let mut results = Vec::new();
    
    for provider in providers {
        let result = test_provider(provider).await;
        results.push(result);
    }
    
    results
}

/// Save test results to database
pub fn save_test_results(db: &Database, results: &[TestResult], provider_names: &std::collections::HashMap<String, String>) -> anyhow::Result<()> {
    let now = chrono::Utc::now().timestamp();
    
    for result in results {
        let log = StreamCheckLog {
            provider_id: result.provider_id.clone(),
            provider_name: provider_names.get(&result.provider_id).cloned().unwrap_or_default(),
            app_type: "claude".to_string(),
            status: if result.success { "success" } else { "failed" }.to_string(),
            success: result.success,
            message: result.message.clone(),
            response_time_ms: result.response_time_ms.map(|t| t as i64),
            http_status: result.http_status.map(|s| s as i32),
            model_used: None,
            tested_at: now,
        };
        
        db.log_stream_check(&log)?;
        
        // Update health status
        db.update_health(
            &result.provider_id,
            "claude",
            result.success,
            if result.success { None } else { Some(&result.message) },
        )?;
    }
    
    Ok(())
}
