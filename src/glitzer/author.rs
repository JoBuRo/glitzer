use std::{collections::HashMap, path::Path, path::PathBuf};

use crate::glitzer::{
    file_tree::{FileChange, FileTree},
    git_objects::Commit,
    repo::RepositoryAccess,
};
use color_eyre::eyre::Result;

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: String,
    pub commits: Vec<Commit>,
}

impl Author {
    pub fn new(name: String, email: String) -> Self {
        Author {
            name,
            email,
            commits: Vec::new(),
        }
    }

    pub fn add_commit(&mut self, commit: Commit) {
        self.commits.push(commit);
    }

    pub fn set_commits(&mut self, commits: Vec<Commit>) {
        self.commits = commits;
    }

    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    pub fn get_changed_files(&self, repo: &impl RepositoryAccess) -> Result<Vec<PathBuf>> {
        let mut change_map: HashMap<PathBuf, u64> = HashMap::new();
        for commit in &self.commits {
            let tree = FileTree::from_commit(commit, repo)?;
            if let Some(parent) = &commit.parent {
                let parent_commit = repo.get_commit(parent)?;
                let parent_tree = FileTree::from_commit(&parent_commit, repo)?;
                let changes = tree.file_changes(&parent_tree, repo.get_path());
                aggregate_changes(&mut change_map, changes);
            }
        }

        let mut changed_files: Vec<(PathBuf, u64)> = change_map.drain().collect();
        changed_files.sort_by(|(_, a), (_, b)| a.cmp(b));
        Ok(changed_files.iter().map(|(path, _)| path.clone()).collect())
    }
}

fn aggregate_changes(change_map: &mut HashMap<PathBuf, u64>, changes: Vec<FileChange>) {
    for change in changes {
        let lines_touched_new = match &change.diff {
            Some(old_diff) => old_diff.lines_touched(),
            None => 0,
        };

        let lines_touched_old = change_map.get(&change.location).unwrap_or(&0);

        change_map.insert(change.location, lines_touched_new + *lines_touched_old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glitzer::git_objects::{
        Author as GitAuthor, Blob, Commit, EntryMode, GitObject, Tree, TreeEntry,
    };
    use bytes::Bytes;
    use chrono::Utc;
    use color_eyre::eyre::eyre;
    use std::collections::HashMap;

    struct MockRepo {
        objects: HashMap<String, GitObject>,
    }

    impl RepositoryAccess for MockRepo {
        fn get_commits(&self) -> Result<Vec<Commit>> {
            // Not used by these tests
            Ok(vec![])
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
    }

    fn make_author_commit(hash: &str, parent: Option<&str>, tree: &str) -> Commit {
        let author = GitAuthor {
            name: "Test Author".to_string(),
            email: "test@example.com".to_string(),
        };

        Commit {
            hash: hash.to_string(),
            parent: parent.map(|p| p.to_string()),
            tree: tree.to_string(),
            message: "msg".to_string(),
            author: author.clone(),
            authored_at: Utc::now(),
            _committer: author,
            committed_at: Utc::now(),
        }
    }

    #[test]
    fn test_get_changed_files_single_change() {
        let parent_commit = make_author_commit("p", None, "t1");
        let child_commit = make_author_commit("c", Some("p"), "t2");

        let mut objects = HashMap::new();

        objects.insert("p".to_string(), GitObject::Commit(parent_commit.clone()));
        objects.insert("c".to_string(), GitObject::Commit(child_commit.clone()));

        objects.insert(
            "t1".to_string(),
            GitObject::Tree(Tree {
                hash: "t1".to_string(),
                entries: vec![TreeEntry {
                    name: "foo.txt".to_string(),
                    hash: "b1".to_string(),
                    mode: EntryMode::Text,
                }],
            }),
        );

        objects.insert(
            "t2".to_string(),
            GitObject::Tree(Tree {
                hash: "t2".to_string(),
                entries: vec![TreeEntry {
                    name: "foo.txt".to_string(),
                    hash: "b2".to_string(),
                    mode: EntryMode::Text,
                }],
            }),
        );

        objects.insert(
            "b1".to_string(),
            GitObject::Blob(Blob {
                hash: "b1".to_string(),
                content: Bytes::from("old\n"),
            }),
        );

        objects.insert(
            "b2".to_string(),
            GitObject::Blob(Blob {
                hash: "b2".to_string(),
                content: Bytes::from("new\nline2\n"),
            }),
        );

        let repo = MockRepo { objects };

        let mut author = Author::new("Test Author".to_string(), "test@example.com".to_string());
        author.set_commits(vec![child_commit]);

        let changed = author.get_changed_files(&repo).unwrap();

        assert_eq!(changed.len(), 1);
        assert_eq!(
            changed[0],
            std::path::PathBuf::from("mock_repo").join("foo.txt")
        );
    }

    #[test]
    fn test_get_changed_files_aggregated_and_sorted() {
        // Create three commits modifying two files so we can check aggregation and ordering
        // p0 -> c1 -> c2
        let p0 = make_author_commit("p0", None, "t0");
        let c1 = make_author_commit("c1", Some("p0"), "t1");
        let c2 = make_author_commit("c2", Some("c1"), "t2");

        let mut objects = HashMap::new();

        objects.insert("p0".to_string(), GitObject::Commit(p0.clone()));
        objects.insert("c1".to_string(), GitObject::Commit(c1.clone()));
        objects.insert("c2".to_string(), GitObject::Commit(c2.clone()));

        // t0 has foo.txt -> b0, bar.txt -> bb0
        objects.insert(
            "t0".to_string(),
            GitObject::Tree(Tree {
                hash: "t0".to_string(),
                entries: vec![
                    TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b0".to_string(),
                        mode: EntryMode::Text,
                    },
                    TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "bb0".to_string(),
                        mode: EntryMode::Text,
                    },
                ],
            }),
        );

        // c1 (t1) changes foo -> b1 (1 line change), leaves bar as bb0
        objects.insert(
            "t1".to_string(),
            GitObject::Tree(Tree {
                hash: "t1".to_string(),
                entries: vec![
                    TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b1".to_string(),
                        mode: EntryMode::Text,
                    },
                    TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "bb0".to_string(),
                        mode: EntryMode::Text,
                    },
                ],
            }),
        );

        // c2 (t2) changes foo -> b2 (another 2 line change) and changes bar -> bb1 (1 line change)
        objects.insert(
            "t2".to_string(),
            GitObject::Tree(Tree {
                hash: "t2".to_string(),
                entries: vec![
                    TreeEntry {
                        name: "foo.txt".to_string(),
                        hash: "b2".to_string(),
                        mode: EntryMode::Text,
                    },
                    TreeEntry {
                        name: "bar.txt".to_string(),
                        hash: "bb1".to_string(),
                        mode: EntryMode::Text,
                    },
                ],
            }),
        );

        objects.insert(
            "b0".to_string(),
            GitObject::Blob(Blob {
                hash: "b0".to_string(),
                content: Bytes::from("a\n"),
            }),
        );
        objects.insert(
            "bb0".to_string(),
            GitObject::Blob(Blob {
                hash: "bb0".to_string(),
                content: Bytes::from("x\n"),
            }),
        );

        // b1 differs from b0 by 1 line
        objects.insert(
            "b1".to_string(),
            GitObject::Blob(Blob {
                hash: "b1".to_string(),
                content: Bytes::from("a\nnew\n"),
            }),
        );
        // b2 differs from b1 by 2 lines
        objects.insert(
            "b2".to_string(),
            GitObject::Blob(Blob {
                hash: "b2".to_string(),
                content: Bytes::from("repl1\nrepl2\n"),
            }),
        );

        // bb1 differs from bb0 by 1 line
        objects.insert(
            "bb1".to_string(),
            GitObject::Blob(Blob {
                hash: "bb1".to_string(),
                content: Bytes::from("y\n"),
            }),
        );

        let repo = MockRepo { objects };

        let mut author = Author::new("Test Author".to_string(), "test@example.com".to_string());
        author.set_commits(vec![c1.clone(), c2.clone()]);

        let changed = author.get_changed_files(&repo).unwrap();

        // Expect two files
        assert_eq!(changed.len(), 2);

        // bar.txt should have total 1 (changed only in c2), foo.txt should have total 3 (1 in c1 + 2 in c2)
        // Sorting is ascending by touched lines, so bar.txt should come before foo.txt
        assert_eq!(
            changed[0],
            std::path::PathBuf::from("mock_repo").join("bar.txt")
        );
        assert_eq!(
            changed[1],
            std::path::PathBuf::from("mock_repo").join("foo.txt")
        );
    }
}
