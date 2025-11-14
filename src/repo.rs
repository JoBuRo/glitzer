use crate::git_objects::*;
use crate::parser::*;
use bytes::Bytes;
use flate2::read::ZlibDecoder;
use sha1::{Digest, Sha1};
use std::fmt;
use std::io::Read;

pub struct Repository {
    pub path: String,
    head: String,
    current_branch: String,
}

impl Repository {
    pub fn get_object(&self, hash: &str) -> Result<GitObject, String> {
        let file_path = format!("{}/.git/objects/{}/{}", self.path, &hash[0..2], &hash[2..]);
        read_object(&file_path).map_err(|e| format!("Failed to read object {}: {}", hash, e))
    }

    pub fn _get_raw_object(&self, hash: &str) -> Result<RawObject, String> {
        let file_path = format!("{}/.git/objects/{}/{}", self.path, &hash[0..2], &hash[2..]);
        println!("{}", file_path);
        read_raw_object(&file_path)
            .map_err(|e| format!("Failed to read raw object {}: {}", hash, e))
    }

    pub fn get_commits(&self) -> Result<Vec<Commit>, String> {
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
                    return Err(format!(
                        "Expected commit object, found different type for hash {}",
                        current_hash
                    ));
                }
            }
        }

        Ok(commits)
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

pub fn read_repo(path: &str) -> Result<Repository, String> {
    let head_path = format!("{}/.git/HEAD", path);
    let head_content = std::fs::read_to_string(&head_path)
        .map_err(|err| format!("Failed to read HEAD file at {}: {}", head_path, err))?;

    let ref_path = head_content[5..].trim();
    let full_ref_path = format!("{}/.git/{}", path, ref_path);
    let ref_content = std::fs::read_to_string(&full_ref_path).map_err(|err| {
        format!(
            "Failed to read reference file at {}: {}",
            full_ref_path, err
        )
    })?;
    let head_hash = ref_content.trim().to_string();

    let current_branch = ref_path
        .strip_prefix("refs/heads/")
        .unwrap_or(ref_path)
        .to_string();

    let repo = Repository {
        path: path.to_string(),
        head: head_hash,
        current_branch,
    };
    Ok(repo)
}

fn read_bytes(file_path: &str) -> Result<Bytes, String> {
    let data = std::fs::read(file_path).map_err(|e| e.to_string())?;
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decoded_bytes = Vec::new();
    // read_to_end expects &mut Vec<u8>
    decoder
        .read_to_end(&mut decoded_bytes)
        .map_err(|e| e.to_string())?;
    Ok(Bytes::from(decoded_bytes))
}

fn parse_header(header_str: &str) -> Result<ObjectHeader, String> {
    let mut parts = header_str.split(' ');

    let object_type_str = parts.next().ok_or("Missing object type".to_string())?;
    let size_str = parts.next().ok_or("Missing size".to_string())?;

    let object_type = match object_type_str {
        "blob" => ObjectType::Blob,
        "tree" => ObjectType::Tree,
        "commit" => ObjectType::Commit,
        "tag" => ObjectType::AnnotatedTag,
        _ => return Err("Unknown object type".to_string()),
    };

    let size = size_str.parse::<u64>().map_err(|e| e.to_string())?;

    Ok(ObjectHeader { object_type, size })
}

fn get_hash(content: &Bytes) -> String {
    let mut hasher = Sha1::new();

    hasher.update(content);
    let hash_bytes = hasher.finalize();
    hex::encode(hash_bytes)
}

fn parse_object(bytes: &Bytes) -> Result<RawObject, String> {
    let hash = get_hash(bytes);
    let mut bytes_split = bytes.splitn(2, |&b| b == 0);

    let header_bytes = bytes_split.next().ok_or("Missing header".to_string())?;

    let header_str = std::str::from_utf8(header_bytes).map_err(|e| e.to_string())?;

    let header = parse_header(header_str)?;

    let content = bytes_split.next().ok_or("Missing content".to_string())?;

    Ok(RawObject {
        hash,
        header,
        content: Bytes::from(content.to_vec()),
    })
}

fn read_raw_object(file_path: &str) -> Result<RawObject, String> {
    let bytes = read_bytes(file_path)?;
    parse_object(&bytes)
}

pub fn read_object(file_path: &str) -> Result<GitObject, String> {
    let object = read_raw_object(file_path)?;

    match object.header.object_type {
        ObjectType::Blob => Ok(GitObject::Blob(Blob {
            _hash: object.hash.clone(),
            content: object.content.clone(),
        })),
        ObjectType::Tree => {
            let tree = parse_tree(&object.content[..], &object.hash)?;
            Ok(GitObject::Tree(tree))
        }
        ObjectType::Commit => {
            let body = std::str::from_utf8(&object.content[..]).map_err(|err| {
                format!(
                    "Failed to convert commit content to UTF-8 string for {}: {}",
                    &object.hash, err
                )
            })?;
            let commit = parse_commit(object.hash, body)?;
            Ok(GitObject::Commit(commit))
        }
        ObjectType::AnnotatedTag => Err("AnnotatedTag coercion not implemented".to_string()),
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
        let err = result.err().unwrap();
        assert_eq!(err, "Missing size");
    }

    #[test]
    fn test_parse_object_invalid_header_unknown_type() {
        let bytes = Bytes::from(&b"invalid_header 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, "Unknown object type");
    }

    #[test]
    fn test_parse_object_invalid_utf8_header() {
        let bytes = Bytes::from(&b"\xFF\xFF\xFF 5\0Hello, Glitzer!"[..]);
        let result = parse_object(&bytes);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("invalid utf-8"));
    }
}
