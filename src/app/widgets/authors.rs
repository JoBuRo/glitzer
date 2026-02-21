use std::collections::HashMap;

use crate::{app::widgets::SelectableWidget, glitzer::git_objects::Commit};
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

#[derive(Debug)]
pub struct Authors {
    authors: Vec<AuthorInfo>,
    is_selected: bool,
}

#[derive(Debug)]
struct AuthorInfo {
    name: String,
    email: String,
    commit_count: usize,
}

impl Authors {
    pub fn new(commits: Vec<Commit>) -> Self {
        let mut author_map: HashMap<String, AuthorInfo> = std::collections::HashMap::new();

        for commit in commits {
            let key = &commit.author.email;

            if let Some(info) = author_map.get_mut(key) {
                info.commit_count += 1;
            } else {
                author_map.insert(
                    key.clone(),
                    AuthorInfo {
                        name: commit.author.name.clone(),
                        email: commit.author.email.clone(),
                        commit_count: 1,
                    },
                );
            }
        }

        let mut authors: Vec<AuthorInfo> = author_map
            .into_iter()
            .map(|(email, info)| AuthorInfo {
                name: info.name,
                email,
                commit_count: info.commit_count,
            })
            .collect();
        authors.sort_by(|first, second| second.commit_count.cmp(&first.commit_count));

        Authors {
            authors,
            is_selected: false,
        }
    }
}

impl Widget for &Authors {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.get_block();

        let items: Vec<ListItem> = self.authors.iter().map(ListItem::from).collect();

        Widget::render(List::new(items).block(block), area, buf);
    }
}

impl From<&AuthorInfo> for ListItem<'_> {
    fn from(info: &AuthorInfo) -> Self {
        let mut author_text =
            Text::from(Line::from(format!("{} <{}>", info.name, info.email)).bold());
        author_text.push_line(Line::from(format!("{} commits", info.commit_count)).blue());
        ListItem::new(author_text)
    }
}

impl SelectableWidget for Authors {
    fn select(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    fn get_block(&self) -> Block {
        let title = Line::from("  👥 Authors 👥 ".bold());
        let mut block = Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(5));

        if self.is_selected {
            block = block.green();
        }

        block
    }
}
