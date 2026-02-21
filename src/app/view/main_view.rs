use super::super::widgets::authors::Authors;
use super::super::widgets::history::History;
use super::super::widgets::log::Log;
use super::View;
use crate::{app::widgets::SelectableWidget, glitzer::git_objects::Commit};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols::border,
    widgets::Block,
};

#[derive(Debug)]
enum Selection {
    Log,
    History,
    Authors,
}

#[derive(Debug)]
pub struct MainView {
    log: Log,
    history: History,
    authors: Authors,
    selected: Option<Selection>,
}

impl MainView {
    pub fn new(commits: Vec<Commit>) -> Self {
        MainView {
            log: Log::new(commits.clone()),
            history: History::new(commits.clone()),
            authors: Authors::new(commits),
            selected: None,
        }
    }

    fn unselect_widgets(&mut self) {
        self.log.select(false);
        self.history.select(false);
        self.authors.select(false);
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

    fn handle_input(&mut self, input: KeyEvent) {
        match input.code {
            KeyCode::Char('q') => {
                std::process::exit(0);
            }
            KeyCode::Char('h') => match self.selected {
                Some(Selection::Log) => self.selected = Some(Selection::Authors),
                None => self.selected = Some(Selection::Authors),
                _ => {}
            },
            KeyCode::Char('j') => match self.selected {
                Some(Selection::Log) => self.selected = Some(Selection::History),
                Some(Selection::Authors) => self.selected = Some(Selection::History),
                None => self.selected = Some(Selection::History),
                _ => {}
            },
            KeyCode::Char('k') => match self.selected {
                Some(Selection::History) => self.selected = Some(Selection::Log),
                None => self.selected = Some(Selection::Log),
                _ => {}
            },
            KeyCode::Char('l') => match self.selected {
                Some(Selection::Authors) => self.selected = Some(Selection::Log),
                None => self.selected = Some(Selection::Log),
                _ => {}
            },
            _ => {}
        }

        self.unselect_widgets();
        match self.selected {
            Some(Selection::Log) => self.log.select(true),
            Some(Selection::History) => self.history.select(true),
            Some(Selection::Authors) => self.authors.select(true),
            None => {}
        }
    }
}
