use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use gix::Commit;

use super::diff_changes::DeltaProvider;
use super::path_continuity::{PathAliases, register_path_alias, resolve_canonical_path};
use crate::models::hotspot::Hotspot;

#[derive(Debug, Clone)]
pub(crate) struct HotspotDelta {
    pub(crate) location: PathBuf,
    pub(crate) previous_location: Option<PathBuf>,
    pub(crate) lines_added: u64,
    pub(crate) lines_removed: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct CommitMetadata {
    short_id: String,
    author_name: String,
    author_email: String,
    authored_time_seconds: i64,
    committed_at: String,
    message_title: String,
}

pub(crate) trait CommitLike {
    fn metadata(&self) -> Result<CommitMetadata>;
}

impl CommitLike for Commit<'_> {
    fn metadata(&self) -> Result<CommitMetadata> {
        let author_signature = self.author()?.to_owned()?;
        Ok(CommitMetadata {
            short_id: self.short_id()?.to_string(),
            author_name: author_signature.name.to_string(),
            author_email: author_signature.email.to_string(),
            authored_time_seconds: author_signature.time.seconds,
            committed_at: self.committer()?.time.to_string(),
            message_title: self.message()?.title.to_string(),
        })
    }
}

impl HotspotDelta {
    fn lines_touched(&self) -> u64 {
        std::cmp::max(self.lines_added, self.lines_removed)
    }
}

fn recent_points_for_age(age_days: i64) -> u64 {
    match age_days {
        0..=7 => 3,
        8..=30 => 2,
        _ => 1,
    }
}

fn normalize_location(repo_path: &Path, location: &Path) -> String {
    if let Ok(relative) = location.strip_prefix(repo_path) {
        relative.to_string_lossy().to_string()
    } else {
        location.to_string_lossy().to_string()
    }
}

fn get_or_create_hotspot(
    by_path: &mut HashMap<String, Hotspot>,
    location: String,
    age_days: i64,
) -> &mut Hotspot {
    by_path.entry(location.clone()).or_insert_with(|| Hotspot {
        location,
        touches: 0,
        lines_touched: 0,
        recent_points: 0,
        authors: std::collections::HashSet::new(),
        most_recent_days: age_days,
        recent_commits: Vec::new(),
        co_changes: HashMap::new(),
        author_touches: HashMap::new(),
    })
}

fn apply_delta_to_hotspot(
    entry: &mut Hotspot,
    delta: &HotspotDelta,
    commit_metadata: &CommitMetadata,
    age_days: i64,
    recent_points: u64,
) {
    entry.touches += 1;
    let changed_lines = delta.lines_touched();
    entry.lines_touched += changed_lines;
    entry.recent_points += recent_points;
    entry.authors.insert(commit_metadata.author_email.clone());
    entry.most_recent_days = entry.most_recent_days.min(age_days);

    let author_key = format!(
        "{} <{}>",
        commit_metadata.author_name, commit_metadata.author_email
    );
    *entry.author_touches.entry(author_key).or_insert(0) += 1;

    if entry.recent_commits.len() < 8 {
        entry
            .recent_commits
            .push(crate::models::hotspot::CommitEvidence {
                hash: commit_metadata.short_id.clone(),
                author: commit_metadata.author_name.clone(),
                committed_at: commit_metadata.committed_at.clone(),
                message: commit_metadata.message_title.clone(),
                lines_touched: changed_lines,
            });
    }
}

fn record_co_changes(by_path: &mut HashMap<String, Hotspot>, changed_paths: &[String]) {
    for path in changed_paths {
        if let Some(entry) = by_path.get_mut(path) {
            for other in changed_paths {
                if other != path {
                    *entry.co_changes.entry(other.clone()).or_insert(0) += 1;
                }
            }
        }
    }
}

pub(crate) fn build_hotspots_from_commits<C: CommitLike>(
    commits: &[C],
    repo_path: &Path,
    max_commits: usize,
    mut get_hotspot_deltas: impl FnMut(&C) -> Result<Vec<HotspotDelta>>,
) -> Result<Vec<Hotspot>> {
    let mut by_path: HashMap<String, Hotspot> = HashMap::new();
    let mut path_aliases: PathAliases = HashMap::new();
    let now = chrono::Utc::now();

    for commit in commits.iter().take(max_commits) {
        let commit_metadata = commit.metadata()?;
        let age_days = ((now.timestamp() - commit_metadata.authored_time_seconds).max(0)) / 86_400;
        let recent_points = recent_points_for_age(age_days);

        let changes = get_hotspot_deltas(commit)?;
        let mut changed_paths = Vec::new();

        for delta in changes {
            if let Some(previous_location) = &delta.previous_location {
                let old_path = normalize_location(repo_path, previous_location);
                let new_path = normalize_location(repo_path, &delta.location);
                register_path_alias(&mut path_aliases, &old_path, &new_path);
            }

            let location = normalize_location(repo_path, &delta.location);
            let canonical_location = resolve_canonical_path(&location, &path_aliases);

            changed_paths.push(canonical_location.clone());

            let entry = get_or_create_hotspot(&mut by_path, canonical_location, age_days);
            apply_delta_to_hotspot(entry, &delta, &commit_metadata, age_days, recent_points);
        }

        changed_paths.sort();
        changed_paths.dedup();

        record_co_changes(&mut by_path, &changed_paths);
    }

    let mut items: Vec<Hotspot> = by_path.into_values().collect();
    items.sort_by_key(|hotspot| std::cmp::Reverse(hotspot.score()));
    Ok(items)
}

pub(crate) fn get_hotspot_deltas_for_commit<C>(
    repo_path: &Path,
    provider: &impl DeltaProvider<C>,
    commit: &C,
) -> Result<Vec<HotspotDelta>> {
    let changes = provider.delta_changes(commit)?;
    Ok(changes
        .into_iter()
        .filter(|change| !change.is_tree)
        .map(|change| HotspotDelta {
            location: repo_path.join(change.location),
            previous_location: change.previous_location.map(|p| repo_path.join(p)),
            lines_added: change.lines_added,
            lines_removed: change.lines_removed,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff_changes::FileDiffChange;
    use mockall::mock;
    use std::collections::VecDeque;

    mock! {
        CommitLike {}
        impl super::CommitLike for CommitLike {
            fn metadata(&self) -> Result<super::CommitMetadata>;
        }
    }

    #[derive(Debug)]
    struct FakeCommit;

    mock! {
        DeltaProvider {}
        impl crate::git::diff_changes::DeltaProvider<FakeCommit> for DeltaProvider {
            fn delta_changes(&self, commit: &FakeCommit) -> Result<Vec<crate::git::diff_changes::FileDiffChange>>;
        }
    }

    fn commit_metadata(short_id: &str, author: &str, email: &str) -> CommitMetadata {
        CommitMetadata {
            short_id: short_id.to_string(),
            author_name: author.to_string(),
            author_email: email.to_string(),
            authored_time_seconds: chrono::Utc::now().timestamp(),
            committed_at: "1234 +0000".to_string(),
            message_title: format!("msg-{}", short_id),
        }
    }

    #[test]
    fn get_hotspot_deltas_filters_tree_entries_and_joins_repo_path() {
        let repo_path = Path::new("/repo");
        let commit = FakeCommit;
        let mut provider = MockDeltaProvider::new();

        provider.expect_delta_changes().times(1).return_once(|_| {
            Ok(vec![
                FileDiffChange {
                    location: PathBuf::from("src/main.rs"),
                    previous_location: None,
                    is_tree: false,
                    lines_added: 5,
                    lines_removed: 2,
                },
                FileDiffChange {
                    location: PathBuf::from("src"),
                    previous_location: None,
                    is_tree: true,
                    lines_added: 0,
                    lines_removed: 0,
                },
            ])
        });

        let deltas = get_hotspot_deltas_for_commit(repo_path, &provider, &commit)
            .expect("build hotspot deltas from provider");

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].location, PathBuf::from("/repo/src/main.rs"));
        assert_eq!(deltas[0].lines_added, 5);
        assert_eq!(deltas[0].lines_removed, 2);
    }

    #[test]
    fn build_hotspots_from_commits_aggregates_multiple_mock_commits() {
        let mut newest = MockCommitLike::new();
        newest
            .expect_metadata()
            .times(1)
            .return_once(|| Ok(commit_metadata("aaa1111", "Alice", "alice@example.com")));

        let mut older = MockCommitLike::new();
        older
            .expect_metadata()
            .times(1)
            .return_once(|| Ok(commit_metadata("bbb2222", "Alice", "alice@example.com")));

        let commits = vec![newest, older];
        let mut queued_deltas = VecDeque::from([
            vec![HotspotDelta {
                location: PathBuf::from("/repo/src/main.rs"),
                previous_location: None,
                lines_added: 3,
                lines_removed: 0,
            }],
            vec![HotspotDelta {
                location: PathBuf::from("/repo/src/main.rs"),
                previous_location: None,
                lines_added: 1,
                lines_removed: 1,
            }],
        ]);

        let hotspots = build_hotspots_from_commits(&commits, Path::new("/repo"), 100, |_| {
            Ok(queued_deltas
                .pop_front()
                .expect("queued deltas for each commit"))
        })
        .expect("build hotspots from mock commits");

        assert_eq!(hotspots.len(), 1);
        let hotspot = &hotspots[0];
        assert_eq!(hotspot.location, "src/main.rs");
        assert_eq!(hotspot.touches, 2);
        assert_eq!(hotspot.lines_touched, 4);
        assert_eq!(hotspot.author_count(), 1);
        assert_eq!(hotspot.recent_commits.len(), 2);
    }

    #[test]
    fn build_hotspots_from_commits_records_co_change_links() {
        let mut commit = MockCommitLike::new();
        commit
            .expect_metadata()
            .times(1)
            .return_once(|| Ok(commit_metadata("ccc3333", "Bob", "bob@example.com")));

        let hotspots = build_hotspots_from_commits(&[commit], Path::new("/repo"), 100, |_| {
            Ok(vec![
                HotspotDelta {
                    location: PathBuf::from("/repo/src/main.rs"),
                    previous_location: None,
                    lines_added: 2,
                    lines_removed: 0,
                },
                HotspotDelta {
                    location: PathBuf::from("/repo/src/lib.rs"),
                    previous_location: None,
                    lines_added: 4,
                    lines_removed: 0,
                },
            ])
        })
        .expect("build hotspots with co-change data");

        assert_eq!(hotspots.len(), 2);

        let main = hotspots
            .iter()
            .find(|h| h.location == "src/main.rs")
            .expect("hotspot for src/main.rs");
        assert_eq!(main.co_changes.get("src/lib.rs"), Some(&1));

        let lib = hotspots
            .iter()
            .find(|h| h.location == "src/lib.rs")
            .expect("hotspot for src/lib.rs");
        assert_eq!(lib.co_changes.get("src/main.rs"), Some(&1));
    }

    #[test]
    fn recent_points_for_age_uses_expected_buckets() {
        assert_eq!(recent_points_for_age(0), 3);
        assert_eq!(recent_points_for_age(7), 3);
        assert_eq!(recent_points_for_age(8), 2);
        assert_eq!(recent_points_for_age(30), 2);
        assert_eq!(recent_points_for_age(31), 1);
    }

    #[test]
    fn normalize_location_prefers_repo_relative_path() {
        let repo_path = Path::new("/repo");
        let in_repo = Path::new("/repo/src/main.rs");
        let outside_repo = Path::new("/other/path.rs");

        assert_eq!(normalize_location(repo_path, in_repo), "src/main.rs");
        assert_eq!(
            normalize_location(repo_path, outside_repo),
            "/other/path.rs"
        );
    }

    #[test]
    fn build_hotspots_from_commits_merges_rename_history_into_destination_path() {
        let mut commit = MockCommitLike::new();
        commit
            .expect_metadata()
            .times(1)
            .return_once(|| Ok(commit_metadata("ddd4444", "Dana", "dana@example.com")));

        let hotspots = build_hotspots_from_commits(&[commit], Path::new("/repo"), 100, |_| {
            Ok(vec![
                HotspotDelta {
                    location: PathBuf::from("/repo/src/new.rs"),
                    previous_location: Some(PathBuf::from("/repo/src/old.rs")),
                    lines_added: 1,
                    lines_removed: 0,
                },
                HotspotDelta {
                    location: PathBuf::from("/repo/src/old.rs"),
                    previous_location: None,
                    lines_added: 2,
                    lines_removed: 0,
                },
            ])
        })
        .expect("build hotspots with rename continuity");

        assert!(hotspots.iter().any(|h| h.location == "src/new.rs"));
        assert!(!hotspots.iter().any(|h| h.location == "src/old.rs"));
    }
}
