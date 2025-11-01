use crate::repo::*;

pub fn get_repo(path: &str) -> Result<Repository, String> {
    read_repo(path)
}
