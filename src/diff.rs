use crate::git_objects::*;
use similar::{ChangeTag, TextDiff};

pub struct CommitDiff {
    pub added: u64,
    pub removed: u64,
}

pub fn diff_commits(
    old: &Commit,
    new: &Commit,
    get_object: &dyn Fn(&str) -> Result<GitObject, String>,
) -> Result<CommitDiff, String> {
    let old_object = get_object(&old.tree)?;
    let new_object = get_object(&new.tree)?;
    if let (GitObject::Tree(old_tree), GitObject::Tree(new_tree)) = (old_object, new_object) {
        return diff_tree(&old_tree, &new_tree, get_object);
    }
    Err("Commit has no tree".to_string())
}

fn diff_tree(
    old: &Tree,
    new: &Tree,
    get_object: &dyn Fn(&str) -> Result<GitObject, String>,
) -> Result<CommitDiff, String> {
    let mut commit_diff = CommitDiff {
        added: 0,
        removed: 0,
    };

    for old_entry in &old.entries {
        for new_entry in &new.entries {
            if old_entry.name == new_entry.name {
                let entry_diff = diff_entry(old_entry, new_entry, get_object)?;
                commit_diff.added += entry_diff.added;
                commit_diff.removed += entry_diff.removed;
            }
        }
    }

    Ok(commit_diff)
}

fn diff_entry(
    old: &TreeEntry,
    new: &TreeEntry,
    get_object: &dyn Fn(&str) -> Result<GitObject, String>,
) -> Result<CommitDiff, String> {
    let old_object = get_object(&old.hash)?;
    let new_object = get_object(&new.hash)?;
    if old.mode == EntryMode::Text && new.mode == EntryMode::Text {
        if let (GitObject::Blob(old_blob), GitObject::Blob(new_blob)) = (old_object, new_object) {
            return diff_blob(&old_blob, &new_blob);
        }
        Err(format!(
            "Expected blob objects for text entries, found different types for hashes {} and {}",
            old.hash, new.hash
        ))
    } else if old.mode == EntryMode::Tree && new.mode == EntryMode::Tree {
        if let (GitObject::Tree(old_tree), GitObject::Tree(new_tree)) = (old_object, new_object) {
            return diff_tree(&old_tree, &new_tree, get_object);
        }
        Err(format!(
            "Expected tree objects for tree entries, found different types for hashes {} and {}",
            old.hash, new.hash
        ))
    } else {
        // Different types or unsupported types; ignore
        Ok(CommitDiff {
            added: 0,
            removed: 0,
        })
    }
}

fn diff_blob(old: &Blob, new: &Blob) -> Result<CommitDiff, String> {
    let mut commit_diff = CommitDiff {
        added: 0,
        removed: 0,
    };

    let old_text = std::str::from_utf8(&old.content)
        .map_err(|err| format!("Error parsing {:#?}: {}", &old.content[..], err.to_string()))?;
    let new_text = std::str::from_utf8(&new.content)
        .map_err(|err| format!("Error parsing {:#?}: {}", &new.content[..], err.to_string()))?;

    let diff = TextDiff::from_lines(old_text, new_text);

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => commit_diff.removed += 1,
            ChangeTag::Insert => commit_diff.added += 1,
            ChangeTag::Equal => (),
        }
    }

    Ok(commit_diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_diff_blob_no_changes() {
        // Test case 1: No changes
        let old_blob = Blob {
            content: Bytes::from("Hello\nWorld\n"),
            _hash: "".to_string(),
        };
        let new_blob = Blob {
            content: Bytes::from("Hello\nWorld\n"),
            _hash: "".to_string(),
        };
        let diff = diff_blob(&old_blob, &new_blob).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_blob_lines_added() {
        // Test case 2: Added lines
        let old_blob = Blob {
            content: Bytes::from("Hello\nWorld\n"),
            _hash: "".to_string(),
        };
        let new_blob = Blob {
            content: Bytes::from("Hello\nBeautiful\nWorld\n"),
            _hash: "".to_string(),
        };
        let diff = diff_blob(&old_blob, &new_blob).unwrap();
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_blob_lines_removed() {
        // Test case 3: Removed lines
        let old_blob = Blob {
            content: Bytes::from("Hello\nBeautiful\nWorld\n"),
            _hash: "".to_string(),
        };
        let new_blob = Blob {
            content: Bytes::from("Hello\nWorld\n"),
            _hash: "".to_string(),
        };
        let diff = diff_blob(&old_blob, &new_blob).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 1);
    }

    #[test]
    fn test_diff_blob_lines_added_and_removed() {
        // Test case 4: Both additions and removals
        let old_blob = Blob {
            content: Bytes::from("Hello\nOld\nWorld\n"),
            _hash: "".to_string(),
        };
        let new_blob = Blob {
            content: Bytes::from("Hello\nNew\nWorld\n"),
            _hash: "".to_string(),
        };
        let diff = diff_blob(&old_blob, &new_blob).unwrap();
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 1);
    }

    #[test]
    fn test_diff_blob_invalid_utf8() {
        // Test case 5: Invalid UTF-8
        let old_blob = Blob {
            content: Bytes::from(vec![0xFF, 0xFF]),
            _hash: "".to_string(),
        };
        let new_blob = Blob {
            content: Bytes::from("Hello"),
            _hash: "".to_string(),
        };
        assert!(diff_blob(&old_blob, &new_blob).is_err());
    }

    fn mock_get_object(hash: &str) -> Result<GitObject, String> {
        match hash {
            "text1" => Ok(GitObject::Blob(Blob {
                content: Bytes::from("Hello\nWorld\n"),
                _hash: "text1".to_string(),
            })),
            "text2" => Ok(GitObject::Blob(Blob {
                content: Bytes::from("Hello\nNew\nWorld\n"),
                _hash: "text2".to_string(),
            })),
            "tree1" => Ok(GitObject::Tree(Tree {
                entries: vec![],
                hash: "tree1".to_string(),
            })),
            "tree2" => Ok(GitObject::Tree(Tree {
                entries: vec![],
                hash: "tree2".to_string(),
            })),
            _ => Err("Object not found".to_string()),
        }
    }

    #[test]
    fn test_diff_tree_entry_with_text() {
        // Test case 1: Text entries
        let old_text = TreeEntry {
            mode: EntryMode::Text,
            name: "file.txt".to_string(),
            hash: "text1".to_string(),
        };
        let new_text = TreeEntry {
            mode: EntryMode::Text,
            name: "file.txt".to_string(),
            hash: "text2".to_string(),
        };
        let diff = diff_entry(&old_text, &new_text, &mock_get_object).unwrap();
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_entry_with_trees() {
        // Test case 2: Tree entries
        let old_tree = TreeEntry {
            mode: EntryMode::Tree,
            name: "dir".to_string(),
            hash: "tree1".to_string(),
        };
        let new_tree = TreeEntry {
            mode: EntryMode::Tree,
            name: "dir".to_string(),
            hash: "tree2".to_string(),
        };
        let diff = diff_entry(&old_tree, &new_tree, &mock_get_object).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_entry_with_different_modes() {
        let text_entry = TreeEntry {
            mode: EntryMode::Text,
            name: "item".to_string(),
            hash: "text1".to_string(),
        };
        let tree_entry = TreeEntry {
            mode: EntryMode::Tree,
            name: "item".to_string(),
            hash: "tree1".to_string(),
        };
        let diff = diff_entry(&text_entry, &tree_entry, &mock_get_object).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_entry_with_invalid_hash() {
        let text_entry = TreeEntry {
            mode: EntryMode::Text,
            name: "item".to_string(),
            hash: "text1".to_string(),
        };
        let invalid_entry = TreeEntry {
            mode: EntryMode::Text,
            name: "bad.txt".to_string(),
            hash: "nonexistent".to_string(),
        };
        assert!(diff_entry(&invalid_entry, &text_entry, &mock_get_object).is_err());
    }

    #[test]
    fn test_diff_tree_same_files_different_content() {
        // Test case 1: Trees with same files but different content
        let old_tree = Tree {
            entries: vec![TreeEntry {
                mode: EntryMode::Text,
                name: "file1.txt".to_string(),
                hash: "text1".to_string(),
            }],
            hash: "oldtree".to_string(),
        };
        let new_tree = Tree {
            entries: vec![TreeEntry {
                mode: EntryMode::Text,
                name: "file1.txt".to_string(),
                hash: "text2".to_string(),
            }],
            hash: "newtree".to_string(),
        };
        let diff = diff_tree(&old_tree, &new_tree, &mock_get_object).unwrap();
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_different_file_names() {
        // Test case 2: Different file names - should not compare files
        let old_tree = Tree {
            entries: vec![TreeEntry {
                mode: EntryMode::Text,
                name: "file1.txt".to_string(),
                hash: "text1".to_string(),
            }],
            hash: "oldtree".to_string(),
        };
        let new_tree = Tree {
            entries: vec![TreeEntry {
                mode: EntryMode::Text,
                name: "file2.txt".to_string(),
                hash: "text2".to_string(),
            }],
            hash: "newtree".to_string(),
        };
        let diff = diff_tree(&old_tree, &new_tree, &mock_get_object).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_with_empty_trees() {
        // Test case 3: Empty trees
        let old_tree = Tree {
            entries: vec![],
            hash: "oldtree".to_string(),
        };
        let new_tree = Tree {
            entries: vec![],
            hash: "newtree".to_string(),
        };
        let diff = diff_tree(&old_tree, &new_tree, &mock_get_object).unwrap();
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
    }

    #[test]
    fn test_diff_tree_multiple_files() {
        // Test case 4: Trees with multiple files
        let old_tree = Tree {
            entries: vec![
                TreeEntry {
                    mode: EntryMode::Text,
                    name: "file1.txt".to_string(),
                    hash: "text1".to_string(),
                },
                TreeEntry {
                    mode: EntryMode::Text,
                    name: "file2.txt".to_string(),
                    hash: "text2".to_string(),
                },
                TreeEntry {
                    mode: EntryMode::Text,
                    name: "file3.txt".to_string(),
                    hash: "text2".to_string(),
                },
            ],
            hash: "oldtree".to_string(),
        };
        let new_tree = Tree {
            entries: vec![
                TreeEntry {
                    mode: EntryMode::Text,
                    name: "file1.txt".to_string(),
                    hash: "text2".to_string(),
                },
                TreeEntry {
                    mode: EntryMode::Text,
                    name: "file3.txt".to_string(),
                    hash: "text1".to_string(),
                },
            ],
            hash: "newtree".to_string(),
        };
        let diff = diff_tree(&old_tree, &new_tree, &mock_get_object).unwrap();
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 1);
    }
}
