use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

use crate::models::hotspot::Hotspot;

#[derive(Debug)]
pub struct Hotspots {
    items: Vec<Hotspot>,
    selected_index: usize,
}

impl Hotspots {
    pub fn from_items(items: Vec<Hotspot>) -> Self {
        Hotspots {
            items,
            selected_index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if self.items.is_empty() {
            self.selected_index = 0;
            return;
        }
        self.selected_index = index.min(self.items.len().saturating_sub(1));
    }

    pub fn selected_hotspot(&self) -> Option<&Hotspot> {
        self.items.get(self.selected_index)
    }

    fn block(&self) -> Block<'_> {
        let title = Line::from(" Refactoring Attention ".bold());
        Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(2))
    }
}

impl Widget for &Hotspots {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();

        let start = self.selected_index.saturating_sub(3);
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .skip(start)
            .take(6)
            .map(|(idx, hotspot)| hotspot_list_item(idx, hotspot, idx == self.selected_index))
            .collect();

        Widget::render(List::new(items).block(block), area, buf);
    }
}

fn hotspot_list_item(rank: usize, hotspot: &Hotspot, selected: bool) -> ListItem<'static> {
    let mut title = Line::from(format!(
        "{}. {}    {}",
        rank + 1,
        hotspot.location,
        hotspot.score()
    ));
    if selected {
        title = title.bold().green();
    } else {
        title = title.bold().yellow();
    }

    let mut text = Text::from(title);

    text.push_line(
        Line::from(format!(
            "evidence: {} touches, {} lines changed, {} authors, last touched {}d ago",
            hotspot.touches,
            hotspot.lines_touched,
            hotspot.authors.len(),
            hotspot.most_recent_days
        ))
        .blue(),
    );

    ListItem::new(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::test_support::hotspot_with_counts;

    fn render_lines(hotspots: &Hotspots, width: u16, height: u16) -> Vec<String> {
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        Widget::render(hotspots, buf.area, &mut buf);
        (0..buf.area.height).map(|y| line(&buf, y)).collect()
    }

    fn line(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn render_snapshot_basic_list() {
        let hotspots = Hotspots {
            items: vec![
                hotspot_with_counts("src/app.rs", 6, 42, 3, 1),
                hotspot_with_counts("src/lib.rs", 4, 18, 9, 1),
            ],
            selected_index: 0,
        };

        let rendered = render_lines(&hotspots, 100, 8);

        let expected = vec![
            "┌───────────────────────────────────── Refactoring Attention ──────────────────────────────────────┐",
            "│  1. src/app.rs    83                                                                             │",
            "│  evidence: 6 touches, 42 lines changed, 1 authors, last touched 3d ago                           │",
            "│  2. src/lib.rs    49                                                                             │",
            "│  evidence: 4 touches, 18 lines changed, 1 authors, last touched 9d ago                           │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_selection_window_middle() {
        let items: Vec<Hotspot> = (0..10)
            .map(|i| hotspot_with_counts(&format!("file_{i}.rs"), i + 1, (i + 1) * 10, i as i64, 1))
            .collect();
        let hotspots = Hotspots {
            items,
            selected_index: 5,
        };

        let rendered = render_lines(&hotspots, 100, 14);

        let expected = vec![
            "┌───────────────────────────────────── Refactoring Attention ──────────────────────────────────────┐",
            "│  3. file_2.rs    56                                                                              │",
            "│  evidence: 3 touches, 30 lines changed, 1 authors, last touched 2d ago                           │",
            "│  4. file_3.rs    71                                                                              │",
            "│  evidence: 4 touches, 40 lines changed, 1 authors, last touched 3d ago                           │",
            "│  5. file_4.rs    86                                                                              │",
            "│  evidence: 5 touches, 50 lines changed, 1 authors, last touched 4d ago                           │",
            "│  6. file_5.rs    101                                                                             │",
            "│  evidence: 6 touches, 60 lines changed, 1 authors, last touched 5d ago                           │",
            "│  7. file_6.rs    116                                                                             │",
            "│  evidence: 7 touches, 70 lines changed, 1 authors, last touched 6d ago                           │",
            "│  8. file_7.rs    131                                                                             │",
            "│  evidence: 8 touches, 80 lines changed, 1 authors, last touched 7d ago                           │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_selection_window_end() {
        let items: Vec<Hotspot> = (0..8)
            .map(|i| hotspot_with_counts(&format!("file_{i}.rs"), i + 1, (i + 1) * 10, i as i64, 1))
            .collect();
        let hotspots = Hotspots {
            items,
            selected_index: 7,
        };

        let rendered = render_lines(&hotspots, 100, 14);

        let expected = vec![
            "┌───────────────────────────────────── Refactoring Attention ──────────────────────────────────────┐",
            "│  5. file_4.rs    86                                                                              │",
            "│  evidence: 5 touches, 50 lines changed, 1 authors, last touched 4d ago                           │",
            "│  6. file_5.rs    101                                                                             │",
            "│  evidence: 6 touches, 60 lines changed, 1 authors, last touched 5d ago                           │",
            "│  7. file_6.rs    116                                                                             │",
            "│  evidence: 7 touches, 70 lines changed, 1 authors, last touched 6d ago                           │",
            "│  8. file_7.rs    131                                                                             │",
            "│  evidence: 8 touches, 80 lines changed, 1 authors, last touched 7d ago                           │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_empty_state() {
        let hotspots = Hotspots {
            items: Vec::new(),
            selected_index: 0,
        };

        let rendered = render_lines(&hotspots, 60, 6);

        let expected = vec![
            "┌───────────────── Refactoring Attention ──────────────────┐",
            "│                                                          │",
            "│                                                          │",
            "│                                                          │",
            "│                                                          │",
            "└──────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn set_selected_index_clamps_to_last_item() {
        let mut hotspots = Hotspots {
            items: vec![
                hotspot_with_counts("a.rs", 1, 1, 1, 1),
                hotspot_with_counts("b.rs", 1, 1, 1, 1),
            ],
            selected_index: 0,
        };

        hotspots.set_selected_index(99);

        assert_eq!(hotspots.selected_index, 1);
        assert_eq!(
            hotspots.selected_hotspot().map(Hotspot::location),
            Some("b.rs")
        );
    }

    #[test]
    fn set_selected_index_on_empty_keeps_zero() {
        let mut hotspots = Hotspots {
            items: Vec::new(),
            selected_index: 0,
        };

        hotspots.set_selected_index(42);

        assert_eq!(hotspots.selected_index, 0);
    }

    #[test]
    fn selected_hotspot_returns_none_when_empty() {
        let hotspots = Hotspots {
            items: Vec::new(),
            selected_index: 0,
        };

        assert!(hotspots.selected_hotspot().is_none());
    }

    #[test]
    fn render_rank_numbers_are_global_not_window_local() {
        let items: Vec<Hotspot> = (0..8)
            .map(|i| hotspot_with_counts(&format!("file_{i}.rs"), i + 1, (i + 1) * 10, i as i64, 1))
            .collect();
        let hotspots = Hotspots {
            items,
            selected_index: 7,
        };

        let rendered = render_lines(&hotspots, 100, 14).join("\n");

        assert!(rendered.contains("5. file_4.rs"));
        assert!(!rendered.contains("1. file_4.rs"));
    }
}
