# CC-Switch-Pro

Lightweight multi-provider aggregation proxy for Claude Code, written in Rust.

## Features

- **Multi-provider aggregation** — Configure multiple AI providers (Mimo, Kimi, GLM, Qwen, etc.) and switch between them via Claude Code's `/model` command
- **Per-session model selection** — Each terminal session can independently select a different provider/model
- **Anthropic ↔ OpenAI format conversion** — Automatically converts between Anthropic Messages API and OpenAI Chat Completions API
- **SSE streaming support** — Full streaming support with proper Anthropic SSE event format
- **Tool use support** — Converts tool/function calling between Anthropic and OpenAI formats
- **Simple CLI management** — Easy commands to add, remove, list, and set default providers
- **Lightweight** — Single binary, no external dependencies (no Tauri, no Node.js, no SQLite)

## Installation

### Build from source

```bash
# Clone the repository
git clone https://github.com/yourusername/cc-switch-pro.git
cd cc-switch-pro

# Build
cargo build --release

# Binary will be at target/release/cc-switch-pro
```

### Install via cargo

```bash
cargo install cc-switch-pro
```

## Quick Start

### 1. Generate example config

```bash
cc-switch-pro init
```

This creates `~/.cc-switch-pro/config.toml` with example providers.

### 2. Edit config with your API keys

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

[[providers]]
id = "kimi"
name = "Moonshot Kimi"
api_type = "openai"
base_url = "https://api.moonshot.cn/v1"
api_key = "sk-xxx"
model = "kimi-2.5"
display_name = "Kimi 2.5"
```

### 3. Start the proxy

```bash
cc-switch-pro serve
```

### 4. Configure Claude Code

```bash
# In your terminal
export ANTHROPIC_BASE_URL=http://127.0.0.1:15780

# Start Claude Code
claude
```

### 5. Switch models in Claude Code

Use `/model` command to see available models:

```
/model
```

You'll see models like:
- `claude-mimo` — Mimo 2.5 Pro
- `claude-kimi` — Kimi 2.5
- `claude-glm` — GLM 5.1

Select a model and all subsequent requests will be routed to that provider.

## CLI Commands

### `cc-switch-pro init`
Generate example config file.

### `cc-switch-pro serve`
Start the proxy server.

Options:
- `-c, --config <path>` — Config file path (default: `~/.cc-switch-pro/config.toml`)
- `-p, --port <port>` — Server port (overrides config)
- `--host <host>` — Server host (overrides config)

### `cc-switch-pro list`
List all configured providers.

### `cc-switch-pro add`
Add a new provider.

```bash
cc-switch-pro add \
  --id myprovider \
  --name "My Provider" \
  --url "https://api.example.com/v1" \
  --key "sk-xxx" \
  --model "model-name" \
  --display-name "My Model"
```

### `cc-switch-pro remove`
Remove a provider.

```bash
cc-switch-pro remove --id myprovider
```

### `cc-switch-pro set-default`
Set the default provider.

```bash
cc-switch-pro set-default --id mimo
```

## Architecture

```
┌─────────────────┐
│  Claude Code    │
│  Terminal 1     │──► /model → claude-mimo
│                 │
├─────────────────┤
│  Claude Code    │
│  Terminal 2     │──► /model → claude-kimi
│                 │
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
│  │  claude-mimo│  │  claude-kimi│  ...         │
│  └──────┬──────┘  └──────┬──────┘              │
│         │                │                      │
│         ▼                ▼                      │
│  Anthropic → OpenAI  Anthropic → OpenAI         │
│  format conversion   format conversion          │
└────────┬────────────────────┬───────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────┐  ┌─────────────────┐
│  Xiaomi MiMo    │  │  Moonshot Kimi  │
│  API            │  │  API            │
└─────────────────┘  └─────────────────┘
```

## Config File Format

```toml
# Server settings
port = 15780
host = "127.0.0.1"
log_level = "info"  # trace, debug, info, warn, error

# Provider configuration
[[providers]]
id = "mimo"                    # Unique ID (used in model ID: claude-mimo)
name = "Xiaomi MiMo"          # Display name
api_type = "openai"            # API type: openai or anthropic
base_url = "https://api.mimo.xiaomi.com/v1"
api_key = "sk-xxx"
model = "mimo-2.5-pro"        # Model name to send to provider
display_name = "Mimo 2.5 Pro" # Optional: display name in /model picker
is_default = true              # Whether this is the default provider
```

## Environment Variables

- `ANTHROPIC_BASE_URL` — Set to `http://127.0.0.1:15780` to use the proxy
- `RUST_LOG` — Set log level (e.g., `debug`, `info`, `warn`, `error`)

## License

MIT
