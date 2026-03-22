use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Hotspot {
    pub(crate) location: String,
    pub(crate) touches: u64,
    pub(crate) lines_touched: u64,
    pub(crate) recent_points: u64,
    pub(crate) authors: HashSet<String>,
    pub(crate) most_recent_days: i64,
    pub(crate) recent_commits: Vec<CommitEvidence>,
    pub(crate) co_changes: HashMap<String, u64>,
    pub(crate) author_touches: HashMap<String, u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct CommitEvidence {
    pub(crate) hash: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
    pub(crate) message: String,
    pub(crate) lines_touched: u64,
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
