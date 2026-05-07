//! Provider presets
//!
//! Pre-configured provider settings for common AI providers

use serde::{Deserialize, Serialize};

/// Provider preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub display_name: &'static str,
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
            website_url: "https://open.bigmodel.cn",
            api_key_url: Some("https://www.bigmodel.cn/claude-code"),
        },
        ProviderPreset {
            id: "kimi",
            name: "Kimi",
            category: "cn_official",
            base_url: "https://api.moonshot.cn/anthropic",
            model: "kimi-k2.6",
            display_name: "Kimi K2.6",
            website_url: "https://platform.moonshot.cn",
            api_key_url: None,
        },
        ProviderPreset {
            id: "kimi-coding",
            name: "Kimi For Coding",
            category: "cn_official",
            base_url: "https://api.kimi.com/coding/",
            model: "kimi-k2.6",
            display_name: "Kimi K2.6 (Coding)",
            website_url: "https://www.kimi.com/code/docs/",
            api_key_url: None,
        },
        ProviderPreset {
            id: "bailian",
            name: "Bailian",
            category: "cn_official",
            base_url: "https://dashscope.aliyuncs.com/apps/anthropic",
            model: "qwen-max",
            display_name: "Qwen Max",
            website_url: "https://bailian.console.aliyun.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "bailian-coding",
            name: "Bailian For Coding",
            category: "cn_official",
            base_url: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
            model: "qwen-max",
            display_name: "Qwen Max (Coding)",
            website_url: "https://bailian.console.aliyun.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "stepfun",
            name: "StepFun",
            category: "cn_official",
            base_url: "https://api.stepfun.com/step_plan",
            model: "step-3.5-flash-2603",
            display_name: "Step 3.5 Flash",
            website_url: "https://platform.stepfun.com",
            api_key_url: Some("https://platform.stepfun.com/interface-key"),
        },
        ProviderPreset {
            id: "minimax",
            name: "MiniMax",
            category: "cn_official",
            base_url: "https://api.minimaxi.com/anthropic",
            model: "MiniMax-M2.7",
            display_name: "MiniMax M2.7",
            website_url: "https://platform.minimaxi.com",
            api_key_url: Some("https://platform.minimaxi.com/subscribe/coding-plan"),
        },
        ProviderPreset {
            id: "doubao",
            name: "DouBao",
            category: "cn_official",
            base_url: "https://ark.cn-beijing.volces.com/api/coding",
            model: "doubao-seed-2-0-code-preview-latest",
            display_name: "DouBao Seed 2.0",
            website_url: "https://www.volcengine.com/product/doubao",
            api_key_url: None,
        },
        ProviderPreset {
            id: "baidu-qianfan",
            name: "Baidu Qianfan",
            category: "cn_official",
            base_url: "https://qianfan.baidubce.com/anthropic/coding",
            model: "qianfan-code-latest",
            display_name: "Qianfan Code",
            website_url: "https://cloud.baidu.com/product/qianfan_modelbuilder",
            api_key_url: Some("https://console.bce.baidu.com/qianfan/ais/console/applicationConsole/application"),
        },
        ProviderPreset {
            id: "longcat",
            name: "LongCat",
            category: "cn_official",
            base_url: "https://api.longcat.chat/anthropic",
            model: "LongCat-Flash-Chat",
            display_name: "LongCat Flash",
            website_url: "https://longcat.chat/platform",
            api_key_url: Some("https://longcat.chat/platform/api_keys"),
        },
        // Aggregators
        ProviderPreset {
            id: "siliconflow",
            name: "SiliconFlow",
            category: "aggregator",
            base_url: "https://api.siliconflow.cn",
            model: "Pro/MiniMaxAI/MiniMax-M2.7",
            display_name: "MiniMax M2.7",
            website_url: "https://siliconflow.cn",
            api_key_url: Some("https://cloud.siliconflow.cn/i/drGuwc9k"),
        },
        ProviderPreset {
            id: "aihubmix",
            name: "AiHubMix",
            category: "aggregator",
            base_url: "https://aihubmix.com",
            model: "claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            website_url: "https://aihubmix.com",
            api_key_url: None,
        },
        ProviderPreset {
            id: "dmxapi",
            name: "DMXAPI",
            category: "aggregator",
            base_url: "https://www.dmxapi.cn",
            model: "claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            website_url: "https://www.dmxapi.cn",
            api_key_url: None,
        },
        ProviderPreset {
            id: "modelscope",
            name: "ModelScope",
            category: "aggregator",
            base_url: "https://api-inference.modelscope.cn",
            model: "ZhipuAI/GLM-5",
            display_name: "GLM 5",
            website_url: "https://modelscope.cn",
            api_key_url: None,
        },
        // Third-party
        ProviderPreset {
            id: "openrouter",
            name: "OpenRouter",
            category: "third_party",
            base_url: "https://openrouter.ai/api/v1",
            model: "anthropic/claude-sonnet-4",
            display_name: "Claude Sonnet 4",
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
            website_url: "https://together.ai",
            api_key_url: None,
        },
        ProviderPreset {
            id: "fireworks",
            name: "Fireworks AI",
            category: "third_party",
            base_url: "https://api.fireworks.ai/inference/v1",
            model: "accounts/fireworks/models/claude-sonnet-4",
            display_name: "Claude Sonnet 4",
            website_url: "https://fireworks.ai",
            api_key_url: None,
        },
        // Custom / Local
        ProviderPreset {
            id: "ollama",
            name: "Ollama (Local)",
            category: "local",
            base_url: "http://localhost:11434/v1",
            model: "llama3",
            display_name: "Llama 3",
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
            website_url: "https://lmstudio.ai",
            api_key_url: None,
        },
    ]
}

/// Get presets by category
pub fn get_presets_by_category(category: &str) -> Vec<ProviderPreset> {
    get_all_presets()
        .into_iter()
        .filter(|p| p.category == category)
        .collect()
}

/// Get preset by ID
pub fn get_preset_by_id(id: &str) -> Option<ProviderPreset> {
    get_all_presets().into_iter().find(|p| p.id == id)
}

/// Get all categories
pub fn get_categories() -> Vec<&'static str> {
    vec!["official", "cn_official", "aggregator", "third_party", "local"]
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
