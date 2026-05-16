<div align="center">

# cc-gateway

### Lightweight Multi-Provider Aggregation Proxy for Claude Code

[![Version](https://img.shields.io/badge/version-v0.4.0-blue)](https://github.com/KeaneFeng/cc-gateway/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](https://github.com/KeaneFeng/cc-gateway/releases)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

~8MB binary &nbsp;|&nbsp; Zero GUI dependencies &nbsp;|&nbsp; Runs in any terminal

English | [中文](README_zh.md)

</div>

---

Route Claude Code requests to any AI provider — MiMo, Kimi, Qwen, DeepSeek, GLM, OpenRouter, and more. Switch providers per terminal session via `/model`, or manage everything from the built-in interactive TUI dashboard.

---

## Why cc-gateway?

Claude Code only supports one API endpoint at a time. If you want to use different providers in different terminals — or switch between them without editing config files — you need a proxy.

**cc-gateway** is that proxy. It sits between Claude Code and your providers, handling:

- **Multi-provider routing** — different providers for different terminals/projects simultaneously
- **Format conversion** — automatic bidirectional Anthropic ↔ OpenAI API translation
- **Session persistence** — remembers which project uses which provider
- **Interactive TUI** — full dashboard for adding, editing, testing, and managing providers
- **Usage tracking** — request logs, token counts, costs, and latency stats
- **One-command setup** — `cc-gateway start` auto-configures Claude Code's settings.json

Unlike desktop apps (cc-switch, etc.), cc-gateway is a **CLI-only tool** — zero GUI dependencies, runs in any terminal, lightweight and fast.

---

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

> **Note**: Requires Rust 1.75+ (for bundled SQLite). The `cargo install` method builds from crates.io source.

### Build from Source

```bash
git clone https://github.com/KeaneFeng/cc-gateway.git
cd cc-gateway
cargo build --release
# Binary: target/release/cc-gateway
```

### Direct Binary Download

Download pre-built binaries from [Releases](https://github.com/KeaneFeng/cc-gateway/releases):

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `cc-gateway-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | build from source |
| Linux (x86_64) | build from source |
| Linux (ARM64) | build from source |

---

## Quick Start

```bash
# 1. Launch the interactive dashboard (default command)
cc-gateway
# → Press [a] to add a provider, select from 20+ presets
# → Press [d] to set default

# 2. Start the proxy server
cc-gateway start -d

# 3. Use Claude Code — settings.json is auto-configured
claude
# In Claude Code: /model → select claude-mimo, claude-kimi, etc.
```

Multiple terminals can use different providers simultaneously:

```
Terminal 1: claude → /model → claude-mimo     → Xiaomi MiMo 2.5 Pro
Terminal 2: claude → /model → claude-kimi     → Moonshot Kimi K2.6
Terminal 3: claude → /model → claude-deepseek → DeepSeek V4 Pro
```

---

## Interactive TUI Dashboard

Running `cc-gateway` (no arguments) opens a full interactive dashboard:

```
┌──────────────────────────── cc-gateway v0.4.0 ────────────────────────────┐
│          Multi-provider gateway for Claude Code                           │
└───────────────────────────────────────────────────────────────────────────┘

  #   Name               Model            Vision Model   Status
  ──────────────────────────────────────────────────────────────────────────
  1   ▸ Mimo 2.5 Pro     mimo-v2.5-pro    mimo-v2.5      ● Active
  2     Kimi K2.6        kimi-k2.6                       ● Default
  3     Qwen Max         qwen-max                        ● Available

  Proxy: Kimi K2.6 (kimi-k2.6)  |  Providers: 3  |  Routes: 0

  ───────────────────────────────────────────────────
  [e] Language  [a] Add  [d] Default  [p] Projects  [u] Stats
  [l] Logs  [t] Test  [b] Presets  [g] LogLevel  [i] Import
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate provider list |
| `Space` / `Enter` | Open action menu (set default / edit / details / test / copy / delete) |
| `a` | Add new provider |
| `d` | Set selected as default |
| `p` | Project management (map projects to providers) |
| `u` | Usage statistics (7/30/90/all days) |
| `l` | Request logs (20/50/100 entries) |
| `t` | Test provider connections |
| `b` | Browse presets |
| `g` | Toggle log level (info / debug) — takes effect immediately |
| `r` | Refresh balance for selected provider |
| `i` | Import from cc-switch database |
| `e` | Toggle language (English / 中文) |
| `Esc` / `Ctrl+C` | Exit |

---

## Commands

### Server Management

```bash
cc-gateway                  # Interactive dashboard (default)
cc-gateway start            # Start server (foreground)
cc-gateway start -d         # Start server (background daemon)
cc-gateway start -f         # Force start (stop existing instance first)
cc-gateway stop             # Stop running server
cc-gateway restart          # Restart (foreground)
cc-gateway restart -d       # Restart (background)
```

### Provider Management

```bash
cc-gateway add              # Add provider (interactive preset list)
cc-gateway add mimo         # Add from preset
cc-gateway edit [id]        # Edit a provider
cc-gateway remove [id]      # Remove a provider
cc-gateway default [id]     # Set default provider
```

### Diagnostics

```bash
cc-gateway test             # Test all provider connections
cc-gateway test mimo        # Test specific provider
cc-gateway status           # Show provider status table
cc-gateway usage            # Usage statistics (CLI)
```

### Configuration

```bash
cc-gateway presets          # Browse all available presets
cc-gateway config           # Show current config
cc-gateway config --set port --value 8080
cc-gateway import           # Import providers from cc-switch database
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

---

## Available Presets

| Category | Presets |
|----------|---------|
| **Official** | `claude-official` (Anthropic direct) |
| **Chinese Official** | `deepseek` `zhipu` `kimi` `kimi-coding` `bailian` `bailian-coding` `stepfun` `minimax` `doubao` `baidu-qianfan` `longcat` |
| **Aggregator** | `siliconflow` `aihubmix` `dmxapi` `modelscope` |
| **Third Party** | `openrouter` `together` `fireworks` |
| **Local** | `ollama` `lmstudio` |

---

## How It Works

```
┌─────────────┐     POST /v1/messages     ┌──────────────────────────────────┐
│ Claude Code │ ─────────────────────────▶ │  cc-gateway (port 16789)        │
└─────────────┘                           │                                  │
                                          │  Route Selection (3-tier):       │
                                          │    1. Session-based (header)     │
                                          │    2. Model-based (claude-{id})  │
                                          │    3. Default provider           │
                                          │                                  │
                                          │  Format Conversion:              │
                                          │    Anthropic ↔ OpenAI (auto)     │
                                          │                                  │
                                          │  Forward to Upstream Provider    │
                                          │  Stream Response Back            │
                                          └──────────────────────────────────┘
```

### Routing Priority

1. **Session routing** (highest) — `x-claude-code-session-id` header → project-to-provider mapping. Different projects automatically use their assigned provider.
2. **Model routing** — Claude Code's `/model` sends `claude-{id}` in the model field → matched to provider ID.
3. **Default fallback** — uses the currently active provider.

### Key Features

- **Format conversion**: Automatic Anthropic ↔ OpenAI bidirectional translation (request + response + streaming)
- **SSE streaming**: Raw byte forwarding, no double-wrapping
- **Session persistence**: Project-provider mappings survive restarts
- **Settings auto-update**: `cc-gateway start` automatically configures `~/.claude/settings.json`
- **HTTP/1.1 enforcement**: Avoids streaming issues with certain providers
- **Effort normalization**: Handles `xhigh` → `max` mapping for providers that don't support it
- **i18n**: English (default) and Simplified Chinese, toggle with `[e]`
- **Import from cc-switch**: Migrate existing provider configs from cc-switch database

---

## Configuration

### Config File

Location: `~/.cc-gateway/config.toml`

```toml
port = 16789
host = "127.0.0.1"
log_level = "info"

[[providers]]
id = "mimo"
name = "Xiaomi MiMo"
api_format = "OpenAiChat"
base_url = "https://api.mimo.xiaomi.com/v1"
api_key = "sk-xxx"
model = "mimo-v2.5-pro"
vision_model = "mimo-v2.5"
display_name = "Mimo 2.5 Pro"
is_default = true
effort_level = "max"

[[providers]]
id = "kimi"
name = "Moonshot Kimi"
api_format = "OpenAiChat"
base_url = "https://api.moonshot.cn/v1"
api_key = "sk-xxx"
model = "kimi-k2.6"
display_name = "Kimi K2.6"

[[providers]]
id = "claude"
name = "Claude Official"
api_format = "Anthropic"
base_url = "https://api.anthropic.com"
api_key = "sk-ant-xxx"
model = "claude-sonnet-4"
display_name = "Claude Sonnet 4"
```

### API Formats

| Format | Description |
|--------|-------------|
| `Anthropic` | Direct passthrough (no conversion needed) |
| `OpenAiChat` | Automatic Anthropic ↔ OpenAI conversion |
| `OpenAiResponses` | Reserved (not yet implemented) |
| `GeminiNative` | Reserved (not yet implemented) |

### Project-Provider Mapping

Map specific projects to specific providers via the TUI (`[p]` key):

```toml
[project_providers]
"/Users/you/www/project-a" = "mimo"
"/Users/you/www/project-b" = "kimi"
```

---

## Data Files

| Path | Purpose |
|------|---------|
| `~/.cc-gateway/config.toml` | Provider configuration + project mappings |
| `~/.cc-gateway/cc-gateway.db` | SQLite database (usage stats, request logs, cc-switch compatible) |
| `~/.cc-gateway/cc-gateway.pid` | PID file for daemon management |
| `~/.cc-gateway/cc-gateway.log` | Daemon log output |

---

## Architecture

```
src/
├── main.rs              # CLI entrypoint (clap), axum server, settings.json auto-update
├── interactive.rs       # TUI dashboard (crossterm raw mode + dialoguer)
├── mouse_tui.rs         # Alternative mouse-enabled TUI (WIP)
├── balance.rs           # Provider balance/quota query (DeepSeek, SiliconFlow, OpenRouter, etc.)
├── commands/            # CLI subcommands (serve/start/stop/restart, config, presets...)
├── config/              # AppConfig, ProviderConfig, ApiFormat enum, 20+ presets
├── database/            # SQLite via rusqlite, cc-switch-compatible schema
├── error/               # ProxyError with HTTP status mapping
├── provider/            # reqwest HTTP client wrapper
└── proxy/               # handlers (routing), transform (conversion), streaming (SSE)
```

### Key Dependencies

| Purpose | Crate |
|---------|-------|
| Web framework | axum 0.7, hyper 1.0 |
| HTTP client | reqwest 0.12 |
| Async runtime | tokio 1 |
| Serialization | serde, serde_json, toml |
| CLI | clap 4 |
| TUI | crossterm 0.27, dialoguer 0.11 |
| Database | rusqlite 0.31 (bundled SQLite) |

---

## Comparison with cc-switch

| Feature | cc-gateway | cc-switch |
|---------|-----------|-----------|
| Type | CLI proxy server | Desktop app (Tauri) |
| Platform | macOS, Linux, Windows | macOS, Linux, Windows |
| UI | Terminal TUI | Desktop GUI |
| Proxy mode | Built-in (always on) | Optional local proxy |
| Multi-provider routing | Per-session, per-model, per-project | System tray switching |
| API format conversion | Automatic (Anthropic ↔ OpenAI) | Automatic |
| Session persistence | Project-provider mapping | Per-provider configs |
| Usage tracking | Built-in (SQLite) | Built-in |
| Import from each other | Import from cc-switch ✓ | — |
| Resource usage | ~8MB binary, minimal RAM | Desktop app (~100MB+) |
| Best for | Terminal users, automation, CI/CD | Desktop users, visual management |

---

## Changelog

### v0.4.0 — 2026-05-16

**Major TUI rewrite, stability & performance fixes**

**New: Balance/Quota Query**
- Provider balance query for DeepSeek, SiliconFlow, OpenRouter, StepFun, Novita AI, Zhipu
- Shared HTTP client with system proxy support (matches cc-switch behavior)
- Auto-detects `HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY` environment variables

**Fixed: Stability**
- Session scanning no longer holds mutex during filesystem I/O (eliminates blocking under load)
- DB logging uses minimal lock scope (prevents contention during concurrent requests)
- TUI balance queries run in background threads (no more UI freeze on startup or `[r]` refresh)

**Improved: Log Level Hot-Reload**
- Log level changes via TUI `[g]` or `/api/reload` take effect immediately — no server restart required
- Uses `tracing_subscriber::reload` for dynamic filter updates

**TUI**
- Full crossterm raw mode dashboard with arrow-key navigation
- Form editor: all fields visible at once, no line-by-line confirmation
- Custom selects rendered at screen position (action menu near selected provider)
- i18n: English default, Simplified Chinese via `[e]` toggle
- Raw mode safety: AtomicBool state tracker + scopeguard on all error paths
- Text input: crossterm-based with ESC, Backspace, Delete, cursor movement
- Usage stats: direct 7-day view with `←` `→` to switch ranges (7/30/90/all)
- Request logs: direct 50-entry view with `←` `→` to switch (20/50/100)
- `debug!` logging for request headers/body (no longer at `info!` level)
- Project management: arrow-navigate, set/reset provider mapping
- Vertical centering: all pages center content based on terminal size
- Fixed: test connections nested tokio runtime panic
- Fixed: ANSI escape codes breaking column alignment in raw mode
- Fixed: diagonal/garbled terminal output (`\r\n` fix)

### v0.3.0

- Preset system with 20+ provider presets
- Import from cc-switch database
- Shell completion generation
- Daemon management (start/stop/restart)
- Usage tracking with SQLite

---

## Sponsor

If you find cc-gateway useful, consider buying me a coffee! ❤️

<div align="center">

<img src="assets/sponsor-alipay.jpg" width="200" alt="Alipay" />

</div>

---

## License

MIT © KeaneFeng
