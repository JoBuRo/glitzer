use similar::{ChangeTag, TextDiff};

pub struct Diff {
    pub lines_added: u64,
    pub lines_removed: u64,
}

pub fn diff(old_content: &str, new_content: &str) -> Diff {
    let mut commit_diff = Diff {
        lines_added: 0,
        lines_removed: 0,
    };

    let diff = TextDiff::from_lines(old_content, new_content);

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => commit_diff.lines_removed += 1,
            ChangeTag::Insert => commit_diff.lines_added += 1,
            ChangeTag::Equal => (),
        }
    }

    commit_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_no_changes() {
        let diff = diff("Hello\nWorld\n", "Hello\nWorld\n");
        assert_eq!(diff.lines_added, 0);
        assert_eq!(diff.lines_removed, 0);
    }

    #[test]
    fn test_diff_lines_added() {
        let diff = diff("Hello\nWorld\n", "Hello\nBeautiful\nWorld\n");
        assert_eq!(diff.lines_added, 1);
        assert_eq!(diff.lines_removed, 0);
    }

    #[test]
    fn test_diff_lines_removed() {
        let diff = diff("Hello\nBeautiful\nWorld\n", "Hello\nWorld\n");
        assert_eq!(diff.lines_added, 0);
        assert_eq!(diff.lines_removed, 1);
    }

    #[test]
    fn test_diff_lines_added_and_removed() {
        let diff = diff("Hello\nOld\nWorld\n", "Hello\nNew\nWorld\n");
        assert_eq!(diff.lines_added, 1);
        assert_eq!(diff.lines_removed, 1);
    }
}
