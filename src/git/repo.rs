use super::author::Author;

use super::file_tree::{FileChange, FileTree};
use super::git_objects::*;
use super::parser::*;
use crate::models::hotspot::{CommitEvidence, Hotspot};
use crate::models::hotspot_source::HotspotSource;
use bytes::Bytes;
use chrono::Utc;
use color_eyre::eyre::eyre;
use color_eyre::{eyre::WrapErr, Result};
use flate2::read::ZlibDecoder;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::path;
use std::path::Path;

pub trait RepositoryAccess {
    fn get_object(&self, hash: &str) -> Result<GitObject>;

    fn get_commits(&self) -> Result<Vec<Commit>>;

    fn get_commit(&self, hash: &str) -> Result<Commit>;

    fn get_path(&self) -> &Path;

    fn get_file_changes(&self, commit: &Commit) -> Result<Vec<FileChange>>;

    fn get_authors(&self) -> Result<Vec<Author>> {
        let mut author_map: HashMap<String, Author> = std::collections::HashMap::new();
        let commits = self.get_commits()?;

        for commit in commits {
            let key = &commit.author.email;

            if let Some(author) = author_map.get_mut(key) {
                author.add_commit(commit.clone());
            } else {
                let mut author =
                    Author::new(commit.author.name.clone(), commit.author.email.clone());
                author.add_commit(commit.clone());
                author_map.insert(key.clone(), author);
            }
        }

        let authors: Vec<Author> = author_map.into_values().collect();
        Ok(authors)
    }
}

pub struct Repository {
    pub path: String,
    head: String,
    current_branch: String,
}

impl RepositoryAccess for Repository {
    fn get_object(&self, hash: &str) -> Result<GitObject> {
        let file_path = format!("{}/.git/objects/{}/{}", self.path, &hash[0..2], &hash[2..]);
        read_object(&file_path)
    }

    fn get_commit(&self, hash: &str) -> Result<Commit> {
        if let GitObject::Commit(commit) = self.get_object(hash)? {
            return Ok(commit);
        }
        Err(eyre!("Expected commit at {}", hash))
    }

    fn get_commits(&self) -> Result<Vec<Commit>> {
        let mut commits = Vec::new();
        let mut current_hash_opt = Some(self.head.clone());

        while let Some(current_hash) = current_hash_opt {
            let object = self.get_object(&current_hash)?;

            match object {
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

    fn get_path(&self) -> &Path {
        Path::new(&self.path)
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

impl Repository {
    pub fn new(path: String) -> Result<Self> {
        let absolute_path = path::absolute(&path)?
            .into_os_string()
            .into_string()
            .unwrap_or(path.clone());

        let head_path = format!("{}/.git/HEAD", absolute_path);
        let head_content = std::fs::read_to_string(&head_path)
            .wrap_err_with(|| format!("Failed to read HEAD file at {}", head_path))?;

        let ref_path = head_content[5..].trim();
        let full_ref_path = format!("{}/.git/{}", absolute_path, ref_path);
        let ref_content = std::fs::read_to_string(&full_ref_path)
            .wrap_err_with(|| format!("Failed to read reference file at {}", full_ref_path))?;
        let head_hash = ref_content.trim().to_string();

        let current_branch = ref_path
            .strip_prefix("refs/heads/")
            .unwrap_or(ref_path)
            .to_string();

        let repo = Repository {
            path: absolute_path,
            head: head_hash,
            current_branch,
        };
        Ok(repo)
    }
}

pub(crate) fn build_hotspots(
    repo: &impl RepositoryAccess,
    max_commits: usize,
) -> Result<Vec<Hotspot>> {
    let mut by_path: HashMap<String, Hotspot> = HashMap::new();
    let now = Utc::now();
    let commits = repo.get_commits()?;

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
                authors: std::collections::HashSet::new(),
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
    Ok(items)
}

impl HotspotSource for Repository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        build_hotspots(self, max_commits)
    }
}

impl fmt::Debug for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repository at {}, current branch: {}",
            self.path, self.current_branch
        )
    }
}

fn read_bytes(file_path: &str) -> Result<Bytes> {
    let data = std::fs::read(file_path)?;
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decoded_bytes = Vec::new();
    // read_to_end expects &mut Vec<u8>
    decoder.read_to_end(&mut decoded_bytes)?;
    Ok(Bytes::from(decoded_bytes))
}

fn parse_header(header_str: &str) -> Result<ObjectHeader> {
    let mut parts = header_str.split(' ');

    let object_type_str = parts.next().ok_or(eyre!("Missing object type"))?;
    let size_str = parts.next().ok_or(eyre!("Missing size"))?;

    let object_type = match object_type_str {
        "blob" => ObjectType::Blob,
        "tree" => ObjectType::Tree,
        "commit" => ObjectType::Commit,
        "tag" => ObjectType::AnnotatedTag,
        _ => return Err(eyre!("Unknown object type: {}", object_type_str)),
    };

    let size = size_str.parse::<u64>()?;

    Ok(ObjectHeader { object_type, size })
}

fn get_hash(content: &Bytes) -> String {
    let mut hasher = Sha1::new();

    hasher.update(content);
    let hash_bytes = hasher.finalize();
    hex::encode(hash_bytes)
}

fn parse_object(bytes: &Bytes) -> Result<RawObject> {
    let hash = get_hash(bytes);
    let mut bytes_split = bytes.splitn(2, |&b| b == 0);

    let header_bytes = bytes_split.next().ok_or(eyre!("Missing header"))?;

    let header_str = std::str::from_utf8(header_bytes)?;

    let header = parse_header(header_str)?;

    let content = bytes_split.next().ok_or(eyre!("Missing content"))?;

    Ok(RawObject {
        hash,
        header,
        content: Bytes::from(content.to_vec()),
    })
}

fn read_raw_object(file_path: &str) -> Result<RawObject> {
    let bytes = read_bytes(file_path)?;
    parse_object(&bytes)
}

pub fn read_object(file_path: &str) -> Result<GitObject> {
    let object = read_raw_object(file_path)?;

    match object.header.object_type {
        ObjectType::Blob => Ok(GitObject::Blob(Blob {
            hash: object.hash.clone(),
            content: object.content.clone(),
        })),
        ObjectType::Tree => {
            let tree = parse_tree(&object.content[..], &object.hash)?;
            Ok(GitObject::Tree(tree))
        }
        ObjectType::Commit => {
            let body = std::str::from_utf8(&object.content[..])?;
            let commit = parse_commit(object.hash, body)?;
            Ok(GitObject::Commit(commit))
        }
        ObjectType::AnnotatedTag => Err(eyre!("AnnotatedTag coercion not implemented")),
    }
}

#[cfg(test)]
mod tests {
    use super::super::git_objects;
    use super::*;
    use chrono::Utc; // Import to access git_objects::Author

    #[test]
    fn test_parse_object() {
        let bytes = Bytes::from(&b"blob 14\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_ok());
        let object = result.unwrap();
        assert_eq!(object.hash, get_hash(&bytes));
        assert_eq!(object.header.object_type, ObjectType::Blob);
        assert_eq!(object.header.size, 14);
        assert_eq!(object.content, Bytes::from(&b"Hello, Glitzer!"[..]));
    }

    #[test]
    fn test_parse_object_invalid_header_missing_size() {
        let bytes = Bytes::from(&b"blob\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("Missing size"));
    }

    #[test]
    fn test_parse_object_invalid_header_unknown_type() {
        let bytes = Bytes::from(&b"invalid_header 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("Unknown object type"));
    }

    #[test]
    fn test_parse_object_invalid_utf8_header() {
        let bytes = Bytes::from(&b"\xFF\xFF\xFF 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("invalid utf-8"));
    }

    // Shared MockRepo for testing RepositoryAccess implementations
    struct MockRepo {
        objects: HashMap<String, GitObject>,
    }

    impl RepositoryAccess for MockRepo {
        fn get_commits(&self) -> Result<Vec<Commit>> {
            // Return commits by traversing from the synthetic HEAD commit.
            let mut commits = Vec::new();
            let mut current_hash_opt = if self.objects.contains_key("HEAD_COMMIT") {
                Some("HEAD_COMMIT".to_string())
            } else {
                None
            };

            while let Some(current_hash) = current_hash_opt {
                if current_hash == "HEAD_HASH" {
                    if let Some(GitObject::Commit(commit)) = self.objects.get("HEAD_COMMIT") {
                        current_hash_opt = commit.parent.clone();
                        commits.push(commit.clone());
                    } else {
                        break;
                    }
                } else if let Some(GitObject::Commit(commit)) = self.objects.get(&current_hash) {
                    current_hash_opt = commit.parent.clone();
                    commits.push(commit.clone());
                } else {
                    break;
                }
            }

            Ok(commits)
        }

        fn get_object(&self, hash: &str) -> Result<GitObject> {
            self.objects
                .get(hash)
                .cloned()
                .ok_or_else(|| eyre!("Object with hash {} not found", hash))
        }

        fn get_commit(&self, hash: &str) -> Result<Commit> {
            if let GitObject::Commit(commit) = self.get_object(hash)? {
                return Ok(commit);
            }
            Err(eyre!("Object with hash {} is not a commit", hash))
        }

        fn get_path(&self) -> &Path {
            Path::new("mock_repo")
        }

        fn get_file_changes(&self, commit: &Commit) -> Result<Vec<FileChange>> {
            let parent_tree: Option<FileTree> = match &commit.parent {
                Some(parent_hash) => {
                    Some(FileTree::from_commit(&self.get_commit(parent_hash)?, self)?)
                }
                None => None,
            };
            let tree = FileTree::from_commit(commit, self)?;
            Ok(tree.file_changes(parent_tree.as_ref(), self.get_path()))
        }
    }

    fn make_test_commit(
        hash: &str,
        parent: Option<&str>,
        tree: &str,
        name: &str,
        email: &str,
    ) -> Commit {
        // Note: Author in git_objects is different from Author in author.rs
        // We use the git_objects::Author type here
        let author = git_objects::Author {
            name: name.to_string(),
            email: email.to_string(),
        };

        Commit {
            hash: hash.to_string(),
            parent: parent.map(|p| p.to_string()),
            tree: tree.to_string(),
            message: format!("commit {}", hash),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author,
            committed_at: Utc::now(),
        }
    }

    #[test]
    fn test_get_authors() {
        // Create commits from different authors
        let commit1 = make_test_commit("c1", None, "t1", "Alice", "alice@example.com");
        let commit2 = make_test_commit("c2", Some("c1"), "t2", "Bob", "bob@example.com");
        let commit3 = make_test_commit("c3", Some("c2"), "t3", "Alice", "alice@example.com");
        let commit4 = make_test_commit("c4", Some("c3"), "t4", "Charlie", "charlie@example.com");
        let commit5 = make_test_commit("c5", Some("c4"), "t5", "Bob", "bob@example.com");

        let mut objects = HashMap::new();

        // Store commits - need to link them properly
        objects.insert(
            "HEAD_COMMIT".to_string(),
            GitObject::Commit(commit5.clone()),
        );
        objects.insert("c5".to_string(), GitObject::Commit(commit5));
        objects.insert("c4".to_string(), GitObject::Commit(commit4));
        objects.insert("c3".to_string(), GitObject::Commit(commit3));
        objects.insert("c2".to_string(), GitObject::Commit(commit2));
        objects.insert("c1".to_string(), GitObject::Commit(commit1));

        // Add minimal tree objects so FileTree::from_commit doesn't fail
        objects.insert(
            "t1".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t1".to_string(),
                entries: vec![],
            }),
        );
        objects.insert(
            "t2".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t2".to_string(),
                entries: vec![],
            }),
        );
        objects.insert(
            "t3".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t3".to_string(),
                entries: vec![],
            }),
        );
        objects.insert(
            "t4".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t4".to_string(),
                entries: vec![],
            }),
        );
        objects.insert(
            "t5".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t5".to_string(),
                entries: vec![],
            }),
        );

        let repo = MockRepo { objects };

        let authors = repo.get_authors().unwrap();

        // Should have 3 unique authors
        assert_eq!(authors.len(), 3);

        // Verify each author's email and commit count
        let mut author_map: HashMap<String, usize> = HashMap::new();
        for author in &authors {
            author_map.insert(author.email.clone(), author.commits.len());
        }

        assert_eq!(author_map.get("alice@example.com"), Some(&2));
        assert_eq!(author_map.get("bob@example.com"), Some(&2));
        assert_eq!(author_map.get("charlie@example.com"), Some(&1));
    }

    #[test]
    fn test_get_file_changes() {
        // Create parent and child commits with different trees
        let parent_commit = make_test_commit("p", None, "t_parent", "Test", "test@example.com");
        let child_commit = make_test_commit("c", Some("p"), "t_child", "Test", "test@example.com");

        let mut objects = HashMap::new();

        objects.insert("p".to_string(), GitObject::Commit(parent_commit.clone()));
        objects.insert("c".to_string(), GitObject::Commit(child_commit.clone()));

        // Parent tree has one file: foo.txt with content "old\n"
        objects.insert(
            "t_parent".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t_parent".to_string(),
                entries: vec![git_objects::TreeEntry {
                    name: "foo.txt".to_string(),
                    hash: "b_old".to_string(),
                    mode: git_objects::EntryMode::Text,
                }],
            }),
        );

        // Child tree has two files:
        // - foo.txt (modified): "old\nmodified\n"
        // - bar.txt (added): "new content\n"
        objects.insert(
            "t_child".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t_child".to_string(),
                entries: vec![
                    git_objects::TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "b_new_file".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                    git_objects::TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b_modified".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                ],
            }),
        );

        // Original foo.txt
        objects.insert(
            "b_old".to_string(),
            GitObject::Blob(Blob {
                hash: "b_old".to_string(),
                content: Bytes::from("old\n"),
            }),
        );

        // Modified foo.txt - added one line
        objects.insert(
            "b_modified".to_string(),
            GitObject::Blob(Blob {
                hash: "b_modified".to_string(),
                content: Bytes::from("old\nmodified\n"),
            }),
        );

        // New bar.txt - added file
        objects.insert(
            "b_new_file".to_string(),
            GitObject::Blob(Blob {
                hash: "b_new_file".to_string(),
                content: Bytes::from("new content\n"),
            }),
        );

        let repo = MockRepo { objects };

        // Get file changes for the child commit
        let changes = repo.get_file_changes(&child_commit).unwrap();

        // Should have 2 changes: foo.txt (modified) and bar.txt (added)
        assert_eq!(changes.len(), 2);

        // Collect file names from changes
        let mut change_names: Vec<String> = changes
            .iter()
            .map(|c| {
                c.location
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        change_names.sort();

        assert!(change_names.contains(&"bar.txt".to_string()));
        assert!(change_names.contains(&"foo.txt".to_string()));

        // Verify the diff details for each file
        for change in &changes {
            let file_name = change
                .location
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            match file_name.as_str() {
                "foo.txt" => {
                    // Modified file: 1 line added, 0 lines removed
                    assert_eq!(change.diff.lines_added, 1);
                    assert_eq!(change.diff.lines_removed, 0);
                }
                "bar.txt" => {
                    // Added file: 1 line added, 0 lines removed
                    assert_eq!(change.diff.lines_added, 1);
                    assert_eq!(change.diff.lines_removed, 0);
                }
                _ => panic!("Unexpected file: {}", file_name),
            }
        }
    }

    #[test]
    fn test_build_hotspots_aggregates_by_path() {
        let commit1 = make_test_commit("c1", None, "t1", "Alice", "alice@example.com");
        let commit2 = make_test_commit("c2", Some("c1"), "t2", "Bob", "bob@example.com");
        let commit3 = make_test_commit("c3", Some("c2"), "t3", "Alice", "alice@example.com");

        let mut objects = HashMap::new();
        objects.insert(
            "HEAD_COMMIT".to_string(),
            GitObject::Commit(commit3.clone()),
        );
        objects.insert("c1".to_string(), GitObject::Commit(commit1));
        objects.insert("c2".to_string(), GitObject::Commit(commit2));
        objects.insert("c3".to_string(), GitObject::Commit(commit3));

        objects.insert(
            "t1".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t1".to_string(),
                entries: vec![git_objects::TreeEntry {
                    name: "foo.txt".to_string(),
                    hash: "b1".to_string(),
                    mode: git_objects::EntryMode::Text,
                }],
            }),
        );
        objects.insert(
            "t2".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t2".to_string(),
                entries: vec![
                    git_objects::TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b2".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                    git_objects::TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "b3".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                ],
            }),
        );
        objects.insert(
            "t3".to_string(),
            GitObject::Tree(git_objects::Tree {
                hash: "t3".to_string(),
                entries: vec![
                    git_objects::TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b4".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                    git_objects::TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "b3".to_string(),
                        mode: git_objects::EntryMode::Text,
                    },
                ],
            }),
        );

        objects.insert(
            "b1".to_string(),
            GitObject::Blob(Blob {
                hash: "b1".to_string(),
                content: Bytes::from("a\n"),
            }),
        );
        objects.insert(
            "b2".to_string(),
            GitObject::Blob(Blob {
                hash: "b2".to_string(),
                content: Bytes::from("a\nb\n"),
            }),
        );
        objects.insert(
            "b3".to_string(),
            GitObject::Blob(Blob {
                hash: "b3".to_string(),
                content: Bytes::from("new\n"),
            }),
        );
        objects.insert(
            "b4".to_string(),
            GitObject::Blob(Blob {
                hash: "b4".to_string(),
                content: Bytes::from("a\nb\nc\n"),
            }),
        );

        let repo = MockRepo { objects };
        let hotspots = build_hotspots(&repo, 300).unwrap();

        let foo = hotspots.iter().find(|h| h.location() == "foo.txt").unwrap();
        let bar = hotspots.iter().find(|h| h.location() == "bar.txt").unwrap();

        assert_eq!(foo.touches(), 2);
        assert_eq!(bar.touches(), 1);
        assert!(foo.lines_touched() > bar.lines_touched());
    }
}
