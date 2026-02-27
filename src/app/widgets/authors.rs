use crate::{app::widgets::SelectableWidget, glitzer::author::Author};
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

#[derive(Debug)]
pub struct Authors {
    authors: Vec<Author>,
    is_selected: bool,
}

impl Authors {
    pub fn new(authors: Vec<Author>) -> Self {
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

impl From<&Author> for ListItem<'_> {
    fn from(author: &Author) -> Self {
        let mut author_text =
            Text::from(Line::from(format!("{} <{}>", author.name, author.email)).bold());
        author_text.push_line(Line::from(format!("{} commits", author.commit_count())).blue());
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
