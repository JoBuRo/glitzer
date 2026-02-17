use ratatui::{
    Frame,
    prelude::*,
    text::Line,
    widgets::{Block, Widget},
};

pub mod main_view;

pub trait View {
    fn render(&self, frame: &mut Frame);
}
