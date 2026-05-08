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
            .timeout(std::time::Duration::from_secs(300))
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

        // Set Anthropic auth header
        request = request
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01");

        // Forward relevant headers
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            if key_lower != "host"
                && key_lower != "authorization"
                && key_lower != "x-api-key"
                && key_lower != "content-length"
                && key_lower != "anthropic-version"
            {
                request = request.header(&key, &value);
            }
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

        // Set Anthropic auth header
        request = request
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01");

        // Forward relevant headers
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            if key_lower != "host"
                && key_lower != "authorization"
                && key_lower != "x-api-key"
                && key_lower != "content-length"
                && key_lower != "anthropic-version"
            {
                request = request.header(&key, &value);
            }
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
                request = request.header(&key, &value);
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
                request = request.header(&key, &value);
            }
        }

        // Force streaming
        body["stream"] = serde_json::json!(true);

        request.json(&body).send().await
    }
}
