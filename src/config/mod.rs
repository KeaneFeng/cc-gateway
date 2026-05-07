pub mod presets;

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
    /// Preset ID (if created from a preset)
    #[serde(default)]
    pub preset_id: Option<String>,
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
    16789
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

    /// Get provider by ID
    pub fn get_provider_by_id(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
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

    /// Add a provider
    pub fn add_provider(&mut self, provider: ProviderConfig) -> anyhow::Result<()> {
        if self.providers.iter().any(|p| p.id == provider.id) {
            anyhow::bail!("Provider with ID '{}' already exists", provider.id);
        }

        // If setting as default, unset others
        if provider.is_default {
            for p in &mut self.providers {
                p.is_default = false;
            }
        }

        // If this is the first provider, make it default
        let provider = if self.providers.is_empty() {
            ProviderConfig { is_default: true, ..provider }
        } else {
            provider
        };

        self.providers.push(provider);
        Ok(())
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, id: &str) -> anyhow::Result<()> {
        let len_before = self.providers.len();
        self.providers.retain(|p| p.id != id);

        if self.providers.len() == len_before {
            anyhow::bail!("Provider with ID '{}' not found", id);
        }

        // If we removed the default, set the first one as default
        if !self.providers.iter().any(|p| p.is_default) {
            if let Some(first) = self.providers.first_mut() {
                first.is_default = true;
            }
        }

        Ok(())
    }

    /// Update a provider
    pub fn update_provider(&mut self, id: &str, updates: ProviderUpdate) -> anyhow::Result<()> {
        // First, find the provider and check if we need to unset other defaults
        let needs_unset_defaults = updates.is_default == Some(true);
        
        if needs_unset_defaults {
            for p in &mut self.providers {
                p.is_default = false;
            }
        }

        // Now update the provider
        let provider = self.providers.iter_mut().find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider with ID '{}' not found", id))?;

        if let Some(name) = updates.name {
            provider.name = name;
        }
        if let Some(base_url) = updates.base_url {
            provider.base_url = base_url;
        }
        if let Some(api_key) = updates.api_key {
            provider.api_key = api_key;
        }
        if let Some(model) = updates.model {
            provider.model = model;
        }
        if let Some(display_name) = updates.display_name {
            provider.display_name = Some(display_name);
        }
        if let Some(is_default) = updates.is_default {
            provider.is_default = is_default;
        }

        // If no default exists, set the first one
        if !self.providers.iter().any(|p| p.is_default) {
            if let Some(first) = self.providers.first_mut() {
                first.is_default = true;
            }
        }

        Ok(())
    }

    /// Copy a provider
    pub fn copy_provider(&mut self, source_id: &str, new_id: &str) -> anyhow::Result<()> {
        let source = self.providers.iter().find(|p| p.id == source_id)
            .ok_or_else(|| anyhow::anyhow!("Provider with ID '{}' not found", source_id))?
            .clone();

        if self.providers.iter().any(|p| p.id == new_id) {
            anyhow::bail!("Provider with ID '{}' already exists", new_id);
        }

        let new_provider = ProviderConfig {
            id: new_id.to_string(),
            name: format!("{} (Copy)", source.name),
            is_default: false,
            ..source
        };

        self.providers.push(new_provider);
        Ok(())
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, id: &str) -> anyhow::Result<()> {
        let mut found = false;
        for p in &mut self.providers {
            if p.id == id {
                p.is_default = true;
                found = true;
            } else {
                p.is_default = false;
            }
        }

        if !found {
            anyhow::bail!("Provider with ID '{}' not found", id);
        }

        Ok(())
    }
}

/// Provider update fields
#[derive(Debug, Clone, Default)]
pub struct ProviderUpdate {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub display_name: Option<String>,
    pub is_default: Option<bool>,
}

/// Import from cc-switch database (legacy, use database module instead)
pub fn import_from_cc_switch() -> anyhow::Result<AppConfig> {
    let cc_switch_db = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch")
        .join("cc-switch.db");

    if !cc_switch_db.exists() {
        anyhow::bail!("cc-switch database not found at: {}", cc_switch_db.display());
    }

    // Open SQLite database
    let conn = rusqlite::Connection::open(&cc_switch_db)?;

    // Query providers - cc-switch uses app_type = 'claude' for Claude providers
    let mut stmt = conn.prepare(
        "SELECT id, name, settings_config FROM providers WHERE app_type = 'claude'"
    )?;

    let providers: Vec<ProviderConfig> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let settings_config_str: String = row.get(2)?;

            // Parse settings_config JSON
            let settings_config: serde_json::Value = serde_json::from_str(&settings_config_str)
                .unwrap_or(serde_json::json!({}));

            let default_env = serde_json::json!({});
            let env = settings_config.get("env").unwrap_or(&default_env);

            let base_url = env.get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let api_key = env.get("ANTHROPIC_AUTH_TOKEN")
                .or(env.get("ANTHROPIC_API_KEY"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let model = env.get("ANTHROPIC_MODEL")
                .and_then(|v| v.as_str())
                .unwrap_or("claude-sonnet-4")
                .to_string();

            // Skip providers without base_url
            if base_url.is_empty() {
                return Ok(None);
            }

            Ok(Some(ProviderConfig {
                id: id.clone(),
                name,
                api_type: "openai".to_string(),
                base_url,
                api_key,
                model,
                display_name: None,
                is_default: false,
                preset_id: None,
            }))
        })?
        .filter_map(|r| r.transpose())
        .collect::<Result<Vec<_>, _>>()?;

    if providers.is_empty() {
        anyhow::bail!("No valid providers found in cc-switch database");
    }

    let mut config = AppConfig::default();
    config.providers = providers;

    // Set the first provider as default
    if let Some(first) = config.providers.first_mut() {
        first.is_default = true;
    }

    Ok(config)
}

/// Generate example config
pub fn generate_example_config() -> AppConfig {
    AppConfig {
        port: 16789,
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
                preset_id: None,
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
                preset_id: None,
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
                preset_id: None,
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
                preset_id: None,
            },
        ],
    }
}
