use std::collections::HashMap;

use bytes::Bytes;
use color_eyre::{Result, eyre::eyre};

use super::repo::RepositoryAccess;

use super::git_objects::{Commit, EntryMode, GitObject, TreeEntry};

struct Directory {
    name: String,
    hash: String,
    mode: EntryMode,
    content: HashMap<String, FileTree>,
}

struct SourceFile {
    name: String,
    hash: String,
    mode: EntryMode,
    content: Bytes,
}

enum FileTree {
    Node(Directory),
    Leaf(SourceFile),
}

impl FileTree {
    pub fn from_commit(commit: &Commit, repo: &impl RepositoryAccess) -> Result<Self> {
        let tree_object = repo.get_object(&commit.tree)?;

        if let GitObject::Tree(tree) = tree_object {
            let mut root = Directory {
                name: repo.get_path().to_string(),
                hash: commit.tree.clone(),
                mode: EntryMode::Tree,
                content: HashMap::new(),
            };

            for entry in tree.entries {
                let child_tree = Self::from_entry(&entry, repo)?;
                root.content.insert(entry.name, child_tree);
            }

            return Ok(FileTree::Node(root));
        }

        Err(eyre!(
            "Expected tree object for commit {}, found different type",
            commit.hash
        ))
    }

    fn from_entry(entry: &TreeEntry, repo: &impl RepositoryAccess) -> Result<Self> {
        match repo.get_object(&entry.hash)? {
            GitObject::Blob(blob) => Ok(FileTree::Leaf(SourceFile {
                name: entry.name.clone(),
                hash: blob.hash,
                mode: entry.mode,
                content: blob.content,
            })),
            GitObject::Tree(tree) => {
                let mut dir = Directory {
                    name: entry.name.clone(),
                    hash: tree.hash,
                    mode: entry.mode,
                    content: HashMap::new(),
                };

                for entry in tree.entries {
                    let child_tree = Self::from_entry(&entry, repo)?;
                    dir.content.insert(entry.name, child_tree);
                }

                Ok(FileTree::Node(dir))
            }
            GitObject::Commit(_) => Err(eyre!("Unexpected commit object in file tree")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::glitzer::git_objects::{Author, Blob, Tree};

    use super::*;

    struct MockRepo {
        objects: HashMap<String, GitObject>,
    }

    impl RepositoryAccess for MockRepo {
        fn get_commits(&self) -> Result<Vec<Commit>> {
            let author = Author {
                name: "Test Author".to_string(),
                email: "".to_string(),
            };
            Ok(vec![Commit {
                hash: "0".to_string(),
                parent: None,
                tree: "1".to_string(),
                message: "Initial commit".to_string(),
                author: author.clone(),
                authored_at: chrono::Utc::now(),
                _committer: author,
                _committed_at: chrono::Utc::now(),
            }])
        }

        fn get_object(&self, hash: &str) -> Result<GitObject> {
            self.objects
                .get(hash)
                .cloned()
                .ok_or_else(|| eyre!("Object with hash {} not found", hash))
        }

        fn get_path(&self) -> &str {
            "mock_repo"
        }
    }

    #[test]
    fn test_file_tree_from_commit() {
        let repo = MockRepo {
            objects: HashMap::from([
                (
                    "1".to_string(),
                    GitObject::Tree(Tree {
                        hash: "1".to_string(),
                        entries: vec![
                            TreeEntry {
                                name: "file.txt".to_string(),
                                hash: "2".to_string(),
                                mode: EntryMode::Text,
                            },
                            TreeEntry {
                                name: "subdir".to_string(),
                                hash: "3".to_string(),
                                mode: EntryMode::Tree,
                            },
                        ],
                    }),
                ),
                (
                    "2".to_string(),
                    GitObject::Blob(Blob {
                        hash: "2".to_string(),
                        content: Bytes::from("Hello, world!"),
                    }),
                ),
                (
                    "3".to_string(),
                    GitObject::Tree(Tree {
                        hash: "3".to_string(),
                        entries: vec![TreeEntry {
                            name: "nested.txt".to_string(),
                            hash: "4".to_string(),
                            mode: EntryMode::Text,
                        }],
                    }),
                ),
                (
                    "4".to_string(),
                    GitObject::Blob(Blob {
                        hash: "4".to_string(),
                        content: Bytes::from("Nested file content"),
                    }),
                ),
            ]),
        };

        let commit = repo.get_commits().unwrap().pop().unwrap();

        let file_tree = FileTree::from_commit(&commit, &repo).unwrap();

        if let FileTree::Node(root) = file_tree {
            assert_eq!(root.name, "mock_repo");
            assert_eq!(root.content.len(), 2);

            if let FileTree::Leaf(root_file) = root.content.get("file.txt").unwrap() {
                assert_eq!(root_file.name, "file.txt");
                assert_eq!(root_file.hash, "2");
                assert_eq!(root_file.content, Bytes::from("Hello, world!"));
            } else {
                panic!("Expected 'file.txt' to be a Leaf node");
            }

            if let FileTree::Node(subdir) = root.content.get("subdir").unwrap() {
                assert_eq!(subdir.name, "subdir");
                assert_eq!(subdir.content.len(), 1);

                if let FileTree::Leaf(nested_file) = subdir.content.get("nested.txt").unwrap() {
                    assert_eq!(nested_file.name, "nested.txt");
                    assert_eq!(nested_file.hash, "4");
                    assert_eq!(nested_file.content, Bytes::from("Nested file content"));
                } else {
                    panic!("Expected 'nested.txt' to be a Leaf node");
                }
            } else {
                panic!("Expected 'subdir' to be a Directory node");
            }
        } else {
            panic!("Expected root to be a Directory node");
        }
    }
}
