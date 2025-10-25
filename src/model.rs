use bytes::Bytes;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ObejctType {
    Blob,
    Tree,
    Commit,
    AnnotatedTag,
}

pub struct ObjectHeader {
    pub object_type: ObejctType,
    pub size: u64,
}

pub struct Object {
    pub hash: String,
    pub header: ObjectHeader,
    pub content: Bytes,
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = std::str::from_utf8(&self.content).map_err(|_| fmt::Error)?;
        write!(
            f,
            "Object {} :\n  Type: {:?},\n  Size: {},\n  Content:\n--------\n{}\n--------",
            &self.hash[..6],
            self.header.object_type,
            self.header.size,
            content
        )
    }
}
