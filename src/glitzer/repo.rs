use super::author::Author;

use super::git_objects::*;
use super::parser::*;
use bytes::Bytes;
use color_eyre::eyre::eyre;
use color_eyre::{Result, eyre::WrapErr};
use flate2::read::ZlibDecoder;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::path;
use std::path::Path;

pub trait RepositoryAccess {
    fn get_object(&self, hash: &str) -> Result<GitObject>;
    fn get_commits(&self) -> Result<Vec<Commit>>;
    fn get_commit(&self, hash: &str) -> Result<Commit>;
    fn get_path(&self) -> &Path;
    fn get_authors(&self) -> Result<Vec<Author>> {
        let mut author_map: HashMap<String, Author> = std::collections::HashMap::new();
        let commits = self.get_commits()?;

        for commit in commits {
            let key = &commit.author.email;

            if let Some(author) = author_map.get_mut(key) {
                author.add_commit(commit.clone());
            } else {
                let mut author =
                    Author::new(commit.author.name.clone(), commit.author.email.clone());
                author.add_commit(commit.clone());
                author_map.insert(key.clone(), author);
            }
        }

        let authors: Vec<Author> = author_map.into_values().collect();
        Ok(authors)
    }
}

pub struct Repository {
    pub path: String,
    head: String,
    current_branch: String,
}

impl RepositoryAccess for Repository {
    fn get_object(&self, hash: &str) -> Result<GitObject> {
        let file_path = format!("{}/.git/objects/{}/{}", self.path, &hash[0..2], &hash[2..]);
        read_object(&file_path)
    }

    fn get_commit(&self, hash: &str) -> Result<Commit> {
        if let GitObject::Commit(commit) = self.get_object(hash)? {
            return Ok(commit);
        }
        Err(eyre!("Expected commit at {}", hash))
    }

    fn get_commits(&self) -> Result<Vec<Commit>> {
        let mut commits = Vec::new();
        let mut current_hash_opt = Some(self.head.clone());

        while let Some(current_hash) = current_hash_opt {
            let object = self.get_object(&current_hash)?;

            match object {
                GitObject::Commit(commit) => {
                    current_hash_opt = commit.parent.clone();
                    commits.push(commit);
                }
                _ => {
                    return Err(eyre!(
                        "Expected commit object, found different type for hash {}",
                        current_hash
                    ));
                }
            }
        }

        Ok(commits)
    }

    fn get_path(&self) -> &Path {
        Path::new(&self.path)
    }
}

impl Repository {
    pub fn new(path: String) -> Result<Self> {
        let absolute_path = path::absolute(&path)?
            .into_os_string()
            .into_string()
            .unwrap_or(path.clone());

        let head_path = format!("{}/.git/HEAD", absolute_path);
        let head_content = std::fs::read_to_string(&head_path)
            .wrap_err_with(|| format!("Failed to read HEAD file at {}", head_path))?;

        let ref_path = head_content[5..].trim();
        let full_ref_path = format!("{}/.git/{}", absolute_path, ref_path);
        let ref_content = std::fs::read_to_string(&full_ref_path)
            .wrap_err_with(|| format!("Failed to read reference file at {}", full_ref_path))?;
        let head_hash = ref_content.trim().to_string();

        let current_branch = ref_path
            .strip_prefix("refs/heads/")
            .unwrap_or(ref_path)
            .to_string();

        let repo = Repository {
            path: absolute_path,
            head: head_hash,
            current_branch,
        };
        Ok(repo)
    }
}

impl fmt::Debug for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repository at {}, current branch: {}",
            self.path, self.current_branch
        )
    }
}

fn read_bytes(file_path: &str) -> Result<Bytes> {
    let data = std::fs::read(file_path)?;
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decoded_bytes = Vec::new();
    // read_to_end expects &mut Vec<u8>
    decoder.read_to_end(&mut decoded_bytes)?;
    Ok(Bytes::from(decoded_bytes))
}

fn parse_header(header_str: &str) -> Result<ObjectHeader> {
    let mut parts = header_str.split(' ');

    let object_type_str = parts.next().ok_or(eyre!("Missing object type"))?;
    let size_str = parts.next().ok_or(eyre!("Missing size"))?;

    let object_type = match object_type_str {
        "blob" => ObjectType::Blob,
        "tree" => ObjectType::Tree,
        "commit" => ObjectType::Commit,
        "tag" => ObjectType::AnnotatedTag,
        _ => return Err(eyre!("Unknown object type: {}", object_type_str)),
    };

    let size = size_str.parse::<u64>()?;

    Ok(ObjectHeader { object_type, size })
}

fn get_hash(content: &Bytes) -> String {
    let mut hasher = Sha1::new();

    hasher.update(content);
    let hash_bytes = hasher.finalize();
    hex::encode(hash_bytes)
}

fn parse_object(bytes: &Bytes) -> Result<RawObject> {
    let hash = get_hash(bytes);
    let mut bytes_split = bytes.splitn(2, |&b| b == 0);

    let header_bytes = bytes_split.next().ok_or(eyre!("Missing header"))?;

    let header_str = std::str::from_utf8(header_bytes)?;

    let header = parse_header(header_str)?;

    let content = bytes_split.next().ok_or(eyre!("Missing content"))?;

    Ok(RawObject {
        hash,
        header,
        content: Bytes::from(content.to_vec()),
    })
}

fn read_raw_object(file_path: &str) -> Result<RawObject> {
    let bytes = read_bytes(file_path)?;
    parse_object(&bytes)
}

pub fn read_object(file_path: &str) -> Result<GitObject> {
    let object = read_raw_object(file_path)?;

    match object.header.object_type {
        ObjectType::Blob => Ok(GitObject::Blob(Blob {
            hash: object.hash.clone(),
            content: object.content.clone(),
        })),
        ObjectType::Tree => {
            let tree = parse_tree(&object.content[..], &object.hash)?;
            Ok(GitObject::Tree(tree))
        }
        ObjectType::Commit => {
            let body = std::str::from_utf8(&object.content[..])?;
            let commit = parse_commit(object.hash, body)?;
            Ok(GitObject::Commit(commit))
        }
        ObjectType::AnnotatedTag => Err(eyre!("AnnotatedTag coercion not implemented")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_object() {
        let bytes = Bytes::from(&b"blob 14\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_ok());
        let object = result.unwrap();
        assert_eq!(object.hash, get_hash(&bytes));
        assert_eq!(object.header.object_type, ObjectType::Blob);
        assert_eq!(object.header.size, 14);
        assert_eq!(object.content, Bytes::from(&b"Hello, Glitzer!"[..]));
    }

    #[test]
    fn test_parse_object_invalid_header_missing_size() {
        let bytes = Bytes::from(&b"blob\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("Missing size"));
    }

    #[test]
    fn test_parse_object_invalid_header_unknown_type() {
        let bytes = Bytes::from(&b"invalid_header 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("Unknown object type"));
    }

    #[test]
    fn test_parse_object_invalid_utf8_header() {
        let bytes = Bytes::from(&b"\xFF\xFF\xFF 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let report = result.err().unwrap();
        assert!(report.to_string().contains("invalid utf-8"));
    }
}
