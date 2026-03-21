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
