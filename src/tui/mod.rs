//! ratatui TUI runtime for leetrs.
//!
//! Provides the interactive problem browser. The top-level entry point is
//! [`run_tui`], which owns the terminal setup/teardown and re-opens the TUI
//! after Neovim closes so the user can pick another problem without restarting.
pub mod screen;
mod utils;

use crate::tui::screen::selection_screen::{InputMode, SelectionScreen};
use crate::{picker::Picker, tui::screen::help_screen::HelpScreen};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    widgets::ListState,
};
use screen::Screen;
use std::process::Command;
use std::{io, rc::Rc};

use crate::models::{Identifier, Language, ProblemSummary, UserDetail};

/// Which tab is currently displayed.
#[derive(Default)]
pub enum Tab {
    #[default]
    Selection,
    Help,
}

/// Holds the state of the application
/// Top-level application state shared across a single TUI session.
pub struct App {
    pub should_quit: bool,
    /// The full, shared problem list (reference-counted to avoid copying).
    pub problems: Rc<[ProblemSummary]>,
    pub tab: Tab,
    pub selection_screen: SelectionScreen,
    pub help_screen: HelpScreen,
    /// Slug of the problem the user pressed Enter on, if any.
    pub selected_problem: Option<String>,
    pub user_detail: Option<UserDetail>,
    /// One-shot message shown in a modal popup until dismissed.
    pub popup_message: Option<String>,
}

/// Actions that a [`Screen`] can return to the main event loop.
pub enum Action {
    Quit,
    /// The user selected a problem; carries its slug.
    Select(String),
    /// Display a one-shot modal popup with the given message.
    ShowMessage(String),
    /// Open the given URL in the system browser.
    Open(String),
}

impl App {
    pub fn new(problems: Rc<[ProblemSummary]>, user_detail: Option<UserDetail>) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }

        Self {
            should_quit: false,
            selection_screen: SelectionScreen::new(Rc::clone(&problems), user_detail.clone()),
            problems,
            tab: Tab::default(),
            selected_problem: None,
            help_screen: HelpScreen::new(),
            user_detail,
            popup_message: None,
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
/// Initialises [`App`], then enters the TUI event loop.
///
/// After the user selects a problem the terminal is restored, Neovim is
/// launched, and the TUI is re-entered so the user can pick another problem
/// without restarting the process.
pub async fn run_tui(
    problems: Rc<[ProblemSummary]>,
    picker: Picker,
    user_detail: Option<UserDetail>,
    language: &Option<Language>,
) -> anyhow::Result<()> {
    let mut app = App::new(problems, user_detail);
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
                pick_and_open_nvim(&picker, &Identifier::String(problem), language).await;
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
/// Drives rendering and keyboard events for a single TUI session.
///
/// Returns `Ok(Some(slug))` when the user selects a problem, `Ok(None)` when
/// they quit, and `Err` on I/O failure.
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<Option<String>> {
    loop {
        let screen: &mut dyn Screen = match app.tab {
            Tab::Selection => &mut app.selection_screen,
            Tab::Help => &mut app.help_screen,
        };

        let _ = terminal.draw(|f| {
            screen.render(f);
            if let Some(popup_message) = &app.popup_message {
                let centered_area = f
                    .area()
                    .centered(Constraint::Percentage(60), Constraint::Percentage(20));
                f.render_widget(Clear, centered_area);
                let layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                    .split(centered_area);
                let popup_block = Block::bordered().title("Alert");
                let paragraph = Paragraph::new(popup_message.as_str()).block(popup_block);
                f.render_widget(paragraph, layout[0]);
                let hint = Paragraph::new("Press Enter or Esc to close");
                f.render_widget(hint, layout[1]);
            }
        });

        // Poll for keystrokes (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
            {
                if let Some(_) = &app.popup_message {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.popup_message = None;
                        }
                        _ => {}
                    }
                    continue;
                }
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
                                Action::ShowMessage(msg) => {
                                    app.popup_message = Some(msg);
                                    app.should_quit = false;
                                }
                                Action::Open(url) => {
                                    let _ = open::that(url);
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
