use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::models::hotspot::Hotspot;

#[derive(Debug)]
pub struct HotspotDetailWidget {
    name: String,
    score: u64,
    lines_touched: u64,
    touches: u64,
    author_count: usize,
    recent_points: u64,
    most_recent_days: i64,
    default_rank_adjustment_line: Option<String>,
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
                default_rank_adjustment_line: hotspot.default_rank_adjustment_line(),
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
            default_rank_adjustment_line: None,
            has_hotspot: false,
        }
    }

    fn block(&self) -> Block<'_> {
        Block::bordered()
            .title(Line::from(" Selected Hotspot ".bold()).centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(1))
    }
}

impl Widget for &HotspotDetailWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.has_hotspot {
            let mut lines = vec![
                Line::from(format!("Name: {}", self.name)),
                Line::from(format!("Score: {}", self.score)),
            ];

            if let Some(adjustment_line) = &self.default_rank_adjustment_line {
                lines.push(Line::from(adjustment_line.clone()));
            }

            lines.extend([
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
            ]);

            Text::from(lines)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::hotspot::{Hotspot, HotspotTestData};
    use crate::ui::widgets::test_support::hotspot_payment;

    fn render_lines(widget: &HotspotDetailWidget, width: u16, height: u16) -> Vec<String> {
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        Widget::render(widget, buf.area, &mut buf);
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .collect()
    }

    #[test]
    fn render_snapshot_with_hotspot() {
        let hotspot = hotspot_payment();
        let widget = HotspotDetailWidget::from_hotspot(Some(&hotspot));

        let rendered = render_lines(&widget, 88, 14);

        let expected = vec![
            "┌────────────────────────────────── Selected Hotspot ──────────────────────────────────┐",
            "│ Name: src/service/payment.rs                                                         │",
            "│ Score: 310                                                                           │",
            "│                                                                                      │",
            "│ Why it ranks high:                                                                   │",
            "│ - high churn: 220 lines changed over 10 touches                                      │",
            "│ - many authors: 4 contributors touched this file                                     │",
            "│ - sustained activity: recency signal 7                                               │",
            "│ - recent change: last touched 2 days ago                                             │",
            "│                                                                                      │",
            "│ Churn trend / sparkline: coming in next step                                         │",
            "│                                                                                      │",
            "│                                                                                      │",
            "└──────────────────────────────────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_without_hotspot() {
        let widget = HotspotDetailWidget::from_hotspot(None);

        let rendered = render_lines(&widget, 64, 8);

        let expected = vec![
            "┌────────────────────── Selected Hotspot ──────────────────────┐",
            "│ Name: [none]                                                 │",
            "│ Score: 0                                                     │",
            "│                                                              │",
            "│ No hotspots available for this repository.                   │",
            "│                                                              │",
            "│                                                              │",
            "└──────────────────────────────────────────────────────────────┘",
        ];

        assert_eq!(rendered, expected);
    }

    #[test]
    fn from_hotspot_none_sets_fallback_values() {
        let widget = HotspotDetailWidget::from_hotspot(None);

        assert_eq!(widget.name, "[none]");
        assert_eq!(widget.score, 0);
        assert_eq!(widget.lines_touched, 0);
        assert_eq!(widget.touches, 0);
        assert_eq!(widget.author_count, 0);
        assert_eq!(widget.recent_points, 0);
        assert_eq!(widget.most_recent_days, 0);
        assert_eq!(widget.default_rank_adjustment_line, None);
        assert!(!widget.has_hotspot);
    }

    #[test]
    fn render_snapshot_with_lockfile_adjustment() {
        let lockfile = Hotspot::test_fixture(HotspotTestData {
            location: "Cargo.lock",
            touches: 5,
            lines_touched: 30,
            recent_points: 4,
            most_recent_days: 1,
            authors: &["dev@example.com"],
            recent_commits: vec![],
            co_changes: vec![],
            author_touches: vec![("Dev <dev@example.com>", 5)],
            default_rank_multiplier_percent: 20,
        });
        let widget = HotspotDetailWidget::from_hotspot(Some(&lockfile));

        let rendered = render_lines(&widget, 96, 14).join("\n");

        assert!(rendered.contains("Default ranking adjustment: lockfile de-weight x0.20"));
    }
}
