//! Simple Mouse-enabled TUI
//!
//! A simpler, more reliable TUI implementation

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton},
    execute,
    style::{self, Color, Stylize},
    terminal::{self, ClearType},
};
use std::io::{self, Write};
use std::time::Duration;

/// Provider display item
pub struct ProviderItem {
    pub id: String,
    pub name: String,
    pub model: String,
    pub is_default: bool,
}

/// Simple TUI state
pub struct SimpleTui {
    providers: Vec<ProviderItem>,
    selected: usize,
    should_exit: bool,
}

impl SimpleTui {
    pub fn new(providers: Vec<ProviderItem>) -> Self {
        Self {
            providers,
            selected: 0,
            should_exit: false,
        }
    }

    /// Run the TUI and return the selected action
    pub fn run(&mut self) -> io::Result<Option<Action>> {
        let mut stdout = io::stdout();

        // Enable raw mode and mouse capture
        terminal::enable_raw_mode()?;
        execute!(stdout, event::EnableMouseCapture, cursor::Hide)?;

        let result = self.event_loop(&mut stdout);

        // Cleanup
        execute!(stdout, event::DisableMouseCapture, cursor::Show)?;
        terminal::disable_raw_mode()?;

        result
    }

    fn event_loop(&mut self, stdout: &mut io::Stdout) -> io::Result<Option<Action>> {
        // Initial render
        self.render(stdout)?;

        loop {
            // Poll for events with timeout
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if let Some(action) = self.handle_key(key_event) {
                            return Ok(Some(action));
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        if let Some(action) = self.handle_mouse(mouse_event) {
                            return Ok(Some(action));
                        }
                    }
                    _ => {}
                }
            }

            if self.should_exit {
                return Ok(Some(Action::Exit));
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Down => {
                if self.selected < self.providers.len() + 6 { // +6 for menu items
                    self.selected += 1;
                }
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.execute_selected()
            }
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Exit),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Add),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Delete),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::SetDefault),
            KeyCode::Char('t') => Some(Action::Test),
            KeyCode::Char('u') => Some(Action::Usage),
            KeyCode::Char('p') => Some(Action::Projects),
            KeyCode::Char('i') => Some(Action::Import),
            KeyCode::Char('x') => Some(Action::Exit),
            _ => None
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Calculate which item was clicked based on row
                let row = mouse.row;
                if row >= 4 && row < 4 + self.providers.len() as u16 {
                    self.selected = (row - 4) as usize;
                    self.execute_selected()
                } else if row >= 4 + self.providers.len() as u16 + 1 {
                    // Menu items area
                    let menu_start = 4 + self.providers.len() as u16 + 2;
                    let menu_row = row - menu_start;
                    match menu_row {
                        0 => Some(Action::Add),
                        1 => Some(Action::Delete),
                        2 => Some(Action::SetDefault),
                        3 => Some(Action::Test),
                        4 => Some(Action::Usage),
                        5 => Some(Action::Projects),
                        6 => Some(Action::Import),
                        7 => Some(Action::Exit),
                        _ => None
                    }
                } else {
                    None
                }
            }
            _ => None
        }
    }

    fn execute_selected(&self) -> Option<Action> {
        if self.selected < self.providers.len() {
            // Selected a provider - edit it
            Some(Action::Edit(self.providers[self.selected].id.clone()))
        } else {
            // Selected a menu item
            let menu_idx = self.selected - self.providers.len();
            match menu_idx {
                0 => Some(Action::Add),
                1 => Some(Action::Delete),
                2 => Some(Action::SetDefault),
                3 => Some(Action::Test),
                4 => Some(Action::Usage),
                5 => Some(Action::Projects),
                6 => Some(Action::Import),
                7 => Some(Action::Exit),
                _ => None
            }
        }
    }

    fn render(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        // Header
        execute!(
            stdout,
            cursor::MoveTo(1, 0),
            style::PrintStyledContent("cc-gateway v0.3.0".with(Color::Cyan).bold())
        )?;
        execute!(
            stdout,
            cursor::MoveTo(1, 1),
            style::Print("─".repeat(60))
        )?;

        // Provider list
        execute!(
            stdout,
            cursor::MoveTo(1, 2),
            style::PrintStyledContent("Provider 列表:".with(Color::White).bold())
        )?;

        for (i, provider) in self.providers.iter().enumerate() {
            let y = 4 + i as u16;
            let is_selected = i == self.selected;
            let prefix = if is_selected { "► " } else { "  " };
            let default_mark = if provider.is_default { " ⭐" } else { "" };

            execute!(stdout, cursor::MoveTo(1, y))?;

            if is_selected {
                execute!(
                    stdout,
                    style::PrintStyledContent(
                        format!("{}{} ({}){}", prefix, provider.name, provider.model, default_mark)
                            .with(Color::Black)
                            .on(Color::White)
                    )
                )?;
            } else {
                execute!(
                    stdout,
                    style::Print(format!("{}{} ({}){}", prefix, provider.name, provider.model, default_mark))
                )?;
            }
        }

        // Menu separator
        let menu_y = 4 + self.providers.len() as u16 + 1;
        execute!(
            stdout,
            cursor::MoveTo(1, menu_y),
            style::Print("─".repeat(60))
        )?;

        // Menu items
        let menu_items = vec![
            ("[A] 添加 Provider", self.selected == self.providers.len()),
            ("[D] 删除 Provider", self.selected == self.providers.len() + 1),
            ("[S] 设置默认 Provider", self.selected == self.providers.len() + 2),
            ("[T] 测试连接", self.selected == self.providers.len() + 3),
            ("[U] 查看统计", self.selected == self.providers.len() + 4),
            ("[P] 项目管理", self.selected == self.providers.len() + 5),
            ("[I] 从 cc-switch 导入", self.selected == self.providers.len() + 6),
            ("[X] 退出", self.selected == self.providers.len() + 7),
        ];

        for (i, (label, is_selected)) in menu_items.iter().enumerate() {
            let y = menu_y + 1 + i as u16;
            execute!(stdout, cursor::MoveTo(3, y))?;

            if *is_selected {
                execute!(
                    stdout,
                    style::PrintStyledContent(
                        format!("► {}", label).with(Color::Black).on(Color::White)
                    )
                )?;
            } else {
                execute!(stdout, style::Print(format!("  {}", label)))?;
            }
        }

        // Footer
        let footer_y = menu_y + menu_items.len() as u16 + 2;
        execute!(
            stdout,
            cursor::MoveTo(1, footer_y),
            style::Print("─".repeat(60))
        )?;
        execute!(
            stdout,
            cursor::MoveTo(1, footer_y + 1),
            style::PrintStyledContent(
                "↑↓选择 Enter确认 ESC返回 鼠标点击可用".with(Color::DarkGrey)
            )
        )?;

        stdout.flush()?;
        Ok(())
    }
}

/// Actions that can be performed
#[derive(Debug)]
pub enum Action {
    Edit(String),    // Edit provider by ID
    Add,
    Delete,
    SetDefault,
    Test,
    Usage,
    Projects,
    Import,
    Back,
    Exit,
}

/// Create provider items from config
pub fn create_provider_items(providers: &[crate::config::ProviderConfig]) -> Vec<ProviderItem> {
    providers
        .iter()
        .map(|p| {
            let display_name = p.display_name.as_deref().unwrap_or(&p.name);
            ProviderItem {
                id: p.id.clone(),
                name: display_name.to_string(),
                model: p.model.clone(),
                is_default: p.is_default,
            }
        })
        .collect()
}
