# cc-gateway

Multi-provider aggregation gateway for Claude Code. Rust CLI proxy (axum) on port 16789 that routes Anthropic Messages API requests to configurable AI providers with automatic format conversion.

## Quick Reference

- **Config**: `~/.cc-gateway/config.toml`
- **Database**: `~/.cc-gateway/cc-gateway.db` (SQLite, WAL mode, cc-switch-compatible schema)
- **Default port**: 16789
- **Binary**: `target/release/cc-gateway`

## Project Structure

```
src/
  main.rs              CLI entrypoint (clap), axum server, settings.json auto-update
  interactive.rs       TUI dashboard (dialoguer + console)
  mouse_tui.rs         Alternative mouse TUI (crossterm, WIP)
  commands/            CLI subcommands (serve/start/stop/restart, config, presets, status, test, usage)
  config/              AppConfig, ProviderConfig, ApiFormat enum, presets
  database/            SQLite via rusqlite, cc-switch-compatible schema
  error/               ProxyError with HTTP status mapping
  provider/            reqwest HTTP client wrapper
  proxy/               handlers (routing), transform (format conversion), streaming (SSE)
homebrew/              Homebrew formula
```

## Architecture

### Core Data Flow
```
Claude Code -> POST /v1/messages -> handle_messages()
  -> Route selection (session > model > default)
  -> Format conversion (Anthropic or OpenAiChat)
  -> reqwest forward to upstream provider
  -> Response conversion back to Anthropic format
  -> Stream to Claude Code
```

### SharedState Pattern
`AppState = Arc<Mutex<SharedState>>` holds config + providers + session_router + current_provider_id in one lock. Lock briefly, never hold across await.

### Routing Priority (3-tier, order matters)
1. **Session-based project routing** (highest) - x-claude-code-session-id header -> SessionRouter -> project_providers mapping
2. **Model-based routing** - `claude-{id}` format in request model -> match provider id
3. **Current provider** (fallback) - runtime `current_provider_id` (set from `is_default` at startup, switchable via `/api/switch-provider`)

### API Format Conversion
- `ApiFormat::Anthropic` - direct passthrough
- `ApiFormat::OpenAiChat` - Anthropic <-> OpenAI bidirectional conversion (transform.rs + streaming.rs)
- `ApiFormat::OpenAiResponses`, `GeminiNative` - defined but not yet implemented

## Development Commands

```bash
# Build
cargo build --release

# Run
cargo run -- start -d       # Start server (background)
cargo run -- stop           # Stop server
cargo run -- restart -d     # Restart server
cargo run -- serve          # Start proxy server (low-level, used internally)
cargo run --                # Interactive TUI dashboard
cargo run -- test           # Test provider connections
cargo run -- status         # Provider status table
cargo run -- usage          # Usage statistics

# Check
cargo check                 # Type check (fast)
cargo clippy                # Lint
```

## Critical Rules

### MUST follow
- **Routing priority order**: session > model > default. Never change this order.
- **Session ID**: from `x-claude-code-session-id` header, NOT from request body metadata.
- **Mutex discipline**: Lock briefly, never hold `Arc<Mutex<>>` across `.await`. Use `&self` not `&mut self`.
- **HTTP/1.1 only**: Force on reqwest client. HTTP/2 breaks streaming with some providers (Volcengine, etc.).
- **TUI**: Use dialoguer + console (no crossterm mouse). ESC via `interact_opt()`. After-action-menu pattern.
- **Never write** project `.claude/settings.json` for routing.
- **effort normalization**: Normalize unsupported values (xhigh -> max) before forwarding.
- **Header forwarding**: Keep all original headers, only replace host + auth, force `accept-encoding: identity`.
- **SSE streaming**: Raw byte forwarding. Never wrap with `axum::sse::Event` (double-wrap bug).

### MUST NOT do
- Don't match `config.model` during model-based routing (only match `claude-{id}` format)
- Don't hold mutex locks across async operations
- Don't use stdin().read_line() in TUI
- Don't use axum SSE Event wrapper for streaming (causes double-wrap)
- Don't use HTTP/2 for upstream connections

## Key Dependencies

| Purpose | Crate |
|---------|-------|
| Web framework | axum 0.7, hyper 1.0 |
| Async runtime | tokio 1 (full) |
| HTTP client | reqwest 0.12 (json, stream, blocking) |
| Serialization | serde, serde_json, toml |
| CLI | clap 4 (derive) |
| TUI | dialoguer 0.11, console 0.15 |
| Database | rusqlite 0.31 (bundled) |
| TUI (mouse) | crossterm 0.27 (WIP, not main TUI) |
| Shell completions | clap_complete 4 |
| Error handling | anyhow, thiserror |

## Skills

- **code-map** (Step 1) - Module navigation index, read before any development task
- **cc-gateway-dev** (Step 2) - Main development skill with architecture reference, pitfalls, coding patterns
- **rust-best-practices** - Idiomatic Rust patterns, Clippy discipline, performance optimization
- **rust** - Common Rust compile errors and fixes (ownership, borrowing, concurrency)
- **rust-code-review** - Structured Rust code review with severity calibration
