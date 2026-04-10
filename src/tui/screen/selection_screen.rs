use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    models::ProblemSummary,
    tui::{Action, screen::Screen, utils::create_split_item},
};
pub enum InputMode {
    Editing,
    Normal,
}

pub struct SelectionScreen {
    pub all_problems: Vec<ProblemSummary>,
    pub filtered_problems: Vec<ProblemSummary>,
    pub list_state: ListState,
    pub selected_problem: Option<String>,
    pub input: Input,
    pub input_mode: InputMode,
    pub difficulty_filter: Option<u8>,
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

        let items: Vec<ListItem> = self
            .filtered_problems
            .iter()
            .map(|p| {
                let diff_color = match p.difficulty {
                    1 => Color::Green,
                    2 => Color::Yellow,
                    _ => Color::Red,
                };

                // Format: "[ID] Title (Difficulty)"
                let line = format!("[{}] {}", p.id, p.title);
                let acceptance = format!("{:.1}%", p.acceptance * 100.0);
                create_split_item(&line, &acceptance, diff_color, chunks[1].width)
            })
            .collect();

        let title = match self.difficulty_filter {
            Some(v) => match v {
                1 => " Problems (Easy)",
                2 => " Problems (Medium)",
                3 => " Problems (Hard)",
                _ => " Problems ",
            },
            None => " Problems ",
        };

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let bottom_bar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
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
    }

    fn event_loop(&mut self, key_event: &KeyEvent) -> Option<Action> {
        match self.input_mode {
            InputMode::Normal => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Quit),
                KeyCode::Down | KeyCode::Char('j') => self.next(),
                KeyCode::Up | KeyCode::Char('k') => self.previous(),
                KeyCode::Char('/') => self.input_mode = InputMode::Editing,
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected() {
                        if !self.filtered_problems.is_empty() {
                            return Some(Action::Select(self.filtered_problems[i].slug.clone()));
                        }
                    }
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
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected() {
                        if !self.filtered_problems.is_empty() {
                            return Some(Action::Select(self.filtered_problems[i].slug.clone()));
                        }
                    }
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
        None
    }
}

impl SelectionScreen {
    pub fn new(problems: Vec<ProblemSummary>) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }
        //OPTIM: Instead of cloning here, use a single allocation
        Self {
            selected_problem: None,
            filtered_problems: problems.clone(),
            all_problems: problems,
            list_state,
            input: Input::default(),
            input_mode: InputMode::Normal,
            difficulty_filter: None,
        }
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.all_problems.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    // Move cursor up
    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.all_problems.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
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
        let mut filtered = vec![];
        if let Some(difficulty) = self.difficulty_filter {
            for problem in &self.all_problems {
                if problem.difficulty == difficulty {
                    filtered.push(problem.clone());
                }
            }
            self.filtered_problems = filtered;
        } else {
            self.filtered_problems = self.all_problems.clone()
        }
    }

    pub fn update_search(&mut self) {
        let query = self.input.value();
        if query.is_empty() {
            //OPTIM: Instead of cloning here, use a single allocation
            self.filtered_problems = self.all_problems.clone();
            if let Some(diff) = self.difficulty_filter {
                self.switch_difficulty(diff);
            }
        } else {
            let matcher = SkimMatcherV2::default();
            let mut matched = Vec::new();

            for problem in &self.filtered_problems {
                let search_target = format!("{} {}", problem.title, problem.id);
                if let Some(score) = matcher.fuzzy_match(&search_target, query) {
                    //OPTIM: Instead of cloning here, use a single allocation
                    matched.push((score, problem.clone()));
                }
            }

            matched.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered_problems = matched.into_iter().map(|(_, p)| p).collect();
        }

        //NOTE: Reset the cursor to 0 when the list changes so we don't panic out of bounds
        if !self.filtered_problems.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }
}
