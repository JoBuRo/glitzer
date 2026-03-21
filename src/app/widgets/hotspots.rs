use std::collections::{HashMap, HashSet};

use chrono::Utc;
use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, Padding, Widget},
};

use crate::glitzer::repo::RepositoryAccess;

#[derive(Debug)]
pub struct Hotspot {
    location: String,
    touches: u64,
    lines_touched: u64,
    recent_points: u64,
    authors: HashSet<String>,
    most_recent_days: i64,
    recent_commits: Vec<CommitEvidence>,
    co_changes: HashMap<String, u64>,
    author_touches: HashMap<String, u64>,
}

#[derive(Debug)]
struct CommitEvidence {
    hash: String,
    author: String,
    committed_at: String,
    message: String,
    lines_touched: u64,
}

#[cfg(test)]
pub(crate) struct HotspotTestData<'a> {
    pub location: &'a str,
    pub touches: u64,
    pub lines_touched: u64,
    pub recent_points: u64,
    pub most_recent_days: i64,
    pub authors: &'a [&'a str],
    pub recent_commits: Vec<(&'a str, &'a str, &'a str, &'a str, u64)>,
    pub co_changes: Vec<(&'a str, u64)>,
    pub author_touches: Vec<(&'a str, u64)>,
}

impl Hotspot {
    pub fn score(&self) -> u64 {
        self.lines_touched
            + (self.touches * 5)
            + (self.authors.len() as u64 * 3)
            + (self.recent_points * 4)
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn touches(&self) -> u64 {
        self.touches
    }

    pub fn lines_touched(&self) -> u64 {
        self.lines_touched
    }

    pub fn author_count(&self) -> usize {
        self.authors.len()
    }

    pub fn most_recent_days(&self) -> i64 {
        self.most_recent_days
    }

    pub fn recent_points(&self) -> u64 {
        self.recent_points
    }

    pub fn commit_evidence_lines(&self) -> Vec<String> {
        if self.recent_commits.is_empty() {
            return vec![
                "No commits found for this hotspot in the current analysis window.".to_string(),
            ];
        }

        self.recent_commits
            .iter()
            .take(5)
            .map(|commit| {
                format!(
                    "{} {} - {} - {} lines - {}",
                    &commit.hash[..7],
                    commit.author,
                    commit.committed_at,
                    commit.lines_touched,
                    commit.message
                )
            })
            .collect()
    }

    pub fn co_change_evidence_lines(&self) -> Vec<String> {
        if self.co_changes.is_empty() {
            return vec!["No co-change signal found yet for this hotspot.".to_string()];
        }

        let mut entries: Vec<(&String, &u64)> = self.co_changes.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        entries
            .into_iter()
            .take(5)
            .map(|(path, count)| format!("{} ({} commits together)", path, count))
            .collect()
    }

    pub fn ownership_evidence_lines(&self) -> Vec<String> {
        if self.author_touches.is_empty() || self.touches == 0 {
            return vec!["No ownership distribution available yet for this hotspot.".to_string()];
        }

        let mut owners: Vec<(&String, &u64)> = self.author_touches.iter().collect();
        owners.sort_by(|a, b| b.1.cmp(a.1));

        owners
            .into_iter()
            .take(5)
            .map(|(author, touches)| {
                let pct = (*touches as f64 / self.touches as f64) * 100.0;
                format!("{} - {} touches ({:.0}%)", author, touches, pct)
            })
            .collect()
    }

    pub fn notes_evidence_lines(&self) -> Vec<String> {
        let churn_note = if self.lines_touched >= 200 {
            "High churn risk: frequent line movement suggests structural pressure."
        } else if self.lines_touched >= 80 {
            "Moderate churn: file is seeing repeated edits worth grouping into a refactor."
        } else {
            "Lower churn: prioritize if coupled with architectural concerns."
        };

        let ownership_note = if self.author_count() >= 4 {
            "Ownership is distributed; refactoring can reduce coordination overhead."
        } else {
            "Ownership is concentrated; refactor likely has lower alignment cost."
        };

        let recency_note = if self.most_recent_days <= 7 {
            "Recent activity spike: changes are active now, making this a timely candidate."
        } else {
            "Activity is older; still relevant but less urgent than currently hot files."
        };

        vec![
            churn_note.to_string(),
            ownership_note.to_string(),
            recency_note.to_string(),
        ]
    }

    #[cfg(test)]
    pub(crate) fn test_fixture(data: HotspotTestData<'_>) -> Self {
        Hotspot {
            location: data.location.to_string(),
            touches: data.touches,
            lines_touched: data.lines_touched,
            recent_points: data.recent_points,
            authors: data
                .authors
                .iter()
                .map(|author| (*author).to_string())
                .collect(),
            most_recent_days: data.most_recent_days,
            recent_commits: data
                .recent_commits
                .into_iter()
                .map(
                    |(hash, author, committed_at, message, commit_lines_touched)| CommitEvidence {
                        hash: hash.to_string(),
                        author: author.to_string(),
                        committed_at: committed_at.to_string(),
                        message: message.to_string(),
                        lines_touched: commit_lines_touched,
                    },
                )
                .collect(),
            co_changes: data
                .co_changes
                .into_iter()
                .map(|(path, count)| (path.to_string(), count))
                .collect(),
            author_touches: data
                .author_touches
                .into_iter()
                .map(|(author, count)| (author.to_string(), count))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct Hotspots {
    items: Vec<Hotspot>,
    selected_index: usize,
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
            let mut changed_paths = Vec::new();

            for change in changes {
                let location = if let Ok(relative) = change.location.strip_prefix(repo.get_path()) {
                    relative.to_string_lossy().to_string()
                } else {
                    change.location.to_string_lossy().to_string()
                };

                changed_paths.push(location.clone());

                let entry = by_path.entry(location.clone()).or_insert_with(|| Hotspot {
                    location,
                    touches: 0,
                    lines_touched: 0,
                    recent_points: 0,
                    authors: HashSet::new(),
                    most_recent_days: age_days,
                    recent_commits: Vec::new(),
                    co_changes: HashMap::new(),
                    author_touches: HashMap::new(),
                });

                entry.touches += 1;
                let changed_lines = change.diff.lines_touched();
                entry.lines_touched += changed_lines;
                entry.recent_points += recent_points;
                entry.authors.insert(commit.author.email.clone());
                entry.most_recent_days = entry.most_recent_days.min(age_days);

                let author_key = format!("{} <{}>", commit.author.name, commit.author.email);
                *entry.author_touches.entry(author_key).or_insert(0) += 1;

                if entry.recent_commits.len() < 8 {
                    entry.recent_commits.push(CommitEvidence {
                        hash: commit.hash.clone(),
                        author: commit.author.name.clone(),
                        committed_at: commit.committed_at.format("%Y-%m-%d").to_string(),
                        message: commit.message.lines().next().unwrap_or("").to_string(),
                        lines_touched: changed_lines,
                    });
                }
            }

            changed_paths.sort();
            changed_paths.dedup();

            for path in &changed_paths {
                if let Some(entry) = by_path.get_mut(path) {
                    for other in &changed_paths {
                        if other != path {
                            *entry.co_changes.entry(other.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let mut items: Vec<Hotspot> = by_path.into_values().collect();
        items.sort_by_key(|hotspot| std::cmp::Reverse(hotspot.score()));

        Ok(Hotspots {
            items,
            selected_index: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.items.len()
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

    fn block(&self) -> Block {
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
    use crate::app::widgets::test_support::hotspot_with_counts;

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
