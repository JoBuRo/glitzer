use std::fmt;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use gix::prelude::TreeDiffChangeExt;
use gix::{Commit, Id, ObjectId, Repository as Gix, discover};

use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

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
    fn get_commits(&self) -> Result<Vec<Commit<'_>>> {
        let mut commits = Vec::new();
        let mut commit_id_opt = Some(self.head_hash()?);

        while let Some(commit_id) = commit_id_opt {
            let commit = self.repo.find_commit(commit_id)?;
            commit_id_opt = commit.parent_ids().next();
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
        let old_tree = match commit.parent_ids().next() {
            Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash.into())?),
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
