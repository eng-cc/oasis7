use crate::error::{ContextError, Result};
use serde::{Deserialize, Serialize};
pub const MAX_ENTRY_BYTES: usize = 1024 * 1024;
pub const MAX_METADATA_BYTES: usize = 16 * 1024;
pub const MAX_STORE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 5000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StoreConfig {
    pub schema_version: String,
    pub store_id: String,
    pub project_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub schema_version: String,
    pub project_id: String,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub environment: String,
    pub owner: String,
    pub tags: Vec<String>,
    pub state: String,
    pub updated_at: String,
    pub updated_by: String,
    #[serde(deserialize_with = "required_verification")]
    pub verification: Option<Verification>,
    pub credential_refs: Vec<CredentialRef>,
    pub links: Vec<Link>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub method: String,
    pub observed_at: String,
    pub observer: String,
    pub evidence_ref: String,
    pub subject_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CredentialRef {
    pub id: String,
    pub provider: String,
    pub locator: String,
    pub usage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub entry_id: String,
    pub relation: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub metadata: Metadata,
    pub body: String,
}
#[derive(Clone, Debug)]
pub struct EntrySnapshot {
    pub raw_bytes: Vec<u8>,
    pub revision: String,
    pub entry: Entry,
    pub verification_state: VerificationState,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    Unverified,
    ContentChanged,
    FutureDated,
    Expired,
    Current,
}
#[derive(Clone, Debug)]
pub enum WriteCondition {
    Absent,
    Revision(String),
}
#[derive(Clone, Debug, Serialize)]
pub struct SaveResult {
    pub revision: String,
    pub persistence: String,
    pub changed: bool,
}
pub fn validate_id(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 64
        || !s.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && (b == b'_' || b == b'-'))
        })
    {
        return Err(ContextError::new("INVALID_INPUT", "invalid identifier"));
    }
    Ok(())
}
fn text(s: &str, max: usize) -> Result<()> {
    if s.is_empty() || s.len() > max || s.contains('\0') {
        Err(ContextError::new("INVALID_INPUT", "invalid metadata field"))
    } else {
        Ok(())
    }
}
pub fn validate_timestamp(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    if !s.ends_with('Z') {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "timestamp must use UTC Z form",
        ));
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|x| x.with_timezone(&chrono::Utc))
        .map_err(|_| ContextError::new("INVALID_INPUT", "invalid timestamp"))
}
impl StoreConfig {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "oasis7.context_store.v1"
            || self.store_id.len() != 32
            || !self
                .store_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ContextError::new(
                "INVALID_SCHEMA",
                "invalid store configuration",
            ));
        }
        validate_id(&self.project_id)?;
        text(&self.name, 256)
    }
}
impl Metadata {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "oasis7.context_entry.v1" {
            return Err(ContextError::new(
                "INVALID_SCHEMA",
                "unsupported entry schema",
            ));
        }
        validate_id(&self.id)?;
        validate_id(&self.project_id)?;
        validate_id(&self.environment)?;
        text(&self.title, 256)?;
        text(&self.owner, 128)?;
        text(&self.updated_by, 128)?;
        validate_timestamp(&self.updated_at)?;
        if !["machine", "environment", "runbook"].contains(&self.kind.as_str())
            || !["active", "archived"].contains(&self.state.as_str())
        {
            return Err(ContextError::new("INVALID_INPUT", "invalid entry enum"));
        }
        if self.tags.len() > 32 || self.credential_refs.len() > 32 || self.links.len() > 64 {
            return Err(ContextError::new(
                "LIMIT_EXCEEDED",
                "metadata collection exceeds limit",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for t in &self.tags {
            text(t, 64)?;
            if !seen.insert(t) {
                return Err(ContextError::new("INVALID_INPUT", "duplicate tag"));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for c in &self.credential_refs {
            validate_id(&c.id)?;
            text(&c.provider, 64)?;
            text(&c.locator, 1024)?;
            text(&c.usage, 512)?;
            if !seen.insert(&c.id) {
                return Err(ContextError::new(
                    "INVALID_INPUT",
                    "duplicate credential reference",
                ));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for l in &self.links {
            validate_id(&l.entry_id)?;
            text(&l.relation, 64)?;
            if !seen.insert((&l.entry_id, &l.relation)) {
                return Err(ContextError::new("INVALID_INPUT", "duplicate link"));
            }
        }
        if let Some(v) = &self.verification {
            if !["observation", "attestation"].contains(&v.method.as_str())
                || v.subject_sha256.len() != 64
                || !v
                    .subject_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(ContextError::new("INVALID_INPUT", "invalid verification"));
            }
            validate_timestamp(&v.observed_at)?;
            text(&v.observer, 128)?;
            text(&v.evidence_ref, 1024)?;
        }
        Ok(())
    }
}

fn required_verification<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Verification>, D::Error> {
    Option::<Verification>::deserialize(d)
}
