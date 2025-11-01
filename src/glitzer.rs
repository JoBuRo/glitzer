use std::collections::HashMap;
use std::io::Read;

use crate::model::*;
use crate::parser::*;
use bytes::Bytes;
use flate2::read::ZlibDecoder;
use hex;
use sha1::{Digest, Sha1};

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
    let hash = get_hash(&bytes);
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
            hash: object.hash.clone(),
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

fn read_objetcs(path: &str) -> Result<HashMap<String, RawObject>, String> {
    let mut objects = HashMap::new();
    let dir_iter = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    for dir in dir_iter {
        if dir.is_err() {
            println!("Error reading directory entry: {:?}", dir.err());
            continue;
        }
        let subdir_path = dir.unwrap().path();
        if !subdir_path.is_dir() {
            continue;
        }
        let file_iter = std::fs::read_dir(&subdir_path);

        if file_iter.is_err() {
            println!(
                "Error reading subdirectory {:?}: {:?}",
                subdir_path,
                file_iter.err()
            );
            continue;
        }

        for file in file_iter.unwrap() {
            if file.is_err() {
                println!("Error reading file entry: {:?}", file.err());
                continue;
            }
            let file_path = file.unwrap().path();
            if !file_path.is_file() {
                continue;
            }
            let object = read_raw_object(
                file_path
                    .to_str()
                    .ok_or("Failed to convert path to string".to_string())?,
            )?;
            objects.insert(object.hash.clone(), object);
        }
    }
    Ok(objects)
}

pub fn read_repo(path: &str) -> Result<Repository, String> {
    let objects_dir = format!("{}/.git/objects", path);
    let objects = read_objetcs(&objects_dir)?;
    Ok(Repository {
        path: path.to_string(),
        objects: objects,
    })
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

    // #[test]
    // fn test_parse_repo() {
    //     let repo = read_repo("test-repo").unwrap();

    //     assert_eq!(repo.objects.len(), 53);

    //     let tree_object = repo
    //         .get_object("428aaae4e4d5cae8a06eb482e428fc950d6ca85b")
    //         .unwrap();

    //     assert_eq!(tree_object.content, Bytes::from(&b"100644 blob ea8c4bf7f35f6f77f75d92ad8ce8349f6e81ddba    .gitignore\n100644 blob 091aeb6c81d4273b80688d1d89a1eb6a43ef8323    Cargo.lock\n100644 blob 93fb79eb2be36527672cce1d7953328ee4620590    Cargo.toml\n100644 blob 261eeb9e9f8b2b4b0d119366dda99c6fd7d35c64    LICENSE\n040000 tree 305157a396c6858705a9cb625bab219053264ee4    src"[..]));
    // }

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
