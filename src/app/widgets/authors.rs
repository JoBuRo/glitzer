use color_eyre::eyre::Result;
use std::fs::canonicalize;
use std::path::PathBuf;

use crate::{
    app::widgets::SelectableWidget,
    glitzer::{author::Author, repo::RepositoryAccess},
};
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

#[derive(Debug)]
struct AuthorAndFiles {
    author: Author,
    changed_files: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Authors {
    authors: Vec<AuthorAndFiles>,
    is_selected: bool,
}

impl Authors {
    pub fn new(repo: &impl RepositoryAccess) -> Result<Self> {
        let mut authors_and_files = vec![];

        for author in repo.get_authors()? {
            let changed_files = author.get_changed_files(repo)?;
            authors_and_files.push(AuthorAndFiles {
                author,
                changed_files,
            });
        }
        Ok(Authors {
            authors: authors_and_files,
            is_selected: false,
        })
    }
}

impl Widget for &Authors {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.get_block();

        let items: Vec<ListItem> = self.authors.iter().map(ListItem::from).collect();

        Widget::render(List::new(items).block(block), area, buf);
    }
}

impl From<&AuthorAndFiles> for ListItem<'_> {
    fn from(author_and_files: &AuthorAndFiles) -> Self {
        let author = &author_and_files.author;
        let mut author_text =
            Text::from(Line::from(format!("{} <{}>", author.name, author.email)).bold());
        author_text.push_line(Line::from(format!("{} commits", author.commit_count())).blue());

        author_text.push_line(Line::from("Top Changed Files").bold().yellow());
        for (i, changed_file) in author_and_files.changed_files.iter().take(5).enumerate() {
            author_text.push_line(
                Line::from(format!(
                    "    {}. {}",
                    i + 1,
                    canonicalize(changed_file)
                        .unwrap_or(PathBuf::from(changed_file))
                        .as_os_str()
                        .to_str()
                        .unwrap_or("?")
                ))
                .yellow(),
            );
        }
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
