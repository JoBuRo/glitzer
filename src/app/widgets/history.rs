use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Widget},
};

use crate::glitzer::git_objects::Commit;

#[derive(Debug)]
pub struct History {
    commits: Vec<Commit>,
}

impl History {
    pub fn new(commits: Vec<Commit>) -> Self {
        History { commits }
    }
}

impl Widget for &History {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Placeholder rendering logic for History widget
        let title = Line::from("  📜 Commit History 📜 ".bold());

        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::PLAIN);

        let buckets = buckets_for_days(&self.commits);

        let bars: Vec<Bar> = buckets
            .iter()
            .map(|(label, count)| bucket_to_bar(label, *count))
            .collect();

        BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .bar_width(10)
            .bar_gap(2)
            .max(buckets.iter().map(|(_, count)| *count).max().unwrap_or(1))
            .render(area, buf);
    }
}

fn buckets_for_days(commits: &[Commit]) -> Vec<(&str, u64)> {
    let mut buckets = vec![
        ("Today", 0),
        ("Yesterday", 0),
        ("3 days", 0),
        ("4 days", 0),
        ("5 days", 0),
        ("6 days", 0),
        ("7 days", 0),
    ];

    let now = chrono::Utc::now();

    for commit in commits {
        let age = now.signed_duration_since(commit.authored_at).num_days();
        match age {
            0..=1 => buckets[0].1 += 1,
            2..=3 => buckets[1].1 += 1,
            4..=5 => buckets[2].1 += 1,
            6..=7 => buckets[3].1 += 1,
            _ => {}
        }
    }

    buckets
}

fn bucket_to_bar(label: &str, count: u64) -> Bar {
    Bar::default().label(Line::from(label)).value(count)
}
