use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Widget},
};

use crate::glitzer::git_objects::Commit;

#[derive(Debug)]
pub struct Log {
    commits: Vec<Commit>,
    index: usize,
}

impl Widget for &Log {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("  🪵 Commit Log 🪵 ".bold());

        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let items: Vec<ListItem> = self
            .commits
            .iter()
            .skip(self.index)
            .take(5)
            .map(ListItem::from)
            .collect();

        Widget::render(List::new(items).block(block), area, buf);
    }
}

impl Log {
    pub fn new(commits: Vec<Commit>) -> Self {
        Log { commits, index: 0 }
    }
}

impl From<&Commit> for ListItem<'_> {
    fn from(commit: &Commit) -> Self {
        let line = Line::from(format!(
            "{} - {}",
            &commit.hash[..7],
            commit.message.lines().next().unwrap_or("")
        ))
        .centered();
        ListItem::new(line)
    }
}
