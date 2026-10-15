use super::{Result, error};
use sha2::{Digest, Sha256};

/// Provider-independent upload bytes. Image extraction is a separate caller.
pub struct Source {
    pub(super) bytes: Vec<u8>,
    pub(super) mime: String,
    pub(super) variant: String,
}

impl Source {
    pub fn new(mime: &str, bytes: Vec<u8>) -> Result<Self> {
        let valid = mime.split_once('/').is_some_and(|(kind, subtype)| {
            !kind.is_empty() && !subtype.is_empty() && !subtype.contains('/')
        }) && mime
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-+._".contains(&c));
        if !valid || bytes.is_empty() {
            return Err(error("上传文件必须有内容和有效的媒体类型"));
        }
        let mut hash = Sha256::new();
        hash.update(mime);
        hash.update([0]);
        hash.update(&bytes);
        Ok(Self {
            bytes,
            mime: mime.into(),
            variant: format!("{:x}", hash.finalize()),
        })
    }
}
