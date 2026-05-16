//! Interactive TUI Dashboard
//!
//! cc-gateway TUI - crossterm-based interactive management
//! Arrow keys navigate providers, space opens action menu

use crate::config::{presets, ApiFormat, AppConfig, ProviderConfig, ProviderUpdate};
use console::style;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};

use std::io::{stdout, Write};
use std::path::Path;
use std::sync::{mpsc, LazyLock};

/// Simple snowflake-like ID generator (timestamp + monotonic counter)
fn generate_snowflake_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let c = COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFF;
    format!("{:x}{:04x}", ts, c)
}

/// Shared HTTP client for TUI API calls (reload, switch, status).
/// Avoids creating a new reqwest::blocking::Client (and tokio runtime) per call.
static TUI_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .expect("Failed to build TUI HTTP client")
});

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── Visual Constants ───────────────────────────────────────────

const BOX_TL: &str = "╭";
const BOX_TR: &str = "╮";
const BOX_BL: &str = "╰";
const BOX_BR: &str = "╯";
const BOX_H: &str = "─";
const BOX_V: &str = "│";
const BULLET: &str = "●";
const ARROW: &str = "▸";
const DIAMOND: &str = "◆";

struct ShortcutItem {
    key: &'static str,
    name_zh: &'static str,
    name_en: &'static str,
    desc_zh: &'static str,
    desc_en: &'static str,
}

const SHORTCUTS: &[ShortcutItem] = &[
    ShortcutItem {
        key: "e",
        name_zh: "语言",
        name_en: "Language",
        desc_zh: "切换界面语言",
        desc_en: "Toggle UI language",
    },
    ShortcutItem {
        key: "a",
        name_zh: "添加",
        name_en: "Add",
        desc_zh: "添加新提供商",
        desc_en: "Add new provider",
    },
    ShortcutItem {
        key: "d",
        name_zh: "默认",
        name_en: "Default",
        desc_zh: "设为默认提供商",
        desc_en: "Set as default",
    },
    ShortcutItem {
        key: "p",
        name_zh: "项目",
        name_en: "Projects",
        desc_zh: "项目提供商映射",
        desc_en: "Project-provider mapping",
    },
    ShortcutItem {
        key: "u",
        name_zh: "统计",
        name_en: "Stats",
        desc_zh: "使用量统计",
        desc_en: "Usage statistics",
    },
    ShortcutItem {
        key: "l",
        name_zh: "日志",
        name_en: "Logs",
        desc_zh: "请求日志",
        desc_en: "Request logs",
    },
    ShortcutItem {
        key: "t",
        name_zh: "测试",
        name_en: "Test",
        desc_zh: "测试连接状态",
        desc_en: "Test connections",
    },
    ShortcutItem {
        key: "b",
        name_zh: "预设",
        name_en: "Presets",
        desc_zh: "浏览所有预设",
        desc_en: "Browse all presets",
    },
    ShortcutItem {
        key: "r",
        name_zh: "余额",
        name_en: "Balance",
        desc_zh: "查询当前余额",
        desc_en: "Query balance",
    },
    ShortcutItem {
        key: "R",
        name_zh: "刷新",
        name_en: "Refresh",
        desc_zh: "刷新所有余额",
        desc_en: "Refresh all balances",
    },
    ShortcutItem {
        key: "g",
        name_zh: "日志级别",
        name_en: "LogLevel",
        desc_zh: "切换日志级别",
        desc_en: "Toggle log level",
    },
    ShortcutItem {
        key: "i",
        name_zh: "导入",
        name_en: "Import",
        desc_zh: "从cc-switch导入",
        desc_en: "Import from cc-switch",
    },
    ShortcutItem {
        key: ";",
        name_zh: "端口",
        name_en: "Port",
        desc_zh: "修改监听端口",
        desc_en: "Change listen port",
    },
];

// ─── Terminal Output (crossterm-only) ───────────────────────────

fn term_clear() {
    let _ = execute!(
        stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    );
}

fn term_print(line: &str) {
    let out = format!("{}\r\n", line);
    let _ = write!(stdout(), "{}", out);
    let _ = stdout().flush();
}

fn term_print_raw(text: &str) {
    // text already has \n separators, convert to \r\n for raw mode
    let converted = text.replace('\n', "\r\n");
    let _ = write!(stdout(), "{}", converted);
    let _ = stdout().flush();
}

// ─── Key Abstraction ───────────────────────────────────────────

enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Space,
    Esc,
    Char(char),
    Backspace,
    Delete,
    CtrlC,
    Other,
}

fn read_key() -> Key {
    let mut non_key_count = 0;
    loop {
        match event::poll(std::time::Duration::from_millis(200)) {
            Ok(true) => {}                  // Event available, try to read
            Ok(false) => return Key::Other, // Timeout, no event
            Err(_) => return Key::Other,    // Error, bail out
        }
        match event::read() {
            Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) => {
                if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                    return Key::CtrlC;
                }
                drain_event_queue();
                return match code {
                    KeyCode::Up => Key::Up,
                    KeyCode::Down => Key::Down,
                    KeyCode::Left => Key::Left,
                    KeyCode::Right => Key::Right,
                    KeyCode::Enter => Key::Enter,
                    KeyCode::Char(' ') => Key::Space,
                    KeyCode::Esc => Key::Esc,
                    KeyCode::Char(c) => Key::Char(c),
                    KeyCode::Backspace => Key::Backspace,
                    KeyCode::Delete => Key::Delete,
                    _ => Key::Other,
                };
            }
            Ok(_) => {
                // Non-key event (resize, focus, etc.) — skip but guard against infinite loop
                non_key_count += 1;
                if non_key_count > 50 {
                    return Key::Other;
                }
                continue;
            }
            Err(_) => return Key::Other,
        }
    }
}

/// Drain all queued events without blocking to prevent rapid-fire input
/// (e.g. mouse scroll wheel generating multiple Up/Down key events).
fn drain_event_queue() {
    while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
        let _ = event::read();
    }
}

/// Non-blocking key poll with timeout (millis). Returns None on timeout.
#[allow(dead_code)]
fn poll_key(timeout_ms: u64) -> Option<Key> {
    if event::poll(std::time::Duration::from_millis(timeout_ms)).ok()? {
        if let Ok(Event::Key(KeyEvent {
            code, modifiers, ..
        })) = event::read()
        {
            if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                return Some(Key::CtrlC);
            }
            return Some(match code {
                KeyCode::Up => Key::Up,
                KeyCode::Down => Key::Down,
                KeyCode::Left => Key::Left,
                KeyCode::Right => Key::Right,
                KeyCode::Enter => Key::Enter,
                KeyCode::Char(' ') => Key::Space,
                KeyCode::Esc => Key::Esc,
                KeyCode::Char(c) => Key::Char(c),
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Delete => Key::Delete,
                _ => Key::Other,
            });
        }
    }
    None
}

// ─── Raw Mode Control ──────────────────────────────────────────

use std::sync::atomic::{AtomicBool, Ordering};
static RAW_MODE: AtomicBool = AtomicBool::new(false);

fn enable_raw() {
    if !RAW_MODE.load(Ordering::Relaxed) {
        let _ = execute!(
            stdout(),
            terminal::EnterAlternateScreen,
            crossterm::event::DisableMouseCapture
        );
        let _ = terminal::enable_raw_mode();
        let _ = execute!(stdout(), cursor::Hide);
        RAW_MODE.store(true, Ordering::Relaxed);
    }
}

fn disable_raw() {
    if RAW_MODE.load(Ordering::Relaxed) {
        let _ = execute!(stdout(), cursor::Show);
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        RAW_MODE.store(false, Ordering::Relaxed);
    }
}

/// Safe exit: restores terminal state before exiting.
/// Use instead of `std::process::exit()` everywhere.
fn safe_exit() -> ! {
    disable_raw();
    std::process::exit(0)
}

// ─── Visual Helpers ─────────────────────────────────────────────

fn reload_proxy_config(port: u16) {
    let _ = TUI_CLIENT
        .post(format!("http://127.0.0.1:{}/api/reload", port))
        .send();
}

fn get_current_provider_id(port: u16) -> Option<String> {
    let resp = TUI_CLIENT
        .get(format!("http://127.0.0.1:{}/api/current-provider", port))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .ok()?;
    let json: serde_json::Value = resp.json().ok()?;
    json.get("provider_id")?.as_str().map(|s| s.to_string())
}

fn provider_name(p: &ProviderConfig) -> &str {
    p.display_name.as_deref().unwrap_or(&p.name)
}

fn format_balance_result(result: &crate::balance::BalanceResult) -> String {
    let mut parts = Vec::new();
    for d in &result.data {
        // If plan_name already contains formatted info (e.g. Zhipu: "5小时: 100% 已用"),
        // show it directly with validity marker
        if d.plan_name.contains('%') {
            let valid_marker = if d.is_valid { "" } else { " ⚠" };
            parts.push(format!("{}{}", d.plan_name, valid_marker));
            continue;
        }
        // Standard format: currency balance (DeepSeek, SiliconFlow, etc.)
        let remaining = d
            .remaining
            .map(|v| format!("{:.4}", v))
            .unwrap_or_else(|| "-".to_string());
        let label = if d.plan_name.is_empty() {
            String::new()
        } else {
            format!("{} ", d.plan_name)
        };
        // Only add unit if it's not already in the remaining value
        let unit = if d.unit.is_empty() || remaining.contains(&d.unit) {
            String::new()
        } else {
            format!(" {}", d.unit)
        };
        let valid_marker = if d.is_valid { "" } else { " ⚠" };
        parts.push(format!("{}{}{}{}", label, remaining, unit, valid_marker));
    }
    parts.join(" | ")
}

fn format_ts(unix: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(unix, 0)
        .single()
        .map(|dt| dt.format("%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn spinner() -> &'static str {
    "⠋"
}

// ─── i18n ──────────────────────────────────────────────────────

static mut LANG: Lang = Lang::En;

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    En,
    Zh,
}

fn t(zh: &'static str, en: &'static str) -> &'static str {
    unsafe {
        if LANG == Lang::Zh {
            zh
        } else {
            en
        }
    }
}

/// Get current language for use by other modules
#[allow(dead_code)]
pub fn get_lang() -> &'static str {
    unsafe {
        if LANG == Lang::Zh {
            "zh"
        } else {
            "en"
        }
    }
}

// ─── Custom Select (crossterm, left-aligned) ────────────────────

fn custom_select(prompt: &str, items: &[String], default: usize) -> Option<usize> {
    custom_select_at(prompt, items, default, 0)
}

fn custom_select_at(prompt: &str, items: &[String], default: usize, at_row: u16) -> Option<usize> {
    let mut selected = default.min(items.len().saturating_sub(1));

    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("  {}", style(prompt).bold()));
        for (i, item) in items.iter().enumerate() {
            let marker = if i == selected {
                format!("{} ", style(ARROW).cyan().bold())
            } else {
                "  ".to_string()
            };
            let text = if i == selected {
                style(item).cyan().bold().to_string()
            } else {
                item.to_string()
            };
            lines.push(format!("  {}{}", marker, text));
        }
        lines.push(format!(
            "  {} ↑↓ {}  Enter {}  ESC {}",
            style("·").dim(),
            t("选择", "select"),
            t("确认", "confirm"),
            t("返回", "back"),
        ));

        let content_height = lines.len();
        let top_pad = if at_row == 0 && term_h > content_height + 2 {
            (term_h - content_height) / 3
        } else {
            0
        };

        if at_row == 0 {
            term_clear();
            for _ in 0..top_pad {
                term_print("");
            }
            for line in &lines {
                term_print(line);
            }
        } else {
            let _ = execute!(stdout(), cursor::MoveTo(0, at_row));
            for i in 0..=(lines.len() as u16 + 1) {
                let _ = execute!(
                    stdout(),
                    cursor::MoveTo(0, at_row + i),
                    terminal::Clear(ClearType::CurrentLine)
                );
            }
            let _ = execute!(stdout(), cursor::MoveTo(0, at_row));
            term_print_raw(&format!("{}\n", lines.join("\r\n")));
        }

        match read_key() {
            Key::Up => {
                if selected > 0 {
                    selected -= 1;
                } else {
                    selected = items.len() - 1;
                }
            }
            Key::Down => {
                selected = (selected + 1) % items.len();
            }
            Key::Enter => {
                return Some(selected);
            }
            Key::Esc | Key::CtrlC => return None,
            _ => {}
        }
    }
}

// ─── Dialoguer Wrappers ─────────────────────────────────────────

fn input_with_esc(prompt: &str, default: Option<&str>) -> Option<String> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut buffer = default.unwrap_or("").to_string();
    let mut cursor_pos = buffer.len();

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let top_pad = if term_h > 8 { (term_h - 6) / 3 } else { 0 };

        // Build display with visible cursor
        let before = &buffer[..cursor_pos];
        let after = &buffer[cursor_pos..];
        let cursor_char = "█";
        let display = format!("> {}{}{}", before, style(cursor_char).on_blue(), after);

        term_clear();
        for _ in 0..top_pad { term_print(""); }
        term_print(&format!("  {}", style(prompt).bold()));
        term_print(&format!("  {}", display));
        term_print("");
        term_print(&format!("  {} {}  {}  ESC {}",
            style(t("提示:", "Hint:")).dim(),
            t("← → 移动光标", "← → move cursor"),
            t("Enter 确认", "Enter confirm"),
            t("取消", "cancel"),
        ));

        match read_key() {
            Key::Enter => {
                let result = buffer.trim().to_string();
                if result.is_empty() && default.is_none() { continue; }
                return Some(result);
            }
            Key::Esc | Key::CtrlC => return None,
            Key::Char(c) => {
                buffer.insert(cursor_pos, c);
                cursor_pos += 1;
            }
            Key::Space => {
                buffer.insert(cursor_pos, ' ');
                cursor_pos += 1;
            }
            Key::Backspace if cursor_pos > 0 => {
                cursor_pos -= 1;
                buffer.remove(cursor_pos);
            }
            Key::Delete if cursor_pos < buffer.len() => {
                buffer.remove(cursor_pos);
            }
            Key::Left => cursor_pos = cursor_pos.saturating_sub(1),
            Key::Right if cursor_pos < buffer.len() => cursor_pos += 1,
            _ => {}
        }
    }
}
fn confirm_with_esc(prompt: &str, default: bool) -> Option<bool> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut choice = default;

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let top_pad = if term_h > 6 { (term_h - 4) / 3 } else { 0 };

        term_clear();
        for _ in 0..top_pad { term_print(""); }
        term_print(&format!("  {}", style(prompt).bold()));
        term_print("");

        let yes_label = if choice {
            format!("[{}]", style("Y").cyan().bold())
        } else {
            format!("[{}]", style("y").dim())
        };
        let no_label = if !choice {
            format!("[{}]", style("N").red().bold())
        } else {
            format!("[{}]", style("n").dim())
        };

        term_print(&format!(
            "  {} {} {}  {}",
            style(t("确认:", "Confirm:")).dim(),
            yes_label,
            no_label,
            style(t("← → 切换, Y/N 直接确认, ESC 取消", "← → toggle, Y/N confirm, ESC cancel")).dim(),
        ));

        match read_key() {
            Key::Left => choice = true,
            Key::Right => choice = false,
            Key::Char('y') | Key::Char('Y') => return Some(true),
            Key::Char('n') | Key::Char('N') => return Some(false),
            Key::Enter | Key::Space => return Some(choice),
            Key::Esc | Key::CtrlC => return None,
            _ => {}
        }
    }
}

fn after_action_menu() -> bool {
    term_print("");
    term_print(&format!(
        "  {} Enter {}  ESC {}",
        style("·").dim(),
        t("返回", "Back"),
        t("退出", "Exit"),
    ));
    // Raw mode is already enabled by all callers; just read a key
    loop {
        match read_key() {
            Key::Enter => return true,
            Key::Esc | Key::CtrlC => return false,
            _ => {}
        }
    }
}

// ─── Dashboard Renderer ─────────────────────────────────────────

fn cjk_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if ('\u{1100}'..='\u{115f}').contains(&c)
                || ('\u{2e80}'..='\u{a4cf}').contains(&c)
                || ('\u{ac00}'..='\u{d7a3}').contains(&c)
                || ('\u{f900}'..='\u{faff}').contains(&c)
                || ('\u{fe30}'..='\u{fe4f}').contains(&c)
                || ('\u{ff01}'..='\u{ff60}').contains(&c)
                || ('\u{ffe0}'..='\u{ffe6}').contains(&c)
                || ('\u{20000}'..='\u{2fffd}').contains(&c)
                || ('\u{30000}'..='\u{3fffd}').contains(&c)
                || ('\u{2f800}'..='\u{2fa1f}').contains(&c)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

fn cjk_truncate(s: &str, max_w: usize) -> String {
    let mut w = 0;
    let mut result = String::new();
    for c in s.chars() {
        let cw = if ('\u{1100}'..='\u{115f}').contains(&c)
            || ('\u{2e80}'..='\u{a4cf}').contains(&c)
            || ('\u{ac00}'..='\u{d7a3}').contains(&c)
            || ('\u{f900}'..='\u{faff}').contains(&c)
            || ('\u{fe30}'..='\u{fe4f}').contains(&c)
            || ('\u{ff01}'..='\u{ff60}').contains(&c)
            || ('\u{ffe0}'..='\u{ffe6}').contains(&c)
            || ('\u{20000}'..='\u{2fffd}').contains(&c)
            || ('\u{30000}'..='\u{3fffd}').contains(&c)
            || ('\u{2f800}'..='\u{2fa1f}').contains(&c)
        {
            2
        } else {
            1
        };
        if w + cw > max_w {
            break;
        }
        w += cw;
        result.push(c);
    }
    result
}

fn pad(s: &str, width: usize) -> String {
    let w = cjk_width(s);
    if w > width {
        format!("{}..", cjk_truncate(s, width.saturating_sub(2)))
    } else if w < width {
        format!("{}{}", s, " ".repeat(width - w))
    } else {
        s.to_string()
    }
}

fn center_content(text: &str) -> String {
    let (_, th) = terminal::size()
        .map(|(w, h)| (w as usize, h as usize))
        .unwrap_or((80, 24));
    let line_count = text.lines().count();
    // Only add top padding if content is significantly shorter than terminal
    let top_pad = if th > line_count + 6 {
        (th - line_count) / 3
    } else {
        0
    };
    let mut out = String::new();
    for _ in 0..top_pad {
        out.push('\n');
    }
    out.push_str(text);
    out
}

fn render_dashboard(
    config: &AppConfig,
    active_id: Option<&str>,
    selected: usize,
    footer: Option<&str>,
    balance_cache: &std::collections::HashMap<String, String>,
    shortcut_idx: Option<usize>,
) -> String {
    let mut out = String::new();

    // Get terminal width
    let tw = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    // Header box - fit to terminal
    let title = format!("cc-gateway v{}", VERSION);
    let box_w = tw.saturating_sub(4).min(80); // 2 chars padding on each side
    let tp = (box_w.saturating_sub(title.len())) / 2;
    let tr = box_w.saturating_sub(title.len() + tp);

    out.push_str(&format!("  {}{}{}\n", BOX_TL, &BOX_H.repeat(box_w), BOX_TR));
    out.push_str(&format!(
        "  {}{}{}{}\n",
        BOX_V,
        " ".repeat(tp),
        style(&title).cyan().bold(),
        " ".repeat(tr)
    ));
    let sub = t(
        "Multi-provider gateway for Claude Code",
        "Multi-provider gateway for Claude Code",
    );
    let sp = box_w.saturating_sub(sub.len() + 2);
    out.push_str(&format!("  {} {}{}\n", BOX_V, sub, " ".repeat(sp)));
    out.push_str(&format!(
        "  {}{}{}\n\n",
        BOX_BL,
        &BOX_H.repeat(box_w),
        BOX_BR
    ));

    // Column widths (total should be < tw - 2 for "  " prefix)
    let c_num = 3; // #
    let c_name = 16; // Name
    let c_model = 16; // Model
    let c_vis = 14; // Vision model
    let c_status = 12; // Status

    // Table header (raw text, no ANSI) - same format as data row
    let header = format!(
        "  {} {} {} {} {} {}",
        pad("#", c_num),
        " ",
        pad(t("名称", "Name"), c_name),
        pad(t("模型", "Model"), c_model),
        pad(t("图像模型", "Vision"), c_vis),
        pad(t("状态", "Status"), c_status),
    );
    out.push_str(&format!("{}\n", style(&header).dim()));
    out.push_str(&format!(
        "  {}\n",
        &"─".repeat(tw.saturating_sub(4).min(80))
    ));

    if config.providers.is_empty() {
        out.push_str(&format!(
            "  ○ {}\n",
            t(
                "没有配置 Provider，按 [a] 添加",
                "No providers configured. Press [a] to add"
            )
        ));
    } else {
        for (i, p) in config.providers.iter().enumerate() {
            let is_active = active_id.map(|id| id == p.id).unwrap_or(false);
            let is_sel = i == selected;
            let name = provider_name(p);
            let vision = p.vision_model.as_deref().unwrap_or("-");

            // Raw text with manual padding
            let marker = if is_sel { "▸" } else { " " };
            let raw_name = pad(name, c_name);
            let raw_model = pad(&p.model, c_model);
            let raw_vis = pad(vision, c_vis);

            let raw_status = if is_active {
                t("● 当前", "● Active")
            } else if p.is_default {
                t("● 默认", "● Default")
            } else {
                t("● 可用", "● Available")
            };
            let raw_status_padded = pad(raw_status, c_status);

            let num_str = format!("{} ", i + 1);
            let num_padded = pad(&num_str, c_num);

            // Style parts
            let styled_num = format!("{}", style(&num_padded).dim());
            let styled_name = if is_sel {
                format!("{}", style(&raw_name).cyan().bold())
            } else {
                format!("{}", style(&raw_name).green())
            };
            let styled_vis = if vision == "-" {
                format!("{}", style(&raw_vis).dim())
            } else {
                format!("{}", style(&raw_vis).cyan())
            };
            let styled_status = if is_active {
                format!("{}", style(&raw_status_padded).green().bold())
            } else if p.is_default {
                format!("{}", style(&raw_status_padded).yellow())
            } else {
                format!("{}", style(&raw_status_padded).dim())
            };

            out.push_str(&format!(
                "  {} {} {} {} {} {}\n",
                styled_num, marker, styled_name, raw_model, styled_vis, styled_status,
            ));

            // Balance on second line (dim, constrained to name column width for visual hierarchy)
            if let Some(balance_str) = balance_cache.get(&p.id) {
                // Balance aligns to name column start, extends through vision column
                let bal_indent = 2 + c_num + 1 + 1; // "  " + num_padded + " " + marker
                let max_w = c_name + 1 + c_model + 1 + c_vis; // through vision column
                // Strip ANSI for width measurement
                let mut plain = String::new();
                let mut in_escape = false;
                for c in balance_str.chars() {
                    if c == '\u{1b}' {
                        in_escape = true;
                    } else if in_escape {
                        if c.is_alphabetic() {
                            in_escape = false;
                        }
                    } else {
                        plain.push(c);
                    }
                }
                let words: Vec<&str> = plain.split_whitespace().collect();
                let mut wrapped: Vec<String> = Vec::new();
                let mut cur = String::new();
                for word in words {
                    let cur_w = console::measure_text_width(&cur);
                    let word_w = console::measure_text_width(word);
                    let sep_w = if cur.is_empty() { 0 } else { 1 };
                    if cur_w + sep_w + word_w <= max_w {
                        if cur.is_empty() {
                            cur = word.to_string();
                        } else {
                            cur.push(' ');
                            cur.push_str(word);
                        }
                    } else {
                        if !cur.is_empty() {
                            wrapped.push(cur);
                        }
                        cur = word.to_string();
                    }
                }
                if !cur.is_empty() {
                    wrapped.push(cur);
                }
                let indent_str = " ".repeat(bal_indent);
                for line in &wrapped {
                    out.push_str(&format!("{}{}\n", indent_str, style(line).dim()));
                }
            }
        }
    }

    out.push('\n');

    // Status
    if let Some(id) = active_id {
        if let Some(p) = config.get_provider_by_id(id) {
            out.push_str(&format!(
                "  {} {} ({})\n",
                t("代理:", "Proxy:"),
                style(provider_name(p)).cyan().bold(),
                style(&p.model).dim(),
            ));
        }
    }
    out.push_str(&format!(
        "  {} {} | {} {}\n",
        t("提供商:", "Provider:"),
        config.providers.len(),
        t("线路:", "Routes:"),
        config.project_providers.len(),
    ));
    let lang_name = if unsafe { LANG == Lang::Zh } {
        t("中文", "Chinese")
    } else {
        t("英文", "English")
    };
    let log_level = config.log_level.to_uppercase();
    out.push_str(&format!(
        "  {}: {} | {}: {} | {}: {}\n",
        t("端口", "Port"),
        style(format!("{}", config.port)).cyan(),
        t("语言", "Language"),
        style(lang_name).cyan(),
        t("日志级别", "LogLevel"),
        style(&log_level).cyan(),
    ));

    // Footer with all shortcuts (navigable grid, no visual selection)
    if let Some(f) = footer {
        out.push_str(&format!("  {}\n\n", style(f).dim()));
    }

    out.push_str("  ─────────────────────────────────────────────────\n");

    let cols = 4;
    for row in 0..SHORTCUTS.len().div_ceil(cols) {
        let mut line = String::from("  ");
        for col in 0..cols {
            let idx = row * cols + col;
            if idx < SHORTCUTS.len() {
                let s = &SHORTCUTS[idx];
                let name = t(s.name_zh, s.name_en);
                let key_display = format!("[{}]", s.key);
                let cell = format!("{} {}", style(&key_display).dim(), name);
                let cell_w = console::measure_text_width(&format!("[{}] {}", s.key, name));
                let target_w: usize = 18;
                let padding = if col < cols - 1 {
                    target_w.saturating_sub(cell_w)
                } else {
                    0
                };
                line.push_str(&format!("{}{}", cell, " ".repeat(padding)));
            }
        }
        out.push_str(&format!("{}\n", line));
    }

    // Status bar - show which shortcut is selected via arrow navigation
    if let Some(idx) = shortcut_idx {
        if let Some(s) = SHORTCUTS.get(idx) {
            let desc = t(s.desc_zh, s.desc_en);
            let key_label = format!("[{}]", s.key);
            let name = t(s.name_zh, s.name_en);
            out.push_str(&format!(
                "  {} {} {} — {} ({})\n",
                style("▸").cyan(),
                style(&key_label).cyan().bold(),
                style(name).cyan().bold(),
                desc,
                style(t("回车执行", "Enter to execute")).dim(),
            ));
        }
    }

    center_content(&out)
}

// ─── Provider Action Menu ───────────────────────────────────────

fn show_provider_action_menu(
    config_path: &Path,
    provider_idx: usize,
) -> anyhow::Result<Option<bool>> {
    let config = AppConfig::load(config_path)?;
    let p = &config.providers[provider_idx];
    let name = provider_name(p).to_string();
    let pid = p.id.clone();

    let has_balance = crate::balance::is_balance_supported(&p.base_url);

    let option_labels: Vec<&str> = if has_balance {
        vec![
            t("设为默认", "Set Default"),
            t("编辑", "Edit"),
            t("详情", "Details"),
            t("余额", "Balance"),
            t("测试连接", "Test"),
            t("复制", "Copy"),
            t("删除", "Delete"),
            t("返回", "Back"),
        ]
    } else {
        vec![
            t("设为默认", "Set Default"),
            t("编辑", "Edit"),
            t("详情", "Details"),
            t("测试连接", "Test"),
            t("复制", "Copy"),
            t("删除", "Delete"),
            t("返回", "Back"),
        ]
    };
    let options: Vec<String> = option_labels
        .iter()
        .map(|s| format!("{} {}", ARROW, s))
        .collect();

    let selection = custom_select(
        &format!(
            "{} - {}",
            style(&name).cyan().bold(),
            t("选择操作", "Action")
        ),
        &options,
        0,
    );

    let quit = if has_balance {
        match selection {
            Some(0) => {
                disable_raw();
                let _ = set_default(config_path, Some(&pid), config.port);
                enable_raw();
                false
            }
            Some(1) => {
                disable_raw();
                let _ = edit_provider(config_path, Some(&pid));
                enable_raw();
                false
            }
            Some(2) => {
                // show_provider_detail renders in alternate screen, no disable_raw needed
                let _ = show_provider_detail(config_path, &pid);
                false
            }
            Some(3) => {
                disable_raw();
                show_balance_detail(&config, &pid);
                enable_raw();
                false
            }
            Some(4) => {
                disable_raw();
                let _ = test_connections(config_path);
                enable_raw();
                false
            }
            Some(5) => {
                disable_raw();
                let _ = copy_provider_ui(config_path, &pid);
                enable_raw();
                false
            }
            Some(6) => {
                remove_provider(config_path, Some(&pid))?;
                false
            }
            _ => false,
        }
    } else {
        match selection {
            Some(0) => {
                disable_raw();
                let _ = set_default(config_path, Some(&pid), config.port);
                enable_raw();
                false
            }
            Some(1) => {
                disable_raw();
                let _ = edit_provider(config_path, Some(&pid));
                enable_raw();
                false
            }
            Some(2) => {
                // show_provider_detail renders in alternate screen, no disable_raw needed
                let _ = show_provider_detail(config_path, &pid);
                false
            }
            Some(3) => {
                disable_raw();
                let _ = test_connections(config_path);
                enable_raw();
                false
            }
            Some(4) => {
                disable_raw();
                let _ = copy_provider_ui(config_path, &pid);
                enable_raw();
                false
            }
            Some(5) => {
                remove_provider(config_path, Some(&pid))?;
                false
            }
            _ => false,
        }
    };

    enable_raw();
    Ok(Some(quit))
}

// ─── Top Menu (unused - shortcuts moved to homepage) ─────────────

#[allow(dead_code)]
fn show_top_menu(config_path: &Path) -> anyhow::Result<()> {
    disable_raw();
    term_clear();

    let config = AppConfig::load(config_path).ok();
    let current_level = config
        .as_ref()
        .map(|c| c.log_level.as_str())
        .unwrap_or("info");

    let mut cursor: usize = 0;
    let items = [
        (t("项目管理", "Project Management"), "p"),
        (t("使用统计", "Usage Stats"), "u"),
        (t("请求日志", "Request Logs"), "l"),
        (t("从 cc-switch 导入", "Import from cc-switch"), ""),
        (t("浏览 Presets", "Browse Presets"), ""),
        (t("切换日志级别", "Toggle Log Level"), ""),
        (t("返回", "Back"), "q"),
    ];

    enable_raw();

    loop {
        term_clear();
        term_print(&format!("  {} cc-gateway {}", DIAMOND, t("菜单", "Menu")));
        term_print(&format!(
            "  {}: {}",
            t("当前日志级别", "Current Log Level"),
            style(current_level).cyan()
        ));
        term_print("");

        for (i, (name, hint)) in items.iter().enumerate() {
            let marker = if i == cursor {
                format!("{} ", style(ARROW).cyan().bold())
            } else {
                "  ".to_string()
            };
            let hint_str = if !hint.is_empty() {
                format!(" {}", style(format!("[{}]", hint)).dim())
            } else {
                String::new()
            };
            let text = if i == cursor {
                style(name).cyan().bold().to_string()
            } else {
                name.to_string()
            };
            term_print(&format!("  {}{}{}", marker, text, hint_str));
        }

        term_print("");
        term_print(&format!(
            "  {} {}  {}  {} {}  {} {}",
            t("↑↓", "↑↓"),
            t("选择", "select"),
            t("回车确认", "Enter to confirm"),
            t("快捷键", "shortcuts"),
            "p/u/l",
            t("ESC", "ESC"),
            t("返回", "back")
        ));

        match read_key() {
            Key::Up => {
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = items.len() - 1;
                }
            }
            Key::Down => {
                cursor = (cursor + 1) % items.len();
            }
            Key::Enter => {
                disable_raw();
                match cursor {
                    0 => {
                        manage_projects(config_path)?;
                    }
                    1 => {
                        show_usage(config_path)?;
                    }
                    2 => {
                        show_request_logs(config_path)?;
                    }
                    3 => {
                        import_providers(config_path, None)?;
                    }
                    4 => {
                        browse_presets()?;
                    }
                    5 => {
                        let port = config.as_ref().map(|c| c.port).unwrap_or(16789);
                        toggle_log_level(config_path, port)?;
                    }
                    _ => {}
                }
                enable_raw();
            }
            Key::Char('p') => {
                disable_raw();
                manage_projects(config_path)?;
                enable_raw();
            }
            Key::Char('u') => {
                disable_raw();
                show_usage(config_path)?;
                enable_raw();
            }
            Key::Char('l') => {
                disable_raw();
                show_request_logs(config_path)?;
                enable_raw();
            }
            Key::Esc | Key::CtrlC => break,
            _ => {}
        }
    }

    enable_raw();
    Ok(())
}

fn toggle_log_level(config_path: &Path, port: u16) -> anyhow::Result<()> {
    let mut config = AppConfig::load(config_path)?;

    let new_level = if config.log_level == "debug" {
        "info"
    } else {
        "debug"
    };
    config.log_level = new_level.to_string();
    config.save(config_path)?;

    // Apply log level immediately in this process (if TUI and server share the same process)
    crate::set_log_level(new_level);
    // Also notify the server process via API
    reload_proxy_config(port);

    Ok(())
}

// ─── Main Dashboard Loop ────────────────────────────────────────

pub fn run_dashboard(config_path: &str) -> anyhow::Result<()> {
    let path = Path::new(config_path);
    let mut selected: usize = 0;
    let mut shortcut_idx: Option<usize> = None; // None = focus on provider table
    let mut footer_msg: Option<String> = None;
    let mut balance_cache = std::collections::HashMap::new();

    // Launch initial balance queries in a background thread to avoid blocking the TUI
    let (bal_tx, bal_rx) = mpsc::channel::<(String, String)>();
    {
        let init_config = if path.exists() {
            AppConfig::load(path).unwrap_or_default()
        } else {
            AppConfig::default()
        };
        let init_tx = bal_tx.clone();
        std::thread::spawn(move || {
            for p in &init_config.providers {
                if crate::balance::is_balance_supported(&p.base_url) {
                    let display = match crate::balance::query_balance(&p.base_url, &p.api_key) {
                        Some(result) if result.success => format_balance_result(&result),
                        Some(result) => {
                            let err = result.error.unwrap_or_else(|| "Unknown error".to_string());
                            format!("{} {}", style("✗").red(), err)
                        }
                        None => style(t("不支持查询", "Not supported")).dim().to_string(),
                    };
                    let _ = init_tx.send((p.id.clone(), display));
                }
            }
        });
    }
    // On-demand balance query receiver (for 'r' key)
    let mut bal_ondemand_rx: Option<mpsc::Receiver<(String, String)>> = None;

    enable_raw();

    // Load initial port and active provider before the render loop
    let init_config = if path.exists() {
        AppConfig::load(path).unwrap_or_default()
    } else {
        AppConfig::default()
    };
    let port = init_config.port;
    drop(init_config);
    let mut active_id: Option<String> = get_current_provider_id(port);

    loop {
        // Poll for background balance query results (non-blocking)
        while let Ok((id, display)) = bal_rx.try_recv() {
            balance_cache.insert(id, display);
        }
        if let Some(ref rx) = bal_ondemand_rx {
            while let Ok((id, display)) = rx.try_recv() {
                balance_cache.insert(id, display);
                footer_msg = Some(t("余额已刷新", "Balance refreshed").to_string());
            }
        }

        let config = if path.exists() {
            AppConfig::load(path)?
        } else {
            AppConfig::default()
        };
        let port = config.port;

        if !config.providers.is_empty() && selected >= config.providers.len() {
            selected = config.providers.len() - 1;
        } else if config.providers.is_empty() {
            selected = 0;
        }

        let rendered = render_dashboard(
            &config,
            active_id.as_deref(),
            selected,
            footer_msg.as_deref(),
            &balance_cache,
            shortcut_idx,
        );

        term_clear();
        let _ = execute!(stdout(), cursor::MoveTo(0, 0));
        term_print_raw(&rendered);
        let _ = stdout().flush();
        footer_msg = None;

        match read_key() {
            Key::CtrlC => {
                disable_raw();
                term_clear();
                println!("\n  {} {}!", style(DIAMOND).cyan(), t("再见", "Bye"));
                break;
            }
            Key::Esc => {
                if shortcut_idx.is_some() {
                    // Deselect shortcut, back to provider table
                    shortcut_idx = None;
                } else {
                    disable_raw();
                    term_clear();
                    println!("\n  {} {}!", style(DIAMOND).cyan(), t("再见", "Bye"));
                    break;
                }
            }
            Key::Up => {
                shortcut_idx = None;
                if selected > 0 {
                    selected -= 1;
                } else if !config.providers.is_empty() {
                    selected = config.providers.len() - 1;
                }
            }
            Key::Down if !config.providers.is_empty() => {
                shortcut_idx = None;
                selected = (selected + 1) % config.providers.len();
            }
            Key::Left => {
                shortcut_idx = Some(match shortcut_idx {
                    Some(i) if i > 0 => i - 1,
                    Some(_) => SHORTCUTS.len() - 1,
                    None => 0,
                });
            }
            Key::Right => {
                shortcut_idx = Some(match shortcut_idx {
                    Some(i) if i < SHORTCUTS.len() - 1 => i + 1,
                    Some(_) => 0,
                    None => 0,
                });
            }
            Key::Enter if shortcut_idx.is_some() => {
                // Execute selected shortcut
                let idx = shortcut_idx.unwrap();
                shortcut_idx = None;
                if let Some(s) = SHORTCUTS.get(idx) {
                    match s.key {
                        "e" => unsafe {
                            LANG = if LANG == Lang::En { Lang::Zh } else { Lang::En };
                        },
                        "a" => {
                            disable_raw();
                            let _ = add_provider(path, None);
                            enable_raw();
                        }
                        "d" if !config.providers.is_empty() => {
                            disable_raw();
                            let pid = config.providers[selected].id.clone();
                            let _ = set_default(path, Some(&pid), port);
                            enable_raw();
                            active_id = get_current_provider_id(port);
                        }
                        "p" => {
                            disable_raw();
                            let _ = manage_projects(path);
                            enable_raw();
                        }
                        "u" => {
                            disable_raw();
                            let _ = show_usage(path);
                            enable_raw();
                        }
                        "l" => {
                            disable_raw();
                            let _ = show_request_logs(path);
                            enable_raw();
                        }
                        "t" => {
                            disable_raw();
                            let _ = test_connections(path);
                            enable_raw();
                        }
                        "b" => {
                            disable_raw();
                            let _ = browse_presets();
                            enable_raw();
                        }
                        "g" => {
                            let _ = toggle_log_level(path, port);
                        }
                        "i" => {
                            disable_raw();
                            let _ = import_providers(path, None);
                            enable_raw();
                        }
                        "r" if !config.providers.is_empty() => {
                            let p = config.providers[selected].clone();
                            if crate::balance::is_balance_supported(&p.base_url) {
                                footer_msg =
                                    Some(t("正在查询余额...", "Querying balance...").to_string());
                                let (tx, rx) = mpsc::channel();
                                bal_ondemand_rx = Some(rx);
                                std::thread::spawn(move || {
                                    let display = match crate::balance::query_balance(
                                        &p.base_url,
                                        &p.api_key,
                                    ) {
                                        Some(result) if result.success => {
                                            format_balance_result(&result)
                                        }
                                        Some(result) => {
                                            let err = result
                                                .error
                                                .unwrap_or_else(|| "Unknown error".to_string());
                                            format!("{} {}", style("✗").red(), err)
                                        }
                                        None => style(t("不支持查询", "Not supported"))
                                            .dim()
                                            .to_string(),
                                    };
                                    let _ = tx.send((p.id.clone(), display));
                                });
                            } else {
                                footer_msg = Some(
                                    t(
                                        "该 Provider 不支持余额查询",
                                        "This provider does not support balance query",
                                    )
                                    .to_string(),
                                );
                            }
                        }
                        "R" => {
                            footer_msg = Some(
                                t("正在刷新所有余额...", "Refreshing all balances...").to_string(),
                            );
                            let refresh_config = config.clone();
                            let refresh_tx = bal_tx.clone();
                            std::thread::spawn(move || {
                                for p in &refresh_config.providers {
                                    if crate::balance::is_balance_supported(&p.base_url) {
                                        let display = match crate::balance::query_balance(
                                            &p.base_url,
                                            &p.api_key,
                                        ) {
                                            Some(result) if result.success => {
                                                format_balance_result(&result)
                                            }
                                            Some(result) => {
                                                let err = result
                                                    .error
                                                    .unwrap_or_else(|| "Unknown error".to_string());
                                                format!("{} {}", style("✗").red(), err)
                                            }
                                            None => style(t("不支持查询", "Not supported"))
                                                .dim()
                                                .to_string(),
                                        };
                                        let _ = refresh_tx.send((p.id.clone(), display));
                                    }
                                }
                            });
                        }
                        ";" => {
                            disable_raw();
                            let new_port = input_with_esc(
                                &format!(
                                    "  {} ({})",
                                    t("输入新端口", "Enter new port"),
                                    t("ESC 取消", "ESC to cancel")
                                ),
                                Some(&config.port.to_string()),
                            );
                            if let Some(port_str) = new_port {
                                if let Ok(new_port) = port_str.trim().parse::<u16>() {
                                    let mut cfg = config;
                                    cfg.port = new_port;
                                    cfg.save(path)?;
                                    footer_msg = Some(format!(
                                        "{}: {}",
                                        t("端口已更新", "Port updated"),
                                        new_port
                                    ));
                                } else {
                                    footer_msg =
                                        Some(t("无效的端口号", "Invalid port number").to_string());
                                }
                            }
                            enable_raw();
                        }
                        _ => {}
                    }
                }
            }
            Key::Space | Key::Enter if !config.providers.is_empty() && shortcut_idx.is_none() => {
                disable_raw();
                let _ = show_provider_action_menu(path, selected);
                enable_raw();
                active_id = get_current_provider_id(port);
            }
            Key::Char('a') => {
                disable_raw();
                let _ = add_provider(path, None);
                enable_raw();
            }
            Key::Char('d') if !config.providers.is_empty() => {
                disable_raw();
                let pid = config.providers[selected].id.clone();
                let _ = set_default(path, Some(&pid), port);
                enable_raw();
                active_id = get_current_provider_id(port);
            }
            Key::Char('p') => {
                disable_raw();
                let _ = manage_projects(path);
                enable_raw();
            }
            Key::Char('u') => {
                disable_raw();
                let _ = show_usage(path);
                enable_raw();
            }
            Key::Char('l') => {
                disable_raw();
                let _ = show_request_logs(path);
                enable_raw();
            }
            Key::Char('t') => {
                disable_raw();
                let _ = test_connections(path);
                enable_raw();
            }
            Key::Char('b') => {
                disable_raw();
                let _ = browse_presets();
                enable_raw();
            }
            Key::Char('g') => {
                let _ = toggle_log_level(path, port);
            }
            Key::Char('i') => {
                disable_raw();
                let _ = import_providers(path, None);
                enable_raw();
            }
            Key::Char('e') => unsafe {
                LANG = if LANG == Lang::En { Lang::Zh } else { Lang::En };
            },
            Key::Char('r') if !config.providers.is_empty() => {
                let p = config.providers[selected].clone();
                if crate::balance::is_balance_supported(&p.base_url) {
                    footer_msg = Some(t("正在查询余额...", "Querying balance...").to_string());
                    let (tx, rx) = mpsc::channel();
                    bal_ondemand_rx = Some(rx);
                    std::thread::spawn(move || {
                        let display = match crate::balance::query_balance(&p.base_url, &p.api_key) {
                            Some(result) if result.success => format_balance_result(&result),
                            Some(result) => {
                                let err =
                                    result.error.unwrap_or_else(|| "Unknown error".to_string());
                                format!("{} {}", style("✗").red(), err)
                            }
                            None => style(t("不支持查询", "Not supported")).dim().to_string(),
                        };
                        let _ = tx.send((p.id.clone(), display));
                    });
                } else {
                    footer_msg = Some(
                        t(
                            "该 Provider 不支持余额查询",
                            "This provider does not support balance query",
                        )
                        .to_string(),
                    );
                }
            }
            Key::Char('R') => {
                // Refresh ALL balances
                footer_msg =
                    Some(t("正在刷新所有余额...", "Refreshing all balances...").to_string());
                let refresh_config = config.clone();
                let refresh_tx = bal_tx.clone();
                std::thread::spawn(move || {
                    for p in &refresh_config.providers {
                        if crate::balance::is_balance_supported(&p.base_url) {
                            let display =
                                match crate::balance::query_balance(&p.base_url, &p.api_key) {
                                    Some(result) if result.success => {
                                        format_balance_result(&result)
                                    }
                                    Some(result) => {
                                        let err = result
                                            .error
                                            .unwrap_or_else(|| "Unknown error".to_string());
                                        format!("{} {}", style("✗").red(), err)
                                    }
                                    None => {
                                        style(t("不支持查询", "Not supported")).dim().to_string()
                                    }
                                };
                            let _ = refresh_tx.send((p.id.clone(), display));
                        }
                    }
                });
            }
            Key::Char(';') => {
                disable_raw();
                let new_port = input_with_esc(
                    &format!(
                        "  {} ({})",
                        t("输入新端口", "Enter new port"),
                        t("ESC 取消", "ESC to cancel")
                    ),
                    Some(&config.port.to_string()),
                );
                if let Some(port_str) = new_port {
                    if let Ok(new_port) = port_str.trim().parse::<u16>() {
                        let mut cfg = config;
                        cfg.port = new_port;
                        cfg.save(path)?;
                        footer_msg =
                            Some(format!("{}: {}", t("端口已更新", "Port updated"), new_port));
                    } else {
                        footer_msg = Some(t("无效的端口号", "Invalid port number").to_string());
                    }
                }
                enable_raw();
            }
            _ => {}
        }
    }

    Ok(())
}

// ─── Provider Form Editor (arrow-navigate, space-edit) ──────────

struct FormField {
    label: &'static str,
    value: String,
    required: bool,
    is_confirm: bool, // for yes/no fields
    is_select: bool,  // for select fields (api_format)
    select_options: Vec<&'static str>,
    is_json: bool,    // Config JSON preview/import field
    is_save: bool,    // Save button field
}

#[derive(PartialEq)]
enum FormResult {
    Save,
    Discard,
}
fn provider_form_editor(title: &str, fields: &mut [FormField]) -> Option<FormResult> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut cursor: usize = 0;
    let mut scroll_offset: usize = 0;
    // Inline editing state
    let mut editing: bool = false;
    let mut edit_buffer: String = String::new();
    let mut edit_cursor: usize = 0;
    // Status message (shown briefly)
    let mut status_msg: Option<String> = None;

    // Record initial values for dirty tracking
    let initial_values: Vec<String> = fields.iter().map(|f| f.value.clone()).collect();
    let check_dirty = |flds: &[FormField], init: &[String]| -> bool {
        flds.iter().zip(init.iter()).any(|(f, iv)| f.value != *iv)
    };

    // Build Claude Code config JSON preview from current fields
    // Format matches cc-switch: { env: { ANTHROPIC_BASE_URL, ANTHROPIC_AUTH_TOKEN, ... }, effortLevel, ... }
    let build_json_preview = |flds: &[FormField]| -> String {
        // Fields: 0=id, 1=name, 2=api_key, 3=base_url, 4=model, 5=vision_model,
        //         6=api_format, 7=is_default, 8=display_name, 9=notes, 10=effort_level
        let get = |idx: usize| -> &str {
            flds.get(idx).map(|f| f.value.as_str()).unwrap_or("")
        };

        let mut env = serde_json::Map::new();
        if !get(3).is_empty() {
            env.insert("ANTHROPIC_BASE_URL".to_string(), serde_json::Value::String(get(3).to_string()));
        }
        if !get(2).is_empty() {
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), serde_json::Value::String(get(2).to_string()));
        }
        if !get(4).is_empty() {
            env.insert("ANTHROPIC_MODEL".to_string(), serde_json::Value::String(get(4).to_string()));
        }
        // Vision model as haiku/sonnet/opus model defaults
        let vision = get(5);
        if !vision.is_empty() {
            env.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), serde_json::Value::String(vision.to_string()));
            env.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), serde_json::Value::String(vision.to_string()));
            env.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), serde_json::Value::String(vision.to_string()));
        }
        // Common env vars
        env.insert("ENABLE_TOOL_SEARCH".to_string(), serde_json::Value::String("true".to_string()));
        env.insert("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS".to_string(), serde_json::Value::String("1".to_string()));
        env.insert("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(), serde_json::Value::String("1".to_string()));
        env.insert("DISABLE_AUTOUPDATER".to_string(), serde_json::Value::String("1".to_string()));

        let mut obj = serde_json::Map::new();
        obj.insert("env".to_string(), serde_json::Value::Object(env));
        // effortLevel
        let effort = get(10);
        if !effort.is_empty() {
            obj.insert("effortLevel".to_string(), serde_json::Value::String(effort.to_string()));
        } else {
            obj.insert("effortLevel".to_string(), serde_json::Value::String("max".to_string()));
        }
        obj.insert("alwaysThinkingEnabled".to_string(), serde_json::Value::Bool(true));
        obj.insert("model".to_string(), serde_json::Value::String("opus".to_string()));

        let mut perms = serde_json::Map::new();
        perms.insert("allow".to_string(), serde_json::Value::Array(vec![]));
        perms.insert("deny".to_string(), serde_json::Value::Array(vec![]));
        obj.insert("permissions".to_string(), serde_json::Value::Object(perms));

        let mut attr = serde_json::Map::new();
        attr.insert("commit".to_string(), serde_json::Value::String(String::new()));
        attr.insert("pr".to_string(), serde_json::Value::String(String::new()));
        obj.insert("attribution".to_string(), serde_json::Value::Object(attr));

        serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap_or_default()
    };

    // Apply Claude Code config JSON to form fields
    let apply_json_to_fields = |flds: &mut [FormField], json_str: &str| -> Result<(), String> {
        let val: serde_json::Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let obj = val.as_object().ok_or("JSON must be an object".to_string())?;

        // Extract from env block
        if let Some(env) = obj.get("env").and_then(|v| v.as_object()) {
            // field 3 = base_url
            if let Some(v) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                if flds.len() > 3 { flds[3].value = v.to_string(); }
            }
            // field 2 = api_key
            if let Some(v) = env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) {
                if flds.len() > 2 { flds[2].value = v.to_string(); }
            }
            // field 4 = model
            if let Some(v) = env.get("ANTHROPIC_MODEL").and_then(|v| v.as_str()) {
                if flds.len() > 4 { flds[4].value = v.to_string(); }
            }
            // field 5 = vision_model (from any of the model defaults)
            for key in &["ANTHROPIC_DEFAULT_HAIKU_MODEL", "ANTHROPIC_DEFAULT_SONNET_MODEL", "ANTHROPIC_DEFAULT_OPUS_MODEL"] {
                if let Some(v) = env.get(*key).and_then(|v| v.as_str()) {
                    if flds.len() > 5 && !v.is_empty() { flds[5].value = v.to_string(); break; }
                }
            }
        }

        // field 10 = effortLevel
        if let Some(v) = obj.get("effortLevel").and_then(|v| v.as_str()) {
            if flds.len() > 10 { flds[10].value = v.to_string(); }
        }

        Ok(())
    };

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let header_rows = 2;
        let footer_rows = 2;
        let view_count = term_h.saturating_sub(header_rows + footer_rows).max(1);

        if cursor >= fields.len() {
            cursor = fields.len().saturating_sub(1);
        }
        if cursor < scroll_offset {
            scroll_offset = cursor;
        }
        if cursor >= scroll_offset + view_count {
            scroll_offset = cursor - view_count + 1;
        }
        let visible_end = (scroll_offset + view_count).min(fields.len());

        // Build content lines
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("  {} {}", DIAMOND, style(title).cyan().bold()));

        if editing {
            lines.push(format!(
                "  {} {} {}  {}  {} {}",
                style(t("提示:", "Hint:")).dim(),
                t("输入内容", "type text"),
                style(t("回车 完成", "Enter done")).cyan(),
                style(t("ESC 取消", "ESC cancel")).dim(),
                t("← → 移动", "← → move"),
                t("退格 删除", "Backspace delete"),
            ));
        } else {
            lines.push(format!(
                "  {} ↑↓ {}  {}  {}  ESC {}",
                style(t("提示:", "Hint:")).dim(),
                t("移动", "navigate"),
                t("回车 编辑", "Enter to edit"),
                style("c").cyan().to_string() + " " + t("复制", "copy"),
                t("返回", "back"),
            ));
        }

        for (i, f) in fields
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_end - scroll_offset)
        {
            let required_mark = if f.required && !f.is_json && !f.is_save {
                "*"
            } else {
                " "
            };
            let marker = if i == cursor {
                format!("{} ", style("▸").cyan().bold())
            } else {
                "  ".to_string()
            };
            let styled_label = if i == cursor {
                style(f.label).cyan().bold().to_string()
            } else {
                f.label.to_string()
            };

            if f.is_save {
                lines.push(format!(
                    "  {}[{}]  {} {}",
                    marker, required_mark, style("💾").green(), style(&f.value).green().bold()
                ));
            } else if f.is_json {
                // Render full JSON when cursor is on this field, truncated otherwise
                let json_preview = build_json_preview(fields);
                let preview_lines: Vec<&str> = json_preview.lines().collect();
                let show_lines = if i == cursor { preview_lines.len() } else { 5 };
                let show_lines = show_lines.min(preview_lines.len());
                lines.push(format!(
                    "  {}[{}] {}  {}",
                    marker, required_mark, styled_label,
                    style(if i == cursor { "(full)" } else { "(preview)" }).dim()
                ));
                for line in preview_lines.iter().take(show_lines) {
                    lines.push(format!("      {}", style(line).dim()));
                }
                if preview_lines.len() > show_lines {
                    lines.push(format!(
                        "      {}",
                        style(format!("... +{} more", preview_lines.len() - show_lines)).dim()
                    ));
                }
            } else {
                let value_display = if editing && i == cursor {
                    let before = &edit_buffer[..edit_cursor];
                    let after = &edit_buffer[edit_cursor..];
                    let cursor_char = "█";
                    format!("{}{}{}", before, style(cursor_char).on_blue(), after)
                } else if f.is_confirm {
                    if f.value == "true" {
                        style(t("是", "Yes")).green().to_string()
                    } else {
                        style(t("否", "No")).dim().to_string()
                    }
                } else if f.is_select {
                    style(&f.value).cyan().to_string()
                } else if f.value.is_empty() {
                    style(t("(空)", "(empty)")).dim().to_string()
                } else if f.label == "API Key" {
                    style(&f.value).dim().to_string()
                } else {
                    f.value.clone()
                };
                let styled_val = if i == cursor {
                    style(&value_display).bold().to_string()
                } else {
                    value_display
                };
                lines.push(format!(
                    "  {}[{}] {}:  {}",
                    marker, required_mark, styled_label, styled_val
                ));
            }
        }
        // Footer lines
        if scroll_offset > 0 || visible_end < fields.len() {
            lines.push(format!("  · {}/{}", cursor + 1, fields.len()));
        }

        // Show status message if any
        if let Some(ref msg) = status_msg {
            lines.push(format!("  {}", style(msg).yellow()));
        }

        if editing {
            lines.push(format!(
                "  {} {}  {} {}",
                style(t("提示:", "Hint:")).dim(),
                style(t("回车 完成编辑", "Enter to finish")).cyan(),
                style(t("ESC 取消修改", "ESC cancel")).dim(),
                t("← → 移动光标", "← → move cursor"),
            ));
        } else {
            lines.push(format!(
                "  ↑↓ {}  {}  c {}  v {}  r {}  ESC {}",
                t("选择字段", "select field"),
                t("回车 编辑/确认", "Enter edit/confirm"),
                t("复制", "copy"),
                t("粘贴JSON", "paste JSON"),
                t("重置JSON", "reset JSON"),
                t("返回", "back"),
            ));
        }

        // Vertical centering
        let content_height = lines.len();
        let top_pad = if term_h > content_height + 4 {
            (term_h - content_height) / 3
        } else {
            0
        };

        term_clear();
        for _ in 0..top_pad {
            term_print("");
        }
        for line in &lines {
            term_print(line);
        }

        // Read key
        match read_key() {
            Key::Up if !editing => {
                status_msg = None;
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = fields.len() - 1;
                }
            }
            Key::Down if !editing => {
                status_msg = None;
                if cursor < fields.len() - 1 {
                    cursor += 1;
                } else {
                    cursor = 0;
                }
            }
            Key::Space | Key::Enter if !editing => {
                let f = &fields[cursor];
                if f.is_save {
                    // Save button pressed
                    return Some(FormResult::Save);
                } else if f.is_confirm {
                    let label = f.label;
                    let current = f.value == "true";
                    let _ = f;
                    if let Some(v) = confirm_with_esc(&format!("  * {}", label), current) {
                        fields[cursor].value = if v {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        };
                    }
                } else if f.is_select {
                    let label = f.label;
                    let options = f.select_options.clone();
                    let current_val = f.value.clone();
                    let _ = f;
                    let items: Vec<String> = options.iter().map(|s| s.to_string()).collect();
                    let current_idx = options
                        .iter()
                        .position(|s| *s == current_val)
                        .unwrap_or(0);
                    if let Some(idx) =
                        custom_select(&format!("  * {}", label), &items, current_idx)
                    {
                        fields[cursor].value = options[idx].to_string();
                    }
                } else if f.is_json {
                    // Enter inline editing for JSON field, start empty for pasting
                    let json_preview = build_json_preview(fields);
                    editing = true;
                    edit_buffer = json_preview;
                    edit_cursor = edit_buffer.len();
                } else {
                    // Start inline editing
                    let val = f.value.clone();
                    let _ = f;
                    editing = true;
                    edit_buffer = val;
                    edit_cursor = edit_buffer.len();
                }
            }
            Key::Enter if editing => {
                let f = &fields[cursor];
                if f.is_json {
                    // Try to parse JSON and apply to other fields
                    let buf = edit_buffer.clone();
                    fields[cursor].value = buf.clone();
                    editing = false;
                    match apply_json_to_fields(fields, &buf) {
                        Ok(()) => {
                            status_msg = Some(
                                t("JSON 导入成功", "JSON import successful").to_string(),
                            );
                        }
                        Err(e) => {
                            status_msg = Some(format!(
                                "{}: {}",
                                t("JSON 解析失败", "JSON parse error"),
                                e
                            ));
                        }
                    }
                } else {
                    fields[cursor].value = edit_buffer.clone();
                    editing = false;
                }
            }
            Key::Esc if editing => {
                editing = false;
            }
            Key::Esc => {
                if check_dirty(fields, &initial_values) {
                    if confirm_with_esc(
                        &t(
                            "有未保存的更改，确认放弃？",
                            "Unsaved changes, confirm discard?",
                        )
                        .to_string(),
                        false,
                    )
                    .unwrap_or(false)
                    {
                        return Some(FormResult::Discard);
                    }
                    // User chose not to discard, continue editing
                } else {
                    return Some(FormResult::Discard);
                }
            }
            Key::Char('v') if !editing && fields[cursor].is_json => {
                // Paste from clipboard (pbpaste) into JSON field and apply
                match std::process::Command::new("pbpaste").output() {
                    Ok(output) => {
                        let pasted = String::from_utf8_lossy(&output.stdout).to_string();
                        if !pasted.is_empty() {
                            match apply_json_to_fields(fields, &pasted) {
                                Ok(()) => {
                                    status_msg = Some(
                                        t("已从剪贴板导入配置", "Config imported from clipboard").to_string(),
                                    );
                                }
                                Err(e) => {
                                    status_msg = Some(format!(
                                        "{}: {}",
                                        t("JSON 解析失败", "JSON parse error"),
                                        e
                                    ));
                                }
                            }
                        }
                    }
                    Err(_) => {
                        status_msg = Some(
                            t("剪贴板读取失败", "Clipboard read failed").to_string(),
                        );
                    }
                }
            }
            Key::Char('r') if !editing && fields[cursor].is_json => {
                // Reset JSON field to reflect current form values
                status_msg = Some(
                    t("Config JSON 已重置", "Config JSON reset").to_string(),
                );
            }
            Key::Char('c') if !editing => {
                // Copy current field value to clipboard via pbcopy
                let val = fields[cursor].value.clone();
                if !val.is_empty() {
                    match std::process::Command::new("pbcopy")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(mut child) => {
                            if let Some(ref mut stdin) = child.stdin {
                                let _ = stdin.write_all(val.as_bytes());
                            }
                            let _ = child.wait();
                            status_msg = Some(
                                t("已复制到剪贴板", "Copied to clipboard").to_string(),
                            );
                        }
                        Err(_) => {
                            status_msg = Some(
                                t("复制失败", "Copy failed").to_string(),
                            );
                        }
                    }
                }
            }
            Key::CtrlC => {
                return None;
            }
            // Character input during inline editing
            Key::Char(c) if editing => {
                edit_buffer.insert(edit_cursor, c);
                edit_cursor += 1;
            }
            Key::Space if editing => {
                edit_buffer.insert(edit_cursor, ' ');
                edit_cursor += 1;
            }
            Key::Backspace if editing && edit_cursor > 0 => {
                edit_cursor -= 1;
                edit_buffer.remove(edit_cursor);
            }
            Key::Delete if editing && edit_cursor < edit_buffer.len() => {
                edit_buffer.remove(edit_cursor);
            }
            Key::Left if editing => {
                edit_cursor = edit_cursor.saturating_sub(1);
            }
            Key::Right if editing && edit_cursor < edit_buffer.len() => {
                edit_cursor += 1;
            }
            _ => {}
        }
    }
}

// ─── Provider Detail ────────────────────────────────────────────

fn show_balance_detail(config: &AppConfig, provider_id: &str) {
    let p = match config.get_provider_by_id(provider_id) {
        Some(p) => p,
        None => return,
    };
    let name = provider_name(p);
    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let top_pad = if term_h > 6 { (term_h - 6) / 3 } else { 0 };

    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }
    term_print(&format!(
        "  {} {} - {}",
        style(DIAMOND).cyan().bold(),
        t("余额查询", "Balance Query"),
        style(name).cyan()
    ));
    term_print(&format!(
        "  {} {}",
        style(t("查询中...", "Querying...")).dim(),
        spinner()
    ));
    let _ = stdout().flush();

    match crate::balance::query_balance(&p.base_url, &p.api_key) {
        Some(result) if result.success => {
            let mut lines: Vec<String> = Vec::new();
            lines.push(format!(
                "  {} {} - {}",
                style(DIAMOND).cyan().bold(),
                t("余额查询", "Balance Query"),
                style(name).cyan()
            ));
            lines.push("".to_string());
            for d in &result.data {
                // Zhipu-style: plan_name already has full formatted info (e.g. "5小时: 100% 已用")
                if d.plan_name.contains('%') {
                    let valid = if d.is_valid {
                        style(t("有效", "Valid")).green().to_string()
                    } else {
                        style(t("无效", "Invalid")).red().to_string()
                    };
                    let invalid_msg = d
                        .invalid_message
                        .as_ref()
                        .map(|m| format!(" - {}", m))
                        .unwrap_or_default();
                    lines.push(format!(
                        "  {} {}  [{}{}]",
                        style(BULLET).cyan(),
                        style(&d.plan_name).cyan(),
                        valid,
                        invalid_msg,
                    ));
                } else {
                    // Standard format: currency balance
                    let remaining = d
                        .remaining
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    let total_str = d.total.map(|v| format!(" / {:.4}", v)).unwrap_or_default();
                    let used_str = d
                        .used
                        .map(|v| format!("  ({}: {:.4})", t("已用", "Used"), v))
                        .unwrap_or_default();
                    let unit = if d.unit.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", d.unit)
                    };
                    let valid = if d.is_valid {
                        style(t("有效", "Valid")).green().to_string()
                    } else {
                        style(t("无效", "Invalid")).red().to_string()
                    };
                    let invalid_msg = d
                        .invalid_message
                        .as_ref()
                        .map(|m| format!(" - {}", m))
                        .unwrap_or_default();
                    lines.push(format!(
                        "  {} {}: {}{}{}{}  [{}{}]",
                        style(BULLET).cyan(),
                        style(&d.plan_name).cyan(),
                        style(&remaining).bold(),
                        total_str,
                        unit,
                        used_str,
                        valid,
                        invalid_msg,
                    ));
                }
            }
            let content_h = lines.len();
            let pad = if term_h > content_h + 4 {
                (term_h - content_h) / 3
            } else {
                0
            };
            term_clear();
            for _ in 0..pad {
                term_print("");
            }
            for line in &lines {
                term_print(line);
            }
        }
        Some(result) => {
            let top_pad2 = if term_h > 7 { (term_h - 7) / 3 } else { 0 };
            term_clear();
            for _ in 0..top_pad2 {
                term_print("");
            }
            let err = result.error.unwrap_or_else(|| "Unknown error".to_string());
            term_print(&format!(
                "  {} {} - {}",
                style(DIAMOND).cyan().bold(),
                t("余额查询", "Balance Query"),
                style(name).cyan()
            ));
            term_print("");
            term_print(&format!("  {} {}", style("✗").red(), err));
        }
        None => {
            let top_pad2 = if term_h > 7 { (term_h - 7) / 3 } else { 0 };
            term_clear();
            for _ in 0..top_pad2 {
                term_print("");
            }
            term_print(&format!(
                "  {} {} - {}",
                style(DIAMOND).cyan().bold(),
                t("余额查询", "Balance Query"),
                style(name).cyan()
            ));
            term_print("");
            term_print(&format!(
                "  ○ {}",
                t(
                    "该 Provider 不支持余额查询",
                    "This provider does not support balance query"
                )
            ));
        }
    }
    term_print("");

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
}

fn show_provider_detail(config_path: &Path, id: &str) -> anyhow::Result<()> {
    // custom_select returned us to non-raw/main-screen; re-enter raw mode FIRST
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let config = AppConfig::load(config_path)?;
    let p = config
        .get_provider_by_id(id)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", id))?;

    let name = provider_name(p);
    let idx = config
        .providers
        .iter()
        .position(|pp| pp.id == id)
        .unwrap_or(0)
        + 1;
    let default_mark = if p.is_default {
        format!("  {}", t("默认", "Default"))
    } else {
        String::new()
    };

    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("  {}{}{}", BOX_TL, &"─".repeat(60), BOX_TR));
    lines.push(format!(
        "  {} {} #{:2}{}",
        BOX_V,
        style(name).green().bold(),
        idx,
        default_mark
    ));
    lines.push(format!("  {}{}{}", BOX_BL, &"─".repeat(60), BOX_BR));
    lines.push("".to_string());
    lines.push(format!("  ID:            {}", style(&p.id).dim()));
    lines.push(format!("  {}:      {}", t("模型", "Model"), &p.model));
    lines.push(format!(
        "  {}:  {:?}",
        t("API 格式", "API Format"),
        p.api_format
    ));
    lines.push(format!("  Base URL:      {}", style(&p.base_url).dim()));
    if let Some(ref vm) = p.vision_model {
        lines.push(format!(
            "  {}:  {}",
            t("图像模型", "Vision Model"),
            style(vm).cyan()
        ));
    }
    if let Some(ref dl) = p.display_name {
        lines.push(format!("  {}:    {}", t("显示名", "Display Name"), dl));
    }
    if let Some(ref notes) = p.notes {
        lines.push(format!(
            "  {}:      {}",
            t("备注", "Notes"),
            style(notes).dim()
        ));
    }
    if let Some(ref effort) = p.effort_level {
        lines.push(format!("  Effort:        {}", effort));
    }
    if let Some(ref pid) = p.preset_id {
        lines.push(format!("  Preset:        {}", style(pid).dim()));
    }
    let top_pad = if term_h > lines.len() + 4 {
        (term_h - lines.len()) / 3
    } else {
        0
    };
    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }
    for line in &lines {
        term_print(line);
    }

    // Show balance info
    if crate::balance::is_balance_supported(&p.base_url) {
        term_print("");
        term_print(&format!(
            "  {}: {}",
            t("余额", "Balance"),
            style(t("查询中...", "Querying...")).dim()
        ));
        let _ = stdout().flush();
        match crate::balance::query_balance(&p.base_url, &p.api_key) {
            Some(result) if result.success => {
                // Re-render balance section with results (overwrites "Querying..." visually)
                term_print(&format!("  {}:", t("余额", "Balance")));
                for d in &result.data {
                    let remaining = d
                        .remaining
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    let unit = if d.unit.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", d.unit)
                    };
                    let valid = if d.is_valid { "" } else { " ⚠" };
                    term_print(&format!(
                        "    {}{}: {}{}{}",
                        style(&d.plan_name).cyan(),
                        t("", ""),
                        style(&remaining).bold(),
                        unit,
                        valid,
                    ));
                }
            }
            Some(result) => {
                let err = result.error.unwrap_or_else(|| "Unknown error".to_string());
                term_print(&format!("    {} {}", style("✗").red(), err));
            }
            None => {}
        }
    }

    term_print("");

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Add Provider ───────────────────────────────────────────────

pub fn add_provider(config_path: &Path, preset_id: Option<&str>) -> anyhow::Result<()> {
    disable_raw();

    let mut config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    if let Some(preset_id) = preset_id {
        let preset = presets::get_preset_by_id(preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_id))?;
        return add_from_preset_form(&mut config, config_path, &preset);
    }

    let all_presets = presets::get_all_presets();
    let categories = presets::get_categories();

    let mut items: Vec<String> = Vec::new();
    let mut preset_refs: Vec<Option<usize>> = Vec::new();

    for category in &categories {
        let display_name = presets::get_category_display_name(category);
        let category_presets: Vec<_> = all_presets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.category == *category)
            .collect();

        if !category_presets.is_empty() {
            items.push(format!("── {} ──", display_name));
            preset_refs.push(None);
            for (idx, preset) in &category_presets {
                items.push(format!(
                    "  {} {:<16} {} ({})",
                    ARROW, preset.id, preset.display_name, preset.model
                ));
                preset_refs.push(Some(*idx));
            }
        }
    }

    items.push(format!("── {} ──", t("其他", "Other")));
    preset_refs.push(None);
    items.push(format!(
        "  {} {}",
        ARROW,
        t("自定义 (手动输入)", "Custom (Manual)")
    ));
    preset_refs.push(None);

    // The header box + custom_select will be rendered, no need for manual centering here
    // since custom_select now has its own vertical centering
    term_clear();

    let selection = match custom_select(
        &format!(
            "  {}",
            t(
                "选择 Provider (ESC 返回)",
                "Select Provider (ESC to go back)"
            )
        ),
        &items,
        0,
    ) {
        Some(s) => s,
        None => {
            enable_raw();
            return Ok(());
        }
    };

    if let Some(Some(preset_idx)) = preset_refs.get(selection) {
        let preset = &all_presets[*preset_idx];
        return add_from_preset_form(&mut config, config_path, preset);
    }

    add_custom_provider_form(&mut config, config_path)
}

fn add_from_preset_form(
    config: &mut AppConfig,
    config_path: &Path,
    preset: &presets::ProviderPreset,
) -> anyhow::Result<()> {
    let mut fields = vec![
        FormField {
            label: "Provider ID",
            value: generate_snowflake_id(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("名称", "Name"),
            value: preset.name.to_string(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "API Key",
            value: String::new(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Base URL",
            value: preset.base_url.to_string(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("模型", "Model"),
            value: preset.model.to_string(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("图像模型", "Vision Model"),
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("API 格式", "API Format"),
            value: format!("{:?}", preset.api_format),
            required: true,
            is_confirm: false,
            is_select: true,
            select_options: vec!["Anthropic", "OpenAiChat", "OpenAiResponses", "GeminiNative"],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("设为默认", "Set Default"),
            value: "false".to_string(),
            required: false,
            is_confirm: true,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("显示名", "Display Name"),
            value: preset.display_name.to_string(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("备注", "Notes"),
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Effort Level",
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        // Config JSON preview/import
        FormField {
            label: "Config JSON",
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: true,
            is_save: false,
        },
        // Save button
        FormField {
            label: "",
            value: t("保存", "Save").to_string(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: true,
        },
    ];

    let result = provider_form_editor(
        &format!(
            "{}: {}",
            t("添加 Provider", "Add Provider"),
            preset.display_name
        ),
        &mut fields,
    );

    match result {
        Some(FormResult::Save) => {}    // proceed with save
        Some(FormResult::Discard) | None => {
            enable_raw();
            return Ok(());
        }
    }

    // Build provider from fields
    let api_format = match fields[6].value.as_str() {
        "Anthropic" => ApiFormat::Anthropic,
        "OpenAiChat" => ApiFormat::OpenAiChat,
        "OpenAiResponses" => ApiFormat::OpenAiResponses,
        "GeminiNative" => ApiFormat::GeminiNative,
        _ => ApiFormat::OpenAiChat,
    };

    let vision = if fields[5].value.is_empty() {
        None
    } else {
        Some(fields[5].value.clone())
    };
    let display = if fields[8].value.is_empty() {
        None
    } else {
        Some(fields[8].value.clone())
    };
    let notes = if fields[9].value.is_empty() {
        None
    } else {
        Some(fields[9].value.clone())
    };
    let effort = if fields[10].value.is_empty() {
        None
    } else {
        Some(fields[10].value.clone())
    };

    let provider = ProviderConfig {
        id: fields[0].value.clone(),
        name: fields[1].value.clone(),
        api_format,
        base_url: fields[3].value.clone(),
        api_key: fields[2].value.clone(),
        model: fields[4].value.clone(),
        vision_model: vision,
        display_name: display,
        is_default: fields[7].value == "true",
        preset_id: Some(preset.id.to_string()),
        notes,
        effort_level: effort,
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;
    reload_proxy_config(config.port);

    disable_raw();
    term_print(&format!(
        "\n  {} Provider '{}' {}",
        style(BULLET).green(),
        style(&provider_id).cyan(),
        style(t("已添加", "added")).green()
    ));

    let options = vec![
        t("返回主菜单", "Back to Menu").to_string(),
        t("继续添加其他 Provider", "Add Another Provider").to_string(),
        t("退出", "Exit").to_string(),
    ];
    match custom_select(&format!("  {}", t("添加完成", "Added")), &options, 0) {
        Some(1) => {
            enable_raw();
            add_provider(config_path, None)?;
        }
        Some(2) => safe_exit(),
        _ => {}
    }
    Ok(())
}

fn add_custom_provider_form(config: &mut AppConfig, config_path: &Path) -> anyhow::Result<()> {
    let mut fields = vec![
        FormField {
            label: "Provider ID",
            value: generate_snowflake_id(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("名称", "Name"),
            value: String::new(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "API Key",
            value: String::new(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Base URL",
            value: String::new(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("模型", "Model"),
            value: String::new(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("图像模型", "Vision Model"),
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("API 格式", "API Format"),
            value: "Anthropic".to_string(),
            required: true,
            is_confirm: false,
            is_select: true,
            select_options: vec!["Anthropic", "OpenAiChat", "OpenAiResponses", "GeminiNative"],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("设为默认", "Set Default"),
            value: "false".to_string(),
            required: false,
            is_confirm: true,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("显示名", "Display Name"),
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("备注", "Notes"),
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Effort Level",
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        // Config JSON preview/import
        FormField {
            label: "Config JSON",
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: true,
            is_save: false,
        },
        // Save button
        FormField {
            label: "",
            value: t("保存", "Save").to_string(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: true,
        },
    ];

    let result = provider_form_editor(t("自定义 Provider", "Custom Provider"), &mut fields);

    match result {
        Some(FormResult::Save) => {} // proceed with save
        Some(FormResult::Discard) | None => {
            enable_raw();
            return Ok(());
        }
    }

    let api_format = match fields[6].value.as_str() {
        "Anthropic" => ApiFormat::Anthropic,
        "OpenAiChat" => ApiFormat::OpenAiChat,
        "OpenAiResponses" => ApiFormat::OpenAiResponses,
        "GeminiNative" => ApiFormat::GeminiNative,
        _ => ApiFormat::OpenAiChat,
    };

    let vision = if fields[5].value.is_empty() {
        None
    } else {
        Some(fields[5].value.clone())
    };
    let display = if fields[8].value.is_empty() {
        None
    } else {
        Some(fields[8].value.clone())
    };
    let notes = if fields[9].value.is_empty() {
        None
    } else {
        Some(fields[9].value.clone())
    };
    let effort = if fields[10].value.is_empty() {
        None
    } else {
        Some(fields[10].value.clone())
    };

    let provider = ProviderConfig {
        id: fields[0].value.clone(),
        name: fields[1].value.clone(),
        api_format,
        base_url: fields[3].value.clone(),
        api_key: fields[2].value.clone(),
        model: fields[4].value.clone(),
        vision_model: vision,
        display_name: display,
        is_default: fields[7].value == "true",
        preset_id: None,
        notes,
        effort_level: effort,
    };

    let provider_id = provider.id.clone();
    config.add_provider(provider)?;
    config.save(config_path)?;
    reload_proxy_config(config.port);

    disable_raw();
    term_print(&format!(
        "\n  {} Provider '{}' {}",
        style(BULLET).green(),
        style(&provider_id).cyan(),
        style(t("已添加", "added")).green()
    ));

    let options = vec![
        t("返回主菜单", "Back to Menu").to_string(),
        t("继续添加其他 Provider", "Add Another Provider").to_string(),
        t("退出", "Exit").to_string(),
    ];
    match custom_select(&format!("  {}", t("添加完成", "Added")), &options, 0) {
        Some(1) => {
            enable_raw();
            add_provider(config_path, None)?;
        }
        Some(2) => safe_exit(),
        _ => {}
    }
    Ok(())
}

// ─── Edit Provider ──────────────────────────────────────────────

pub fn edit_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    disable_raw();

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term_print(&format!(
            "  ○ {}",
            t("没有可编辑的 Provider", "No providers to edit")
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    let provider_idx = if let Some(id) = id {
        config
            .providers
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", id))?
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default {
                    format!("  {}", t("默认", "Default"))
                } else {
                    String::new()
                };
                format!(
                    "{} ({}){}",
                    style(provider_name(p)).green(),
                    p.model,
                    default
                )
            })
            .collect();
        match custom_select(
            &format!(
                "  {}",
                t(
                    "选择要编辑的 Provider (ESC 返回)",
                    "Select Provider to Edit (ESC to go back)"
                )
            ),
            &items,
            0,
        ) {
            Some(s) => s,
            None => {
                enable_raw();
                return Ok(());
            }
        }
    };

    let provider = config.providers[provider_idx].clone();
    let provider_id = provider.id.clone();

    let mut fields = vec![
        FormField {
            label: "Provider ID",
            value: provider.id.clone(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("名称", "Name"),
            value: provider.name.clone(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "API Key",
            value: provider.api_key.clone(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Base URL",
            value: provider.base_url.clone(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("模型", "Model"),
            value: provider.model.clone(),
            required: true,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("图像模型", "Vision Model"),
            value: provider.vision_model.clone().unwrap_or_default(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("API 格式", "API Format"),
            value: format!("{:?}", provider.api_format),
            required: true,
            is_confirm: false,
            is_select: true,
            select_options: vec!["Anthropic", "OpenAiChat", "OpenAiResponses", "GeminiNative"],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("设为默认", "Set Default"),
            value: if provider.is_default {
                "true".to_string()
            } else {
                "false".to_string()
            },
            required: false,
            is_confirm: true,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("显示名", "Display Name"),
            value: provider.display_name.clone().unwrap_or_default(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: t("备注", "Notes"),
            value: provider.notes.clone().unwrap_or_default(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        FormField {
            label: "Effort Level",
            value: provider.effort_level.clone().unwrap_or_default(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: false,
        },
        // Config JSON preview/import
        FormField {
            label: "Config JSON",
            value: String::new(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: true,
            is_save: false,
        },
        // Save button
        FormField {
            label: "",
            value: t("保存", "Save").to_string(),
            required: false,
            is_confirm: false,
            is_select: false,
            select_options: vec![],
            is_json: false,
            is_save: true,
        },
    ];

    let result = provider_form_editor(
        &format!(
            "{}: {}",
            t("编辑 Provider", "Edit Provider"),
            provider_name(&provider)
        ),
        &mut fields,
    );

    match result {
        Some(FormResult::Save) => {}     // proceed with save
        Some(FormResult::Discard) | None => {
            enable_raw();
            return Ok(());
        }
    }

    let api_format = match fields[6].value.as_str() {
        "Anthropic" => ApiFormat::Anthropic,
        "OpenAiChat" => ApiFormat::OpenAiChat,
        "OpenAiResponses" => ApiFormat::OpenAiResponses,
        "GeminiNative" => ApiFormat::GeminiNative,
        _ => ApiFormat::OpenAiChat,
    };

    let vision = if fields[5].value.is_empty() {
        None
    } else {
        Some(fields[5].value.clone())
    };
    let display = if fields[8].value.is_empty() {
        None
    } else {
        Some(fields[8].value.clone())
    };
    let notes = if fields[9].value.is_empty() {
        None
    } else {
        Some(fields[9].value.clone())
    };
    let effort = if fields[10].value.is_empty() {
        None
    } else {
        Some(fields[10].value.clone())
    };

    let updates = ProviderUpdate {
        name: Some(fields[1].value.clone()),
        base_url: Some(fields[3].value.clone()),
        api_key: Some(fields[2].value.clone()),
        model: Some(fields[4].value.clone()),
        vision_model: Some(vision),
        display_name: Some(display.unwrap_or_default()),
        is_default: Some(fields[7].value == "true"),
        notes: Some(notes.unwrap_or_default()),
        effort_level: Some(effort),
    };

    config.update_provider(&provider_id, updates)?;

    // Update api_format directly
    if let Some(p) = config.providers.iter_mut().find(|p| p.id == provider_id) {
        p.api_format = api_format;
    }

    config.save(config_path)?;
    reload_proxy_config(config.port);

    term_print(&format!(
        "\n  {} Provider '{}' {}",
        style(BULLET).green(),
        style(&provider_id).cyan(),
        style(t("已更新", "updated")).green()
    ));

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Remove Provider ────────────────────────────────────────────

pub fn remove_provider(config_path: &Path, id: Option<&str>) -> anyhow::Result<()> {
    disable_raw();

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term_print(&format!(
            "  ○ {}",
            t("没有可删除的 Provider", "No providers to delete")
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    let provider_id = if let Some(id) = id {
        id.to_string()
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default {
                    format!("  {}", t("默认", "Default"))
                } else {
                    String::new()
                };
                format!(
                    "{} ({}){}",
                    style(provider_name(p)).green(),
                    p.model,
                    default
                )
            })
            .collect();
        match custom_select(
            &format!(
                "  {}",
                t(
                    "选择要删除的 Provider (ESC 返回)",
                    "Select Provider to Delete (ESC to go back)"
                )
            ),
            &items,
            0,
        ) {
            Some(s) => config.providers[s].id.clone(),
            None => {
                enable_raw();
                return Ok(());
            }
        }
    };

    // Single confirmation — direct Y/N input supported
    let confirmed = match confirm_with_esc(
        &format!(
            "  {} '{}' {}",
            t("确认删除", "Confirm delete"),
            style(&provider_id).red(),
            t("[Y/N]", "[Y/N]"),
        ),
        false,
    ) {
        Some(v) => v,
        None => {
            enable_raw();
            return Ok(());
        }
    };

    if confirmed {
        config.remove_provider(&provider_id)?;
        config.save(config_path)?;
        reload_proxy_config(config.port);
    }

    enable_raw();
    Ok(())
}

// ─── Set Default ────────────────────────────────────────────────

pub fn set_default(config_path: &Path, id: Option<&str>, port: u16) -> anyhow::Result<()> {
    disable_raw();

    let mut config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        term_clear();
        term_print(&format!(
            "  ○ {}",
            t("没有配置 Provider", "No providers configured")
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    let provider_id = if let Some(id) = id {
        id.to_string()
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| {
                let default = if p.is_default {
                    format!("  {}", t("默认", "Default"))
                } else {
                    String::new()
                };
                format!(
                    "{} ({}){}",
                    style(provider_name(p)).green(),
                    p.model,
                    default
                )
            })
            .collect();
        match custom_select(
            &format!(
                "  {}",
                t(
                    "选择默认 Provider (ESC 返回)",
                    "Select Default Provider (ESC to go back)"
                )
            ),
            &items,
            0,
        ) {
            Some(s) => config.providers[s].id.clone(),
            None => {
                enable_raw();
                return Ok(());
            }
        }
    };

    config.set_default_provider(&provider_id)?;
    config.save(config_path)?;
    reload_proxy_config(port);

    let switch_result = TUI_CLIENT
        .post(format!("http://127.0.0.1:{}/api/switch-provider", port))
        .json(&serde_json::json!({ "provider_id": provider_id }))
        .send();

    let name_display = config
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .map(|p| provider_name(p).to_string())
        .unwrap_or_else(|| provider_id.clone());

    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let top_pad = if term_h > 8 { (term_h - 8) / 3 } else { 0 };

    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }

    match switch_result {
        Ok(resp) if resp.status().is_success() => {
            term_print(&format!(
                "  {} {} {} {}",
                style(BULLET).green(),
                t("默认 Provider", "Default Provider"),
                style(&name_display).cyan().bold(),
                style(t("已切换 (即时生效)", "switched (effective now)")).green()
            ));
        }
        _ => {
            term_print(&format!(
                "  {} {} {} {}",
                style(BULLET).green(),
                t("默认 Provider", "Default Provider"),
                style(&name_display).cyan(),
                style(t(
                    "已设为默认 (重启后生效)",
                    "set as default (effective after restart)"
                ))
                .dim()
            ));
        }
    }

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Copy Provider ──────────────────────────────────────────────

fn copy_provider_ui(config_path: &Path, source_id: &str) -> anyhow::Result<()> {
    disable_raw();

    let mut config = AppConfig::load(config_path)?;
    let source = config
        .get_provider_by_id(source_id)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", source_id))?
        .clone();

    let name = provider_name(&source);
    term_clear();
    term_print(&format!(
        "  {} '{}' {}",
        style(BULLET).cyan(),
        style(name).cyan().bold(),
        style(t("(输入新 ID)", "(enter new ID)")).dim(),
    ));
    term_print("");

    let new_id = match input_with_esc(
        &format!(
            "  {} ({}: {})",
            t("新 Provider ID", "New Provider ID"),
            t("原始", "original"),
            source_id
        ),
        Some(&format!("{}-copy", source_id)),
    ) {
        Some(v) => v,
        None => {
            enable_raw();
            return Ok(());
        }
    };

    config.copy_provider(source_id, &new_id)?;
    config.save(config_path)?;
    reload_proxy_config(config.port);

    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let top_pad = if term_h > 6 { (term_h - 6) / 3 } else { 0 };
    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }
    term_print(&format!(
        "  {} '{}' → '{}' {}",
        style(BULLET).green(),
        style(source_id).dim(),
        style(&new_id).cyan(),
        style(t("已复制", "copied")).green()
    ));

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Test Connections ───────────────────────────────────────────

fn test_connections(config_path: &Path) -> anyhow::Result<()> {
    disable_raw();
    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let top_pad = if term_h > 12 { (term_h - 12) / 3 } else { 0 };
    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }

    term_print(&format!("  {}{}{}", BOX_TL, &"─".repeat(60), BOX_TR));
    term_print(&format!(
        "  {} {}{}",
        BOX_V,
        t("测试连接", "Test Connections"),
        " ".repeat(40)
    ));
    term_print(&format!("  {}{}{}", BOX_BL, &"─".repeat(60), BOX_BR));
    term_print("");
    term_print(&format!(
        "  {} {}\n",
        BULLET,
        t("正在测试...", "Testing...")
    ));

    let path_str = config_path.to_string_lossy().to_string();
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(crate::commands::test::run_test(&path_str, None))
    });
    result?;

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Usage Statistics ───────────────────────────────────────────

fn show_usage(config_path: &Path) -> anyhow::Result<()> {
    let config = AppConfig::load(config_path)?;

    if config.providers.is_empty() {
        disable_raw();
        term_print(&format!(
            "  ○ {}",
            t("没有配置 Provider", "No providers configured")
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    let days_values = [7, 30, 90, 36500];
    let days_labels = [
        t("7 天", "7 Days"),
        t("30 天", "30 Days"),
        t("90 天", "90 Days"),
        t("全部", "All"),
    ];
    let mut days_idx: usize = 0;

    let db = crate::database::Database::open_cc_switch_compatible()
        .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    loop {
        let days = days_values[days_idx];
        let label = days_labels[days_idx];
        let mut out = String::new();

        out.push_str(&format!(
            "  {} {} - {} {}    {} {}  ESC {}\n\n",
            style("◆").cyan().bold(),
            t("使用统计", "Usage Stats"),
            t("最近", "Last"),
            style(label).cyan().bold(),
            style("← →").dim(),
            t("切换", "switch"),
            t("返回", "back")
        ));

        // Summary
        out.push_str(&format!(
            "  {}\n",
            style(t("── 汇总 ──", "── Summary ──")).bold()
        ));
        match db.get_usage_summary("claude", days) {
            Ok(s) => {
                out.push_str(&format!(
                    "  {}:     {}\n",
                    t("总请求数", "Total Requests"),
                    s.total_requests
                ));
                out.push_str(&format!(
                    "  {}:     {}\n",
                    t("成功请求", "Success"),
                    s.total_success
                ));
                out.push_str(&format!(
                    "  {}:  {}\n",
                    t("输入 Tokens", "Input Tokens"),
                    s.total_input_tokens
                ));
                out.push_str(&format!(
                    "  {}:  {}\n",
                    t("输出 Tokens", "Output Tokens"),
                    s.total_output_tokens
                ));
                out.push_str(&format!(
                    "  {}:       ${:.4}\n",
                    t("总费用", "Total Cost"),
                    s.total_cost_usd
                ));
                out.push_str(&format!(
                    "  {}:     {} ms\n",
                    t("平均延迟", "Avg Latency"),
                    s.avg_latency_ms
                ));
            }
            Err(e) => {
                out.push_str(&format!(
                    "  ✗ {}: {}\n",
                    t("获取统计失败", "Failed to get stats"),
                    e
                ));
            }
        }
        out.push('\n');

        // Per-provider
        out.push_str(&format!(
            "  {}\n",
            style(t("── 按 Provider ──", "── By Provider ──")).bold()
        ));
        match db.get_usage_by_provider("claude", days) {
            Ok(stats) => {
                if stats.is_empty() {
                    out.push_str(&format!("  ○ {}\n", t("暂无记录", "No records")));
                } else {
                    let name_map: std::collections::HashMap<String, String> = config
                        .providers
                        .iter()
                        .map(|p| (p.id.clone(), provider_name(p).to_string()))
                        .collect();
                    out.push_str(&format!(
                        "  {:<16} {:>6} {:>8} {:>8} {:>8} {:>8}\n",
                        "Provider",
                        t("请求数", "Reqs"),
                        t("输入", "Input"),
                        t("输出", "Output"),
                        t("费用", "Cost"),
                        t("延迟", "Latency")
                    ));
                    out.push_str(&format!("  {}\n", &"─".repeat(56)));
                    for s in &stats {
                        let display = name_map.get(&s.provider_id).cloned().unwrap_or_else(|| {
                            if s.provider_id.len() > 16 {
                                format!("{}…", &s.provider_id[..15])
                            } else {
                                s.provider_id.clone()
                            }
                        });
                        out.push_str(&format!(
                            "  {:<16} {:>6} {:>8} {:>8} ${:<6.2} {:>6}ms\n",
                            style(display).green(),
                            s.total_requests,
                            s.total_input_tokens,
                            s.total_output_tokens,
                            s.total_cost_usd,
                            s.avg_latency_ms
                        ));
                    }
                }
            }
            Err(e) => {
                out.push_str(&format!("  ✗ {}\n", e));
            }
        }
        out.push('\n');

        // Per-model
        out.push_str(&format!(
            "  {}\n",
            style(t("── 按 Model ──", "── By Model ──")).bold()
        ));
        match db.get_usage_by_model("claude", days) {
            Ok(stats) => {
                if stats.is_empty() {
                    out.push_str(&format!("  ○ {}\n", t("暂无记录", "No records")));
                } else {
                    out.push_str(&format!(
                        "  {:<24} {:>6} {:>8} {:>8} {:>8} {:>8}\n",
                        "Model",
                        t("请求数", "Reqs"),
                        t("输入", "Input"),
                        t("输出", "Output"),
                        t("费用", "Cost"),
                        t("延迟", "Latency")
                    ));
                    out.push_str(&format!("  {}\n", &"─".repeat(64)));
                    for s in &stats {
                        let display = if s.model.len() > 24 {
                            format!("{}…", &s.model[..23])
                        } else {
                            s.model.clone()
                        };
                        out.push_str(&format!(
                            "  {:<24} {:>6} {:>8} {:>8} ${:<6.2} {:>6}ms\n",
                            style(display).cyan(),
                            s.total_requests,
                            s.total_input_tokens,
                            s.total_output_tokens,
                            s.total_cost_usd,
                            s.avg_latency_ms
                        ));
                    }
                }
            }
            Err(e) => {
                out.push_str(&format!("  ✗ {}\n", e));
            }
        }

        term_clear();
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let content_lines = out.lines().count();
        let top_pad = if term_h > content_lines + 2 {
            (term_h - content_lines) / 3
        } else {
            0
        };
        for _ in 0..top_pad {
            term_print("");
        }
        term_print_raw(&out);

        match read_key() {
            Key::Left => {
                days_idx = if days_idx > 0 {
                    days_idx - 1
                } else {
                    days_values.len() - 1
                };
            }
            Key::Right => {
                days_idx = (days_idx + 1) % days_values.len();
            }
            Key::Esc | Key::CtrlC => break,
            _ => {}
        }
    }

    Ok(())
}

// ─── Request Logs ───────────────────────────────────────────────

fn show_request_logs(config_path: &Path) -> anyhow::Result<()> {
    let _ = AppConfig::load(config_path)?;

    let limit_values = [20, 50, 100];
    let mut limit_idx: usize = 0; // default: 20

    let db = crate::database::Database::open_cc_switch_compatible()
        .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    loop {
        let limit = limit_values[limit_idx];
        let mut out = String::new();

        out.push_str(&format!(
            "  {} {} ({} {})    {} {}  ESC {}\n\n",
            style("◆").cyan().bold(),
            t("请求日志", "Request Logs"),
            t("最近", "Last"),
            style(limit).cyan().bold(),
            style("← →").dim(),
            t("切换", "switch"),
            t("返回", "back")
        ));

        match db.get_recent_request_logs("claude", limit) {
            Ok(logs) => {
                if logs.is_empty() {
                    out.push_str(&format!("  ○ {}\n", t("暂无请求记录", "No request logs")));
                } else {
                    out.push_str(&format!(
                        "  {:<14} {:<16} {:<20} {:>5} {:>8} {:>6} {:>7}\n",
                        t("时间", "Time"),
                        "Provider",
                        "Model",
                        t("状态", "Status"),
                        "Tokens",
                        t("延迟", "Latency"),
                        t("费用", "Cost")
                    ));
                    out.push_str(&format!("  {}\n", &"─".repeat(78)));
                    for log in &logs {
                        let ts = format_ts(log.created_at);
                        let prov = if log.provider_id.len() > 16 {
                            format!("{}…", &log.provider_id[..15])
                        } else {
                            log.provider_id.clone()
                        };
                        let model = if log.model.len() > 20 {
                            format!("{}…", &log.model[..19])
                        } else {
                            log.model.clone()
                        };
                        let status = if log.status_code >= 200 && log.status_code < 300 {
                            style(format!("{}", log.status_code)).green().to_string()
                        } else {
                            style(format!("{}", log.status_code)).red().to_string()
                        };
                        let tokens = format!("{}/{}", log.input_tokens, log.output_tokens);
                        let latency = format!("{}ms", log.latency_ms);

                        out.push_str(&format!(
                            "  {:<14} {:<16} {:<20} {:>5} {:>8} {:>6} ${:<6.2}\n",
                            ts, prov, model, status, tokens, latency, log.total_cost_usd
                        ));

                        if let Some(ref err) = log.error_message {
                            if !err.is_empty() {
                                out.push_str(&format!(
                                    "    {} {}\n",
                                    style("✗").red(),
                                    style(err).red()
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                out.push_str(&format!(
                    "  ✗ {}: {}\n",
                    t("获取日志失败", "Failed to get logs"),
                    e
                ));
            }
        }

        term_clear();
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let content_lines = out.lines().count();
        let top_pad = if term_h > content_lines + 2 {
            (term_h - content_lines) / 3
        } else {
            0
        };
        for _ in 0..top_pad {
            term_print("");
        }
        term_print_raw(&out);

        match read_key() {
            Key::Left => {
                limit_idx = if limit_idx > 0 {
                    limit_idx - 1
                } else {
                    limit_values.len() - 1
                };
            }
            Key::Right => {
                limit_idx = (limit_idx + 1) % limit_values.len();
            }
            Key::Esc | Key::CtrlC => break,
            _ => {}
        }
    }

    Ok(())
}

// ─── Import ─────────────────────────────────────────────────────

pub fn import_providers(config_path: &Path, db_path: Option<&str>) -> anyhow::Result<()> {
    disable_raw();

    let cc_switch_db = if let Some(db) = db_path {
        std::path::PathBuf::from(db)
    } else {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cc-switch")
            .join("cc-switch.db")
    };

    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let content_h = if cc_switch_db.exists() { 5 } else { 4 };
    let top_pad = if term_h > content_h + 4 {
        (term_h - content_h) / 3
    } else {
        0
    };

    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }
    term_print(&format!("  {}{}{}", BOX_TL, &"─".repeat(60), BOX_TR));
    term_print(&format!(
        "  {} {}{}",
        BOX_V,
        t("从 cc-switch 导入", "Import from cc-switch"),
        " ".repeat(25)
    ));
    term_print(&format!("  {}{}{}", BOX_BL, &"─".repeat(60), BOX_BR));
    term_print("");

    if !cc_switch_db.exists() {
        term_print(&format!(
            "  {} {}: {}",
            style("✗").red(),
            t("未找到", "Not found"),
            style(cc_switch_db.display()).dim()
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    term_print(&format!(
        "  {} {}",
        style(BULLET).green(),
        style(cc_switch_db.display()).dim()
    ));

    let confirm = match confirm_with_esc(
        &format!("  {}?", t("导入 Provider", "Import Providers")),
        true,
    ) {
        Some(v) => v,
        None => {
            enable_raw();
            return Ok(());
        }
    };

    if !confirm {
        term_print(&format!("  {}", t("已取消", "Cancelled")));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    let config = crate::config::import_from_cc_switch()?;
    let count = config.providers.len();

    let mut existing_config = if config_path.exists() {
        AppConfig::load(config_path)?
    } else {
        AppConfig::default()
    };

    for provider in config.providers {
        if !existing_config
            .providers
            .iter()
            .any(|p| p.id == provider.id)
        {
            existing_config.providers.push(provider);
        }
    }

    if existing_config
        .providers
        .iter()
        .filter(|p| p.is_default)
        .count()
        == 0
    {
        if let Some(first) = existing_config.providers.first_mut() {
            first.is_default = true;
        }
    }

    existing_config.save(config_path)?;
    reload_proxy_config(existing_config.port);

    term_print(&format!(
        "\n  {} {}",
        style(BULLET).green(),
        style(format!(
            "{} {} {} {}",
            t("已导入", "Imported"),
            count,
            t("个", ""),
            t("Provider!", "Provider(s)!")
        ))
        .green()
    ));

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Browse Presets ─────────────────────────────────────────────

fn browse_presets() -> anyhow::Result<()> {
    disable_raw();

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("  {}{}{}", BOX_TL, &"─".repeat(60), BOX_TR));
    lines.push(format!(
        "  {} {}{}",
        BOX_V,
        t("可用 Presets", "Available Presets"),
        " ".repeat(38)
    ));
    lines.push(format!("  {}{}{}", BOX_BL, &"─".repeat(60), BOX_BR));
    lines.push("".to_string());

    for category in presets::get_categories() {
        let display_name = presets::get_category_display_name(category);
        let presets_list = presets::get_presets_by_category(category);
        lines.push(format!(
            "  {} {}",
            style(BULLET).yellow(),
            style(display_name).yellow().bold()
        ));
        for preset in &presets_list {
            lines.push(format!(
                "    {} {:<16} {} ({})",
                ARROW,
                style(preset.id).green(),
                preset.display_name,
                style(preset.model).dim()
            ));
        }
        lines.push("".to_string());
    }

    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let content_h = lines.len();
    let top_pad = if term_h > content_h + 4 {
        (term_h - content_h) / 3
    } else {
        0
    };
    term_clear();
    for _ in 0..top_pad {
        term_print("");
    }
    for line in &lines {
        term_print(line);
    }

    enable_raw();
    if !after_action_menu() {
        safe_exit();
    }
    Ok(())
}

// ─── Project Management ─────────────────────────────────────────

fn manage_projects(config_path: &Path) -> anyhow::Result<()> {
    let mut config = AppConfig::load(config_path)?;

    let claude_projects_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("projects");

    let mut project_map: std::collections::HashMap<String, (usize, i64)> =
        std::collections::HashMap::new();

    if claude_projects_dir.exists() {
        find_jsonl_files(&claude_projects_dir, &mut |path| {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines().take(20) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(cwd) = value.get("cwd").and_then(|v| v.as_str()) {
                            let project_path = cwd.to_string();
                            let timestamp = value
                                .get("timestamp")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as i64;
                            let entry = project_map.entry(project_path).or_insert((0, 0));
                            entry.0 += 1;
                            if timestamp > entry.1 {
                                entry.1 = timestamp;
                            }
                            break;
                        }
                    }
                }
            }
        });
    }

    let mut projects: Vec<(String, usize, i64, String)> = project_map
        .into_iter()
        .map(|(path, (count, last_used))| {
            let provider_id = config
                .project_providers
                .get(&path)
                .cloned()
                .unwrap_or_else(|| t("未设置", "Not Set").to_string());
            let provider_name_display = config
                .providers
                .iter()
                .find(|p| p.id == provider_id)
                .map(|p| provider_name(p).to_string())
                .unwrap_or(provider_id);
            (path, count, last_used, provider_name_display)
        })
        .collect();

    projects.sort_by_key(|p| std::cmp::Reverse(p.2));

    if projects.is_empty() {
        disable_raw();
        term_print(&format!(
            "  ○ {}",
            t(
                "没有找到 Claude Code 使用记录",
                "No Claude Code session records found"
            )
        ));
        term_print(&format!(
            "  {}",
            t(
                "请确保已使用 Claude Code 创建过会话",
                "Make sure you have used Claude Code to create sessions"
            )
        ));
        enable_raw();
        if !after_action_menu() {
            safe_exit();
        }
        return Ok(());
    }

    // Arrow-navigate project list (like provider homepage)
    let mut cursor: usize = 0;

    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    loop {
        // Get default provider name for display
        let default_name = config
            .providers
            .iter()
            .find(|p| p.is_default)
            .map(|p| provider_name(p).to_string())
            .unwrap_or_else(|| t("未设置", "Not Set").to_string());

        let mut out = String::new();
        out.push_str(&format!(
            "  {} {}  {} {}\n",
            style(DIAMOND).cyan().bold(),
            t("项目管理", "Project Management"),
            t("共", "Total"),
            projects.len()
        ));
        out.push_str(&format!(
            "  {}: {}\n\n",
            t("默认 Provider", "Default Provider"),
            style(&default_name).cyan()
        ));

        for (i, (path, count, _, provider_name)) in projects.iter().enumerate() {
            let short_path = path.replace(
                &dirs::home_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "~",
            );
            let is_sel = i == cursor;
            let marker = if is_sel { "▸" } else { " " };

            let provider_display = if provider_name == t("未设置", "Not Set") {
                style(t("未设置", "Not Set")).dim().to_string()
            } else {
                style(provider_name).yellow().to_string()
            };

            let line = format!(
                "{}. {} ({}) [{}]",
                style(i + 1).dim(),
                short_path,
                count,
                provider_display,
            );

            if is_sel {
                out.push_str(&format!(
                    "  {} {}{}\n",
                    style(marker).cyan().bold(),
                    style(&line).cyan().bold(),
                    style(format!("  ← {}", t("回车配置", "Enter to config"))).dim()
                ));
            } else {
                out.push_str(&format!("    {}\n", line));
            }
        }

        out.push_str(&format!(
            "\n  {} {}  [a] {}  ESC {}\n",
            style(t("提示:", "Hint:")).dim(),
            t("回车配置", "Enter to config"),
            t("全部添加", "Add All"),
            t("返回", "back")
        ));

        term_clear();
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let content_lines = out.lines().count();
        let top_pad = if term_h > content_lines + 2 {
            (term_h - content_lines) / 3
        } else {
            0
        };
        for _ in 0..top_pad {
            term_print("");
        }
        term_print_raw(&out);

        match read_key() {
            Key::Up => {
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = projects.len() - 1;
                }
            }
            Key::Down => {
                cursor = (cursor + 1) % projects.len();
            }
            Key::Enter => {
                // Select project → show provider picker with reset option
                let selected_path = projects[cursor].0.clone();
                let result = select_provider_for_project(&config)?;
                match result {
                    ProviderAction::Assign(pid) => {
                        config
                            .project_providers
                            .insert(selected_path.clone(), pid.clone());
                        config.save(config_path)?;
                        reload_proxy_config(config.port);
                        if let Some(p) = config.providers.iter().find(|p| p.id == pid) {
                            projects[cursor].3 = provider_name(p).to_string();
                        }
                    }
                    ProviderAction::Reset => {
                        config.project_providers.remove(&selected_path);
                        config.save(config_path)?;
                        reload_proxy_config(config.port);
                        projects[cursor].3 = t("未设置", "Not Set").to_string();
                    }
                    ProviderAction::Cancel => {}
                }
            }
            Key::Char('a') => {
                // Add all unmanaged projects
                for (path, _, _, _) in &projects {
                    if !config.project_providers.contains_key(path) {
                        let default_provider = config
                            .providers
                            .iter()
                            .find(|p| p.is_default)
                            .map(|p| p.id.clone())
                            .unwrap_or_default();
                        config
                            .project_providers
                            .insert(path.clone(), default_provider);
                    }
                }
                config.save(config_path)?;
                reload_proxy_config(config.port);
                // Refresh display names
                for (path, _, _, pname) in &mut projects {
                    let pid = config
                        .project_providers
                        .get(path)
                        .cloned()
                        .unwrap_or_default();
                    *pname = config
                        .providers
                        .iter()
                        .find(|p| p.id == pid)
                        .map(|p| provider_name(p).to_string())
                        .unwrap_or(pid);
                }
            }
            Key::Esc | Key::CtrlC => break,
            _ => {}
        }
    }

    Ok(())
}

enum ProviderAction {
    Assign(String),
    Reset,
    Cancel,
}

fn select_provider_for_project(config: &AppConfig) -> anyhow::Result<ProviderAction> {
    if config.providers.is_empty() {
        return Ok(ProviderAction::Cancel);
    }

    // Raw mode is managed by the caller — do not add enable_raw/scopeguard here
    let mut cursor: usize = 0;
    let total = config.providers.len() + 1; // +1 for reset option

    loop {
        let mut out = String::new();
        out.push_str(&format!("  {}\n\n", t("选择 Provider", "Select Provider")));

        for (i, p) in config.providers.iter().enumerate() {
            let is_sel = i == cursor;
            let marker = if is_sel { "▸" } else { " " };
            let default_mark = if p.is_default {
                format!("  {}", t("默认", "Default"))
            } else {
                String::new()
            };
            let name = provider_name(p);
            let line = format!("{} ({}){}", style(name).green(), p.model, default_mark);

            if is_sel {
                out.push_str(&format!(
                    "  {} {}{}\n",
                    style(marker).cyan().bold(),
                    style(&line).cyan().bold(),
                    style(format!("  ← {}", t("回车选择", "Enter to select"))).dim()
                ));
            } else {
                out.push_str(&format!("    {}\n", line));
            }
        }

        // Reset option
        let reset_idx = config.providers.len();
        let is_reset_sel = cursor == reset_idx;
        let marker = if is_reset_sel { "▸" } else { " " };
        let reset_line = style(t(
            "重置 (移除项目 Provider 映射)",
            "Reset (Remove project-provider mapping)",
        ))
        .red();
        if is_reset_sel {
            out.push_str(&format!(
                "  {} {}{}\n",
                style(marker).cyan().bold(),
                reset_line.bold(),
                style(format!("  ← {}", t("回车选择", "Enter to select"))).dim()
            ));
        } else {
            out.push_str(&format!("    {}\n", reset_line));
        }

        out.push_str(&format!(
            "\n  {} {}  ESC {}\n",
            style(t("提示:", "Hint:")).dim(),
            t("回车选择", "Enter to select"),
            t("取消", "cancel")
        ));

        term_clear();
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let content_lines = out.lines().count();
        let top_pad = if term_h > content_lines + 2 {
            (term_h - content_lines) / 3
        } else {
            0
        };
        for _ in 0..top_pad {
            term_print("");
        }
        term_print_raw(&out);

        match read_key() {
            Key::Up => {
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = total - 1;
                }
            }
            Key::Down => {
                cursor = (cursor + 1) % total;
            }
            Key::Enter => {
                if cursor == reset_idx {
                    return Ok(ProviderAction::Reset);
                } else {
                    return Ok(ProviderAction::Assign(config.providers[cursor].id.clone()));
                }
            }
            Key::Esc | Key::CtrlC => return Ok(ProviderAction::Cancel),
            _ => {}
        }
    }
}

fn find_jsonl_files(dir: &Path, callback: &mut dyn FnMut(&Path)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_jsonl_files(&path, callback);
            } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                callback(&path);
            }
        }
    }
}
