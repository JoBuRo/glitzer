use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};

use super::hotspots::Hotspot;

#[derive(Debug)]
pub struct HotspotDetailWidget {
    name: String,
    score: u64,
    lines_touched: u64,
    touches: u64,
    author_count: usize,
    recent_points: u64,
    most_recent_days: i64,
    has_hotspot: bool,
}

impl HotspotDetailWidget {
    pub fn from_hotspot(hotspot: Option<&Hotspot>) -> Self {
        if let Some(hotspot) = hotspot {
            return HotspotDetailWidget {
                name: hotspot.location().to_string(),
                score: hotspot.score(),
                lines_touched: hotspot.lines_touched(),
                touches: hotspot.touches(),
                author_count: hotspot.author_count(),
                recent_points: hotspot.recent_points(),
                most_recent_days: hotspot.most_recent_days(),
                has_hotspot: true,
            };
        }

        HotspotDetailWidget {
            name: "[none]".to_string(),
            score: 0,
            lines_touched: 0,
            touches: 0,
            author_count: 0,
            recent_points: 0,
            most_recent_days: 0,
            has_hotspot: false,
        }
    }

    fn block(&self) -> Block {
        Block::bordered()
            .title(Line::from(" Selected Hotspot ".bold()).centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(1))
    }
}

impl Widget for &HotspotDetailWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.has_hotspot {
            Text::from(vec![
                Line::from(format!("Name: {}", self.name)),
                Line::from(format!("Score: {}", self.score)),
                Line::from(""),
                Line::from("Why it ranks high:"),
                Line::from(format!(
                    "- high churn: {} lines changed over {} touches",
                    self.lines_touched, self.touches
                )),
                Line::from(format!(
                    "- many authors: {} contributors touched this file",
                    self.author_count
                )),
                Line::from(format!(
                    "- sustained activity: recency signal {}",
                    self.recent_points
                )),
                Line::from(format!(
                    "- recent change: last touched {} days ago",
                    self.most_recent_days
                )),
                Line::from(""),
                Line::from("Churn trend / sparkline: coming in next step"),
            ])
        } else {
            Text::from(vec![
                Line::from("Name: [none]"),
                Line::from("Score: 0"),
                Line::from(""),
                Line::from("No hotspots available for this repository."),
            ])
        };

        Widget::render(Paragraph::new(text).block(self.block()), area, buf);
    }
}
