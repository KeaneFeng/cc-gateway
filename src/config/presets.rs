//! Provider presets
//!
//! Pre-configured provider settings for common AI providers

use serde::{Deserialize, Serialize};
use crate::config::ApiFormat;

/// Provider preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub display_name: &'static str,
    pub api_format: ApiFormat,
    pub website_url: &'static str,
    pub api_key_url: Option<&'static str>,
}

/// Get all available presets
pub fn get_all_presets() -> Vec<ProviderPreset> {
    vec![
        // Official
        ProviderPreset {
            id: "claude-official",
            name: "Claude Official",
            category: "official",
            base_url: "https://api.anthropic.com",
            model: "claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::Anthropic,
            website_url: "https://www.anthropic.com/claude-code",
            api_key_url: None,
        },
        // Chinese Official Providers
        ProviderPreset {
            id: "deepseek",
            name: "DeepSeek",
            category: "cn_official",
            base_url: "https://api.deepseek.com/anthropic",
            model: "deepseek-v4-pro",
            display_name: "DeepSeek V4 Pro",
            api_format: ApiFormat::Anthropic,
            website_url: "https://platform.deepseek.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "zhipu",
            name: "Zhipu GLM",
            category: "cn_official",
            base_url: "https://open.bigmodel.cn/api/anthropic",
            model: "glm-5",
            display_name: "GLM 5",
            api_format: ApiFormat::Anthropic,
            website_url: "https://www.bigmodel.cn/claude-code",
            api_key_url: Some("https://www.bigmodel.cn/claude-code"),
        },
        ProviderPreset {
            id: "kimi",
            name: "Kimi",
            category: "cn_official",
            base_url: "https://api.moonshot.cn/v1",
            model: "kimi-k2.6",
            display_name: "Kimi K2.6",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://platform.moonshot.cn",
            api_key_url: None,
        },
        ProviderPreset {
            id: "kimi-coding",
            name: "Kimi For Coding",
            category: "cn_official",
            base_url: "https://api.moonshot.cn/v1",
            model: "kimi-k2.6-coding",
            display_name: "Kimi K2.6 (Coding)",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://platform.moonshot.cn",
            api_key_url: None,
        },
        ProviderPreset {
            id: "bailian",
            name: "Bailian",
            category: "cn_official",
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            model: "qwen-max",
            display_name: "Qwen Max",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://bailian.console.aliyun.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "bailian-coding",
            name: "Bailian For Coding",
            category: "cn_official",
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            model: "qwen-max-coding",
            display_name: "Qwen Max (Coding)",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://bailian.console.aliyun.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "stepfun",
            name: "StepFun",
            category: "cn_official",
            base_url: "https://api.stepfun.com/v1",
            model: "step-3.5-flash",
            display_name: "Step 3.5 Flash",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://platform.stepfun.com",
            api_key_url: Some("https://platform.stepfun.com/interface-key"),
        },
        ProviderPreset {
            id: "minimax",
            name: "MiniMax",
            category: "cn_official",
            base_url: "https://api.minimaxi.com/v1",
            model: "minimax-m2.7",
            display_name: "MiniMax M2.7",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://platform.minimaxi.com",
            api_key_url: Some("https://platform.minimaxi.com/subscribe/coding-plan"),
        },
        ProviderPreset {
            id: "doubao",
            name: "DouBao",
            category: "cn_official",
            base_url: "https://ark.cn-beijing.volces.com/api/v3",
            model: "doubao-seed-2.0",
            display_name: "DouBao Seed 2.0",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://console.volcengine.com/ark",
            api_key_url: None,
        },
        ProviderPreset {
            id: "baidu-qianfan",
            name: "Baidu Qianfan",
            category: "cn_official",
            base_url: "https://qianfan.baidubce.com/v2",
            model: "qianfan-code-latest",
            display_name: "Qianfan Code",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://cloud.baidu.com/product/qianfan_modelbuilder",
            api_key_url: Some(
                "https://console.bce.baidu.com/qianfan/ais/console/applicationConsole/application",
            ),
        },
        ProviderPreset {
            id: "longcat",
            name: "LongCat",
            category: "cn_official",
            base_url: "https://api.longcat.chat/v1",
            model: "longcat-flash",
            display_name: "LongCat Flash",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://longcat.chat",
            api_key_url: Some("https://longcat.chat/platform/api_keys"),
        },
        // Aggregator Providers
        ProviderPreset {
            id: "siliconflow",
            name: "SiliconFlow",
            category: "aggregator",
            base_url: "https://api.siliconflow.cn/v1",
            model: "minimax-m2.7",
            display_name: "MiniMax M2.7",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://cloud.siliconflow.cn",
            api_key_url: Some("https://cloud.siliconflow.cn/i/drGuwc9k"),
        },
        ProviderPreset {
            id: "aihubmix",
            name: "AiHubMix",
            category: "aggregator",
            base_url: "https://aihubmix.com/v1",
            model: "claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://aihubmix.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "dmxapi",
            name: "DMXAPI",
            category: "aggregator",
            base_url: "https://www.dmxapi.com/v1",
            model: "claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://www.dmxapi.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "modelscope",
            name: "ModelScope",
            category: "aggregator",
            base_url: "https://api.modelscope.cn/v1",
            model: "glm-5",
            display_name: "GLM 5",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://modelscope.cn",
            api_key_url: None,
        },
        // Third Party Providers
        ProviderPreset {
            id: "openrouter",
            name: "OpenRouter",
            category: "third_party",
            base_url: "https://openrouter.ai/api/v1",
            model: "anthropic/claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://openrouter.ai",
            api_key_url: None,
        },
        ProviderPreset {
            id: "together",
            name: "Together AI",
            category: "third_party",
            base_url: "https://api.together.xyz/v1",
            model: "anthropic/claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://together.ai",
            api_key_url: None,
        },
        ProviderPreset {
            id: "fireworks",
            name: "Fireworks AI",
            category: "third_party",
            base_url: "https://api.fireworks.ai/inference/v1",
            model: "anthropic/claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://fireworks.ai",
            api_key_url: None,
        },
        // Local Providers
        ProviderPreset {
            id: "ollama",
            name: "Ollama (Local)",
            category: "local",
            base_url: "http://localhost:11434/v1",
            model: "llama3",
            display_name: "Llama 3",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://ollama.ai",
            api_key_url: None,
        },
        ProviderPreset {
            id: "lmstudio",
            name: "LM Studio (Local)",
            category: "local",
            base_url: "http://localhost:1234/v1",
            model: "local-model",
            display_name: "Local Model",
            api_format: ApiFormat::OpenAiChat,
            website_url: "https://lmstudio.ai",
            api_key_url: None,
        },
    ]
}

/// Get preset by ID
pub fn get_preset_by_id(id: &str) -> Option<ProviderPreset> {
    get_all_presets().into_iter().find(|p| p.id == id)
}

/// Get presets by category
pub fn get_presets_by_category(category: &str) -> Vec<ProviderPreset> {
    get_all_presets()
        .into_iter()
        .filter(|p| p.category == category)
        .collect()
}

/// Get all categories
pub fn get_categories() -> Vec<&'static str> {
    vec![
        "official",
        "cn_official",
        "aggregator",
        "third_party",
        "local",
    ]
}

/// Get category display name
pub fn get_category_display_name(category: &str) -> &'static str {
    match category {
        "official" => "Official",
        "cn_official" => "Chinese Official",
        "aggregator" => "Aggregator",
        "third_party" => "Third Party",
        "local" => "Local",
        _ => "Other",
    }
}
