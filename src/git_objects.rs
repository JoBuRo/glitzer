use bytes::Bytes;
use chrono::prelude::*;
use std::fmt;

#[derive(Debug)]
pub struct Blob {
    pub _hash: String,
    pub content: Bytes,
}

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: String,
}

pub struct Commit {
    pub hash: String,
    pub parent: Option<String>,
    pub tree: String,
    pub message: String,
    pub author: Author,
    pub authored_at: DateTime<Utc>,
    pub _committer: Author,
    pub _committed_at: DateTime<Utc>,
}

impl fmt::Debug for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parent = self.parent.as_deref().unwrap_or("None");
        write!(
            f,
            "Commit {}:\n  Parent: {}\n  Tree: {}\n  Author: {} <{}>\n  Date: {}\n  Message: {}\n",
            &self.hash,
            parent,
            self.tree,
            self.author.name,
            self.author.email,
            self.authored_at,
            self.message
        )
    }
}

pub struct Tree {
    pub hash: String,
    pub entries: Vec<TreeEntry>,
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tree {}:", &self.hash[..7])?;
        for entry in &self.entries {
            let mode = match entry.mode {
                EntryMode::Text => "Text",
                EntryMode::Exe => "Executable",
                EntryMode::Symlink => "Symlink",
                EntryMode::Tree => "Tree",
                EntryMode::Gitlink => "Gitlink",
            };
            writeln!(f, "  {} {} {}", mode, entry.name, entry.hash)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TreeEntry {
    pub mode: EntryMode,
    pub hash: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EntryMode {
    Text,
    Exe,
    Symlink,
    Tree,
    Gitlink,
}

pub enum GitObject {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
}

impl fmt::Debug for GitObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitObject::Blob(blob) => write!(f, "{:?}", blob),
            GitObject::Tree(tree) => write!(f, "{:?}", tree),
            GitObject::Commit(commit) => write!(f, "{:?}", commit),
        }
    }
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
