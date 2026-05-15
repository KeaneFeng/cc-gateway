# cc-gateway

Multi-provider aggregation gateway for [Claude Code](https://docs.anthropic.com/en/docs/claude-code).

Use multiple AI providers (Mimo, Kimi, Qwen, DeepSeek, GLM, etc.) with Claude Code — switch via `/model` per terminal.

```
Terminal 1: claude -> /model -> claude-mimo    -> Xiaomi Mimo 2.5 Pro
Terminal 2: claude -> /model -> claude-kimi    -> Moonshot Kimi 2.5
Terminal 3: claude -> /model -> claude-qwen    -> Alibaba Qwen 3.6 Plus
Terminal 4: claude -> /model -> claude-deepseek -> DeepSeek R1
```

## Install

### Homebrew (macOS / Linux)

```bash
brew tap KeaneFeng/cc-gateway https://github.com/KeaneFeng/cc-gateway
brew install cc-gateway
```

### Cargo

```bash
cargo install cc-gateway
```

### Build from source

```bash
git clone https://github.com/KeaneFeng/cc-gateway.git
cd cc-gateway
cargo build --release
# Binary at: target/release/cc-gateway
```

## Quick Start

```bash
# 1. Add providers from presets
cc-gateway add mimo
cc-gateway add kimi

# 2. Start the server (background)
cc-gateway start -d

# 3. Use Claude Code — it auto-configures ~/.claude/settings.json
claude
# In Claude Code: /model -> select provider
```

## Commands

### Server Management

```bash
cc-gateway start              # Start server (foreground, Ctrl+C to stop)
cc-gateway start -d           # Start server (background daemon)
cc-gateway start -f           # Force start (auto-stops existing instance)
cc-gateway stop               # Stop the running server
cc-gateway restart            # Restart (stop + start, foreground)
cc-gateway restart -d         # Restart (background)
```

### Provider Management

```bash
cc-gateway                    # Interactive dashboard (default)
cc-gateway add [preset]       # Add provider (interactive or from preset)
cc-gateway edit [id]          # Edit a provider
cc-gateway remove [id]        # Remove a provider
cc-gateway default [id]       # Set default provider
```

### Diagnostics

```bash
cc-gateway test [id]          # Test provider connections
cc-gateway status             # Show provider status table
```

### Configuration

```bash
cc-gateway presets            # Browse all available presets
cc-gateway presets --category chinese  # Filter by category
cc-gateway config             # Show current config
cc-gateway config --set port --value 8080
cc-gateway import             # Import from cc-switch
```

### Shell Completions

```bash
# Bash
cc-gateway completion bash > ~/.bash_completion.d/cc-gateway

# Zsh
cc-gateway completion zsh > ~/.zfunc/_cc-gateway

# Fish
cc-gateway completion fish > ~/.config/fish/completions/cc-gateway.fish
```

## Available Presets

| Category | Presets |
|----------|---------|
| Chinese Official | `deepseek` `zhipu` `kimi` `kimi-coding` `bailian` `bailian-coding` `stepfun` `minimax` `doubao` `baidu-qianfan` `longcat` |
| Aggregator | `siliconflow` `aihubmix` `dmxapi` `modelscope` |
| Third Party | `openrouter` `together` `fireworks` |
| Local | `ollama` `lmstudio` |

## How It Works

```
Claude Code  ->  POST /v1/messages  ->  cc-gateway (port 16789)
                                           |
                                     Route selection:
                                       1. Session-based (x-claude-code-session-id header)
                                       2. Model-based (claude-{id} in request)
                                       3. Default provider (fallback)
                                           |
                                     Format conversion
                                       Anthropic <-> OpenAI (automatic)
                                           |
                                     Forward to upstream provider
                                           |
                                     Stream response back to Claude Code
```

- **Session routing**: Different terminals/projects can use different providers simultaneously
- **Format conversion**: Automatic bidirectional conversion between Anthropic and OpenAI API formats
- **SSE streaming**: Full streaming support with raw byte forwarding
- **Settings auto-update**: `cc-gateway start` automatically configures `~/.claude/settings.json`

## Config File

Location: `~/.cc-gateway/config.toml`

```toml
port = 16789
host = "127.0.0.1"
log_level = "info"

[[providers]]
id = "mimo"
name = "Xiaomi MiMo"
api_type = "anthropic"
base_url = "https://api.mimo.xiaomi.com/v1"
api_key = "sk-xxx"
model = "mimo-2.5-pro"
display_name = "Mimo 2.5 Pro"
is_default = true

[[providers]]
id = "kimi"
name = "Moonshot Kimi"
api_type = "anthropic"
base_url = "https://api.moonshot.cn/v1"
api_key = "sk-xxx"
model = "kimi-2.5"
display_name = "Kimi 2.5"
```

## Files

| Path | Purpose |
|------|---------|
| `~/.cc-gateway/config.toml` | Provider configuration |
| `~/.cc-gateway/cc-gateway.db` | SQLite database (usage, health) |
| `~/.cc-gateway/cc-gateway.pid` | PID file (for start/stop) |
| `~/.cc-gateway/cc-gateway.log` | Daemon log (when using `start -d`) |

## License

MIT
