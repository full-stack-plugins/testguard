use crate::obligation::{digest, require};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_COMPONENT_BYTES: usize = 256;
pub const MAX_REFERENCE_BYTES: usize = 4096;
pub const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MEDIA_TYPE_BYTES: usize = 256;
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    #[schemars(length(max = 524))]
    pub uri: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub digest: String,
    #[schemars(length(min = 1, max = 256), regex(pattern = "^[ -~]+$"))]
    pub media_type: String,
    #[schemars(range(max = 16777216))]
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
        && s.len() <= MAX_COMPONENT_BYTES
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
        require(
            bytes.len() <= MAX_ARTIFACT_BYTES,
            "artifact byte budget exceeded",
        )?;
        Ok(Self {
            uri: format!("evidence://{attempt}/{name}"),
            digest: bytes_digest(bytes),
            media_type: "application/octet-stream".into(),
            size: bytes.len() as u64,
        })
    }
    /// Bounded domain metadata only; never resolves a URI or authenticates a producer.
    pub fn parse(bytes: &[u8], attempt: &str) -> Result<Self, String> {
        require(
            bytes.len() <= MAX_REFERENCE_BYTES,
            "artifact reference budget exceeded",
        )?;
        require(component(attempt), "invalid artifact attempt")?;
        let reference: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        reference.validate(attempt)?;
        Ok(reference)
    }
    pub fn validate(&self, attempt: &str) -> Result<(), String> {
        require(component(attempt), "invalid artifact attempt")?;
        let prefix = format!("evidence://{attempt}/");
        require(
            self.uri.strip_prefix(&prefix).is_some_and(component)
                && digest(&self.digest)
                && self.size <= MAX_ARTIFACT_BYTES as u64
                && !self.media_type.trim().is_empty()
                && self.media_type.len() <= MAX_MEDIA_TYPE_BYTES
                && self.media_type.bytes().all(|b| (0x20..=0x7e).contains(&b)),
            "invalid artifact ref or foreign attempt",
        )
    }
    pub fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        require(
            bytes.len() <= MAX_ARTIFACT_BYTES,
            "artifact byte budget exceeded",
        )?;
        require(
            self.size == bytes.len() as u64 && self.digest == bytes_digest(bytes),
            "artifact size/digest conflict",
        )
    }
}
