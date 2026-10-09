use crate::obligation::{digest, require};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub uri: String,
    pub digest: String,
    pub media_type: String,
    pub size: u64,
}
pub fn bytes_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Sorted-key serde_json representation. No floating point normalization claim.
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    Ok(bytes_digest(
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    ))
}
fn component(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
}
impl ArtifactRef {
    pub fn from_bytes(attempt: &str, name: &str, bytes: &[u8]) -> Result<Self, String> {
        require(
            component(attempt) && component(name),
            "unsafe artifact location",
        )?;
        Ok(Self {
            uri: format!("evidence://{attempt}/{name}"),
            digest: bytes_digest(bytes),
            media_type: "application/octet-stream".into(),
            size: bytes.len() as u64,
        })
    }
    pub fn validate(&self, attempt: &str) -> Result<(), String> {
        let prefix = format!("evidence://{attempt}/");
        require(
            component(attempt)
                && self.uri.strip_prefix(&prefix).is_some_and(component)
                && digest(&self.digest)
                && !self.media_type.trim().is_empty(),
            "invalid artifact ref or foreign attempt",
        )
    }
    pub fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        require(
            self.size == bytes.len() as u64 && self.digest == bytes_digest(bytes),
            "artifact size/digest conflict",
        )
    }
}
