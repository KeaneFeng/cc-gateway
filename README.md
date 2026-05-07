# CC-Switch-Pro

Lightweight multi-provider aggregation proxy for Claude Code, written in Rust.

## Features

- **Multi-provider aggregation** — Configure multiple AI providers and switch via `/model`
- **Interactive TUI** — Arrow keys, TAB, Enter navigation for easy management
- **21+ pre-configured presets** — Quick add from popular providers
- **Import from cc-switch** — One-click import of existing providers
- **Anthropic ↔ OpenAI format conversion** — Automatic API format conversion
- **SSE streaming support** — Full streaming with Anthropic SSE events
- **Tool use support** — Converts tool/function calling between formats
- **Lightweight** — Single binary (5.2MB), no external dependencies

## Installation

```bash
git clone https://github.com/yourusername/cc-switch-pro.git
cd cc-switch-pro
cargo build --release
```

## Quick Start

### Interactive Mode (Recommended)

```bash
cc-switch-pro interactive
```

Use arrow keys (↑↓) to navigate, Enter to select, TAB to complete inputs.

### Import from cc-switch

```bash
cc-switch-pro import
cc-switch-pro serve
```

### Add from presets

```bash
cc-switch-pro presets                              # List presets
cc-switch-pro add --preset deepseek --key YOUR_KEY # Add provider
cc-switch-pro serve                                # Start proxy
```

## CLI Commands

### Interactive Mode

```bash
cc-switch-pro interactive    # Launch TUI with arrow key navigation
```

Features:
- ↑↓ Arrow keys — Navigate menus
- Enter — Select/confirm
- TAB — Auto-complete inputs
- Type to filter/search

### Provider Management

```bash
# List providers
cc-switch-pro list              # Detailed view
cc-switch-pro list --table      # Table view

# Add from preset
cc-switch-pro add --preset <PRESET_ID> --key YOUR_API_KEY

# Add custom
cc-switch-pro add --id myprovider --name "My Provider" \
  --url https://api.example.com/v1 --key YOUR_KEY --model model-name

# Edit provider
cc-switch-pro edit --id myprovider --key NEW_KEY
cc-switch-pro edit --id myprovider --url NEW_URL --model NEW_MODEL

# Copy provider
cc-switch-pro copy --from source-id --to new-id

# Remove provider
cc-switch-pro remove --id myprovider

# Set default
cc-switch-pro set-default --id myprovider
```

### Presets

```bash
cc-switch-pro presets                    # List all
cc-switch-pro presets --category cn_official  # By category
cc-switch-pro presets --detail           # With details
```

### Import from cc-switch

```bash
cc-switch-pro import    # Auto-detect cc-switch database
```

### Server

```bash
cc-switch-pro serve               # Start proxy
cc-switch-pro serve --port 8080   # Custom port
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

## Usage with Claude Code

```bash
# Terminal 1: Start proxy
cc-switch-pro serve

# Terminal 2: Use Claude Code
export ANTHROPIC_BASE_URL=http://127.0.0.1:15780
claude

# In Claude Code: /model → select provider
/model → claude-deepseek
/model → claude-kimi
/model → claude-zhipu
```

## Config File

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
