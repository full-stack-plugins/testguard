use super::{ExecutorProfile, RawArtifactSet};
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CASES: usize = 4096;
pub const MAX_XML_NODES: u32 = 16384;
const MAX_XML_DEPTH: usize = 64;

pub struct ObservationBudget {
    count: usize,
    bytes: usize,
    scope_bytes: usize,
}
impl ObservationBudget {
    pub fn new(raw: &RawArtifactSet, profile: &ExecutorProfile) -> Result<Self, String> {
        if raw.output.len() > MAX_REPORT_BYTES
            || raw.inventory.len() > MAX_REPORT_BYTES
            || raw.artifact.uri.len() > 4096
            || raw.artifact.media_type.len() > 128
            || raw.attempt_id.len() > 128
            || profile.features.len() > 64
        {
            return Err("native artifact input budget exceeded".into());
        }
        let scope_bytes = [
            &profile.tool,
            &profile.version,
            &profile.protocol,
            &profile.target,
            &profile.parameters,
            &profile.environment,
        ]
        .into_iter()
        .chain(profile.features.iter())
        .fold(raw.artifact.uri.len(), |n, value| {
            n.saturating_add(value.len())
        });
        if scope_bytes > 16 * 1024 {
            return Err("native executor scope budget exceeded".into());
        }
        Ok(Self {
            count: 0,
            bytes: 0,
            scope_bytes,
        })
    }
    // Reserve before identity serialization or copying shared scope into a case.
    pub fn reserve(&mut self, identity_bytes: usize) -> Result<(), String> {
        self.count += 1;
        self.bytes = self.bytes.saturating_add(
            identity_bytes
                .saturating_add(self.scope_bytes)
                .saturating_mul(6)
                .saturating_add(512),
        );
        if self.count > MAX_CASES || identity_bytes > 16 * 1024 || self.bytes > 8 * 1024 * 1024 {
            return Err("native observation expansion budget exceeded".into());
        }
        Ok(())
    }
}

/// Non-recursive lexical bound before the XML parser. The real parser still validates XML grammar.
/// Quoted attributes, comments, PI and CDATA do not create fake element nesting.
pub fn xml_preflight(input: &str) -> Result<(), String> {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    let mut depth = 0usize;
    let mut nodes = 0u32;
    while let Some(offset) = bytes[cursor..].iter().position(|b| *b == b'<') {
        cursor += offset;
        nodes += 1;
        if nodes > MAX_XML_NODES {
            return Err("XML node budget exceeded".into());
        }
        let rest = &input[cursor..];
        let special = if rest.starts_with("<!--") {
            Some((4, "-->"))
        } else if rest.starts_with("<![CDATA[") {
            Some((9, "]]>"))
        } else if rest.starts_with("<?") {
            Some((2, "?>"))
        } else {
            None
        };
        if let Some((prefix, end)) = special {
            cursor += prefix + rest[prefix..].find(end).ok_or("truncated XML section")? + end.len();
            continue;
        }
        if rest.starts_with("<!") {
            return Err("XML declarations/entities are forbidden".into());
        }
        let start = cursor;
        let mut quote = None;
        cursor += 1;
        while cursor < bytes.len() {
            let b = bytes[cursor];
            match quote {
                Some(q) if b == q => quote = None,
                Some(_) => {}
                None if b == b'\'' || b == b'"' => quote = Some(b),
                None if b == b'>' => break,
                None => {}
            }
            cursor += 1;
        }
        if cursor == bytes.len() {
            return Err("truncated XML tag".into());
        }
        if cursor - start > 64 * 1024 {
            return Err("XML tag budget exceeded".into());
        }
        if rest.starts_with("</") {
            depth = depth.checked_sub(1).ok_or("unbalanced XML close")?;
        } else if bytes[cursor - 1] != b'/' {
            depth += 1;
            if depth > MAX_XML_DEPTH {
                return Err("XML nesting limit".into());
            }
        }
        cursor += 1;
    }
    if depth != 0 {
        return Err("truncated XML document".into());
    }
    Ok(())
}
