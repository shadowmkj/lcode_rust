use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::{Action, screen::Screen, utils::create_split_item};
pub enum InputMode {
    Editing,
    Normal,
}

pub struct HelpScreen;

impl Screen for HelpScreen {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(frame.area());

        let help_area = chunks[0];

        let items: Vec<ListItem> = vec![
            create_split_item("Global", "", Color::White, help_area.width),
            create_split_item(
                "Tab",
                "Switch between Problems and Help",
                Color::Cyan,
                help_area.width,
            ),
            create_split_item(
                "q / Esc",
                "Quit the application (from any tab)",
                Color::Cyan,
                help_area.width,
            ),
            create_split_item("", "", Color::White, help_area.width),
            create_split_item("Problems", "", Color::White, help_area.width),
            create_split_item(
                "/",
                "Start searching problems",
                Color::Green,
                help_area.width,
            ),
            create_split_item(
                "j / k or ↓ / ↑",
                "Move selection down / up",
                Color::Green,
                help_area.width,
            ),
            create_split_item(
                "Enter",
                "Select the highlighted problem",
                Color::Green,
                help_area.width,
            ),
            create_split_item(
                "1 / 2 / 3 / 4",
                "Filter by difficulty (Easy / Med / Hard / All)",
                Color::Green,
                help_area.width,
            ),
            create_split_item(
                "Ctrl+j / Ctrl+k",
                "Move selection while searching",
                Color::Green,
                help_area.width,
            ),
            create_split_item("", "", Color::White, help_area.width),
            create_split_item("Search mode", "", Color::White, help_area.width),
            create_split_item(
                "Esc",
                "Return to list from search",
                Color::Yellow,
                help_area.width,
            ),
            create_split_item(
                "Esc",
                "Return to list from search",
                Color::Yellow,
                help_area.width,
            ),
        ];

        let help_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help - Key Bindings ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(help_list, help_area);

        // Bottom hint bar
        let hint = Paragraph::new("Press Tab to return to Problems tab.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[1]);
    }

    fn event_loop(&mut self, key_event: &KeyEvent) -> Option<Action> {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            _ => None,
        }
    }
}

impl HelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self
    }
}
