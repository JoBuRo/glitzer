use std::fmt;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use gix::prelude::TreeDiffChangeExt;
use gix::{Commit, Id, ObjectId, Repository as Gix, discover};

use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

#[derive(Debug, Copy, Clone)]
enum TraversalPolicy {
    FirstParent,
}

const TRAVERSAL_POLICY: TraversalPolicy = TraversalPolicy::FirstParent;

#[derive(Debug, Clone)]
struct HotspotDelta {
    location: PathBuf,
    lines_added: u64,
    lines_removed: u64,
}

#[derive(Debug, Clone)]
struct CommitMetadata {
    short_id: String,
    author_name: String,
    author_email: String,
    authored_time_seconds: i64,
    committed_at: String,
    message_title: String,
}

trait CommitLike {
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

#[derive(Debug, Clone)]
struct FileDiffChange {
    location: PathBuf,
    is_tree: bool,
    lines_added: u64,
    lines_removed: u64,
}

trait DeltaProvider<C> {
    fn delta_changes(&self, commit: &C) -> Result<Vec<FileDiffChange>>;
}

impl HotspotDelta {
    fn lines_touched(&self) -> u64 {
        std::cmp::max(self.lines_added, self.lines_removed)
    }
}

pub struct GixRepository {
    repo: Gix,
    path: PathBuf,
}

impl GixRepository {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let repo = discover(path)?;
        let path = repo
            .workdir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo.path().to_path_buf());
        Ok(Self { repo, path })
    }

    fn head_hash(&self) -> Result<Id<'_>> {
        let mut head = self.repo.head()?;
        let id = head
            .try_peel_to_id()?
            .ok_or_else(|| eyre!("HEAD does not resolve to an object id"))?;
        Ok(id)
    }
}

fn build_hotspots_from_commits<C: CommitLike>(
    commits: &[C],
    repo_path: &Path,
    max_commits: usize,
    mut get_hotspot_deltas: impl FnMut(&C) -> Result<Vec<HotspotDelta>>,
) -> Result<Vec<Hotspot>> {
    let mut by_path: std::collections::HashMap<String, Hotspot> = std::collections::HashMap::new();
    let now = chrono::Utc::now();

    for commit in commits.iter().take(max_commits) {
        let commit_metadata = commit.metadata()?;
        let age_days = ((now.timestamp() - commit_metadata.authored_time_seconds).max(0)) / 86_400;
        let recent_points = match age_days {
            0..=7 => 3,
            8..=30 => 2,
            _ => 1,
        };

        let changes = get_hotspot_deltas(commit)?;
        let mut changed_paths = Vec::new();

        for delta in changes {
            let location = if let Ok(relative) = delta.location.strip_prefix(repo_path) {
                relative.to_string_lossy().to_string()
            } else {
                delta.location.to_string_lossy().to_string()
            };

            changed_paths.push(location.clone());

            let entry = by_path.entry(location.clone()).or_insert_with(|| Hotspot {
                location,
                touches: 0,
                lines_touched: 0,
                recent_points: 0,
                authors: std::collections::HashSet::new(),
                most_recent_days: age_days,
                recent_commits: Vec::new(),
                co_changes: std::collections::HashMap::new(),
                author_touches: std::collections::HashMap::new(),
            });

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
    Ok(items)
}

fn get_hotspot_deltas_for_commit<C>(
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
            lines_added: change.lines_added,
            lines_removed: change.lines_removed,
        })
        .collect())
}

impl GixRepository {
    fn selected_parent_id(commit: &Commit<'_>) -> Option<ObjectId> {
        match TRAVERSAL_POLICY {
            TraversalPolicy::FirstParent => commit.parent_ids().next().map(Into::into),
        }
    }

    fn get_commits(&self) -> Result<Vec<Commit<'_>>> {
        let mut commits = Vec::new();
        let mut commit_id_opt: Option<ObjectId> = Some(self.head_hash()?.into());

        while let Some(commit_id) = commit_id_opt {
            let commit = self.repo.find_commit(commit_id)?;
            commit_id_opt = Self::selected_parent_id(&commit);
            commits.push(commit);
        }

        Ok(commits)
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn tree_for_commit_hash(&self, object_id: ObjectId) -> Result<gix::Tree<'_>> {
        Ok(self.repo.find_commit(object_id)?.tree()?)
    }

    fn get_hotspot_deltas(&self, commit: &Commit) -> Result<Vec<HotspotDelta>> {
        get_hotspot_deltas_for_commit(self.get_path(), self, commit)
    }
}

impl DeltaProvider<Commit<'_>> for GixRepository {
    fn delta_changes(&self, commit: &Commit<'_>) -> Result<Vec<FileDiffChange>> {
        let new_tree = self.tree_for_commit_hash(commit.id)?;
        let old_tree = match GixRepository::selected_parent_id(commit) {
            Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash)?),
            None => None,
        };

        let mut diff_opts = gix::diff::Options::default();
        diff_opts.track_path().track_rewrites(None);

        let changes =
            self.repo
                .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(diff_opts))?;

        let mut resource_cache = self.repo.diff_resource_cache_for_tree_diff()?;
        let mut changes_for_commit = Vec::new();

        for change in changes {
            let attached = change.attach(&self.repo, &self.repo);

            let (is_tree, location) = match &change {
                gix::object::tree::diff::ChangeDetached::Addition {
                    entry_mode,
                    location,
                    ..
                }
                | gix::object::tree::diff::ChangeDetached::Deletion {
                    entry_mode,
                    location,
                    ..
                }
                | gix::object::tree::diff::ChangeDetached::Modification {
                    entry_mode,
                    location,
                    ..
                }
                | gix::object::tree::diff::ChangeDetached::Rewrite {
                    entry_mode,
                    location,
                    ..
                } => (
                    entry_mode.is_tree(),
                    PathBuf::from(String::from_utf8_lossy(location.as_ref()).into_owned()),
                ),
            };

            if is_tree {
                resource_cache.clear_resource_cache_keep_allocation();
                continue;
            }

            let line_counts = attached.diff(&mut resource_cache)?.line_counts()?;
            let (lines_added, lines_removed) = match line_counts {
                Some(counts) => (u64::from(counts.insertions), u64::from(counts.removals)),
                None => (0, 0),
            };

            changes_for_commit.push(FileDiffChange {
                location,
                is_tree,
                lines_added,
                lines_removed,
            });

            resource_cache.clear_resource_cache_keep_allocation();
        }

        Ok(changes_for_commit)
    }
}

impl HotspotSource for GixRepository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        let commits = self.get_commits()?;
        build_hotspots_from_commits(&commits, self.get_path(), max_commits, |commit| {
            self.get_hotspot_deltas(commit)
        })
    }
}

impl fmt::Debug for GixRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GixRepository at {}", self.path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        impl super::DeltaProvider<FakeCommit> for DeltaProvider {
            fn delta_changes(&self, commit: &FakeCommit) -> Result<Vec<super::FileDiffChange>>;
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
                    is_tree: false,
                    lines_added: 5,
                    lines_removed: 2,
                },
                FileDiffChange {
                    location: PathBuf::from("src"),
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
                lines_added: 3,
                lines_removed: 0,
            }],
            vec![HotspotDelta {
                location: PathBuf::from("/repo/src/main.rs"),
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
                    lines_added: 2,
                    lines_removed: 0,
                },
                HotspotDelta {
                    location: PathBuf::from("/repo/src/lib.rs"),
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
}
