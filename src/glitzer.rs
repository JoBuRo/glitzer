use std::io::Read;

use crate::model::*;
use bytes::Bytes;
use flate2::read::ZlibDecoder;
use hex;
use sha1::{Digest, Sha1};

fn read_bytes(file_path: &str) -> Result<Bytes, String> {
    let data = std::fs::read(file_path).expect("Failed to read file");
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decoded_bytes = Vec::new();
    decoder
        .read_to_end(decoded_bytes.as_mut())
        .map_err(|e| e.to_string())?;
    Ok(Bytes::from(decoded_bytes))
}

fn parse_header(header_str: &str) -> Result<ObjectHeader, String> {
    let mut parts = header_str.split(' ');

    let object_type_str = parts.next().ok_or("Missing object type".to_string())?;
    let size_str = parts.next().ok_or("Missing size".to_string())?;

    let object_type = match object_type_str {
        "blob" => ObejctType::Blob,
        "tree" => ObejctType::Tree,
        "commit" => ObejctType::Commit,
        "tag" => ObejctType::AnnotatedTag,
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

fn parse_object(bytes: &Bytes) -> Result<Object, String> {
    let hash = get_hash(&bytes);
    let mut bytes_split = bytes.split(|&b| b == 0);

    let header_bytes = bytes_split.next().ok_or("Missing header".to_string())?;

    let header_str = std::str::from_utf8(header_bytes).map_err(|e| e.to_string())?;

    let header = parse_header(header_str)?;

    let content = bytes_split.next().ok_or("Missing content".to_string())?;

    Ok(Object {
        hash,
        header,
        content: Bytes::from(content.to_vec()),
    })
}

pub fn read_object(file_path: &str) -> Result<Object, String> {
    let bytes = read_bytes(file_path)?;
    parse_object(&bytes)
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
        assert_eq!(object.header.object_type, ObejctType::Blob);
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
