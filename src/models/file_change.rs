use std::path::PathBuf;
use std::{cmp::max, ops};

#[derive(Clone)]
pub struct Diff {
    pub lines_added: u64,
    pub lines_removed: u64,
}

impl Diff {
    pub fn lines_touched(&self) -> u64 {
        max(self.lines_added, self.lines_removed)
    }
}

impl ops::Add<Diff> for Diff {
    type Output = Diff;
    fn add(self, rhs: Diff) -> Self::Output {
        Diff {
            lines_added: self.lines_added + rhs.lines_added,
            lines_removed: self.lines_removed + rhs.lines_removed,
        }
    }
}

#[derive(Clone)]
pub enum FileChangeType {
    Added,
    Removed,
    Modified,
}

pub struct FileChange {
    pub location: PathBuf,
    pub _change_type: FileChangeType,
    pub diff: Diff,
}
