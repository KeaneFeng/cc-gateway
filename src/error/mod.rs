use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("No providers configured")]
    NoProvidersConfigured,

    #[error("Request transformation failed: {0}")]
    TransformError(String),

    #[error("Upstream request failed: {0}")]
    UpstreamError(String),

    #[error("Stream processing failed: {0}")]
    #[allow(dead_code)]
    StreamError(String),

    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    ConfigError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            ProxyError::ProviderNotFound(_) => (
                StatusCode::NOT_FOUND,
                "provider_not_found",
                self.to_string(),
            ),
            ProxyError::NoProvidersConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "no_providers",
                self.to_string(),
            ),
            ProxyError::TransformError(_) => {
                (StatusCode::BAD_REQUEST, "transform_error", self.to_string())
            }
            ProxyError::UpstreamError(_) => {
                (StatusCode::BAD_GATEWAY, "upstream_error", self.to_string())
            }
            ProxyError::StreamError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_error",
                self.to_string(),
            ),
            ProxyError::ConfigError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "config_error",
                self.to_string(),
            ),
            ProxyError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request", self.to_string()),
            ProxyError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                self.to_string(),
            ),
        };

        let body = json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
