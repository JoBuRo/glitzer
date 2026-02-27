use crate::glitzer::git_objects::Commit;

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
}
