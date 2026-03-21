use super::super::widgets::authors::Authors;
use super::super::widgets::history::History;
use super::super::widgets::hotspots::Hotspots;
use super::View;
use crate::{app::widgets::SelectableWidget, glitzer::repo::RepositoryAccess};
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols::border,
    widgets::Block,
};

#[derive(Debug)]
enum Selection {
    Authors,
    History,
    Hotspots,
}

#[derive(Debug)]
pub struct MainView {
    authors: Authors,
    history: History,
    hotspots: Hotspots,
    selected: Option<Selection>,
}

impl MainView {
    pub fn new(repo: &impl RepositoryAccess) -> Result<Self> {
        Ok(MainView {
            authors: Authors::new(repo)?,
            history: History::new(repo.get_commits()?),
            hotspots: Hotspots::new(repo)?,
            selected: None,
        })
    }

    fn unselect_widgets(&mut self) {
        self.authors.select(false);
        self.history.select(false);
        self.hotspots.select(false);
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
        frame.render_widget(&self.hotspots, upper_layout[0]);
        frame.render_widget(&self.authors, upper_layout[1]);
        frame.render_widget(&self.history, outer_layout[1].inner(Margin::new(1, 1)));
    }

    fn handle_input(&mut self, input: KeyEvent) {
        match input.code {
            KeyCode::Char('q') => {
                std::process::exit(0);
            }
            KeyCode::Char('h') => match self.selected {
                Some(Selection::Authors) => self.selected = Some(Selection::Hotspots),
                None => self.selected = Some(Selection::Hotspots),
                _ => {}
            },
            KeyCode::Char('j') => match self.selected {
                Some(Selection::Authors) => self.selected = Some(Selection::History),
                Some(Selection::Hotspots) => self.selected = Some(Selection::History),
                None => self.selected = Some(Selection::History),
                _ => {}
            },
            KeyCode::Char('k') => match self.selected {
                Some(Selection::History) => self.selected = Some(Selection::Authors),
                None => self.selected = Some(Selection::Authors),
                _ => {}
            },
            KeyCode::Char('l') => match self.selected {
                Some(Selection::Hotspots) => self.selected = Some(Selection::Authors),
                None => self.selected = Some(Selection::Authors),
                _ => {}
            },
            _ => {}
        }

        self.unselect_widgets();
        match self.selected {
            Some(Selection::Authors) => self.authors.select(true),
            Some(Selection::History) => self.history.select(true),
            Some(Selection::Hotspots) => self.hotspots.select(true),
            None => {}
        }
    }
}
