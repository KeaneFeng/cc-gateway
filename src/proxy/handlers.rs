//! Request handlers
//!
//! Handles HTTP requests for the proxy server

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::database::{Database, RequestLog};
use crate::error::ProxyError;
use crate::provider::Provider;
use crate::proxy::{streaming, transform};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub providers: Vec<Provider>,
    pub db: Arc<Mutex<Database>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let providers: Vec<Provider> = config
            .providers
            .iter()
            .map(|c| Provider::new(c.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let db = Database::open_cc_switch_compatible()
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

        Ok(Self {
            config: Arc::new(config),
            providers,
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Get provider by model ID
    pub fn get_provider_by_model(&self, model_id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| {
            let full_id = format!("claude-{}", p.config.id);
            full_id == model_id || p.config.model == model_id
        })
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> Option<&Provider> {
        self.providers
            .iter()
            .find(|p| p.config.is_default)
            .or_else(|| self.providers.first())
    }
}

/// Check if provider uses Anthropic format (direct) or OpenAI format (needs conversion)
fn is_anthropic_format(provider: &Provider) -> bool {
    let base_url = provider.config.base_url.to_lowercase();
    // If the URL contains "anthropic" or ends with common Anthropic-compatible paths
    base_url.contains("anthropic") || 
    base_url.contains("/api/coding") ||  // Volcengine coding endpoint
    base_url.contains("/apps/anthropic") ||  // Bailian
    base_url.contains("/api/anthropic")  // Direct Anthropic API
}

/// GET /v1/models - List available models
pub async fn list_models(State(state): State<AppState>) -> Json<Value> {
    let models: Vec<Value> = state
        .providers
        .iter()
        .map(|p| {
            json!({
                "id": format!("claude-{}", p.config.id),
                "object": "model",
                "created": chrono::Utc::now().timestamp(),
                "owned_by": "cc-switch-pro",
                "display_name": p.display_name(),
            })
        })
        .collect();

    Json(json!({
        "object": "list",
        "data": models
    }))
}

/// POST /v1/messages - Handle Claude API requests
pub async fn handle_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ProxyError> {
    // Extract model from request
    let model_id = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    // Find provider by model ID
    let provider = if model_id.is_empty() {
        state
            .get_default_provider()
            .ok_or(ProxyError::NoProvidersConfigured)?
    } else {
        state
            .get_provider_by_model(model_id)
            .or_else(|| state.get_default_provider())
            .ok_or(ProxyError::ProviderNotFound(model_id.to_string()))?
    };

    // Log the routing decision
    info!(
        "🔵 Request: model_id={} → provider={} (name={}, url={}, format={})",
        model_id,
        provider.config.id,
        provider.config.name,
        provider.config.base_url,
        if is_anthropic_format(provider) { "anthropic" } else { "openai" }
    );

    // Replace model in body with provider's actual model name
    let mut body = body.clone();
    body["model"] = serde_json::json!(provider.config.model);
    info!("📝 Model replaced: {} → {}", model_id, provider.config.model);

    // Check if streaming is requested
    let is_stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // Check if provider uses Anthropic format directly
    let use_anthropic_format = is_anthropic_format(provider);

    // Extract headers to forward
    let headers_vec: Vec<(String, String)> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
        })
        .collect();

    if use_anthropic_format {
        // Forward directly as Anthropic format (no conversion needed)
        let endpoint = "v1/messages";
        
        if is_stream {
            let response = provider
                .forward_anthropic_streaming(endpoint, body, headers_vec)
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ProxyError::UpstreamError(format!(
                    "Upstream returned {}: {}",
                    status, body
                )));
            }

            // For Anthropic format, directly forward the raw SSE stream
            // The upstream already sends proper SSE format
            let stream = response.bytes_stream();
            let body_stream = stream.map(|result| {
                result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            });
            
            Ok(axum::response::Response::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("connection", "keep-alive")
                .body(axum::body::Body::from_stream(body_stream))
                .unwrap())
        } else {
            let response = provider
                .forward_anthropic_request(endpoint, body, headers_vec)
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ProxyError::UpstreamError(format!(
                    "Upstream returned {}: {}",
                    status, body
                )));
            }

            let anthropic_response: Value = response
                .json()
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            // Log usage for Anthropic format
            let usage = anthropic_response.get("usage");
            let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let cache_read = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let cache_creation = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            
            let request_id = format!("req_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
            let db = state.db.lock().unwrap();
            match db.log_request(&RequestLog {
                request_id,
                provider_id: provider.config.id.clone(),
                app_type: "claude".to_string(),
                model: provider.config.model.clone(),
                input_tokens,
                output_tokens,
                cache_read_tokens: cache_read,
                cache_creation_tokens: cache_creation,
                total_cost_usd: 0.0,
                latency_ms: 0,
                first_token_ms: None,
                status_code: 200,
                error_message: None,
                session_id: None,
                is_streaming: false,
                created_at: chrono::Utc::now().timestamp(),
            }) {
                Ok(_) => info!("📊 Usage logged: provider={} tokens={}/{}", provider.config.name, input_tokens, output_tokens),
                Err(e) => warn!("⚠️ Failed to log usage: {}", e),
            }

            Ok(Json(anthropic_response).into_response())
        }
    } else {
        // Convert Anthropic request to OpenAI format
        let openai_body = transform::anthropic_to_openai(body.clone())
            .map_err(|e| ProxyError::TransformError(e))?;

        if is_stream {
            let response = provider
                .forward_streaming_request("chat/completions", openai_body, headers_vec)
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ProxyError::UpstreamError(format!(
                    "Upstream returned {}: {}",
                    status, body
                )));
            }

            let byte_stream = response.bytes_stream();
            let string_stream = byte_stream.map(|result| {
                result
                    .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            });

            let model = provider.config.model.clone();
            let message_id = format!("msg_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
            let anthropic_stream = streaming::openai_to_anthropic_stream(string_stream, model, message_id);

            let sse_stream = anthropic_stream.map(|result| {
                result.map(|data| axum::response::sse::Event::default().data(data))
            });

            Ok(Sse::new(sse_stream).into_response())
        } else {
            let response = provider
                .forward_request("chat/completions", openai_body, headers_vec)
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ProxyError::UpstreamError(format!(
                    "Upstream returned {}: {}",
                    status, body
                )));
            }

            let openai_response: Value = response
                .json()
                .await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            let anthropic_response =
                transform::openai_to_anthropic_response(openai_response, &provider.config.model);

            // Log usage
            let usage = anthropic_response.get("usage");
            let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let cache_read = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            let cache_creation = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
            
            let request_id = format!("req_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
            let db = state.db.lock().unwrap();
            match db.log_request(&RequestLog {
                request_id,
                provider_id: provider.config.id.clone(),
                app_type: "claude".to_string(),
                model: provider.config.model.clone(),
                input_tokens,
                output_tokens,
                cache_read_tokens: cache_read,
                cache_creation_tokens: cache_creation,
                total_cost_usd: 0.0,
                latency_ms: 0,
                first_token_ms: None,
                status_code: 200,
                error_message: None,
                session_id: None,
                is_streaming: false,
                created_at: chrono::Utc::now().timestamp(),
            }) {
                Ok(_) => info!("📊 Usage logged: provider={} tokens={}/{}", provider.config.name, input_tokens, output_tokens),
                Err(e) => warn!("⚠️ Failed to log usage: {}", e),
            }
            
            info!("✅ Response: provider={} completed, tokens={}/{}", provider.config.name, input_tokens, output_tokens);
            Ok(Json(anthropic_response).into_response())
        }
    }
}

/// GET /health - Health check
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// GET /status - Get proxy status
pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let providers: Vec<Value> = state
        .providers
        .iter()
        .map(|p| {
            json!({
                "id": p.config.id,
                "name": p.config.name,
                "model_id": format!("claude-{}", p.config.id),
                "model": p.config.model,
                "base_url": p.config.base_url,
                "is_default": p.config.is_default,
                "format": if is_anthropic_format(p) { "anthropic" } else { "openai" },
            })
        })
        .collect();

    Json(json!({
        "status": "running",
        "providers": providers,
        "total_providers": providers.len(),
    }))
}
