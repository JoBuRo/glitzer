use std::fmt;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use color_eyre::eyre::{Result, eyre};
use gix::prelude::TreeDiffChangeExt;
use gix::{Repository as Gix, discover};

use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

use super::git_objects::{Blob, Commit, GitObject};
use super::parser::{parse_commit, parse_tree};

#[derive(Debug, Clone)]
struct HotspotDelta {
    location: PathBuf,
    lines_added: u64,
    lines_removed: u64,
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

    fn head_hash(&self) -> Result<String> {
        let mut head = self.repo.head()?;
        let id = head
            .try_peel_to_id()?
            .ok_or_else(|| eyre!("HEAD does not resolve to an object id"))?;
        Ok(id.to_string())
    }
}

fn collect_commits_from_head(
    head_hash: String,
    mut get_object: impl FnMut(&str) -> Result<GitObject>,
) -> Result<Vec<Commit>> {
    let mut commits = Vec::new();
    let mut current_hash_opt = Some(head_hash);

    while let Some(current_hash) = current_hash_opt {
        match get_object(&current_hash)? {
            GitObject::Commit(commit) => {
                current_hash_opt = commit.parent.clone();
                commits.push(commit);
            }
            _ => {
                return Err(eyre!(
                    "Expected commit object, found different type for hash {}",
                    current_hash
                ));
            }
        }
    }

    Ok(commits)
}

fn build_hotspots_from_commits(
    commits: &[Commit],
    repo_path: &Path,
    max_commits: usize,
    mut get_hotspot_deltas: impl FnMut(&Commit) -> Result<Vec<HotspotDelta>>,
) -> Result<Vec<Hotspot>> {
    let mut by_path: std::collections::HashMap<String, Hotspot> = std::collections::HashMap::new();
    let now = chrono::Utc::now();

    for commit in commits.iter().take(max_commits) {
        let age_days = now
            .signed_duration_since(commit.authored_at)
            .num_days()
            .max(0);
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
            entry.authors.insert(commit.author.email.clone());
            entry.most_recent_days = entry.most_recent_days.min(age_days);

            let author_key = format!("{} <{}>", commit.author.name, commit.author.email);
            *entry.author_touches.entry(author_key).or_insert(0) += 1;

            if entry.recent_commits.len() < 8 {
                entry
                    .recent_commits
                    .push(crate::models::hotspot::CommitEvidence {
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
    Ok(items)
}

impl GixRepository {
    fn get_object(&self, hash: &str) -> Result<GitObject> {
        let object_id = gix::hash::ObjectId::from_hex(hash.as_bytes())
            .map_err(|err| eyre!("Invalid object id '{}': {err}", hash))?;
        let object = self.repo.find_object(object_id)?;
        let kind = object.kind;
        let data = &object.data;

        match kind {
            gix::objs::Kind::Blob => Ok(GitObject::Blob(Blob {
                hash: hash.to_string(),
                content: Bytes::copy_from_slice(data),
            })),
            gix::objs::Kind::Tree => Ok(GitObject::Tree(parse_tree(data, hash)?)),
            gix::objs::Kind::Commit => {
                let commit = parse_commit(hash.to_string(), std::str::from_utf8(data)?)?;
                Ok(GitObject::Commit(commit))
            }
            gix::objs::Kind::Tag => Err(eyre!("Annotated tag objects are not supported yet")),
        }
    }

    fn get_commits(&self) -> Result<Vec<Commit>> {
        collect_commits_from_head(self.head_hash()?, |hash| self.get_object(hash))
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn tree_for_commit_hash(&self, hash: &str) -> Result<gix::Tree<'_>> {
        let object_id = gix::hash::ObjectId::from_hex(hash.as_bytes())
            .map_err(|err| eyre!("Invalid object id '{}': {err}", hash))?;
        Ok(self.repo.find_commit(object_id)?.tree()?)
    }

    fn get_hotspot_deltas(&self, commit: &Commit) -> Result<Vec<HotspotDelta>> {
        let new_tree = self.tree_for_commit_hash(&commit.hash)?;
        let old_tree = match commit.parent.as_deref() {
            Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash)?),
            None => None,
        };

        let mut diff_opts = gix::diff::Options::default();
        diff_opts.track_path().track_rewrites(None);

        let changes =
            self.repo
                .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(diff_opts))?;

        let mut resource_cache = self.repo.diff_resource_cache_for_tree_diff()?;
        let mut deltas = Vec::new();

        for change in changes {
            let attached = change.attach(&self.repo, &self.repo);

            let location = match &change {
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
                } => {
                    if entry_mode.is_tree() {
                        resource_cache.clear_resource_cache_keep_allocation();
                        continue;
                    }

                    PathBuf::from(String::from_utf8_lossy(location.as_ref()).into_owned())
                }
            };

            let line_counts = attached.diff(&mut resource_cache)?.line_counts()?;
            let (lines_added, lines_removed) = match line_counts {
                Some(counts) => (u64::from(counts.insertions), u64::from(counts.removals)),
                None => (0, 0),
            };

            deltas.push(HotspotDelta {
                location: self.get_path().join(location),
                lines_added,
                lines_removed,
            });

            resource_cache.clear_resource_cache_keep_allocation();
        }

        Ok(deltas)
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
    use crate::git::git_objects::{Author, Blob};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    #[test]
    fn collect_commits_from_head_follows_parent_chain() {
        let author = Author {
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
        };

        let commit_a = Commit {
            hash: "a".to_string(),
            parent: None,
            tree: "tree-a".to_string(),
            message: "A".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author.clone(),
            committed_at: Utc::now(),
        };

        let commit_b = Commit {
            hash: "b".to_string(),
            parent: Some("a".to_string()),
            tree: "tree-b".to_string(),
            message: "B".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author.clone(),
            committed_at: Utc::now(),
        };

        let commit_c = Commit {
            hash: "c".to_string(),
            parent: Some("b".to_string()),
            tree: "tree-c".to_string(),
            message: "C".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author,
            committed_at: Utc::now(),
        };

        let objects: HashMap<String, GitObject> = HashMap::from([
            ("a".to_string(), GitObject::Commit(commit_a)),
            ("b".to_string(), GitObject::Commit(commit_b)),
            ("c".to_string(), GitObject::Commit(commit_c)),
        ]);

        let commits = collect_commits_from_head("c".to_string(), |hash| {
            objects
                .get(hash)
                .cloned()
                .ok_or_else(|| eyre!("Object {} not found", hash))
        })
        .unwrap();

        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].hash, "c");
        assert_eq!(commits[1].hash, "b");
        assert_eq!(commits[2].hash, "a");
    }

    #[test]
    fn collect_commits_from_head_rejects_non_commit_objects() {
        let objects: HashMap<String, GitObject> = HashMap::from([(
            "c".to_string(),
            GitObject::Blob(Blob {
                hash: "c".to_string(),
                content: Bytes::from_static(b"blob"),
            }),
        )]);

        let result = collect_commits_from_head("c".to_string(), |hash| {
            objects
                .get(hash)
                .cloned()
                .ok_or_else(|| eyre!("Object {} not found", hash))
        });

        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("Expected commit object")
        );
    }

    #[test]
    fn build_hotspots_counts_root_commit_additions() {
        let author = Author {
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
        };

        let root = Commit {
            hash: "root".to_string(),
            parent: None,
            tree: "tree-root".to_string(),
            message: "root".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author.clone(),
            committed_at: Utc::now(),
        };

        let child = Commit {
            hash: "child".to_string(),
            parent: Some("root".to_string()),
            tree: "tree-child".to_string(),
            message: "child".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author,
            committed_at: Utc::now(),
        };

        let commits = vec![child, root];
        let repo_path = Path::new("/repo");

        let hotspots = build_hotspots_from_commits(&commits, repo_path, 100, |commit| {
            let delta = match commit.hash.as_str() {
                "root" => HotspotDelta {
                    location: PathBuf::from("/repo/src/main.rs"),
                    lines_added: 3,
                    lines_removed: 0,
                },
                "child" => HotspotDelta {
                    location: PathBuf::from("/repo/src/main.rs"),
                    lines_added: 1,
                    lines_removed: 1,
                },
                _ => unreachable!(),
            };

            Ok(vec![delta])
        })
        .unwrap();

        assert_eq!(hotspots.len(), 1);
        assert_eq!(hotspots[0].location, "src/main.rs");
        assert_eq!(hotspots[0].touches, 2);
        assert_eq!(hotspots[0].lines_touched, 4);
    }
}
