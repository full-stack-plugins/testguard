use crate::report::normalize::ArtifactRef;
use std::path::{Path, PathBuf};
const MAX_BYTES: usize = crate::adapters::limits::MAX_REPORT_BYTES;
pub const PROFILE: &str = "testguard.artifact-collector/linux-v1";
pub struct ArtifactCollector {
    #[cfg(target_os = "linux")]
    root: std::os::fd::OwnedFd,
}
pub struct CollectedArtifact {
    pub reference: ArtifactRef,
    pub bytes: Vec<u8>,
}
pub struct Request {
    pub path: PathBuf,
    pub reference: ArtifactRef,
}
pub struct CollectionBatch {
    pub artifacts: Vec<CollectedArtifact>,
    pub errors: Vec<String>,
}
impl CollectionBatch {
    pub fn is_complete(&self) -> bool {
        !self.artifacts.is_empty() && self.errors.is_empty()
    }
}
impl ArtifactCollector {
    /// Pin the caller-selected root. This is a read boundary, not an execution sandbox.
    pub fn open(root: &Path) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            use rustix::fs::{Mode, OFlags, open};
            let root = open(
                root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| format!("artifact root: {e}"))?;
            Ok(Self { root })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = root;
            Err("artifact collection platform unsupported".into())
        }
    }
    pub fn read(
        &self,
        path: &Path,
        reference: &ArtifactRef,
        attempt: &str,
    ) -> Result<CollectedArtifact, String> {
        #[cfg(target_os = "linux")]
        {
            use rustix::fs::{FileType, Mode, OFlags, fstat, openat};
            use std::{io::Read, os::fd::AsFd};
            let name = path.to_str().ok_or("artifact path must be UTF-8")?;
            if name.len() > 4096
                || name.contains(['\\', '%', '\0'])
                || reference.size > MAX_BYTES as u64
                || reference.uri.len() > 4096
                || reference.media_type.len() > 128
                || attempt.len() > 128
            {
                return Err("artifact reference/path/size limit".into());
            }
            let parts: Vec<_> = name.split('/').collect();
            if parts.len() > 32
                || parts
                    .iter()
                    .any(|p| p.is_empty() || *p == "." || *p == "..")
            {
                return Err("artifact path must be a strict relative path".into());
            }
            reference.validate(attempt)?;
            let mut directory = None;
            for (index, part) in parts.iter().enumerate() {
                let last = index + 1 == parts.len();
                let base = directory.as_ref().unwrap_or(&self.root).as_fd();
                let mut flags =
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
                if !last {
                    flags |= OFlags::DIRECTORY;
                }
                let fd = openat(base, *part, flags, Mode::empty())
                    .map_err(|e| format!("artifact open: {e}"))?;
                if !last {
                    directory = Some(fd);
                    continue;
                }
                let stat = fstat(&fd).map_err(|e| format!("artifact metadata: {e}"))?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
                    || stat.st_nlink != 1
                    || stat.st_size < 0
                    || stat.st_size as u64 > MAX_BYTES as u64
                {
                    return Err("artifact must be a bounded regular single-link file".into());
                }
                let file = std::fs::File::from(fd);
                let mut bytes = Vec::new();
                file.take(MAX_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| format!("artifact read: {e}"))?;
                if bytes.len() > MAX_BYTES {
                    return Err("artifact grew past size limit".into());
                }
                if [
                    b"\x1f\x8b".as_slice(),
                    b"PK\x03\x04",
                    b"PK\x05\x06",
                    b"BZh",
                    b"\xfd7zXZ\0",
                    b"\x28\xb5\x2f\xfd",
                ]
                .iter()
                .any(|magic| bytes.starts_with(magic))
                {
                    return Err(
                        "compressed artifacts are unsupported; no decompression performed".into(),
                    );
                }
                std::str::from_utf8(&bytes).map_err(|_| "native reports must be UTF-8")?;
                reference.verify(&bytes)?;
                return Ok(CollectedArtifact {
                    reference: reference.clone(),
                    bytes,
                });
            }
            Err("empty artifact path".into())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, reference, attempt);
            Err("artifact collection platform unsupported".into())
        }
    }
    pub fn collect_batch(&self, requests: &[Request], attempt: &str) -> CollectionBatch {
        let mut result = CollectionBatch {
            artifacts: vec![],
            errors: vec![],
        };
        if requests.len() > 64 {
            result.errors.push("collection request count limit".into());
            return result;
        }
        let mut bytes = 0usize;
        let mut paths = std::collections::BTreeSet::new();
        let mut references = std::collections::BTreeSet::new();
        for (index, request) in requests.iter().enumerate() {
            if request.path.as_os_str().len() > 4096 || request.reference.uri.len() > 4096 {
                result
                    .errors
                    .push(format!("artifact {index}: path/reference length limit"));
                continue;
            }
            if !paths.insert(&request.path) || !references.insert(&request.reference.uri) {
                result
                    .errors
                    .push(format!("artifact {index}: duplicate path or identity"));
                continue;
            }
            if request.reference.size > (16 * 1024 * 1024usize).saturating_sub(bytes) as u64 {
                result
                    .errors
                    .push(format!("artifact {index}: collection byte budget exceeded"));
                continue;
            }
            match self.read(&request.path, &request.reference, attempt) {
                Ok(artifact) => {
                    bytes += artifact.bytes.len();
                    result.artifacts.push(artifact);
                }
                Err(error) => result.errors.push(format!("artifact {index}: {error}")),
            }
        }
        result
    }
}
