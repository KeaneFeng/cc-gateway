# cc-gateway

Multi-provider aggregation gateway for Claude Code.

## What is this?

A lightweight proxy that lets you use multiple AI providers (Mimo, Kimi, Qwen, GLM, etc.) with Claude Code. Configure providers once, then switch between them via `/model` in Claude Code — each terminal independently.

```
Terminal 1: claude → /model → claude-mimo    → Xiaomi Mimo 2.5 Pro
Terminal 2: claude → /model → claude-kimi    → Moonshot Kimi 2.5
Terminal 3: claude → /model → claude-qwen    → Alibaba Qwen 2.6 Plus
Terminal 4: claude → /model → claude-glm     → Zhipu GLM 5.1
```

## Install

```bash
# Homebrew (macOS/Linux)
brew tap yourusername/cc-gateway
brew install cc-gateway

# Cargo
cargo install cc-gateway

# Build from source
git clone https://github.com/yourusername/cc-gateway.git
cd cc-gateway
cargo build --release
```

## Quick Start

```bash
# 1. Launch interactive dashboard (default)
cc-gateway

# 2. Add providers from presets
cc-gateway add mimo      # Xiaomi Mimo
cc-gateway add kimi      # Moonshot Kimi
cc-gateway add glm       # Zhipu GLM

# 3. Start the proxy
cc-gateway serve

# 4. Use Claude Code
export ANTHROPIC_BASE_URL=http://127.0.0.1:16789
claude
# In Claude Code: /model → select provider
```

## Commands

```bash
cc-gateway              # Interactive dashboard (default)
cc-gateway serve        # Start proxy server
cc-gateway add [preset] # Add provider (interactive or from preset)
cc-gateway edit [id]    # Edit provider
cc-gateway remove [id]  # Remove provider
cc-gateway default [id] # Set default provider
cc-gateway test [id]    # Test connections
cc-gateway status       # Show provider status
cc-gateway import       # Import from cc-switch
cc-gateway presets      # Browse presets
cc-gateway config       # Show/edit config
```

## Available Presets

### Chinese Official
`deepseek` `zhipu` `kimi` `kimi-coding` `bailian` `bailian-coding` `stepfun` `minimax` `doubao` `baidu-qianfan` `longcat`

### Aggregator
`siliconflow` `aihubmix` `dmxapi` `modelscope`

### Third Party
`openrouter` `together` `fireworks`

### Local
`ollama` `lmstudio`

## Config File

```toml
port = 16789
host = "127.0.0.1"
log_level = "info"

[[providers]]
id = "mimo"
name = "Xiaomi MiMo"
api_type = "openai"
base_url = "https://api.mimo.xiaomi.com/v1"
api_key = "sk-xxx"
model = "mimo-2.5-pro"
display_name = "Mimo 2.5 Pro"
is_default = true

[[providers]]
id = "kimi"
name = "Moonshot Kimi"
api_type = "openai"
base_url = "https://api.moonshot.cn/v1"
api_key = "sk-xxx"
model = "kimi-2.5"
display_name = "Kimi 2.5"
```

## Features

- **Multi-provider aggregation** — Configure multiple providers, switch via `/model`
- **Interactive dashboard** — Default mode with arrow key navigation
- **21+ presets** — Quick add from popular providers
- **Import from cc-switch** — One-click migration
- **Connection testing** — Test provider connectivity
- **Usage tracking** — Token usage and cost monitoring
- **Health monitoring** — Provider health status
- **Format conversion** — Anthropic ↔ OpenAI automatic conversion
- **SSE streaming** — Full streaming support
- **Lightweight** — Single binary, no external dependencies

## License

MIT
