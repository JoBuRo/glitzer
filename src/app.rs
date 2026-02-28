mod view;
mod widgets;

use std::io;

use color_eyre::eyre::Result;
use crossterm::event;
use ratatui::Frame;

use super::glitzer::repo::RepositoryAccess;
use view::{View, main_view::MainView};

pub struct App {
    current_view: Box<dyn View>,
}

impl App {
    pub fn new(repo: impl RepositoryAccess) -> Result<Self> {
        Ok(App {
            current_view: Box::new(MainView::new(&repo)?),
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
        if let event::Event::Key(key_event) = event::read()? {
            self.current_view.handle_input(key_event);
        }
        Ok(())
    }
}
