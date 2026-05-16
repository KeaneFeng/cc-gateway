# cc-gateway

<div align="center">

**轻量级多 Provider 聚合代理，专为 Claude Code 设计** — 使用 Rust 编写

</div>

---

将 Claude Code 的请求路由到任意 AI Provider —— MiMo、Kimi、通义千问、DeepSeek、GLM、OpenRouter 等。不同终端可以使用不同 Provider，通过 `/model` 切换，或使用内置的交互式 TUI 仪表盘管理。

<div align="center">

[`v0.4.0`](#) &nbsp;|&nbsp; macOS / Linux / Windows &nbsp;|&nbsp; Built with Rust &nbsp;|&nbsp; ~8MB 二进制

</div>

<div align="center">

[English](README.md) | 中文

</div>

---

## 为什么选择 cc-gateway？

Claude Code 一次只能使用一个 API 端点。如果你想在不同的终端使用不同的 Provider，或者在不编辑配置文件的情况下切换它们，你需要一个代理。

**cc-gateway** 就是这个代理。它位于 Claude Code 和你的 Provider 之间，负责处理：

- **多 Provider 路由** — 不同终端/项目同时使用不同 Provider
- **格式转换** — 自动双向 Anthropic ↔ OpenAI API 转换
- **会话持久化** — 记住每个项目使用哪个 Provider
- **交互式 TUI** — 完整的仪表盘，用于添加、编辑、测试和管理 Provider
- **使用统计** — 请求日志、Token 统计、费用和延迟数据
- **一键设置** — `cc-gateway start` 自动配置 Claude Code 的 settings.json

与桌面应用（cc-switch 等）不同，cc-gateway 是一个**纯 CLI 工具** — 零 GUI 依赖，在任意终端运行，轻量快速。

---

## 安装

### Homebrew (macOS / Linux)

```bash
brew tap KeaneFeng/cc-gateway https://github.com/KeaneFeng/cc-gateway
brew install cc-gateway
```

### Cargo

```bash
cargo install cc-gateway
```

> **注意**：需要 Rust 1.75+（用于内置 SQLite）。`cargo install` 会从 crates.io 编译源码。

### 从源码构建

```bash
git clone https://github.com/KeaneFeng/cc-gateway.git
cd cc-gateway
cargo build --release
# 二进制文件: target/release/cc-gateway
```

### 直接下载二进制文件

从 [Releases](https://github.com/KeaneFeng/cc-gateway/releases) 下载预编译二进制文件：

| 平台 | 二进制 |
|------|--------|
| macOS (Apple Silicon) | `cc-gateway-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | 从源码构建 |
| Linux (x86_64) | 从源码构建 |
| Linux (ARM64) | 从源码构建 |

---

## 快速开始

```bash
# 1. 启动交互式仪表盘（默认命令）
cc-gateway
# → 按 [a] 添加 Provider，从 20+ 预设中选择
# → 按 [d] 设为默认

# 或者用命令行添加:
cc-gateway add mimo
cc-gateway add kimi

# 2. 启动代理服务器
cc-gateway start -d

# 3. 使用 Claude Code（自动配置 settings.json）
claude
# 在 Claude Code 中: /model → 选择 claude-mimo, claude-kimi 等
```

多个终端可以同时使用不同的 Provider:

```
终端 1: claude → /model → claude-mimo     → 小米 MiMo 2.5 Pro
终端 2: claude → /model → claude-kimi     → 月之暗面 Kimi K2.6
终端 3: claude → /model → claude-deepseek → DeepSeek V4 Pro
```

---

## 交互式 TUI 仪表盘

运行 `cc-gateway`（不带参数）会打开完整的交互式仪表盘：

```
  ╭─────────────────────────── cc-gateway v0.4.0 ───────────────────────────╮
  │        Multi-provider gateway for Claude Code                           │
  ╰─────────────────────────────────────────────────────────────────────────╯

  #   Name               Model            Vision Model  Status
  ──────────────────────────────────────────────────────────────────────────
  1   ▸ Mimo 2.5 Pro     mimo-v2.5-pro    mimo-v2.5     ● Active
  2     Kimi K2.6        kimi-k2.6                      ● Default
  3     Qwen Max         qwen-max                       ● Available

  Proxy: Kimi K2.6 (kimi-k2.6)  |  Provider: 3 | Routes: 0

  ─────────────────────────────────────────────────
  [e] 简体中文  [a] 添加  [d] 默认  [p] 项目  [u] 统计
  [l] 日志  [t] 测试  [b] 预设  [g] 日志级别  [i] 导入
```

### 快捷键

| 按键 | 操作 |
|------|------|
| `↑` `↓` | 浏览 Provider 列表 |
| `空格` / `回车` | 操作菜单（设为默认 / 编辑 / 详情 / 测试 / 复制 / 删除） |
| `a` | 添加新 Provider |
| `d` | 设为默认 Provider |
| `p` | 项目管理（映射项目到 Provider） |
| `u` | 使用统计（7/30/90天/全部） |
| `l` | 请求日志（20/50/100 条） |
| `t` | 测试连接 |
| `b` | 浏览预设 |
| `g` | 切换日志级别 (info / debug) — 立即生效，无需重启 |
| `r` | 刷新选中 Provider 的余额 |
| `i` | 从 cc-switch 导入 |
| `e` | 切换语言 (English / 中文) |
| `Esc` / `Ctrl+C` | 退出 |

---

## 命令

### 服务管理

```bash
cc-gateway                  # 交互式仪表盘（默认）
cc-gateway start            # 启动服务（前台）
cc-gateway start -d         # 启动服务（后台守护进程）
cc-gateway start -f         # 强制启动（先停止已有实例）
cc-gateway stop             # 停止服务
cc-gateway restart          # 重启（前台）
cc-gateway restart -d       # 重启（后台）
```

### Provider 管理

```bash
cc-gateway add              # 添加 Provider（交互式预设列表）
cc-gateway add mimo         # 从预设添加
cc-gateway edit [id]        # 编辑 Provider
cc-gateway remove [id]      # 删除 Provider
cc-gateway default [id]     # 设为默认 Provider
```

### 诊断

```bash
cc-gateway test             # 测试所有 Provider 连接
cc-gateway test mimo        # 测试指定 Provider
cc-gateway status           # 显示 Provider 状态表
cc-gateway usage            # 使用统计（CLI）
```

### 配置

```bash
cc-gateway presets          # 浏览所有可用预设
cc-gateway config           # 显示当前配置
cc-gateway config --set port --value 8080
cc-gateway import           # 从 cc-switch 数据库导入 Provider
```

### Shell 补全

```bash
# Bash
cc-gateway completion bash > ~/.bash_completion.d/cc-gateway

# Zsh
cc-gateway completion zsh > ~/.zfunc/_cc-gateway

# Fish
cc-gateway completion fish > ~/.config/fish/completions/cc-gateway.fish
```

---

## 可用预设

| 分类 | 预设 |
|------|------|
| **Official** | `claude-official`（Anthropic 官方直连） |
| **Chinese Official** | `deepseek` `zhipu` `kimi` `kimi-coding` `bailian` `bailian-coding` `stepfun` `minimax` `doubao` `baidu-qianfan` `longcat` |
| **Aggregator** | `siliconflow` `aihubmix` `dmxapi` `modelscope` |
| **Third Party** | `openrouter` `together` `fireworks` |
| **Local** | `ollama` `lmstudio` |

---

## 工作原理

```
Claude Code  ──POST /v1/messages──▶  cc-gateway (端口 16789)
                                        │
                                  路由选择（三级优先）:
                                    1. 会话路由 (x-claude-code-session-id)
                                    2. 模型路由 (claude-{id} 格式)
                                    3. 默认 Provider (兜底)
                                        │
                                  格式转换
                                    Anthropic ↔ OpenAI (自动)
                                        │
                                  转发到上游 Provider
                                        │
                                  流式响应返回给 Claude Code
```

### 路由优先级

1. **会话路由**（最高） — `x-claude-code-session-id` 请求头 → 项目-Provider 映射。不同项目自动使用各自的 Provider。
2. **模型路由** — Claude Code 的 `/model` 发送 `claude-{id}` 格式的模型字段 → 匹配到 Provider ID。
3. **默认兜底** — 使用当前活跃的 Provider。

### 核心特性

- **格式转换**: Anthropic ↔ OpenAI 双向自动转换（请求 + 响应 + 流式）
- **SSE 流式**: 原始字节转发，无双重包装
- **会话持久化**: 项目-Provider 映射重启后保留
- **自动配置**: `cc-gateway start` 自动更新 `~/.claude/settings.json`
- **HTTP/1.1 强制**: 避免某些 Provider 的流式兼容问题
- **Effort 归一化**: 处理 `xhigh` → `max` 等不支持值的映射
- **国际化**: 英文（默认）和简体中文，按 `[e]` 切换
- **cc-switch 导入**: 一键迁移 cc-switch 的 Provider 配置

---

## 配置文件

位置: `~/.cc-gateway/config.toml`

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

### API 格式

| 格式 | 说明 |
|------|------|
| `Anthropic` | 直接透传（无需转换） |
| `OpenAiChat` | 自动 Anthropic ↔ OpenAI 转换 |
| `OpenAiResponses` | 预留（尚未实现） |
| `GeminiNative` | 预留（尚未实现） |

### 项目-Provider 映射

通过 TUI 的 `[p]` 键，将特定项目映射到特定 Provider：

```toml
[project_providers]
"/Users/you/www/project-a" = "mimo"
"/Users/you/www/project-b" = "kimi"
```

---

## 数据文件

| 路径 | 用途 |
|------|------|
| `~/.cc-gateway/config.toml` | Provider 配置 + 项目映射 |
| `~/.cc-gateway/cc-gateway.db` | SQLite 数据库（使用统计、请求日志，兼容 cc-switch） |
| `~/.cc-gateway/cc-gateway.pid` | PID 文件（守护进程管理） |
| `~/.cc-gateway/cc-gateway.log` | 守护进程日志 |

---

## 架构

```
src/
  main.rs              CLI 入口 (clap)、axum 服务器、settings.json 自动更新
  interactive.rs       TUI 仪表盘 (crossterm raw mode + dialoguer confirm)
  mouse_tui.rs         备选鼠标 TUI (WIP)
  balance.rs           Provider 余额/配额查询 (DeepSeek、SiliconFlow、OpenRouter 等)
  commands/            CLI 子命令 (serve/start/stop/restart, config, presets, status, test, usage)
  config/              AppConfig、ProviderConfig、ApiFormat 枚举、20+ Provider 预设
  database/            SQLite via rusqlite，兼容 cc-switch 的数据库结构
  error/               ProxyError 错误类型及 HTTP 状态码映射
  provider/            reqwest HTTP 客户端封装
  proxy/               handlers (路由)、transform (格式转换)、streaming (SSE)
```

### 主要依赖

| 用途 | Crate |
|------|-------|
| Web 框架 | axum 0.7, hyper 1.0 |
| HTTP 客户端 | reqwest 0.12 |
| 异步运行时 | tokio 1 |
| 序列化 | serde, serde_json, toml |
| CLI | clap 4 |
| TUI | crossterm 0.27, dialoguer 0.11 |
| 数据库 | rusqlite 0.31 (内置 SQLite) |

---

## 与 cc-switch 对比

| 特性 | cc-gateway | cc-switch |
|------|-----------|-----------|
| 类型 | CLI 代理服务器 | 桌面应用 (Tauri) |
| 平台 | macOS, Linux, Windows | macOS, Linux, Windows |
| UI | 终端 TUI | 桌面 GUI |
| 代理模式 | 内置（始终运行） | 可选本地代理 |
| 多 Provider 路由 | 按会话、按模型、按项目 | 系统托盘切换 |
| API 格式转换 | 自动 (Anthropic ↔ OpenAI) | 自动 |
| 会话持久化 | 项目-Provider 映射 | 按 Provider 配置 |
| 使用统计 | 内置 (SQLite) | 内置 |
| 互相导入 | 从 cc-switch 导入 ✓ | — |
| 资源占用 | ~8MB 二进制，极低内存 | 桌面应用 (~100MB+) |
| 适用场景 | 终端用户、自动化、CI/CD | 桌面用户、可视化管理 |

---

## Changelog

### v0.4.0 — 2026-05-16

**重大 TUI 重构、稳定性与性能修复**

**新增：余额/配额查询**
- 支持 DeepSeek、SiliconFlow、OpenRouter、StepFun、Novita AI、智谱的余额查询
- 共享 HTTP 客户端，支持系统代理（与 cc-switch 行为一致）
- 自动检测 `HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY` 环境变量

**修复：稳定性**
- 会话扫描不再在文件系统 I/O 期间持有互斥锁（消除高负载下的阻塞）
- 数据库日志使用最小锁范围（防止并发请求时的锁竞争）
- TUI 余额查询在后台线程运行（启动和按 `[r]` 刷新时不再卡顿）

**改进：日志级别热更新**
- 通过 TUI `[g]` 或 `/api/reload` 切换日志级别后立即生效，无需重启服务器
- 使用 `tracing_subscriber::reload` 实现动态过滤器更新

**TUI**
- 完整的 crossterm raw mode 仪表盘，支持方向键导航
- 表单编辑器：所有字段一次显示，无需逐行确认
- 自定义选择支持屏幕位置渲染（操作菜单在选中 Provider 附近显示）
- 国际化：英文默认，按 `[e]` 切换简体中文
- 原始模式安全：AtomicBool 状态跟踪器 + 所有错误路径的 scopeguard
- 文本输入：基于 crossterm，支持 ESC、退格、删除、光标移动
- 使用统计：直接显示 7 天数据，`←` `→` 切换范围（7/30/90/全部）
- 请求日志：直接显示 50 条记录，`←` `→` 切换数量（20/50/100）
- `debug!` 日志用于请求头/请求体（不再显示在 `info!` 级别）
- 项目管理：方向键导航，设置/重置 Provider 映射
- 垂直居中：所有页面根据终端大小居中内容
- 修复：测试连接时 tokio 嵌套运行时崩溃
- 修复：ANSI 转义码破坏原始模式列对齐
- 修复：终端斜线/乱码输出（`\r\n` 修复）

### v0.3.0

- 预设系统，20+ Provider 预设
- 从 cc-switch 数据库导入
- Shell 补全生成
- 守护进程管理（启动/停止/重启）
- SQLite 使用统计

---

## 赞助 ❤️

如果你觉得 cc-gateway 对你有帮助，欢迎赞助支持！

<div align="center">

<img src="assets/sponsor-alipay.jpg" width="200" alt="Alipay" />

</div>

---

## License

MIT © KeaneFeng
