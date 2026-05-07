# CC-Switch-Pro

Lightweight multi-provider aggregation proxy for Claude Code, written in Rust.

## Features

- **Multi-provider aggregation** — Configure multiple AI providers and switch via `/model`
- **Interactive TUI** — Arrow keys, TAB, Enter navigation
- **21+ pre-configured presets** — Quick add from popular providers
- **Import from cc-switch** — One-click migration with SQLite compatibility
- **Connection testing** — Test provider connectivity
- **Usage statistics** — Track token usage and costs
- **Health monitoring** — Monitor provider health status
- **Proxy configuration** — Configure proxy settings
- **Anthropic ↔ OpenAI format conversion** — Automatic API format conversion
- **SSE streaming support** — Full streaming with Anthropic SSE events
- **Lightweight** — Single binary (~6MB), no external dependencies

## Installation

### Homebrew (macOS/Linux)

```bash
# Add tap
brew tap yourusername/cc-switch-pro

# Install
brew install cc-switch-pro

# Upgrade
brew upgrade cc-switch-pro
```

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

### Interactive Mode (Recommended)

```bash
cc-switch-pro interactive
```

Use arrow keys (↑↓), TAB, Enter to navigate.

### Import from cc-switch

```bash
cc-switch-pro import
cc-switch-pro serve
```

### Add from presets

```bash
cc-switch-pro presets
cc-switch-pro add --preset deepseek --key YOUR_KEY
cc-switch-pro serve
```

## CLI Commands

### Provider Management

```bash
cc-switch-pro list                    # List providers
cc-switch-pro list --table            # Table view
cc-switch-pro add --preset <ID> --key KEY  # Add from preset
cc-switch-pro edit --id <ID> --key KEY     # Edit provider
cc-switch-pro copy --from <ID> --to <ID>   # Copy provider
cc-switch-pro remove --id <ID>             # Remove provider
cc-switch-pro set-default --id <ID>        # Set default
```

### Connection Testing

```bash
cc-switch-pro test              # Test all providers
cc-switch-pro test --id <ID>    # Test specific provider
cc-switch-pro test --save       # Save results to database
```

### Usage Statistics

```bash
cc-switch-pro usage             # Show last 30 days
cc-switch-pro usage --days 7    # Show last 7 days
cc-switch-pro usage --provider <ID>  # Provider-specific
```

### Health Monitoring

```bash
cc-switch-pro health            # Show provider health status
```

### Proxy Configuration

```bash
cc-switch-pro proxy-config --show              # Show config
cc-switch-pro proxy-config --enable true       # Enable proxy
cc-switch-pro proxy-config --port 8080         # Set port
cc-switch-pro proxy-config --failover true     # Enable failover
```

### Import from cc-switch

```bash
cc-switch-pro import            # Auto-detect cc-switch database
cc-switch-pro import --db /path/to/cc-switch.db  # Custom path
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
export ANTHROPIC_BASE_URL=http://127.0.0.1:16789
claude

# In Claude Code: /model → select provider
/model → claude-deepseek
/model → claude-kimi
```

## Database Compatibility

CC-Switch-Pro uses the same SQLite schema as cc-switch, stored at:
- `~/.cc-switch-pro/cc-switch-pro.db`

This allows:
- Seamless migration from cc-switch
- Shared usage tracking data
- Compatible health monitoring

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
```

## License

MIT
