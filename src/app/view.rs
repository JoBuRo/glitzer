use crossterm::event::KeyEvent;
use ratatui::{Frame, prelude::*};

pub mod main_view;

pub trait View {
    fn render(&self, frame: &mut Frame);
    fn handle_input(&mut self, input: KeyEvent);
}
