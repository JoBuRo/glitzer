use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

use crate::glitzer::git_objects::Commit;

#[derive(Debug)]
pub struct Log {
    commits: Vec<Commit>,
    index: usize,
}

impl Widget for &Log {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();

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

    fn block(&self) -> Block {
        let title = Line::from("  🪵 Commit Log 🪵 ".bold());
        Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(2))
    }
}

impl From<&Commit> for ListItem<'_> {
    fn from(commit: &Commit) -> Self {
        let mut commit_text = Text::from(
            Line::from(format!(
                "({}) {}",
                &commit.hash[..7],
                commit.message.lines().next().unwrap_or(""),
            ))
            .bold()
            .yellow(),
        );
        commit_text.push_line(
            Line::from(commit.committed_at.format("%Y-%m-%d %H:%M:%S").to_string()).blue(),
        );
        for line in commit.message.lines().skip(2) {
            commit_text.push_line(Line::from(format!("    {}", line)));
        }
        commit_text.push_line(
            Line::from(format!(
                "~ {} <{}>",
                commit.author.name, commit.author.email
            ))
            .right_aligned(),
        );
        ListItem::new(commit_text)
    }
}
