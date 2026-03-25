use std::fmt;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use color_eyre::eyre::{Result, eyre};
use gix::{Repository as Gix, discover};

use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

use super::file_tree::{FileTree, TreeAccess};
use super::git_objects::{Blob, Commit, GitObject};
use super::parser::{parse_commit, parse_tree};
use crate::models::file_change::FileChange;

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
    mut get_file_changes: impl FnMut(&Commit) -> Result<Vec<FileChange>>,
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

        let changes = get_file_changes(commit)?;
        let mut changed_paths = Vec::new();

        for change in changes {
            let location = if let Ok(relative) = change.location.strip_prefix(repo_path) {
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
                authors: std::collections::HashSet::new(),
                most_recent_days: age_days,
                recent_commits: Vec::new(),
                co_changes: std::collections::HashMap::new(),
                author_touches: std::collections::HashMap::new(),
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

    fn get_commit(&self, hash: &str) -> Result<Commit> {
        if let GitObject::Commit(commit) = self.get_object(hash)? {
            return Ok(commit);
        }
        Err(eyre!("Expected commit at {}", hash))
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn get_file_changes(&self, commit: &Commit) -> Result<Vec<FileChange>> {
        let parent_tree: Option<FileTree> = match &commit.parent {
            Some(parent_hash) => Some(FileTree::from_commit(&self.get_commit(parent_hash)?, self)?),
            None => None,
        };
        let tree = FileTree::from_commit(commit, self)?;
        Ok(tree.file_changes(parent_tree.as_ref(), self.get_path()))
    }
}

impl TreeAccess for GixRepository {
    fn get_object(&self, hash: &str) -> Result<GitObject> {
        GixRepository::get_object(self, hash)
    }

    fn get_path(&self) -> &Path {
        GixRepository::get_path(self)
    }
}

impl HotspotSource for GixRepository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        let commits = self.get_commits()?;
        build_hotspots_from_commits(&commits, self.get_path(), max_commits, |commit| {
            self.get_file_changes(commit)
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
}
