use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::io;

use crate::models::ProblemSummary;

/// Holds the state of the application
pub struct App {
    pub should_quit: bool,
    pub problems: Vec<ProblemSummary>,
    pub list_state: ListState,
    pub selected_problem: Option<String>,
}

impl App {
    pub fn new(problems: Vec<ProblemSummary>) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }
        Self {
            should_quit: false,
            selected_problem: None,
            problems,
            list_state,
        }
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.problems.len() - 1 {
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
                    self.problems.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
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
        // Draw the UI
        let _ = terminal.draw(|f| ui(f, app));

        // Poll for keystrokes (non-blocking)
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Enter => {
                        if let Some(i) = app.list_state.selected() {
                            app.selected_problem = Some(app.problems[i].slug.clone());
                            app.should_quit = true;
                        }
                    }
                    _ => {}
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
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(f.area());

    let items: Vec<ListItem> = app
        .problems
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
