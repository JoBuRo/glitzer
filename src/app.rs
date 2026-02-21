mod view;
mod widgets;

use std::io;

use color_eyre::eyre::{Result, eyre};
use crossterm::event;
use ratatui::{Frame, prelude::*};

use super::glitzer::repo::RepositoryAccess;
use view::{View, main_view::MainView};

pub struct App {
    current_view: Box<dyn View>,
}

impl App {
    pub fn new(repo: impl RepositoryAccess) -> Result<Self> {
        let commits_res = repo.get_commits();
        if commits_res.is_err() {
            return Err(eyre!(
                "Failed to get commits: {}",
                commits_res.err().unwrap()
            ));
        }
        let commits = commits_res.unwrap();
        Ok(App {
            current_view: Box::new(MainView::new(commits)),
        })
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
        loop {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            self.handle_events()?;
        }
    }

    fn render(&self, frame: &mut Frame) {
        self.current_view.render(frame);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            event::Event::Key(key_event) => {
                self.current_view.handle_input(key_event);
            }
            _ => {}
        }
        Ok(())
    }
}
