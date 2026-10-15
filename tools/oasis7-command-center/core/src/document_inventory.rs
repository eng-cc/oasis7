//! Versioned file coverage for the existing tracked working-tree chapter index.
//! This is not the independent tree snapshot/relationship model.
use crate::{CommandCenter, Error, Result, context, domain::DocumentLifecycle, index, load};
use serde::{Deserialize, Serialize};
use std::path::Path;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IndexedDocument {
    pub file_key: String,
    pub path: String,
    pub title: String,
    pub kind: String,
    pub state: String,
    pub reason: Option<String>,
    pub revision: Option<String>,
    pub bytes: Option<usize>,
    pub dirty: bool,
    pub lifecycle: Option<DocumentLifecycle>,
    /// None in pre-pairing caches; refresh is required before claiming absence.
    #[serde(default)]
    pub pairings: Option<DocumentPairings>,
    /// Explicit design-header upstream declarations; None means not yet parsed.
    #[serde(default)]
    pub upstream: Option<DocumentPairings>,
}

/// Explicit header link declarations, grouped as pairings or design upstreams.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPairings {
    pub targets: Vec<DocumentPairing>,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPairing {
    pub declaration_line: usize,
    pub destination: String,
    pub target: Option<String>,
    /// resolved / fragment_unchecked (legacy) / missing / missing_fragment / ambiguous / unread / outside_scope
    pub resolution: String,
    #[serde(default)]
    pub target_section: Option<String>,
    #[serde(default)]
    pub target_revision: Option<String>,
    /// Candidate path classification, not proof of authority or implementation.
    #[serde(default)]
    pub target_kind: Option<String>,
}

impl IndexedDocument {
    pub(crate) fn new(project_id: &str, path: &str, dirty: bool) -> Self {
        let kind = if path.ends_with("/README.md") || path == "AGENTS.md" {
            "entry"
        } else if path.ends_with("/prd.md") || path.ends_with(".prd.md") {
            "prd"
        } else if path.ends_with("/design.md") || path.ends_with(".design.md") {
            "design"
        } else {
            "other"
        };
        Self {
            file_key: index::digest(format!("{project_id}:{path}").as_bytes()),
            path: path.into(),
            title: path.into(),
            kind: kind.into(),
            state: "read".into(),
            reason: None,
            revision: None,
            bytes: None,
            dirty,
            lifecycle: None,
            pairings: None,
            upstream: None,
        }
    }
    pub(crate) fn omit(&mut self, reason: &str) {
        self.state = match reason {
            "deleted" => "deleted",
            "permission_denied" | "read_error" | "path_unavailable" | "invalid_utf8" => "error",
            _ => "skipped",
        }
        .into();
        self.reason = Some(reason.into());
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Inventory {
    pub schema_version: u32,
    pub project_id: String,
    pub baseline_oid: String,
    pub snapshot_id: String,
    pub generated_at: String,
    pub source_generation: String,
    pub sections_digest: String,
    pub files: Vec<IndexedDocument>,
}

pub(crate) fn sections_digest(sections: &[crate::domain::DocumentSection]) -> String {
    let identities: Vec<_> = sections
        .iter()
        .map(|s| {
            (
                &s.path,
                &s.id,
                &s.revision,
                &s.committed_revision,
                s.dirty,
                s.line,
            )
        })
        .collect();
    index::digest(&serde_json::to_vec(&identities).expect("section identities serialize"))
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInventoryQuery {
    pub schema_version: u32,
    pub project_id: String,
    pub snapshot_id: Option<String>,
    pub query: String,
    /// all / read / omitted
    pub read_filter: String,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInventoryPage {
    pub schema_version: u32,
    pub project_id: String,
    pub snapshot_id: String,
    pub baseline_oid: String,
    pub generated_at: String,
    pub candidates: usize,
    pub read: usize,
    pub omitted: usize,
    pub read_bytes: usize,
    pub total_matches: usize,
    pub partial: bool,
    pub stale: bool,
    pub error: Option<String>,
    pub files: Vec<IndexedDocument>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Cursor {
    snapshot: String,
    filter: String,
    offset: usize,
}

impl CommandCenter {
    pub fn document_inventory(
        &self,
        query: DocumentInventoryQuery,
    ) -> Result<DocumentInventoryPage> {
        if query.schema_version != 1 {
            return Err(Error::Invalid(
                "unsupported document inventory schema".into(),
            ));
        }
        if query.query.len() > 4096
            || query.cursor.as_ref().is_some_and(|c| c.len() > 4096)
            || !["all", "read", "omitted"].contains(&query.read_filter.as_str())
        {
            return Err(Error::Invalid(
                "invalid document inventory filter or cursor".into(),
            ));
        }
        let project = self
            .current
            .project
            .as_ref()
            .filter(|p| p.id == query.project_id)
            .ok_or_else(|| Error::Invalid("document inventory project changed".into()))?;
        context::check_project(&self.db, Path::new(&project.path))?;
        let cached: serde_json::Value = load(
            &self.db,
            "cache",
            &format!("document_inventory_v1:{}", project.id),
        )?
        .ok_or_else(|| {
            Error::Invalid("document inventory unavailable; refresh local facts".into())
        })?;
        if cached
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(Error::Invalid(
                "unsupported cached document inventory schema; refresh local facts".into(),
            ));
        }
        let inventory: Inventory = serde_json::from_value(cached)?;
        let source = self.current.sources.iter().find(|s| s.kind == "local");
        if source.is_none_or(|s| {
            s.source_revision.as_deref() != Some(inventory.baseline_oid.as_str())
                || (s.error.is_none() && s.generation != inventory.source_generation)
        }) {
            return Err(Error::Invalid(
                "document inventory expired; refresh local facts".into(),
            ));
        }
        if inventory.project_id != project.id
            || inventory.sections_digest != sections_digest(&self.current.sections)
            || query
                .snapshot_id
                .as_ref()
                .is_some_and(|id| id != &inventory.snapshot_id)
        {
            return Err(Error::Invalid(
                "document inventory expired; reload the first page".into(),
            ));
        }
        let text = query.query.trim().to_lowercase();
        let filter = index::digest(&serde_json::to_vec(&(&text, &query.read_filter))?);
        let offset = if let Some(cursor) = &query.cursor {
            let cursor: Cursor = serde_json::from_str(cursor)
                .map_err(|_| Error::Invalid("invalid document inventory cursor".into()))?;
            if cursor.snapshot != inventory.snapshot_id || cursor.filter != filter {
                return Err(Error::Invalid(
                    "document inventory cursor expired; reload the first page".into(),
                ));
            }
            cursor.offset
        } else {
            0
        };
        let matches: Vec<_> = inventory
            .files
            .iter()
            .filter(|file| {
                (query.read_filter == "all"
                    || (query.read_filter == "read") == (file.state == "read"))
                    && (text.is_empty()
                        || file.path.to_lowercase().contains(&text)
                        || file.title.to_lowercase().contains(&text))
            })
            .collect();
        if offset > matches.len() {
            return Err(Error::Invalid(
                "document inventory cursor outside result range".into(),
            ));
        }
        let end = offset.saturating_add(100).min(matches.len());
        let next_cursor = (end < matches.len())
            .then(|| {
                serde_json::to_string(&Cursor {
                    snapshot: inventory.snapshot_id.clone(),
                    filter,
                    offset: end,
                })
            })
            .transpose()?;
        let read = inventory
            .files
            .iter()
            .filter(|file| file.state == "read")
            .count();
        Ok(DocumentInventoryPage {
            schema_version: 1,
            project_id: inventory.project_id.clone(),
            snapshot_id: inventory.snapshot_id.clone(),
            baseline_oid: inventory.baseline_oid.clone(),
            generated_at: inventory.generated_at.clone(),
            candidates: inventory.files.len(),
            read,
            omitted: inventory.files.len() - read,
            read_bytes: inventory.files.iter().filter_map(|file| file.bytes).sum(),
            total_matches: matches.len(),
            partial: read != inventory.files.len(),
            stale: source.is_none_or(|s| s.error.is_some()),
            error: source.and_then(|s| s.error.clone()),
            files: matches[offset..end]
                .iter()
                .map(|file| (*file).clone())
                .collect(),
            next_cursor,
        })
    }
}
