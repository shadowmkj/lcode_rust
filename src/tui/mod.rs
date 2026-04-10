pub mod screen;
mod utils;

use crate::tui::screen::help_screen::HelpScreen;
use crate::tui::screen::selection_screen::SelectionScreen;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    widgets::ListState,
};
use screen::Screen;
use std::io;

use crate::models::ProblemSummary;

#[derive(Default)]
pub enum Tab {
    #[default]
    Selection,
    Help,
}

/// Holds the state of the application
pub struct App {
    pub should_quit: bool,
    pub problems: Vec<ProblemSummary>,
    pub tab: Tab,
    pub selection_screen: SelectionScreen,
    pub help_screen: HelpScreen,
    pub selected_problem: Option<String>,
}

pub enum Action {
    Quit,
    Select(String),
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
            selection_screen: SelectionScreen::new(problems.clone()),
            problems,
            tab: Tab::default(),
            selected_problem: None,
            help_screen: HelpScreen::new(),
        }
    }

    pub fn switch(&mut self) {
        self.tab = match self.tab {
            Tab::Help => Tab::Selection,
            Tab::Selection => Tab::Help,
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
        let screen: &mut dyn Screen = match app.tab {
            Tab::Selection => &mut app.selection_screen,
            Tab::Help => &mut app.help_screen,
        };

        let _ = terminal.draw(|f| screen.render(f));

        // Poll for keystrokes (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            //TODO: Match screen here and do appropriate event listening/handling
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Tab => app.switch(),
                        KeyCode::Char('?') => app.tab = Tab::Help,
                        _ => {
                            if let Some(action) = screen.event_loop(&key) {
                                match action {
                                    Action::Quit => {
                                        app.should_quit = true;
                                    }
                                    Action::Select(problem) => {
                                        app.selected_problem = Some(problem);
                                        app.should_quit = true;
                                    }
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
