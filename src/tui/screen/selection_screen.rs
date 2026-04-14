use std::rc::Rc;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    models::{ProblemSummary, UserDetail},
    tui::{Action, screen::Screen},
};
pub enum InputMode {
    Editing,
    Normal,
}

pub struct SelectionScreen {
    pub all_problems: Rc<[ProblemSummary]>,
    pub filtered_problems: Vec<usize>,
    pub table_state: TableState,
    pub selected_problem: Option<String>,
    pub input: Input,
    pub input_mode: InputMode,
    pub difficulty_filter: Option<u8>,
    pub previous_key: Option<KeyCode>,
    pub user_detail: Option<UserDetail>,
}

impl Screen for SelectionScreen {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(frame.area());

        let title = format!(" Search ({} matches) ", self.filtered_problems.len());
        let input_widget = Paragraph::new(self.input.value())
            .style(match self.input_mode {
                InputMode::Editing => Style::default().fg(Color::Yellow),
                InputMode::Normal => Style::default(),
            })
            .block(Block::default().borders(Borders::ALL).title(title));

        frame.render_widget(input_widget, chunks[0]);

        // Handle blinking cursor in Editing mode
        if let InputMode::Editing = self.input_mode {
            // We set the cursor position right after the text
            frame.set_cursor_position((
                chunks[0].x + self.input.visual_cursor() as u16 + 1,
                chunks[0].y + 1,
            ));
        }

        let title = match self.difficulty_filter {
            Some(v) => match v {
                1 => " Problems (Easy)",
                2 => " Problems (Medium)",
                3 => " Problems (Hard)",
                _ => " Problems ",
            },
            None => " Problems ",
        };

        let header_cells = ["ID", "Name", "Acceptance", "Topics", "Premium?", "Done"]
            .into_iter()
            .map(|h| Cell::from(h).style(Style::default().fg(Color::Yellow)));
        let header = Row::new(header_cells).style(Style::default());

        let rows = self.filtered_problems.iter().map(|&p| {
            let p = &self.all_problems[p];
            let diff_color = match p.difficulty {
                1 => Color::Green,
                2 => Color::Yellow,
                _ => Color::Red,
            };

            let id_cell = Cell::from(Span::styled(
                format!("[{}]", p.id),
                Style::default().fg(diff_color),
            ));
            let name_cell = Cell::from(Span::styled(
                p.title.as_str(),
                Style::default().fg(diff_color),
            ));
            let acceptance_text = format!("{:.1}%", p.acceptance * 100.0);
            let acceptance_cell = Cell::from(acceptance_text);
            let done_text = if let Some(status) = &p.status {
                match status.as_str() {
                    "ac" => "",
                    "notac" => "",
                    _ => "",
                }
            } else {
                ""
            };

            let done_cell = match done_text {
                "" => Cell::from(done_text).style(Style::default().fg(Color::Green)),
                _ => Cell::from(done_text).style(Style::default().fg(Color::White)),
            };

            let premium_text = match &p.is_paid {
                true => "󰌾",
                false => "",
            };

            let premium_cell = Cell::from(premium_text).style(Style::default().fg(Color::Red));

            let slice = p.topics.get(..1).unwrap_or(&p.topics);
            let topics_cell = Cell::from(slice.join("|"));

            Row::new(vec![
                id_cell,
                name_cell,
                acceptance_cell,
                topics_cell,
                premium_cell,
                done_cell,
            ])
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),
                Constraint::Percentage(45),
                Constraint::Min(10),
                Constraint::Fill(10),
                Constraint::Min(8),
                Constraint::Length(6),
            ],
        )
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL))
        .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, chunks[1], &mut self.table_state);

        let bottom_bar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .split(chunks[2]);

        let instructions = match self.input_mode {
            InputMode::Normal => (
                "Press '/' to search, 'j'/'k' to scroll, 'Enter' to select, 'q' to quit.",
                Style::default().fg(Color::DarkGray),
            ),
            InputMode::Editing => (
                "Type to filter, press 'Esc' to return to list, press 'Enter' to select.",
                Style::default().fg(Color::Yellow),
            ),
        };
        let instructions = Paragraph::new(instructions.0).style(instructions.1);
        frame.render_widget(instructions, bottom_bar[0]);

        if let InputMode::Normal = self.input_mode {
            let filter_hint = Paragraph::new(
                "Press to filter based on difficulty => 1: Easy, 2: Medium, 3: Hard, 4: All ",
            )
            .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(filter_hint, bottom_bar[1]);
        }

        let help_hint =
            Paragraph::new("Press ? to view help.").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help_hint, bottom_bar[2]);
    }

    fn event_loop(&mut self, key_event: &KeyEvent) -> Option<Action> {
        if let KeyCode::Enter = key_event.code {
            if let Some(i) = self.table_state.selected()
                && !self.filtered_problems.is_empty()
            {
                let index = self.filtered_problems[i];
                let selected_problem = &self.all_problems[index];
                if let Some(user) = &self.user_detail {
                    if selected_problem.is_paid && !user.is_premium {
                        return Some(Action::ShowMessage(
                            "This problem is premium. please subscribe to access it.".to_string(),
                        ));
                    }
                }

                return Some(Action::Select(self.all_problems[index].slug.clone()));
            }
        }
        match self.input_mode {
            InputMode::Normal => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Quit),
                KeyCode::Down | KeyCode::Char('j') => self.next(),
                KeyCode::Up | KeyCode::Char('k') => self.previous(),
                KeyCode::Left | KeyCode::Char('h') => self.table_state.select_next_column(),
                KeyCode::Right | KeyCode::Char('l') => self.table_state.select_previous_column(),
                KeyCode::Char('/') => self.input_mode = InputMode::Editing,
                KeyCode::Char('g') => {
                    if let Some(prev_key) = self.previous_key
                        && prev_key == KeyCode::Char('g')
                    {
                        self.table_state.select(Some(0));
                    }
                }
                KeyCode::Char('G') => {
                    self.table_state.select_last();
                }
                KeyCode::Char('d') => {
                    self.table_state.scroll_down_by(10);
                }
                KeyCode::Char('u') => {
                    self.table_state.scroll_up_by(10);
                }
                KeyCode::Char(c) => {
                    if let Some(number) = c.to_digit(10) {
                        self.switch_difficulty(number as u8);
                    }
                }
                _ => {}
            },

            InputMode::Editing => match key_event.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char('j') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.next();
                }
                KeyCode::Char('k') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.previous();
                }
                _ => {
                    self.input.handle_event(&Event::Key(*key_event));
                    if let Some(diff) = self.difficulty_filter {
                        self.switch_difficulty(diff);
                    }
                    self.update_search();
                }
            },
        }
        self.previous_key = Some(key_event.code);
        None
    }
}

impl SelectionScreen {
    pub fn new(problems: Rc<[ProblemSummary]>, user_detail: Option<UserDetail>) -> Self {
        let mut list_state = TableState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }

        Self {
            selected_problem: None,
            filtered_problems: (0..problems.len()).collect(),
            all_problems: problems,
            table_state: list_state,
            input: Input::default(),
            input_mode: InputMode::Normal,
            difficulty_filter: None,
            previous_key: None,
            user_detail,
        }
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.all_problems.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    // Move cursor up
    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.all_problems.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn switch_difficulty(&mut self, difficulty: u8) {
        if difficulty > 0 && difficulty < 4 {
            self.difficulty_filter = Some(difficulty)
        } else {
            self.difficulty_filter = None;
        }
        self.filter_problems();
    }

    pub fn filter_problems(&mut self) {
        self.filtered_problems = match self.difficulty_filter {
            Some(difficulty) => self
                .all_problems
                .iter()
                .enumerate()
                .filter(|(_, p)| p.difficulty == difficulty)
                .map(|(i, _)| i)
                .collect(),
            None => (0..self.all_problems.len()).collect(),
        }
    }

    pub fn update_search(&mut self) {
        let query = self.input.value();
        if query.is_empty() {
            self.filtered_problems = (0..self.all_problems.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut matched: Vec<(i64, usize)> = Vec::new();
            for (idx, problem) in self.all_problems.iter().enumerate() {
                if !self.filtered_problems.contains(&idx) {
                    continue;
                }
                let search_target = format!("{} {}", problem.title, problem.id);
                if let Some(score) = matcher.fuzzy_match(&search_target, query) {
                    matched.push((score, idx));
                }
            }

            matched.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered_problems = matched.into_iter().map(|(_, p)| p).collect();
        }

        //NOTE: Reset the cursor to 0 when the list changes so we don't panic out of bounds
        if !self.filtered_problems.is_empty() {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }
}
