use std::collections::HashMap;

use bytes::Bytes;
use color_eyre::{Result, eyre::eyre};
use std::path::{Path, PathBuf};

use super::repo::RepositoryAccess;

use super::diff::{Diff, diff};
use super::git_objects::{Commit, EntryMode, GitObject, TreeEntry};

#[derive(Debug, Clone)]
struct FileInfo {
    name: String,
    hash: String,
    mode: EntryMode,
}

enum FileChangeType {
    Added,
    Removed,
    Modified,
}

struct FileChange {
    location: PathBuf,
    change_type: FileChangeType,
    diff: Option<Diff>,
}

struct Directory {
    info: FileInfo,
    content: HashMap<String, FileTree>,
}

impl Directory {
    fn file_changes(&self, old: &Directory, root: &Path) -> Vec<FileChange> {
        let mut changed = Vec::new();

        for (name, new_tree) in &self.content {
            if let Some(other_tree) = old.content.get(name) {
                changed.extend(new_tree.file_changes(other_tree, root));
            } else {
                changed.push(FileChange {
                    location: root.join(name),
                    change_type: FileChangeType::Added,
                    diff: None,
                });
            }
        }

        for name in old.content.keys() {
            if !self.content.contains_key(name) {
                changed.push(FileChange {
                    location: root.join(name),
                    change_type: FileChangeType::Removed,
                    diff: None,
                });
            }
        }

        changed
    }
}

#[derive(Debug, Clone)]
struct SourceFile {
    info: FileInfo,
    content: String,
}

struct BlobFile {
    info: FileInfo,
    content: Bytes,
}

enum LeafFile {
    Source(SourceFile),
    Blob(BlobFile),
}

impl LeafFile {
    fn file_changes(&self, old: &LeafFile, root: &Path) -> Vec<FileChange> {
        match (self, old) {
            (LeafFile::Source(new_src), LeafFile::Source(old_src)) => {
                let file_diff = diff(&old_src.content, &new_src.content);
                if file_diff.lines_added > 0 || file_diff.lines_removed > 0 {
                    return vec![FileChange {
                        location: root.join(&new_src.info.name),
                        change_type: FileChangeType::Modified,
                        diff: Some(file_diff),
                    }];
                }
                vec![]
            }
            (LeafFile::Blob(new_blob), LeafFile::Blob(old_blob)) => {
                if new_blob.info.hash != old_blob.info.hash {
                    return vec![FileChange {
                        location: root.join(&new_blob.info.name),
                        change_type: FileChangeType::Modified,
                        diff: None,
                    }];
                }
                vec![]
            }
            _ => vec![],
        }
    }
}

enum FileTree {
    Node(Directory),
    Leaf(LeafFile),
}

impl FileTree {
    pub fn file_changes(&self, old: &FileTree, root: &Path) -> Vec<FileChange> {
        if self.get_info().hash == old.get_info().hash {
            return vec![];
        }

        match (self, old) {
            (FileTree::Node(new_dir), FileTree::Node(old_dir)) => {
                return new_dir.file_changes(old_dir, root);
            }
            (FileTree::Leaf(new_file), FileTree::Leaf(old_file)) => {
                return new_file.file_changes(old_file, root);
            }
            _ => {
                return vec![];
            }
        }
    }

    fn get_info(&self) -> &FileInfo {
        match self {
            FileTree::Node(dir) => &dir.info,
            FileTree::Leaf(file) => match file {
                LeafFile::Source(src) => &src.info,
                LeafFile::Blob(blob) => &blob.info,
            },
        }
    }

    pub fn from_commit(commit: &Commit, repo: &impl RepositoryAccess) -> Result<Self> {
        let tree_object = repo.get_object(&commit.tree)?;

        if let GitObject::Tree(tree) = tree_object {
            let mut root = Directory {
                info: FileInfo {
                    name: repo.get_path().to_string(),
                    hash: commit.tree.clone(),
                    mode: EntryMode::Tree,
                },
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
            GitObject::Blob(blob) => {
                let parse_result = std::str::from_utf8(&blob.content);

                match parse_result {
                    Ok(content_str) => Ok(FileTree::Leaf(LeafFile::Source(
                        (SourceFile {
                            info: FileInfo {
                                name: entry.name.clone(),
                                hash: blob.hash,
                                mode: entry.mode,
                            },
                            content: content_str.to_string(),
                        }),
                    ))),
                    Err(_) => Ok(FileTree::Leaf(LeafFile::Blob(
                        (BlobFile {
                            info: FileInfo {
                                name: entry.name.clone(),
                                hash: blob.hash,
                                mode: entry.mode,
                            },
                            content: blob.content.clone(),
                        }),
                    ))),
                }
            }
            GitObject::Tree(tree) => {
                let mut dir = Directory {
                    info: FileInfo {
                        name: entry.name.clone(),
                        hash: tree.hash,
                        mode: entry.mode,
                    },
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
                committed_at: chrono::Utc::now(),
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
                        entries: vec![
                            TreeEntry {
                                name: "nested.txt".to_string(),
                                hash: "4".to_string(),
                                mode: EntryMode::Text,
                            },
                            TreeEntry {
                                name: "blob.exe".to_string(),
                                hash: "5".to_string(),
                                mode: EntryMode::Exe,
                            },
                        ],
                    }),
                ),
                (
                    "4".to_string(),
                    GitObject::Blob(Blob {
                        hash: "4".to_string(),
                        content: Bytes::from("Nested file content"),
                    }),
                ),
                (
                    "5".to_string(),
                    GitObject::Blob(Blob {
                        hash: "5".to_string(),
                        content: Bytes::from(&b"\xc3\x28"[..]),
                    }),
                ),
            ]),
        };

        let commit = repo.get_commits().unwrap().pop().unwrap();

        let file_tree = FileTree::from_commit(&commit, &repo).unwrap();

        if let FileTree::Node(root) = file_tree {
            assert_eq!(root.info.name, "mock_repo");
            assert_eq!(root.content.len(), 2);

            if let FileTree::Leaf(root_file) = root.content.get("file.txt").unwrap() {
                if let LeafFile::Source(root_source) = root_file {
                    assert_eq!(root_source.info.name, "file.txt");
                    assert_eq!(root_source.info.hash, "2");
                    assert_eq!(root_source.content, "Hello, world!");
                } else {
                    panic!("Expected 'file.txt' to be a Source file");
                }
            } else {
                panic!("Expected 'file.txt' to be a Leaf node");
            }

            if let FileTree::Node(subdir) = root.content.get("subdir").unwrap() {
                assert_eq!(subdir.info.name, "subdir");
                assert_eq!(subdir.content.len(), 2);

                if let FileTree::Leaf(nested_file) = subdir.content.get("nested.txt").unwrap() {
                    if let LeafFile::Source(nested_source) = nested_file {
                        assert_eq!(nested_source.info.name, "nested.txt");
                        assert_eq!(nested_source.info.hash, "4");
                        assert_eq!(nested_source.content, "Nested file content");
                    } else {
                        panic!("Expected 'nested.txt' to be a Source file");
                    }
                } else {
                    panic!("Expected 'nested.txt' to be a Leaf node");
                }

                if let FileTree::Leaf(nested_file) = subdir.content.get("blob.exe").unwrap() {
                    if let LeafFile::Blob(nested_blob) = nested_file {
                        assert_eq!(nested_blob.info.name, "blob.exe");
                        assert_eq!(nested_blob.info.hash, "5");
                        assert_eq!(nested_blob.content, Bytes::from(&b"\xc3\x28"[..]));
                    } else {
                        panic!("Expected 'blob.exe' to be a Blob file");
                    }
                } else {
                    panic!("Expected 'blob.exe' to be a Leaf node");
                }
            } else {
                panic!("Expected 'subdir' to be a Directory node");
            }
        } else {
            panic!("Expected root to be a Directory node");
        }
    }
}
