use std::fmt;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use color_eyre::eyre::{Result, eyre};
use gix::{Repository as Gix, discover};

use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

use super::file_tree::{FileChange, FileTree};
use super::git_objects::{Blob, Commit, GitObject};
use super::parser::{parse_commit, parse_tree};
use super::repo::{GitDataAccess, build_hotspots};

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

impl GitDataAccess for GixRepository {
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

impl HotspotSource for GixRepository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        build_hotspots(self, max_commits)
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
