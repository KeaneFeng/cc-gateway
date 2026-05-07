# CC-Switch-Pro

Lightweight multi-provider aggregation proxy for Claude Code, written in Rust.

## Features

- **Multi-provider aggregation** — Configure multiple AI providers and switch between them via Claude Code's `/model` command
- **Per-session model selection** — Each terminal session can independently select a different provider/model
- **21+ pre-configured presets** — Quick add from popular providers (DeepSeek, Kimi, GLM, Qwen, etc.)
- **Import from cc-switch** — One-click import of your existing cc-switch providers
- **Anthropic ↔ OpenAI format conversion** — Automatically converts between API formats
- **SSE streaming support** — Full streaming support with proper Anthropic SSE event format
- **Tool use support** — Converts tool/function calling between formats
- **Simple CLI management** — Easy commands to add, edit, copy, remove providers
- **Lightweight** — Single binary (5.2MB), no external dependencies

## Installation

### Build from source

```bash
git clone https://github.com/yourusername/cc-switch-pro.git
cd cc-switch-pro
cargo build --release
```

### Install via cargo

```bash
cargo install cc-switch-pro
```

## Quick Start

### Option 1: Import from cc-switch (recommended)

If you already have cc-switch configured:

```bash
# Import all your providers from cc-switch
cc-switch-pro import

# Start the proxy
cc-switch-pro serve
```

### Option 2: Add from presets

```bash
# List available presets
cc-switch-pro presets

# Add a provider from preset
cc-switch-pro add --preset deepseek --key YOUR_API_KEY
cc-switch-pro add --preset kimi --key YOUR_API_KEY
cc-switch-pro add --preset zhipu --key YOUR_API_KEY

# Start the proxy
cc-switch-pro serve
```

### Option 3: Manual configuration

```bash
# Generate example config
cc-switch-pro init

# Edit config file
vim ~/.cc-switch-pro/config.toml

# Start the proxy
cc-switch-pro serve
```

## CLI Commands

### Provider Management

```bash
# List providers
cc-switch-pro list              # Detailed view
cc-switch-pro list --table      # Table view

# Add from preset
cc-switch-pro add --preset <PRESET_ID> --key YOUR_API_KEY

# Add custom provider
cc-switch-pro add --id myprovider --name "My Provider" \
  --url https://api.example.com/v1 --key YOUR_KEY --model model-name

# Edit provider
cc-switch-pro edit --id myprovider --key NEW_KEY
cc-switch-pro edit --id myprovider --url NEW_URL --model NEW_MODEL

# Copy provider
cc-switch-pro copy --from source-id --to new-id

# Remove provider
cc-switch-pro remove --id myprovider

# Set default provider
cc-switch-pro set-default --id myprovider
```

### Presets

```bash
# List all presets
cc-switch-pro presets

# List presets by category
cc-switch-pro presets --category cn_official
cc-switch-pro presets --category aggregator
cc-switch-pro presets --category third_party
cc-switch-pro presets --category local

# Show preset details
cc-switch-pro presets --detail
```

### Import from cc-switch

```bash
# Import all Claude providers from cc-switch
cc-switch-pro import
```

### Server

```bash
# Start proxy server
cc-switch-pro serve

# With custom port
cc-switch-pro serve --port 8080

# With custom config
cc-switch-pro serve --config /path/to/config.toml
```

## Available Presets

### Official
- `claude-official` — Claude Official

### Chinese Official
- `deepseek` — DeepSeek
- `zhipu` — Zhipu GLM
- `kimi` — Kimi
- `kimi-coding` — Kimi For Coding
- `bailian` — Bailian (Qwen)
- `bailian-coding` — Bailian For Coding
- `stepfun` — StepFun
- `minimax` — MiniMax
- `doubao` — DouBao
- `baidu-qianfan` — Baidu Qianfan
- `longcat` — LongCat

### Aggregator
- `siliconflow` — SiliconFlow
- `aihubmix` — AiHubMix
- `dmxapi` — DMXAPI
- `modelscope` — ModelScope

### Third Party
- `openrouter` — OpenRouter
- `together` — Together AI
- `fireworks` — Fireworks AI

### Local
- `ollama` — Ollama (Local)
- `lmstudio` — LM Studio (Local)

## Usage with Claude Code

```bash
# Start the proxy
cc-switch-pro serve

# In another terminal
export ANTHROPIC_BASE_URL=http://127.0.0.1:15780
claude

# In Claude Code, use /model to switch
/model → claude-deepseek
/model → claude-kimi
/model → claude-zhipu
```

## Architecture

```
┌─────────────────┐
│  Claude Code    │
│  Terminal 1     │──► /model → claude-deepseek
├─────────────────┤
│  Claude Code    │
│  Terminal 2     │──► /model → claude-kimi
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│              CC-Switch-Pro Proxy                │
│                                                 │
│  GET /v1/models → List all providers            │
│  POST /v1/messages → Route by model field       │
│                                                 │
│  ┌─────────────┐  ┌─────────────┐              │
│  │claude-deepseek│ │claude-kimi │  ...         │
│  └──────┬──────┘  └──────┬──────┘              │
│         ▼                ▼                      │
│  Anthropic → OpenAI  Anthropic → OpenAI         │
└────────┬────────────────────┬───────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────┐  ┌─────────────────┐
│  DeepSeek API   │  │  Kimi API       │
└─────────────────┘  └─────────────────┘
```

## Config File Format

```toml
port = 15780
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
```

## License

MIT
