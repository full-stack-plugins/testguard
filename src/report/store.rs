//! Controller-owned local evidence storage; no authentication or execution grant.
use super::{AttemptRecord, normalize::ArtifactRef};
use crate::plan::FrozenPlan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorePolicy {
    pub id: String,
    pub retention_seconds: u64,
    pub evidence_seconds: u64,
    pub max_attempts: usize,
    pub max_total_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub execution_source: String,
    pub tool: String,
    pub tool_version: String,
    pub analyzer: String,
    pub analyzer_version: String,
    pub config_digest: String,
    pub parent_attempt: Option<String>,
    pub command: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub approval_refs: Vec<String>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Evidence,
    Log,
}
pub struct InputArtifact<'a> {
    pub reference: &'a ArtifactRef,
    pub bytes: &'a [u8],
    pub kind: ArtifactKind,
}
pub struct AppendInput<'a> {
    pub plan: &'a FrozenPlan,
    pub attempt: &'a AttemptRecord,
    pub provenance: &'a Provenance,
    pub artifacts: &'a [InputArtifact<'a>],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    namespace_digest: String,
    attempt: String,
    manifest_digest: String,
}
impl Receipt {
    /// Storage location is diagnostic metadata, not an access grant.
    pub fn storage_key(&self) -> String {
        format!(
            "{}-{}",
            self.namespace_digest,
            super::normalize::bytes_digest(self.attempt.as_bytes())
        )
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 2048 {
            return Err("receipt size limit".into());
        }
        let r: Self = serde_json::from_slice(bytes).map_err(|_| "receipt schema")?;
        if !crate::obligation::digest(&r.namespace_digest)
            || !crate::obligation::digest(&r.manifest_digest)
            || !safe_id(&r.attempt)
        {
            return Err("invalid receipt".into());
        }
        Ok(r)
    }
}
#[derive(Clone)]
pub struct Administration {
    owner: std::sync::Arc<()>,
}
#[derive(Clone)]
pub struct Access {
    owner: std::sync::Arc<()>,
    namespace_digest: String,
}
#[derive(Clone)]
pub struct RawAuditAccess(Access);
pub struct Consumption {
    pub attempt: AttemptRecord,
    pub plan: FrozenPlan,
    pub evidence: Vec<crate::runner::collect::CollectedArtifact>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    name: String,
    digest: String,
    size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedArtifact {
    reference: ArtifactRef,
    kind: ArtifactKind,
    raw: Blob,
    view: Option<Blob>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: String,
    namespace_digest: String,
    attempt_id: String,
    plan_digest: String,
    binding: crate::plan::Binding,
    baseline_digest: String,
    approved_revision: String,
    approval_ref: String,
    requirement_ids: Vec<String>,
    finished: bool,
    exit_code: Option<i32>,
    case_counts: BTreeMap<String, u64>,
    policy: StorePolicy,
    created_at: u64,
    expires_at: u64,
    retain_until: u64,
    provenance: Provenance,
    original_provenance_digest: String,
    plan: Blob,
    attempt: Blob,
    raw_provenance: Blob,
    artifacts: Vec<SavedArtifact>,
}
fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
const MAX_FILE: usize = 4 * 1024 * 1024;
const MAX_ATTEMPT: usize = 16 * 1024 * 1024;
fn encode<T: Serialize>(v: &T, limit: usize) -> Result<Vec<u8>, String> {
    struct Limited {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl std::io::Write for Limited {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if b.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("store input budget"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Limited {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, v).map_err(|_| "store serialization budget")?;
    Ok(writer.bytes)
}
#[cfg(target_os = "linux")]
pub use linux::EvidenceStore;
#[cfg(not(target_os = "linux"))]
pub struct EvidenceStore;
#[cfg(not(target_os = "linux"))]
impl EvidenceStore {
    pub fn open(
        _: &std::path::Path,
        _: StorePolicy,
        _: Vec<String>,
    ) -> Result<(Self, Administration), String> {
        Err("evidence store unsupported platform".into())
    }
}
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use rustix::fs::{self, AtFlags, FileType, FlockOperation, Mode, OFlags, RenameFlags};
    use std::{
        io::{Read, Write},
        os::fd::{AsRawFd, OwnedFd},
        path::Path,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    pub struct EvidenceStore {
        root: OwnedFd,
        owner: Arc<()>,
        policy: StorePolicy,
        secrets: Vec<String>,
    }
    fn io<T>(v: rustix::io::Result<T>) -> Result<T, String> {
        v.map_err(|e| format!("store filesystem: {e}"))
    }
    fn directory(root: &OwnedFd, name: &str) -> Result<OwnedFd, String> {
        io(fs::openat(
            root,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ))
    }
    fn check_file(fd: &OwnedFd) -> Result<u64, String> {
        let s = io(fs::fstat(fd))?;
        if FileType::from_raw_mode(s.st_mode) != FileType::RegularFile
            || s.st_nlink != 1
            || s.st_uid != rustix::process::geteuid().as_raw()
            || s.st_mode & 0o077 != 0
            || s.st_size < 0
        {
            return Err("store file access/type violation".into());
        }
        Ok(s.st_size as u64)
    }
    fn read_blob(dir: &OwnedFd, blob: &Blob) -> Result<Vec<u8>, String> {
        if !safe_id(&blob.name) || blob.size > MAX_FILE as u64 {
            return Err("store blob limits".into());
        }
        let fd = io(fs::openat(
            dir,
            blob.name.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        ))?;
        if check_file(&fd)? != blob.size {
            return Err("store blob size mismatch".into());
        }
        let mut bytes = Vec::new();
        std::fs::File::from(fd)
            .take(MAX_FILE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() != blob.size as usize
            || super::super::normalize::bytes_digest(&bytes) != blob.digest
        {
            return Err("store blob digest mismatch".into());
        }
        Ok(bytes)
    }
    fn write_blob(dir: &OwnedFd, name: &str, bytes: &[u8]) -> Result<Blob, String> {
        if !safe_id(name) || bytes.len() > MAX_FILE {
            return Err("store blob limit".into());
        }
        let fd = io(fs::openat(
            dir,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        ))?;
        let mut file = std::fs::File::from(fd);
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        io(fs::fchmod(&file, Mode::RUSR))?;
        Ok(Blob {
            name: name.into(),
            digest: super::super::normalize::bytes_digest(bytes),
            size: bytes.len() as u64,
        })
    }
    fn names(fd: &OwnedFd, limit: usize) -> Result<Vec<String>, String> {
        // /proc/self/fd is an internal reference to our pinned directory, never caller input.
        let entries = std::fs::read_dir(format!("/proc/self/fd/{}", fd.as_raw_fd()))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            out.push(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "store name UTF-8")?,
            );
            if out.len() > limit {
                return Err("store entry count limit".into());
            }
        }
        Ok(out)
    }
    struct Stage<'a> {
        root: &'a OwnedFd,
        fd: OwnedFd,
        name: String,
        published: bool,
    }
    impl Drop for Stage<'_> {
        fn drop(&mut self) {
            if !self.published {
                let _ = fs::fchmod(&self.fd, Mode::RUSR | Mode::WUSR | Mode::XUSR);
                if let Ok(entries) = names(&self.fd, 140) {
                    for name in entries {
                        let _ = fs::unlinkat(&self.fd, name, AtFlags::empty());
                    }
                }
                let _ = fs::unlinkat(self.root, self.name.as_str(), AtFlags::REMOVEDIR);
            }
        }
    }
    fn counts(attempt: &AttemptRecord) -> BTreeMap<String, u64> {
        let mut counts = BTreeMap::new();
        for o in &attempt.observations {
            let status = match o.status {
                super::super::CaseStatus::Pass => "pass",
                super::super::CaseStatus::Fail => "fail",
                super::super::CaseStatus::Skip => "skip",
                super::super::CaseStatus::Unknown => "unknown",
            };
            *counts.entry(status.into()).or_insert(0) += 1;
        }
        counts
    }
    impl EvidenceStore {
        /// The controller alone opens this private root and distributes opaque
        /// namespace capabilities. This does not authenticate an external actor.
        pub fn open(
            root: &Path,
            policy: StorePolicy,
            mut secrets: Vec<String>,
        ) -> Result<(Self, Administration), String> {
            if !safe_id(&policy.id)
                || policy.evidence_seconds == 0
                || policy.retention_seconds < policy.evidence_seconds
                || policy.retention_seconds > 366 * 86400
                || policy.max_attempts == 0
                || policy.max_attempts > 1024
                || policy.max_total_bytes == 0
                || policy.max_total_bytes > 256 * 1024 * 1024
                || secrets.len() > 32
                || secrets.iter().any(|s| s.is_empty() || s.len() > 1024)
            {
                return Err("explicit store policy outside limits".into());
            }
            let root = io(fs::open(
                root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ))?;
            let s = io(fs::fstat(&root))?;
            if s.st_uid != rustix::process::geteuid().as_raw() || s.st_mode & 0o077 != 0 {
                return Err("store root must be private to current owner".into());
            }
            secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
            secrets.dedup();
            let owner = Arc::new(());
            let admin = Administration {
                owner: owner.clone(),
            };
            Ok((
                Self {
                    root,
                    owner,
                    policy,
                    secrets,
                },
                admin,
            ))
        }
        pub fn grant(
            &self,
            admin: &Administration,
            namespace: &str,
        ) -> Result<(Access, RawAuditAccess), String> {
            if !Arc::ptr_eq(&self.owner, &admin.owner) {
                return Err("store administration denied".into());
            }
            if !safe_id(namespace) {
                return Err("invalid namespace".into());
            }
            let access = Access {
                owner: self.owner.clone(),
                namespace_digest: super::super::normalize::bytes_digest(namespace.as_bytes()),
            };
            Ok((access.clone(), RawAuditAccess(access)))
        }
        fn authorize(&self, access: &Access, receipt: Option<&Receipt>) -> Result<(), String> {
            if !Arc::ptr_eq(&self.owner, &access.owner)
                || receipt.is_some_and(|r| r.namespace_digest != access.namespace_digest)
            {
                return Err("store access denied".into());
            }
            Ok(())
        }
        fn lock(&self) -> Result<OwnedFd, String> {
            let fd = io(fs::openat(
                &self.root,
                ".lock",
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ))?;
            check_file(&fd)?;
            io(fs::flock(&fd, FlockOperation::NonBlockingLockExclusive))?;
            Ok(fd)
        }
        fn quota(&self, incoming: u64) -> Result<(), String> {
            let entries = names(&self.root, self.policy.max_attempts + 1)?;
            let mut attempts = 0;
            let mut total = incoming;
            for name in entries {
                if name == ".lock" {
                    continue;
                }
                attempts += 1;
                if attempts >= self.policy.max_attempts {
                    return Err("store attempt quota".into());
                }
                let dir = directory(&self.root, &name)?;
                for name in names(&dir, 140)? {
                    let fd = io(fs::openat(
                        &dir,
                        name,
                        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                        Mode::empty(),
                    ))?;
                    total = total
                        .checked_add(check_file(&fd)?)
                        .ok_or("store quota overflow")?;
                    if total > self.policy.max_total_bytes {
                        return Err("store byte quota".into());
                    }
                }
            }
            if total > self.policy.max_total_bytes {
                return Err("store byte quota".into());
            }
            Ok(())
        }
        fn redact(&self, text: &str) -> Result<String, String> {
            let mut out = String::new();
            let mut at = 0;
            while at < text.len() {
                let tail = &text[at..];
                let (piece, advance) = if let Some(secret) =
                    self.secrets.iter().find(|s| tail.starts_with(s.as_str()))
                {
                    ("[REDACTED]", secret.len())
                } else {
                    let n = tail.chars().next().unwrap().len_utf8();
                    (&tail[..n], n)
                };
                if piece.len() > MAX_FILE.saturating_sub(out.len()) {
                    return Err("redacted view budget".into());
                }
                out.push_str(piece);
                at += advance;
            }
            Ok(out)
        }
        fn redacted_provenance(&self, p: &Provenance) -> Result<Provenance, String> {
            let mut view = serde_json::to_value(p).map_err(|e| e.to_string())?;
            fn walk(store: &EvidenceStore, v: &mut serde_json::Value) -> Result<(), String> {
                match v {
                    serde_json::Value::String(s) => *s = store.redact(s)?,
                    serde_json::Value::Array(a) => {
                        for x in a {
                            walk(store, x)?
                        }
                    }
                    serde_json::Value::Object(o) => {
                        for x in o.values_mut() {
                            walk(store, x)?
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            walk(self, &mut view)?;
            let mut environment = serde_json::Map::new();
            for (key, value) in &p.environment {
                let key = self.redact(key)?;
                if environment
                    .insert(key, serde_json::Value::String(self.redact(value)?))
                    .is_some()
                {
                    return Err("redacted environment key collision".into());
                }
            }
            view["environment"] = serde_json::Value::Object(environment);
            serde_json::from_value(view).map_err(|e| e.to_string())
        }
        pub fn append(
            &self,
            access: &Access,
            input: AppendInput<'_>,
            now: u64,
        ) -> Result<Receipt, String> {
            self.authorize(access, None)?;
            if !safe_id(&input.attempt.attempt_id)
                || input.artifacts.is_empty()
                || input.artifacts.len() > 64
                || input.attempt.observations.len() > 4096
            {
                return Err("store input count/id limit".into());
            }
            crate::coverage::preflight(input.plan, Some(input.attempt), &[], &[])?;
            let plan_bytes = encode(input.plan, 1024 * 1024)?;
            let attempt_bytes = encode(input.attempt, 1024 * 1024)?;
            let provenance_bytes = encode(input.provenance, 64 * 1024)?;
            input.plan.validate()?;
            input.attempt.validate()?;
            if input.attempt.plan_digest != super::super::normalize::canonical_digest(input.plan)?
                || input.artifacts.len() != input.attempt.artifacts.len()
            {
                return Err("store exact plan/artifact mismatch".into());
            }
            if [
                &input.provenance.execution_source,
                &input.provenance.tool,
                &input.provenance.tool_version,
                &input.provenance.analyzer,
                &input.provenance.analyzer_version,
            ]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 256)
                || !crate::obligation::digest(&input.provenance.config_digest)
                || input.provenance.command.is_empty()
                || input
                    .provenance
                    .parent_attempt
                    .as_ref()
                    .is_some_and(|id| !safe_id(id) || id == &input.attempt.attempt_id)
            {
                return Err("store audit fields incomplete".into());
            }
            let expires_at = now
                .checked_add(self.policy.evidence_seconds)
                .ok_or("time overflow")?;
            let retain_until = now
                .checked_add(self.policy.retention_seconds)
                .ok_or("time overflow")?;
            let mut charged = plan_bytes.len() + attempt_bytes.len() + provenance_bytes.len();
            let mut views = Vec::new();
            for (artifact, expected) in input.artifacts.iter().zip(&input.attempt.artifacts) {
                if artifact.reference.uri != expected.uri
                    || artifact.reference.digest != expected.digest
                    || artifact.reference.size != expected.size
                    || artifact.reference.media_type != expected.media_type
                    || artifact.bytes.len() > MAX_FILE
                {
                    return Err("artifact identity/size mismatch".into());
                }
                artifact.reference.verify(artifact.bytes)?;
                let view = if artifact.kind == ArtifactKind::Log {
                    if input
                        .attempt
                        .observations
                        .iter()
                        .any(|o| o.artifact_uri == artifact.reference.uri)
                    {
                        return Err("native observations cannot cite redacted log views".into());
                    }
                    Some(
                        self.redact(std::str::from_utf8(artifact.bytes).map_err(|_| "log UTF-8")?)?
                            .into_bytes(),
                    )
                } else {
                    None
                };
                charged = charged
                    .saturating_add(artifact.bytes.len())
                    .saturating_add(view.as_ref().map_or(0, Vec::len));
                if charged > MAX_ATTEMPT {
                    return Err("attempt storage byte budget".into());
                }
                views.push(view);
            }
            let provenance = self.redacted_provenance(input.provenance)?;
            let _lock = self.lock()?;
            self.quota(charged as u64 + 1024 * 1024)?; // conservative manifest reservation
            let stage_name = format!(
                ".stage-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            io(fs::mkdirat(
                &self.root,
                stage_name.as_str(),
                Mode::RUSR | Mode::WUSR | Mode::XUSR,
            ))?;
            let mut stage = Stage {
                root: &self.root,
                fd: directory(&self.root, &stage_name)?,
                name: stage_name,
                published: false,
            };
            let plan = write_blob(&stage.fd, "plan.json", &plan_bytes)?;
            let attempt = write_blob(&stage.fd, "attempt.json", &attempt_bytes)?;
            let raw_provenance = write_blob(&stage.fd, "provenance.json", &provenance_bytes)?;
            let mut artifacts = Vec::new();
            for (index, (input, view)) in input.artifacts.iter().zip(views).enumerate() {
                let raw = write_blob(&stage.fd, &format!("raw-{index}"), input.bytes)?;
                let view = view
                    .map(|bytes| write_blob(&stage.fd, &format!("view-{index}"), &bytes))
                    .transpose()?;
                artifacts.push(SavedArtifact {
                    reference: input.reference.clone(),
                    kind: input.kind,
                    raw,
                    view,
                });
            }
            let manifest = Manifest {
                version: "testguard.store/linux-v1".into(),
                namespace_digest: access.namespace_digest.clone(),
                attempt_id: input.attempt.attempt_id.clone(),
                plan_digest: input.attempt.plan_digest.clone(),
                binding: input.plan.binding().clone(),
                baseline_digest: input.plan.obligations().baseline_digest.clone(),
                approved_revision: input.plan.obligations().revision.clone(),
                approval_ref: input.plan.obligations().approval_ref.clone(),
                requirement_ids: input.plan.obligations().requirements.clone(),
                finished: input.attempt.finished,
                exit_code: input.attempt.exit_code,
                case_counts: counts(input.attempt),
                policy: self.policy.clone(),
                created_at: now,
                expires_at,
                retain_until,
                provenance,
                original_provenance_digest: raw_provenance.digest.clone(),
                plan,
                attempt,
                raw_provenance,
                artifacts,
            };
            let manifest_bytes = encode(&manifest, 1024 * 1024)?;
            let manifest_blob = write_blob(&stage.fd, "manifest.json", &manifest_bytes)?;
            let receipt = Receipt {
                namespace_digest: access.namespace_digest.clone(),
                attempt: input.attempt.attempt_id.clone(),
                manifest_digest: manifest_blob.digest,
            };
            io(fs::fchmod(&stage.fd, Mode::RUSR | Mode::XUSR))?;
            io(fs::fsync(&stage.fd))?;
            io(fs::renameat_with(
                &self.root,
                stage.name.as_str(),
                &self.root,
                receipt.storage_key(),
                RenameFlags::NOREPLACE,
            ))?;
            stage.published = true;
            io(fs::fsync(&self.root))?;
            Ok(receipt)
        }
        fn manifest(
            &self,
            access: &Access,
            receipt: &Receipt,
        ) -> Result<(OwnedFd, Manifest), String> {
            self.authorize(access, Some(receipt))?;
            if !safe_id(&receipt.attempt) || !crate::obligation::digest(&receipt.manifest_digest) {
                return Err("invalid receipt".into());
            }
            let dir = directory(&self.root, &receipt.storage_key())?;
            let fd = io(fs::openat(
                &dir,
                "manifest.json",
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            ))?;
            let size = check_file(&fd)?;
            if size > 1024 * 1024 {
                return Err("manifest budget".into());
            }
            let bytes = read_blob(
                &dir,
                &Blob {
                    name: "manifest.json".into(),
                    size,
                    digest: receipt.manifest_digest.clone(),
                },
            )?;
            let m: Manifest =
                serde_json::from_slice(&bytes).map_err(|_| "store manifest schema")?;
            if m.version != "testguard.store/linux-v1"
                || m.namespace_digest != access.namespace_digest
                || m.attempt_id != receipt.attempt
                || m.artifacts.len() > 64
                || m.expires_at <= m.created_at
                || m.retain_until < m.expires_at
            {
                return Err("store manifest binding".into());
            }
            let mut total = m
                .plan
                .size
                .saturating_add(m.attempt.size)
                .saturating_add(m.raw_provenance.size);
            if m.plan.size > 1024 * 1024
                || m.attempt.size > 1024 * 1024
                || m.raw_provenance.size > 64 * 1024
            {
                return Err("stored metadata budget".into());
            }
            for a in &m.artifacts {
                total = total
                    .saturating_add(a.raw.size)
                    .saturating_add(a.view.as_ref().map_or(0, |b| b.size));
            }
            if total > MAX_ATTEMPT as u64 {
                return Err("stored attempt byte budget".into());
            }
            Ok((dir, m))
        }
        pub fn consume(
            &self,
            access: &Access,
            receipt: &Receipt,
            now: u64,
        ) -> Result<Consumption, String> {
            let _lock = self.lock()?;
            let (dir, m) = self.manifest(access, receipt)?;
            if now < m.created_at || now >= m.expires_at {
                return Err("evidence outside validity interval; rerun required".into());
            }
            let plan: FrozenPlan = serde_json::from_slice(&read_blob(&dir, &m.plan)?)
                .map_err(|_| "stored plan schema")?;
            let attempt: AttemptRecord = serde_json::from_slice(&read_blob(&dir, &m.attempt)?)
                .map_err(|_| "stored attempt schema")?;
            crate::coverage::preflight(&plan, Some(&attempt), &[], &[])?;
            plan.validate()?;
            attempt.validate()?;
            read_blob(&dir, &m.raw_provenance)?;
            if attempt.plan_digest != super::super::normalize::canonical_digest(&plan)?
                || attempt.plan_digest != m.plan_digest
                || attempt.attempt_id != m.attempt_id
                || attempt.exit_code != m.exit_code
                || attempt.finished != m.finished
                || counts(&attempt) != m.case_counts
                || plan.obligations().requirements != m.requirement_ids
                || plan.obligations().revision != m.approved_revision
                || plan.obligations().approval_ref != m.approval_ref
            {
                return Err("stored attempt binding mismatch".into());
            }
            let mut evidence = Vec::new();
            for a in &m.artifacts {
                let bytes = read_blob(&dir, &a.raw)?;
                a.reference.validate(&m.attempt_id)?;
                a.reference.verify(&bytes)?;
                if let Some(view) = &a.view {
                    read_blob(&dir, view)?;
                }
                if a.kind == ArtifactKind::Evidence {
                    evidence.push(crate::runner::collect::CollectedArtifact {
                        reference: a.reference.clone(),
                        bytes,
                    });
                }
            }
            Ok(Consumption {
                attempt,
                plan,
                evidence,
            })
        }
        pub fn view(
            &self,
            access: &Access,
            receipt: &Receipt,
        ) -> Result<serde_json::Value, String> {
            let _lock = self.lock()?;
            let (dir, m) = self.manifest(access, receipt)?;
            let mut view = serde_json::to_value(&m).map_err(|e| e.to_string())?;
            let logs:Vec<_>=m.artifacts.iter().filter_map(|a|a.view.as_ref().map(|v|(a,v))).map(|(a,v)|serde_json::json!({"uri":a.reference.uri,"redacted":read_blob(&dir,v).ok().and_then(|b|String::from_utf8(b).ok()),"is_native_evidence":false})).collect();
            view["logs"] = serde_json::json!(logs);
            Ok(view)
        }
        pub fn raw_artifact(
            &self,
            access: &RawAuditAccess,
            receipt: &Receipt,
            uri: &str,
        ) -> Result<Vec<u8>, String> {
            let _lock = self.lock()?;
            let (dir, m) = self.manifest(&access.0, receipt)?;
            let a = m
                .artifacts
                .iter()
                .find(|a| a.reference.uri == uri)
                .ok_or("artifact URI not in attempt")?;
            read_blob(&dir, &a.raw)
        }
        pub fn raw_provenance(
            &self,
            access: &RawAuditAccess,
            receipt: &Receipt,
        ) -> Result<Vec<u8>, String> {
            let _lock = self.lock()?;
            let (dir, m) = self.manifest(&access.0, receipt)?;
            read_blob(&dir, &m.raw_provenance)
        }
        /// Purge expired retained payloads, preserving the immutable audit manifest.
        /// This never makes an expired attempt consumable again.
        pub fn purge(
            &self,
            admin: &Administration,
            access: &Access,
            receipt: &Receipt,
            now: u64,
        ) -> Result<(), String> {
            if !Arc::ptr_eq(&self.owner, &admin.owner) {
                return Err("store administration denied".into());
            }
            let _lock = self.lock()?;
            let (dir, m) = self.manifest(access, receipt)?;
            if now < m.retain_until {
                return Err("retention has not elapsed".into());
            }
            io(fs::fchmod(&dir, Mode::RUSR | Mode::WUSR | Mode::XUSR))?;
            let result = (|| {
                let mut names = vec![&m.plan.name, &m.attempt.name, &m.raw_provenance.name];
                for a in &m.artifacts {
                    names.push(&a.raw.name);
                    if let Some(v) = &a.view {
                        names.push(&v.name);
                    }
                }
                for name in names {
                    if !safe_id(name) {
                        return Err("invalid purge name".into());
                    }
                    match fs::unlinkat(&dir, name.as_str(), AtFlags::empty()) {
                        Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                io(fs::fsync(&dir))
            })();
            let restore = io(fs::fchmod(&dir, Mode::RUSR | Mode::XUSR));
            result.and(restore)
        }
    }
}
