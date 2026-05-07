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
use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::ProxyError;
use crate::provider::Provider;
use crate::proxy::{streaming, transform};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub providers: Vec<Provider>,
}

impl AppState {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let providers: Vec<Provider> = config
            .providers
            .iter()
            .map(|c| Provider::new(c.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            config: Arc::new(config),
            providers,
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

    // Check if streaming is requested
    let is_stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // Convert Anthropic request to OpenAI format
    let openai_body = transform::anthropic_to_openai(body.clone())
        .map_err(|e| ProxyError::TransformError(e))?;

    // Extract headers to forward
    let headers_vec: Vec<(String, String)> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
        })
        .collect();

    if is_stream {
        // Handle streaming response
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

        // Convert response stream
        let byte_stream = response.bytes_stream();
        let string_stream = byte_stream.map(|result| {
            result
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        });

        let model = provider.config.model.clone();
        let message_id = format!("msg_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let anthropic_stream = streaming::openai_to_anthropic_stream(string_stream, model, message_id);

        // Return SSE response
        let sse_stream = anthropic_stream.map(|result| {
            result.map(|data| axum::response::sse::Event::default().data(data))
        });

        Ok(Sse::new(sse_stream).into_response())
    } else {
        // Handle non-streaming response
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

        // Convert OpenAI response to Anthropic format
        let anthropic_response =
            transform::openai_to_anthropic_response(openai_response, &provider.config.model);

        Ok(Json(anthropic_response).into_response())
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
            })
        })
        .collect();

    Json(json!({
        "status": "running",
        "providers": providers,
        "total_providers": providers.len(),
    }))
}
