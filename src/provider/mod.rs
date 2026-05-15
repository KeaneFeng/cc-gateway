use crate::config::ProviderConfig;
use serde_json::Value;

/// Provider instance with runtime state
#[derive(Debug, Clone)]
pub struct Provider {
    pub config: ProviderConfig,
    pub client: reqwest::Client,
}

impl Provider {
    /// Create a new provider instance
    pub fn new(config: ProviderConfig) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            // Force HTTP/1.1 — many providers (Volcengine, etc.) don't handle HTTP/2 properly
            // cc-switch also uses HTTP/1.1 via hyper directly
            .http1_only()
            // Disable auto-decompression — we handle accept-encoding manually
            // Prevents reqwest from interfering with SSE streams and provider responses
            .no_gzip()
            .no_brotli()
            .no_deflate()
            // Timeouts: 30s connect, 600s total (matches cc-switch pattern)
            .connect_timeout(std::time::Duration::from_secs(30))
            .timeout(std::time::Duration::from_secs(600))
            .build()?;

        Ok(Self { config, client })
    }

    /// Get the model ID for /v1/models endpoint (claude-xxx format)
    pub fn model_id(&self) -> String {
        format!("claude-{}", self.config.id)
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        self.config
            .display_name
            .clone()
            .unwrap_or_else(|| self.config.name.clone())
    }

    /// Forward a request in Anthropic format (no conversion)
    pub async fn forward_anthropic_request(
        &self,
        endpoint: &str,
        body: Value,
        headers: Vec<(String, String)>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        let mut request = self.client.post(&url);

        // Set auth header (provider's API key)
        request = request.header("x-api-key", &self.config.api_key);

        // Forward ALL original headers, replacing only auth-related ones
        let mut has_anthropic_version = false;
        for (key, value) in &headers {
            let key_lower = key.to_lowercase();
            // Skip: host, auth headers (we replace with provider's key)
            if key_lower == "host" || key_lower == "authorization" || key_lower == "x-api-key" {
                continue;
            }
            // Skip content-length (reqwest recalculates from body)
            if key_lower == "content-length" {
                continue;
            }
            // Force identity encoding — prevents compressed responses that break parsing
            // (matches cc-switch pattern: no_gzip on client + identity on request)
            if key_lower == "accept-encoding" {
                request = request.header("accept-encoding", "identity");
                continue;
            }
            if key_lower == "anthropic-version" {
                has_anthropic_version = true;
            }
            request = request.header(key.as_str(), value.as_str());
        }

        // Set anthropic-version only if client didn't send it
        if !has_anthropic_version {
            request = request.header("anthropic-version", "2023-06-01");
        }

        request.json(&body).send().await
    }

    /// Forward a streaming request in Anthropic format
    pub async fn forward_anthropic_streaming(
        &self,
        endpoint: &str,
        mut body: Value,
        headers: Vec<(String, String)>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        let mut request = self.client.post(&url);

        // Set auth header (provider's API key)
        request = request.header("x-api-key", &self.config.api_key);

        // Forward ALL original headers, replacing only auth-related ones
        let mut has_anthropic_version = false;
        for (key, value) in &headers {
            let key_lower = key.to_lowercase();
            // Skip: host, auth headers (we replace with provider's key)
            if key_lower == "host" || key_lower == "authorization" || key_lower == "x-api-key" {
                continue;
            }
            // Skip content-length (reqwest recalculates from body)
            if key_lower == "content-length" {
                continue;
            }
            // Force identity encoding for streaming (critical for SSE!)
            if key_lower == "accept-encoding" {
                request = request.header("accept-encoding", "identity");
                continue;
            }
            if key_lower == "anthropic-version" {
                has_anthropic_version = true;
            }
            request = request.header(key.as_str(), value.as_str());
        }

        // Set anthropic-version only if client didn't send it
        if !has_anthropic_version {
            request = request.header("anthropic-version", "2023-06-01");
        }

        // Force streaming
        body["stream"] = serde_json::json!(true);

        request.json(&body).send().await
    }

    /// Forward a request in OpenAI format (with conversion)
    pub async fn forward_request(
        &self,
        endpoint: &str,
        body: Value,
        headers: Vec<(String, String)>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        let mut request = self.client.post(&url);

        // Set auth header based on API type
        if self.config.api_format == crate::config::ApiFormat::Anthropic {
            request = request
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            request = request.header("Authorization", format!("Bearer {}", self.config.api_key));
        }

        // Forward relevant headers
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            if key_lower != "host"
                && key_lower != "authorization"
                && key_lower != "x-api-key"
                && key_lower != "content-length"
            {
                // Force identity encoding
                if key_lower == "accept-encoding" {
                    request = request.header("accept-encoding", "identity");
                } else {
                    request = request.header(&key, &value);
                }
            }
        }

        request.json(&body).send().await
    }

    /// Forward a streaming request in OpenAI format
    pub async fn forward_streaming_request(
        &self,
        endpoint: &str,
        mut body: Value,
        headers: Vec<(String, String)>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        let mut request = self.client.post(&url);

        // Set auth header based on API type
        if self.config.api_format == crate::config::ApiFormat::Anthropic {
            request = request
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            request = request.header("Authorization", format!("Bearer {}", self.config.api_key));
        }

        // Forward relevant headers
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            if key_lower != "host"
                && key_lower != "authorization"
                && key_lower != "x-api-key"
                && key_lower != "content-length"
            {
                // Force identity encoding
                if key_lower == "accept-encoding" {
                    request = request.header("accept-encoding", "identity");
                } else {
                    request = request.header(&key, &value);
                }
            }
        }

        // Force streaming
        body["stream"] = serde_json::json!(true);

        request.json(&body).send().await
    }
}
