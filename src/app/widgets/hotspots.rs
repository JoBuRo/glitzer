use std::collections::{HashMap, HashSet};

use chrono::Utc;
use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

use crate::{app::widgets::SelectableWidget, glitzer::repo::RepositoryAccess};

#[derive(Debug)]
struct Hotspot {
    location: String,
    touches: u64,
    lines_touched: u64,
    recent_points: u64,
    authors: HashSet<String>,
    most_recent_days: i64,
}

impl Hotspot {
    fn score(&self) -> u64 {
        self.lines_touched
            + (self.touches * 5)
            + (self.authors.len() as u64 * 3)
            + (self.recent_points * 4)
    }
}

#[derive(Debug)]
pub struct Hotspots {
    items: Vec<Hotspot>,
    is_selected: bool,
}

impl Hotspots {
    pub fn new(repo: &impl RepositoryAccess) -> Result<Self> {
        let mut by_path: HashMap<String, Hotspot> = HashMap::new();
        let now = Utc::now();
        let commits = repo.get_commits()?;

        for commit in commits.iter().take(300) {
            let age_days = now
                .signed_duration_since(commit.authored_at)
                .num_days()
                .max(0);
            let recent_points = match age_days {
                0..=7 => 3,
                8..=30 => 2,
                _ => 1,
            };

            let changes = repo.get_file_changes(commit)?;
            for change in changes {
                let location = if let Ok(relative) = change.location.strip_prefix(repo.get_path()) {
                    relative.to_string_lossy().to_string()
                } else {
                    change.location.to_string_lossy().to_string()
                };

                let entry = by_path.entry(location.clone()).or_insert_with(|| Hotspot {
                    location,
                    touches: 0,
                    lines_touched: 0,
                    recent_points: 0,
                    authors: HashSet::new(),
                    most_recent_days: age_days,
                });

                entry.touches += 1;
                entry.lines_touched += change.diff.lines_touched();
                entry.recent_points += recent_points;
                entry.authors.insert(commit.author.email.clone());
                entry.most_recent_days = entry.most_recent_days.min(age_days);
            }
        }

        let mut items: Vec<Hotspot> = by_path.into_values().collect();
        items.sort_by_key(|hotspot| std::cmp::Reverse(hotspot.score()));

        Ok(Hotspots {
            items,
            is_selected: false,
        })
    }
}

impl Widget for &Hotspots {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.get_block();

        let items: Vec<ListItem> = self.items.iter().take(6).map(ListItem::from).collect();

        Widget::render(List::new(items).block(block), area, buf);
    }
}

impl SelectableWidget for Hotspots {
    fn select(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    fn get_block(&self) -> Block {
        let title = Line::from("  🔧 Refactoring Attention 🔧 ".bold());
        let mut block = Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(2));

        if self.is_selected {
            block = block.green();
        }

        block
    }
}

impl From<&Hotspot> for ListItem<'_> {
    fn from(hotspot: &Hotspot) -> Self {
        let mut text = Text::from(
            Line::from(format!("{} (score {})", hotspot.location, hotspot.score()))
                .bold()
                .yellow(),
        );

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
}
