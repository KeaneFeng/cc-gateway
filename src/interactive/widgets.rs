//! Terminal I/O, raw mode, and UX widgets.
//!
//! UX improvements over original:
//! - input_with_cursor: visible cursor in text editing
//! - bottom_confirm: confirm at bottom bar instead of full-page
//! - type_y_to_confirm: type 'Y' directly instead of arrow navigation

use super::types::*;
use console::style;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};

// ─── Terminal Output ────────────────────────────────────────────

pub fn term_clear() {
    let _ = execute!(
        stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    );
}

pub fn term_print(line: &str) {
    let out = format!("{}\r\n", line);
    let _ = write!(stdout(), "{}", out);
    let _ = stdout().flush();
}

pub fn term_print_raw(text: &str) {
    let converted = text.replace('\n', "\r\n");
    let _ = write!(stdout(), "{}", converted);
    let _ = stdout().flush();
}

pub fn center_content(text: &str) -> String {
    let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
    let lines: Vec<&str> = text.lines().collect();
    let content_h = lines.len();
    let top_pad = if term_h > content_h + 4 {
        (term_h - content_h) / 3
    } else {
        0
    };
    let mut result = String::new();
    for _ in 0..top_pad {
        result.push('\n');
    }
    for line in &lines {
        result.push_str(line);
        result.push('\n');
    }
    result
}

// ─── Raw Mode Control ───────────────────────────────────────────

pub fn enable_raw() {
    if !is_raw_mode() {
        let _ = execute!(
            stdout(),
            terminal::EnterAlternateScreen,
            crossterm::event::DisableMouseCapture
        );
        let _ = terminal::enable_raw_mode();
        let _ = execute!(stdout(), cursor::Hide);
        set_raw_mode(true);
    }
}

pub fn disable_raw() {
    if is_raw_mode() {
        let _ = execute!(stdout(), cursor::Show);
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        set_raw_mode(false);
    }
}

pub fn safe_exit() -> ! {
    disable_raw();
    std::process::exit(0)
}

// ─── Key Reading ────────────────────────────────────────────────

pub fn read_key() -> Key {
    loop {
        match event::poll(std::time::Duration::from_secs(1)) {
            Ok(true) => {}
            Ok(false) => return Key::Other,
            Err(_) => return Key::Other,
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
            Ok(_) => continue,
            Err(_) => return Key::Other,
        }
    }
}

fn drain_event_queue() {
    while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
        let _ = event::read();
    }
}

// ─── String Utilities ───────────────────────────────────────────

pub fn cjk_width(s: &str) -> usize {
    s.chars()
        .map(|c| if c.width().unwrap_or(1) > 1 { 2 } else { 1 })
        .sum()
}

pub fn cjk_truncate(s: &str, max_w: usize) -> String {
    let mut result = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = if c.width().unwrap_or(1) > 1 { 2 } else { 1 };
        if w + cw > max_w {
            break;
        }
        result.push(c);
        w += cw;
    }
    result
}

pub fn pad(s: &str, width: usize) -> String {
    let w = cjk_width(s);
    if w >= width {
        cjk_truncate(s, width)
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

// ─── Widget: Select ─────────────────────────────────────────────

pub fn custom_select(prompt: &str, items: &[String], default: usize) -> Option<usize> {
    custom_select_at(prompt, items, default, 0)
}

pub fn custom_select_at(
    prompt: &str,
    items: &[String],
    default: usize,
    _at_row: u16,
) -> Option<usize> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut cursor = default;
    let mut scroll_offset = 0usize;

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let header_rows = 2;
        let footer_rows = 2;
        let view_count = term_h.saturating_sub(header_rows + footer_rows).max(1);

        if cursor >= items.len() {
            cursor = items.len().saturating_sub(1);
        }
        if cursor < scroll_offset {
            scroll_offset = cursor;
        }
        if cursor >= scroll_offset + view_count {
            scroll_offset = cursor - view_count + 1;
        }
        let visible_end = (scroll_offset + view_count).min(items.len());

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("  {}", style(prompt).bold()));
        for (i, item) in items
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_end - scroll_offset)
        {
            let marker = if i == cursor {
                format!("{} ", style("▸").cyan().bold())
            } else {
                "  ".to_string()
            };
            let styled_item = if i == cursor {
                style(item).cyan().to_string()
            } else {
                item.to_string()
            };
            lines.push(format!("  {}{}", marker, styled_item));
        }
        lines.push(format!(
            "  {} ↑↓ {}  {}  ESC {}",
            style(t("提示:", "Hint:")).dim(),
            t("选择", "select"),
            style(t("回车", "Enter")).cyan(),
            t("确认", "confirm"),
        ));

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

        match read_key() {
            Key::Up => {
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = items.len() - 1;
                }
            }
            Key::Down => {
                if cursor < items.len() - 1 {
                    cursor += 1;
                } else {
                    cursor = 0;
                }
            }
            Key::Enter | Key::Space => return Some(cursor),
            Key::Esc => return None,
            Key::CtrlC => return None,
            _ => {}
        }
    }
}

// ─── Widget: Text Input with Visible Cursor ─────────────────────
/// UX improvement: cursor blinks at current position so user knows where they're editing.

pub fn input_with_esc(prompt: &str, default: Option<&str>) -> Option<String> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut buffer = default.unwrap_or("").to_string();
    let mut cursor_pos = buffer.len();
    let mut blink_on = true;

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let top_pad = if term_h > 8 { (term_h - 6) / 3 } else { 0 };

        // Build display with cursor
        let before = &buffer[..cursor_pos];
        let after = &buffer[cursor_pos..];
        let cursor_char = if blink_on { "█" } else { " " };
        let display = format!("> {}{}{}", before, style(cursor_char).on_blue(), after);

        term_clear();
        for _ in 0..top_pad {
            term_print("");
        }
        term_print(&format!("  {}", style(prompt).bold()));
        term_print(&format!("  {}", display));
        term_print("");
        term_print(&format!(
            "  {} {}  {}  ESC {}",
            style(t("提示:", "Hint:")).dim(),
            t("← → 移动光标", "← → move cursor"),
            t("Enter 确认", "Enter confirm"),
            t("取消", "cancel"),
        ));

        blink_on = !blink_on;

        match read_key() {
            Key::Enter => {
                let result = buffer.trim().to_string();
                if result.is_empty() && default.is_none() {
                    continue;
                }
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

// ─── Widget: Bottom Bar Confirmation ───────────────────────────
/// UX improvement: shows confirmation bar at the bottom of the current view
/// instead of opening a new full-page dialog. Type 'y' or 'n' directly.

pub fn bottom_confirm(prompt: &str, default: bool) -> Option<bool> {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut choice = default;

    loop {
        let term_h = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let top_pad = if term_h > 6 { (term_h - 4) / 3 } else { 0 };

        term_clear();
        for _ in 0..top_pad {
            term_print("");
        }
        term_print(&format!("  {}", style(prompt).bold()));
        term_print("");

        let yes_label = if choice {
            style(" Y ").cyan().bold().on_bright_black()
        } else {
            style(" y ").dim()
        };
        let no_label = if !choice {
            style(" N ").red().bold().on_bright_black()
        } else {
            style(" n ").dim()
        };

        term_print(&format!(
            "  {} {} {}  {}",
            style(t("确认:", "Confirm:")).dim(),
            yes_label,
            no_label,
            style(t(
                "← → 切换, Enter/Y/N 确认, ESC 取消",
                "← → toggle, Enter/Y/N confirm, ESC cancel"
            ))
            .dim(),
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

// ─── Widget: After Action Menu ──────────────────────────────────

pub fn after_action_menu() -> bool {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let options = vec![t("返回", "Back").to_string(), t("退出", "Exit").to_string()];

    match custom_select(&format!("  {}", t("下一步", "Next Step")), &options, 0) {
        Some(1) => {
            disable_raw();
            term_print("");
            false
        }
        Some(0) | None => {
            enable_raw();
            true
        }
        _ => true,
    }
}

// ─── Widget: Form Editor ────────────────────────────────────────
use crate::config::ProviderUpdate;
/// UX improvement:
/// - When editing a field, cursor is visible
/// - ESC saves and returns (consistent behavior)
use crossterm::event::UnicodeWidthChar;

pub fn provider_form_editor(title: &str, fields: &mut Vec<FormField>) {
    enable_raw();
    let _guard = scopeguard::guard((), |_| disable_raw());

    let mut cursor: usize = 0;
    let mut scroll_offset: usize = 0;

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

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("  {} {}", DIAMOND, style(title).cyan().bold()));
        lines.push(format!(
            "  {} ↑↓ {}  {}  ESC {}",
            style(t("提示:", "Hint:")).dim(),
            t("移动", "navigate"),
            t("空格/回车 编辑", "Space/Enter to edit"),
            t("保存返回", "save & back"),
        ));

        for (i, f) in fields
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_end - scroll_offset)
        {
            let required_mark = if f.required { "*" } else { " " };
            let value_display = if f.is_confirm {
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

        if scroll_offset > 0 || visible_end < fields.len() {
            lines.push(format!("  · {}/{}", cursor + 1, fields.len()));
        }
        lines.push(format!(
            "  ↑↓ {}  {}  ESC {}",
            t("选择字段", "select field"),
            t("空格/回车 编辑", "Space/Enter to edit"),
            t("保存返回", "save & back"),
        ));

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

        match read_key() {
            Key::Up => {
                if cursor > 0 {
                    cursor -= 1;
                } else {
                    cursor = fields.len() - 1;
                }
            }
            Key::Down => {
                if cursor < fields.len() - 1 {
                    cursor += 1;
                } else {
                    cursor = 0;
                }
            }
            Key::Space | Key::Enter => {
                disable_raw();
                let f = &mut fields[cursor];
                if f.is_confirm {
                    let current = f.value == "true";
                    if let Some(v) = bottom_confirm(&format!("* {}", f.label), current) {
                        f.value = if v {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        };
                    }
                } else if f.is_select {
                    let items: Vec<String> =
                        f.select_options.iter().map(|s| s.to_string()).collect();
                    let current_idx = f
                        .select_options
                        .iter()
                        .position(|s| *s == f.value)
                        .unwrap_or(0);
                    if let Some(idx) = custom_select(&format!("* {}", f.label), &items, current_idx)
                    {
                        f.value = f.select_options[idx].to_string();
                    }
                } else {
                    let default = if f.value.is_empty() {
                        None
                    } else {
                        Some(f.value.as_str())
                    };
                    if let Some(v) = input_with_esc(
                        &format!("{} {}", if f.required { "*" } else { " " }, f.label),
                        default,
                    ) {
                        f.value = v;
                    }
                }
                enable_raw();
            }
            Key::Esc => return,
            Key::CtrlC => return,
            _ => {}
        }
    }
}
