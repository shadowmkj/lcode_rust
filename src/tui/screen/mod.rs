pub mod help_screen;
pub mod selection_screen;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::tui::Action;

pub trait Screen {
    fn render(&mut self, frame: &mut Frame);
    fn event_loop(&mut self, event: &KeyEvent) -> Option<Action>;
}
