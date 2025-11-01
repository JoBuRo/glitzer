use crate::parser::parse_commit;
use bytes::Bytes;
use chrono::prelude::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct Blob {
    pub hash: String,
    pub content: Bytes,
}

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
pub struct Commit {
    pub hash: String,
    pub parent: String,
    pub tree: String,
    pub message: String,
    pub author: Author,
    pub authored_at: DateTime<Utc>,
    pub committer: Author,
    pub committed_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Tree {
    pub hash: String,
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug)]
pub struct TreeEntry {
    pub mode: String,
    pub object_type: ObjectType,
    pub hash: String,
    pub name: String,
}

#[derive(Debug)]
pub enum GitObject {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
}

#[derive(Debug, PartialEq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    AnnotatedTag,
}

pub struct ObjectHeader {
    pub object_type: ObjectType,
    pub size: u64,
}

pub struct RawObject {
    pub hash: String,
    pub header: ObjectHeader,
    pub content: Bytes,
}

fn parse_commit_from_body(body: &str, hash: &str) -> Result<Commit, String> {
    let commit = parse_commit(hash.to_string(), body)?;
    Ok(commit)
}

impl fmt::Debug for RawObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Object {} :\n  Type: {:?},\n  Size: {},\n  Content:\n--------\n{:?}\n--------",
            &self.hash[..6],
            self.header.object_type,
            self.header.size,
            self.content
        )
    }
}

pub struct Repository {
    pub path: String,
    pub objects: HashMap<String, RawObject>,
}

impl Repository {
    pub fn get_object(&self, hash: &str) -> Option<&RawObject> {
        self.objects.get(hash)
    }
}

impl fmt::Debug for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repository at {}: {} objects",
            self.path,
            self.objects.len()
        )
    }
}
