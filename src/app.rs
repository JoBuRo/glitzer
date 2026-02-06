mod log;

use std::io;

use color_eyre::eyre::{Result, eyre};
use crossterm::event;
use ratatui::{
    Frame,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

use super::glitzer::repo::Repository;
use log::Log;

#[derive(Debug)]
pub struct App {
    repo: Repository,
    log: Log,
}

impl App {
    pub fn new(repo: Repository) -> Result<Self> {
        let commits_res = repo.get_commits();
        if commits_res.is_err() {
            return Err(eyre!(
                "Failed to get commits: {}",
                commits_res.err().unwrap()
            ));
        }
        Ok(App {
            repo,
            log: Log::new(commits_res.unwrap()),
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
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.area());
        let upper_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .margin(1)
            .split(outer_layout[0]);
        frame.render_widget(self, frame.area());
        frame.render_widget(&self.log, upper_layout[1]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            event::Event::Key(key_event) => {
                if key_event.code == event::KeyCode::Char('q') {
                    std::process::exit(0);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let title = Line::from("  ✨ Glitzer ✨ ".bold());

        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .render(area, buf);
    }
}
