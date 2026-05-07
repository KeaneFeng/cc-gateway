use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Unique provider ID
    pub id: String,
    /// Display name
    pub name: String,
    /// API type (openai, anthropic)
    #[serde(default = "default_api_type")]
    pub api_type: String,
    /// Base URL for the provider API
    pub base_url: String,
    /// API key
    pub api_key: String,
    /// Model ID to use when forwarding to this provider
    pub model: String,
    /// Display name for the model (shown in /model picker)
    #[serde(default)]
    pub display_name: Option<String>,
    /// Whether this is the default provider
    #[serde(default)]
    pub is_default: bool,
}

fn default_api_type() -> String {
    "openai".to_string()
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Server host
    #[serde(default = "default_host")]
    pub host: String,
    /// Providers configuration
    pub providers: Vec<ProviderConfig>,
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_port() -> u16 {
    15780
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            providers: Vec::new(),
            log_level: default_log_level(),
        }
    }
}

impl AppConfig {
    /// Load config from file
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to file
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default config path
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cc-switch-pro")
            .join("config.toml")
    }

    /// Get provider by model ID (claude-xxx format)
    pub fn get_provider_by_model(&self, model_id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| {
            let full_id = format!("claude-{}", p.id);
            full_id == model_id || p.model == model_id
        })
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.is_default).or_else(|| self.providers.first())
    }

    /// Get all model IDs for the /v1/models endpoint
    pub fn get_model_ids(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| format!("claude-{}", p.id))
            .collect()
    }
}

/// Generate example config
pub fn generate_example_config() -> AppConfig {
    AppConfig {
        port: 15780,
        host: "127.0.0.1".to_string(),
        log_level: "info".to_string(),
        providers: vec![
            ProviderConfig {
                id: "mimo".to_string(),
                name: "Xiaomi MiMo".to_string(),
                api_type: "openai".to_string(),
                base_url: "https://api.mimo.xiaomi.com/v1".to_string(),
                api_key: "sk-xxx".to_string(),
                model: "mimo-2.5-pro".to_string(),
                display_name: Some("Mimo 2.5 Pro".to_string()),
                is_default: true,
            },
            ProviderConfig {
                id: "kimi".to_string(),
                name: "Moonshot Kimi".to_string(),
                api_type: "openai".to_string(),
                base_url: "https://api.moonshot.cn/v1".to_string(),
                api_key: "sk-xxx".to_string(),
                model: "kimi-2.5".to_string(),
                display_name: Some("Kimi 2.5".to_string()),
                is_default: false,
            },
            ProviderConfig {
                id: "glm".to_string(),
                name: "Zhipu GLM".to_string(),
                api_type: "openai".to_string(),
                base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                api_key: "xxx".to_string(),
                model: "glm-5.1".to_string(),
                display_name: Some("GLM 5.1".to_string()),
                is_default: false,
            },
            ProviderConfig {
                id: "qwen".to_string(),
                name: "Alibaba Qwen".to_string(),
                api_type: "openai".to_string(),
                base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                api_key: "sk-xxx".to_string(),
                model: "qwen2.5-plus".to_string(),
                display_name: Some("Qwen 2.5 Plus".to_string()),
                is_default: false,
            },
        ],
    }
}
