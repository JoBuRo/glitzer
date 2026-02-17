use super::super::widgets::authors::Authors;
use super::super::widgets::history::History;
use super::super::widgets::log::Log;
use super::View;
use crate::glitzer::git_objects::Commit;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols::border,
    widgets::{Block, Widget},
};

#[derive(Debug)]
pub struct MainView {
    log: Log,
    history: History,
    authors: Authors,
}

impl MainView {
    pub fn new(commits: Vec<Commit>) -> Self {
        MainView {
            log: Log::new(commits.clone()),
            history: History::new(commits.clone()),
            authors: Authors::new(commits),
        }
    }
}

impl View for MainView {
    fn render(&self, frame: &mut Frame) {
        let title = Line::from("  ✨ Glitzer ✨ ".bold());

        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.area());
        let upper_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .margin(1)
            .split(outer_layout[0]);
        frame.render_widget(block, frame.area());
        frame.render_widget(&self.authors, upper_layout[0]);
        frame.render_widget(&self.log, upper_layout[1]);
        frame.render_widget(&self.history, outer_layout[1].inner(Margin::new(1, 1)));
    }
}
