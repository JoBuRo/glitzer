use ratatui::widgets::Block;
pub mod authors;
pub mod history;
pub mod log;

pub trait SelectableWidget {
    fn select(&mut self, selected: bool);
    fn get_block(&self) -> Block;
}
