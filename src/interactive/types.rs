//! Shared types and constants for the TUI dashboard.

use std::sync::atomic::{AtomicBool, Ordering};

// ─── Box Drawing Characters ────────────────────────────────────
pub const BOX_TL: &str = "╭";
pub const BOX_TR: &str = "╮";
pub const BOX_BL: &str = "╰";
pub const BOX_BR: &str = "╯";
pub const BOX_H: &str = "─";
pub const BOX_V: &str = "│";
pub const BULLET: &str = "●";
pub const ARROW: &str = "▸";
pub const DIAMOND: &str = "◆";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── Raw Mode State ─────────────────────────────────────────────
static RAW_MODE: AtomicBool = AtomicBool::new(false);

pub fn is_raw_mode() -> bool {
    RAW_MODE.load(Ordering::Relaxed)
}

pub fn set_raw_mode(on: bool) {
    RAW_MODE.store(on, Ordering::Relaxed);
}

// ─── Key Abstraction ───────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
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

// ─── Form Field ─────────────────────────────────────────────────
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub required: bool,
    pub is_confirm: bool,
    pub is_select: bool,
    pub select_options: Vec<&'static str>,
}

// ─── Language / i18n ───────────────────────────────────────────
use std::sync::atomic::AtomicU8;
static LANG: AtomicU8 = AtomicU8::new(0); // 0=zh, 1=en

pub fn t(zh: &'static str, en: &'static str) -> &'static str {
    match LANG.load(Ordering::Relaxed) {
        0 => zh,
        _ => en,
    }
}

pub fn get_lang() -> &'static str {
    match LANG.load(Ordering::Relaxed) {
        0 => "zh",
        _ => "en",
    }
}

pub fn toggle_lang() {
    LANG.fetch_xor(1, Ordering::Relaxed);
}

// ─── Shortcut Items ─────────────────────────────────────────────
pub struct ShortcutItem {
    pub key: &'static str,
    pub name_zh: &'static str,
    pub name_en: &'static str,
    pub desc_zh: &'static str,
    pub desc_en: &'static str,
}

pub const SHORTCUTS: &[ShortcutItem] = &[
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
