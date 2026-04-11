pub mod screen;
mod utils;

use crate::tui::screen::selection_screen::{InputMode, SelectionScreen};
use crate::{picker::Picker, tui::screen::help_screen::HelpScreen};
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
use std::process::Command;
use std::{io, rc::Rc};

use crate::models::{Identifier, Language, ProblemSummary};

#[derive(Default)]
pub enum Tab {
    #[default]
    Selection,
    Help,
}

/// Holds the state of the application
pub struct App {
    pub should_quit: bool,
    pub problems: Rc<[ProblemSummary]>,
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
    pub fn new(problems: Rc<[ProblemSummary]>) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }

        Self {
            should_quit: false,
            selection_screen: SelectionScreen::new(Rc::clone(&problems)),
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
pub async fn run_tui(problems: Rc<[ProblemSummary]>, picker: Picker) -> anyhow::Result<()> {
    let mut app = App::new(problems);
    let _result = loop {
        enable_raw_mode().map_err(anyhow::Error::from)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(anyhow::Error::from)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = run_app(&mut terminal, &mut app).await;

        disable_raw_mode().map_err(anyhow::Error::from)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(anyhow::Error::from)?;
        terminal.show_cursor().map_err(anyhow::Error::from)?;

        match result {
            Ok(Some(problem)) => {
                pick_and_open_nvim(&picker, &Identifier::String(problem), &None).await;
                app.selection_screen.input_mode = InputMode::Normal;
                app.should_quit = false;
                app.selected_problem = None;
            }
            Ok(None) => break Ok(()),
            Err(e) => break Err(anyhow::Error::from(e)),
        }
    };
    Ok(())
}

/// The Event Loop
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<Option<String>> {
    loop {
        let screen: &mut dyn Screen = match app.tab {
            Tab::Selection => &mut app.selection_screen,
            Tab::Help => &mut app.help_screen,
        };

        let _ = terminal.draw(|f| screen.render(f));

        // Poll for keystrokes (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
            {
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

        if app.should_quit {
            return Ok(app.selected_problem.clone());
        }
    }
}

pub async fn pick_and_open_nvim(
    picker: &Picker,
    identifier: &Identifier,
    language: &Option<Language>,
) {
    if let Ok((code, desc)) = picker.pick(identifier, language).await {
        // 4. launch neovim with a vertical split
        println!("🚀 launching neovim...");
        let status = Command::new("nvim")
            .arg(&desc)
            .arg("-c")
            .arg(format!("vsplit {}", code)) // Force a vertical split with the code file
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => {
                println!("\n👋 neovim closed.");
            }
            Ok(exit_status) => {
                eprintln!("⚠️ neovim exited with an error code: {}", exit_status);
            }
            Err(e) => {
                eprintln!(
                    "❌ failed to launch neovim. is it installed and in your path? error: {}",
                    e
                );
            }
        }
    }
}
