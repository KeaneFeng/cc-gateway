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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::database::{Database, RequestLog};
use crate::error::ProxyError;
use crate::provider::Provider;
use crate::proxy::{streaming, transform};

/// Session → project path mapping (cached, not scanned per-request)
#[derive(Clone)]
pub struct SessionRouter {
    /// session_id → project_path (e.g. "abc-123" → "/Users/keane/www/apd")
    session_map: Arc<Mutex<HashMap<String, String>>>,
    /// project_path → provider_id (from config)
    project_providers: HashMap<String, String>,
}

impl SessionRouter {
    /// Build from config: scan all JSONL files under ~/.claude/projects/
    pub fn new(config: &AppConfig) -> Self {
        let session_map = Arc::new(Mutex::new(HashMap::new()));
        let project_providers = config.project_providers.clone();

        let router = Self {
            session_map,
            project_providers,
        };
        router.scan_all_jsonl();
        router
    }

    /// Scan all JSONL files to build session_id → project_path mapping
    fn scan_all_jsonl(&self) {
        let home = dirs::home_dir().unwrap_or_default();
        let projects_dir = home.join(".claude").join("projects");
        if !projects_dir.exists() {
            return;
        }

        let mut map = self.session_map.lock().unwrap();
        let mut count = 0;

        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                // Each directory contains .jsonl files
                if let Ok(files) = std::fs::read_dir(&path) {
                    for file in files.flatten() {
                        let fpath = file.path();
                        if fpath.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(&fpath) {
                                for line in content.lines() {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                        if let (Some(sid), Some(cwd)) = (
                                            json.get("sessionId").and_then(|v| v.as_str()),
                                            json.get("cwd").and_then(|v| v.as_str()),
                                        ) {
                                            map.entry(sid.to_string())
                                                .or_insert_with(|| {
                                                    count += 1;
                                                    cwd.to_string()
                                                });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if count > 0 {
            tracing::info!("🗺️ SessionRouter: loaded {} session→project mappings", count);
        }
    }

    /// Resolve session_id → provider_id
    /// Returns None if session has no project-level override
    pub fn resolve_provider(&self, session_id: &str) -> Option<String> {
        // First check cache
        let project_path = {
            let map = self.session_map.lock().unwrap();
            map.get(session_id).cloned()
        };

        let project_path = match project_path {
            Some(p) => p,
            None => {
                // Unknown session: try incremental scan
                self.scan_session_incremental(session_id);
                let map = self.session_map.lock().unwrap();
                map.get(session_id).cloned()?
            }
        };

        self.project_providers.get(&project_path).cloned()
    }

    /// Incremental scan: find the JSONL file containing this session_id
    fn scan_session_incremental(&self, session_id: &str) {
        let home = dirs::home_dir().unwrap_or_default();
        let projects_dir = home.join(".claude").join("projects");
        if !projects_dir.exists() {
            return;
        }

        let mut map = self.session_map.lock().unwrap();
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                if let Ok(files) = std::fs::read_dir(&path) {
                    for file in files.flatten() {
                        let fpath = file.path();
                        if fpath.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(&fpath) {
                                for line in content.lines() {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                        if let (Some(sid), Some(cwd)) = (
                                            json.get("sessionId").and_then(|v| v.as_str()),
                                            json.get("cwd").and_then(|v| v.as_str()),
                                        ) {
                                            if sid == session_id {
                                                tracing::info!("🗺️ SessionRouter: discovered new session {} → {}", sid, cwd);
                                                map.insert(sid.to_string(), cwd.to_string());
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Shared application state
/// Shared mutable state (wrapped in Arc<Mutex> for cross-clone updates)
#[derive(Clone)]
struct SharedState {
    pub config: AppConfig,
    pub providers: Vec<Provider>,
    pub session_router: SessionRouter,
    pub current_provider_id: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub shared: Arc<Mutex<SharedState>>,
    pub db: Arc<Mutex<Database>>,
    pub config_path: String,
}

impl AppState {
    pub fn new(config: AppConfig, config_path: String) -> anyhow::Result<Self> {
        let providers: Vec<Provider> = config
            .providers
            .iter()
            .map(|c| Provider::new(c.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let db = Database::open_cc_switch_compatible()
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

        // Set initial current provider to the default
        let default_id = config
            .providers
            .iter()
            .find(|p| p.is_default)
            .or(config.providers.first())
            .map(|p| p.id.clone());

        // Build session router from JSONL files
        let session_router = SessionRouter::new(&config);

        let shared = SharedState {
            config,
            providers,
            session_router,
            current_provider_id: default_id,
        };

        Ok(Self {
            shared: Arc::new(Mutex::new(shared)),
            db: Arc::new(Mutex::new(db)),
            config_path,
        })
    }

    /// Reload config from file and rebuild all shared state
    pub fn reload_config(&self) -> anyhow::Result<()> {
        let config = AppConfig::load(std::path::Path::new(&self.config_path))?;
        
        let providers: Vec<Provider> = config
            .providers
            .iter()
            .map(|c| Provider::new(c.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let default_id = config
            .providers
            .iter()
            .find(|p| p.is_default)
            .or(config.providers.first())
            .map(|p| p.id.clone());

        let session_router = SessionRouter::new(&config);

        let mut shared = self.shared.lock().unwrap();
        shared.config = config;
        shared.providers = providers;
        shared.session_router = session_router;
        if let Some(id) = default_id {
            shared.current_provider_id = Some(id);
        }

        tracing::info!("🔄 Config reloaded from {}", self.config_path);
        Ok(())
    }

    /// Get the current active provider (runtime-switchable)
    pub fn get_current_provider(&self) -> Option<Provider> {
        let shared = self.shared.lock().unwrap();
        if let Some(ref id) = shared.current_provider_id {
            shared.providers.iter().find(|p| &p.config.id == id).cloned()
        } else {
            shared.providers.first().cloned()
        }
    }

    /// Switch current provider at runtime
    pub fn switch_provider(&self, provider_id: &str) -> bool {
        let shared = self.shared.lock().unwrap();
        if shared.providers.iter().any(|p| p.config.id == provider_id) {
            drop(shared);
            let mut shared = self.shared.lock().unwrap();
            shared.current_provider_id = Some(provider_id.to_string());
            true
        } else {
            false
        }
    }
}

/// Check if provider uses Anthropic format based on configured api_format
fn is_anthropic_format(provider: &Provider) -> bool {
    provider.config.api_format == crate::config::ApiFormat::Anthropic
}

/// GET /v1/models - List available models (Anthropic format for Claude Code gateway discovery)
pub async fn list_models(State(state): State<AppState>) -> Json<Value> {
    let shared = state.shared.lock().unwrap();
    let models: Vec<Value> = shared
        .providers
        .iter()
        .map(|p| {
            json!({
                "type": "model",
                "id": format!("claude-{}", p.config.id),
                "display_name": p.display_name(),
                "created_at": chrono::Utc::now().to_rfc3339(),
            })
        })
        .collect();

    let first_id = models.first().and_then(|m| m["id"].as_str()).unwrap_or("").to_string();
    let last_id = models.last().and_then(|m| m["id"].as_str()).unwrap_or("").to_string();

    Json(json!({
        "data": models,
        "has_more": false,
        "first_id": first_id,
        "last_id": last_id,
    }))
}

/// POST /v1/messages - Handle Claude API requests
pub async fn handle_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ProxyError> {
    // DEBUG: Log full request to identify session info
    info!("📥 REQUEST HEADERS: {:?}", headers);
    info!("📥 REQUEST BODY: {}", serde_json::to_string(&body).unwrap_or_default());

    // Extract model from request
    let model_id = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    // Session-based project routing: extract session_id from header
    let session_id = headers
        .get("x-claude-code-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Find provider: session project routing (highest priority) → current provider → model match
    let mut provider = {
        let shared = state.shared.lock().unwrap();
        
        // 1. Session-based project routing (highest priority)
        let session_provider = if let Some(ref sid) = session_id {
            shared.session_router.resolve_provider(sid).and_then(|provider_id| {
                info!("🗺️ Session {} → project provider: {}", sid, provider_id);
                shared.providers.iter().find(|p| p.config.id == provider_id).cloned()
            })
        } else {
            None
        };

        if let Some(p) = session_provider {
            p
        } else {
            // 2. Current provider (default fallback)
            let current = shared.current_provider_id.as_ref()
                .and_then(|id| shared.providers.iter().find(|p| &p.config.id == id).cloned())
                .or_else(|| shared.providers.first().cloned());

            if !model_id.is_empty() {
                // 3. Model match: only for explicit provider selection (claude-{id} format)
                let model_matched = shared.providers.iter()
                    .find(|p| {
                        let full_id = format!("claude-{}", p.config.id);
                        full_id == model_id
                    })
                    .cloned();

                model_matched
                    .or(current)
                    .ok_or(ProxyError::ProviderNotFound(model_id.to_string()))?
            } else {
                current.ok_or(ProxyError::NoProvidersConfigured)?
            }
        }
    }; // shared lock released here

    // Determine actual model to use (may change for image requests)
    let has_image = check_has_image(&body);
    let actual_model = if has_image {
        if let Some(ref vision_model) = provider.config.vision_model {
            info!("🖼️ Image detected, using vision_model: {}", vision_model);
            vision_model.clone()
        } else {
            provider.config.model.clone()
        }
    } else {
        provider.config.model.clone()
    };

    // Log the routing decision
    info!(
        "🔵 Request: model_id={} → provider={} (name={}, url={}, format={})",
        model_id,
        provider.config.id,
        provider.config.name,
        provider.config.base_url,
        if is_anthropic_format(&provider) { "anthropic" } else { "openai" }
    );

    // Replace model in body with the actual model
    let mut body = body.clone();
    body["model"] = serde_json::json!(actual_model);
    info!("📝 Model replaced: {} → {}", model_id, actual_model);

    // Normalize output_config.effort
    // Priority: provider config effort_level > fallback normalization (xhigh → max)
    if let Some(output_config) = body.get_mut("output_config") {
        if let Some(effort) = output_config.get("effort").and_then(|e| e.as_str()) {
            let normalized = if let Some(ref configured) = provider.config.effort_level {
                // Provider has explicit effort_level configured — use it
                info!("📝 Effort overridden by provider config: {} → {}", effort, configured);
                configured.clone()
            } else {
                // No config override — apply fallback normalization for unsupported values
                match effort {
                    "xhigh" => {
                        info!("📝 Normalized effort: {} → max (provider has no effort_level configured)", effort);
                        "max".to_string()
                    }
                    other => other.to_string(),
                }
            };
            output_config["effort"] = serde_json::json!(normalized);
        }
    }

    // Check if streaming is requested
    let is_stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // Check if provider uses Anthropic format directly
    let use_anthropic_format = is_anthropic_format(&provider);

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
                    .map(|bytes| String::from_utf8_lossy(bytes.as_ref()).to_string())
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
    let shared = state.shared.lock().unwrap();
    let providers: Vec<Value> = shared
        .providers
        .iter()
        .map(|p| {
            let mut info = json!({
                "id": p.config.id,
                "name": p.config.name,
                "model_id": format!("claude-{}", p.config.id),
                "model": p.config.model,
                "base_url": p.config.base_url,
                "is_default": p.config.is_default,
                "format": if is_anthropic_format(p) { "anthropic" } else { "openai" },
            });
            if let Some(ref vm) = p.config.vision_model {
                info["vision_model"] = json!(vm);
            }
            info
        })
        .collect();

    Json(json!({
        "status": "running",
        "providers": providers,
        "total_providers": providers.len(),
    }))
}

/// Check if request body contains image content (recursive for nested tool_result)
fn check_has_image(body: &Value) -> bool {
    // Check messages array for image content
    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        for message in messages {
            if has_image_in_content(message.get("content")) {
                return true;
            }
        }
    }
    false
}

/// Recursively check if a content value contains image (handles nested tool_result)
fn has_image_in_content(content: Option<&Value>) -> bool {
    match content {
        Some(Value::Array(arr)) => {
            for item in arr {
                if let Some(content_type) = item.get("type").and_then(|t| t.as_str()) {
                    if content_type == "image" || content_type == "image_url" {
                        return true;
                    }
                }
                // Check nested content (e.g., tool_result with image content)
                if has_image_in_content(item.get("content")) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// GET /provider/{provider_id}/v1/models - List models for a specific provider
pub async fn list_models_for_provider(
    State(state): State<AppState>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
) -> Json<Value> {
    let shared = state.shared.lock().unwrap();
    let provider = shared.providers.iter().find(|p| p.config.id == provider_id);
    
    match provider {
        Some(p) => {
            let model = json!({
                "type": "model",
                "id": format!("claude-{}", p.config.id),
                "display_name": p.display_name(),
                "created_at": chrono::Utc::now().to_rfc3339(),
            });
            Json(json!({
                "data": [model],
                "has_more": false,
                "first_id": format!("claude-{}", p.config.id),
                "last_id": format!("claude-{}", p.config.id),
            }))
        }
        None => {
            Json(json!({
                "data": [],
                "has_more": false,
                "first_id": "",
                "last_id": "",
            }))
        }
    }
}

/// POST /provider/{provider_id}/v1/messages - Handle requests for a specific provider
pub async fn handle_messages_for_provider(
    State(state): State<AppState>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ProxyError> {
    // Find provider by ID, clone it, release lock
    let provider = {
        let shared = state.shared.lock().unwrap();
        shared.providers.iter()
            .find(|p| p.config.id == provider_id)
            .cloned()
            .ok_or(ProxyError::ProviderNotFound(provider_id.clone()))?
    };

    // Check if request contains image content, if so use vision_model
    let has_image = check_has_image(&body);
    let actual_model = if has_image {
        if let Some(ref vision_model) = provider.config.vision_model {
            info!("🖼️ Image detected, using vision_model: {}", vision_model);
            vision_model.clone()
        } else {
            provider.config.model.clone()
        }
    } else {
        provider.config.model.clone()
    };

    // Log the routing decision
    info!(
        "🔵 Request: provider_id={} → provider={} (name={}, model={}, format={})",
        provider_id,
        provider.config.id,
        provider.config.name,
        actual_model,
        if is_anthropic_format(&provider) { "anthropic" } else { "openai" }
    );

    // Replace model in body with the actual model
    let model_id = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    let mut body = body.clone();
    body["model"] = serde_json::json!(actual_model);
    info!("📝 Model replaced: {} → {}", model_id, actual_model);

    // Check if streaming is requested
    let is_stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // Check if provider uses Anthropic format directly
    let use_anthropic_format = is_anthropic_format(&provider);

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

            let response_body: Value = response.json().await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            // Log usage
            log_usage(&state, &provider, &response_body);

            Ok(Json(response_body).into_response())
        }
    } else {
        // Convert Anthropic format to OpenAI and forward
        let openai_body = transform::anthropic_to_openai(body.clone())
            .map_err(|e| ProxyError::UpstreamError(e))?;
        
        if is_stream {
            let response = provider
                .forward_streaming_request("v1/chat/completions", openai_body, headers_vec)
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
                    .map(|bytes| String::from_utf8_lossy(bytes.as_ref()).to_string())
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
                .forward_request("v1/chat/completions", openai_body, headers_vec)
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

            let openai_response: Value = response.json().await
                .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

            // Convert OpenAI response to Anthropic format
            let anthropic_response = transform::openai_to_anthropic_response(openai_response, &provider.config.model);

            // Log usage
            log_usage(&state, &provider, &anthropic_response);

            Ok(Json(anthropic_response).into_response())
        }
    }
}

/// Log usage to database
fn log_usage(state: &AppState, provider: &Provider, response: &Value) {
    let usage = response.get("usage");
    let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
    let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
    let cache_read = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);
    let cache_creation = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64()).unwrap_or(0);

    let request_id = format!("req_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let db = state.db.lock().unwrap();
    match db.log_request(&crate::database::RequestLog {
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
}

/// POST /api/switch-provider - Switch the current provider at runtime
pub async fn switch_provider(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ProxyError> {
    let provider_id = body
        .get("provider_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProxyError::BadRequest("Missing provider_id".to_string()))?;

    if state.switch_provider(provider_id) {
        let shared = state.shared.lock().unwrap();
        let provider_name = shared.providers.iter()
            .find(|p| p.config.id == provider_id)
            .map(|p| p.config.display_name.as_deref().unwrap_or(&p.config.name).to_string())
            .unwrap_or_default();

        info!("🎯 Provider switched to: {} ({})", provider_name, provider_id);

        Ok(Json(json!({
            "success": true,
            "provider_id": provider_id,
            "provider_name": provider_name
        })))
    } else {
        Err(ProxyError::ProviderNotFound(provider_id.to_string()))
    }
}

/// GET /api/current-provider - Get the current active provider
pub async fn get_current_provider(
    State(state): State<AppState>,
) -> Json<Value> {
    let shared = state.shared.lock().unwrap();
    let provider = shared.current_provider_id.as_ref()
        .and_then(|id| shared.providers.iter().find(|p| &p.config.id == id));
    let provider_id = shared.current_provider_id.clone();

    Json(json!({
        "provider_id": provider_id,
        "provider_name": provider.map(|p| p.config.display_name.as_deref().unwrap_or(&p.config.name)),
        "model": provider.map(|p| &p.config.model),
    }))
}

/// POST /api/reload - Reload config from file
pub async fn reload_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, ProxyError> {
    state.reload_config()
        .map_err(|e| ProxyError::UpstreamError(format!("Failed to reload config: {}", e)))?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Config reloaded successfully"
    })))
}
