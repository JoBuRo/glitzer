use ratatui::Frame;

pub mod main_view;

pub trait View {
    fn render(&self, frame: &mut Frame);
}
