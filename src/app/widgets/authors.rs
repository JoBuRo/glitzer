use std::collections::HashMap;

use crate::glitzer::git_objects::Commit;
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

#[derive(Debug)]
pub struct Authors {
    authors: Vec<AuthorInfo>,
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

        Authors { authors }
    }
}

impl Widget for &Authors {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("  👥 Authors 👥 ".bold());

        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(5));

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
