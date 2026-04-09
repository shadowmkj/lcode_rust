use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::models::ProblemSummary;

pub enum InputMode {
    Normal,
    Editing,
}

/// Holds the state of the application
pub struct App {
    pub should_quit: bool,
    pub all_problems: Vec<ProblemSummary>,
    pub filtered_problems: Vec<ProblemSummary>,
    pub list_state: ListState,
    pub selected_problem: Option<String>,
    pub input: Input,
    pub input_mode: InputMode,
    pub difficulty_filter: Option<u8>,
    pub debug: String,
}

impl App {
    pub fn new(problems: Vec<ProblemSummary>) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }
        //OPTIM: Instead of cloning here, use a single allocation
        Self {
            should_quit: false,
            selected_problem: None,
            filtered_problems: problems.clone(),
            all_problems: problems,
            list_state,
            input: Input::default(),
            input_mode: InputMode::Normal,
            difficulty_filter: None,
            debug: "Nothing".to_string(),
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

/// The main entry point for the TUI
pub async fn run_tui(
    problems: Vec<ProblemSummary>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // 1. Setup Terminal (Enter raw mode and alternate screen)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Create app state and run the event loop
    let mut app = App::new(problems);
    let res = run_app(&mut terminal, &mut app).await;

    // 3. Restore Terminal (Crucial: If we don't do this, the terminal will break on exit)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error running TUI: {:?}", err);
    }

    Ok(app.selected_problem)
}

/// The Event Loop
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        let _ = terminal.draw(|f| ui(f, app));

        // Poll for keystrokes (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            match app.input_mode {
                InputMode::Normal => {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                                KeyCode::Down | KeyCode::Char('j') => app.next(),
                                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                                KeyCode::Char('/') => app.input_mode = InputMode::Editing,
                                KeyCode::Enter => {
                                    if let Some(i) = app.list_state.selected() {
                                        if !app.filtered_problems.is_empty() {
                                            app.selected_problem =
                                                Some(app.filtered_problems[i].slug.clone());
                                            app.should_quit = true;
                                        }
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if let Some(number) = c.to_digit(10) {
                                        app.switch_difficulty(number as u8);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                InputMode::Editing => {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Esc => {
                                    app.input_mode = InputMode::Normal;
                                }
                                KeyCode::Enter => {
                                    if let Some(i) = app.list_state.selected() {
                                        if !app.filtered_problems.is_empty() {
                                            app.selected_problem =
                                                Some(app.filtered_problems[i].slug.clone());
                                            app.should_quit = true;
                                        }
                                    }
                                }
                                _ => {
                                    app.input.handle_event(&Event::Key(key));
                                    if let Some(diff) = app.difficulty_filter {
                                        app.switch_difficulty(diff);
                                    }
                                    app.update_search();
                                }
                            }
                        }
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// The View (Renders widgets to the screen)
fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(f.area());

    match app.input_mode {
        InputMode::Normal => (
            "Press '/' to search, 'j'/'k' to scroll, 'Enter' to select, 'q' to quit.",
            Style::default().fg(Color::DarkGray),
        ),
        InputMode::Editing => (
            "Type to filter, press 'Esc' or 'Enter' to return to list.",
            Style::default().fg(Color::Yellow),
        ),
    };

    let title = format!(" Search ({} matches) ", app.filtered_problems.len());
    let input_widget = Paragraph::new(app.input.value())
        .style(match app.input_mode {
            InputMode::Editing => Style::default().fg(Color::Yellow),
            InputMode::Normal => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input_widget, chunks[0]);

    // Handle blinking cursor in Editing mode
    if let InputMode::Editing = app.input_mode {
        // We set the cursor position right after the text
        f.set_cursor(
            chunks[0].x + app.input.visual_cursor() as u16 + 1,
            chunks[0].y + 1,
        );
    }

    let items: Vec<ListItem> = app
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
            ListItem::new(line).style(Style::default().fg(diff_color))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Problems ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol(">> "); // Arrow pointing to selected item

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);
}
