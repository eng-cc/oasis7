//! On-demand path projection of the existing inventory, never a filesystem read.
use crate::{
    CommandCenter, Error, Result,
    document_inventory::{DocumentInventoryQuery, IndexedDocument},
    index,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDirectoryQuery {
    pub inventory: DocumentInventoryQuery,
    /// Empty for root; otherwise an exact repository-relative directory path.
    pub parent_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDirectoryEntry {
    pub path: String,
    pub name: String,
    /// directory / prd / design / entry. No semantic ownership is inferred.
    pub kind: String,
    pub matching_files: usize,
    pub depth_limited: bool,
    pub file: Option<IndexedDocument>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDirectoryPage {
    pub schema_version: u32,
    pub project_id: String,
    pub snapshot_id: String,
    pub baseline_oid: String,
    pub generated_at: String,
    pub parent_path: String,
    /// Full scan coverage, independent of filters and this projection's kind scope.
    pub candidates: usize,
    pub read: usize,
    pub omitted: usize,
    pub stale: bool,
    pub error: Option<String>,
    pub eligible_files: usize,
    /// Matching unique files across the full projection, not just this page/parent.
    pub matching_files: usize,
    pub total_children: usize,
    pub entries: Vec<DocumentDirectoryEntry>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    snapshot: String,
    filter: String,
    offset: usize,
}

impl CommandCenter {
    pub fn document_directory(
        &self,
        request: DocumentDirectoryQuery,
    ) -> Result<DocumentDirectoryPage> {
        let query = &request.inventory;
        let parent = &request.parent_path;
        if parent.len() > 4096
            || (!parent.is_empty()
                && (parent
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == "..")
                    || parent.split('/').count() >= 64))
        {
            return Err(Error::Invalid(
                "invalid document directory parent or depth limit".into(),
            ));
        }
        let inventory = self.load_document_inventory(query)?;
        let eligible: Vec<_> = inventory
            .files
            .iter()
            .filter(|f| ["prd", "design", "entry"].contains(&f.kind.as_str()))
            .collect();
        let prefix = if parent.is_empty() {
            String::new()
        } else {
            format!("{parent}/")
        };
        if !parent.is_empty() && !eligible.iter().any(|f| f.path.starts_with(&prefix)) {
            return Err(Error::Invalid(
                "document directory parent is outside inventory".into(),
            ));
        }
        let text = query.query.trim().to_lowercase();
        let filter = index::digest(&serde_json::to_vec(&(
            "document-directory-v1",
            parent,
            &text,
            &query.read_filter,
        ))?);
        let offset = if let Some(cursor) = &query.cursor {
            let cursor: Cursor = serde_json::from_str(cursor)
                .map_err(|_| Error::Invalid("invalid document directory cursor".into()))?;
            if cursor.snapshot != inventory.snapshot_id || cursor.filter != filter {
                return Err(Error::Invalid(
                    "document directory cursor expired; reload this level".into(),
                ));
            }
            cursor.offset
        } else {
            0
        };
        let matching: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|f| f.matches_filter(&text, &query.read_filter))
            .collect();
        let mut children: BTreeMap<String, DocumentDirectoryEntry> = BTreeMap::new();
        for file in &matching {
            let Some(relative) = file.path.strip_prefix(&prefix) else {
                continue;
            };
            if let Some((name, _)) = relative.split_once('/') {
                let path = format!("{prefix}{name}");
                let entry =
                    children
                        .entry(path.clone())
                        .or_insert_with(|| DocumentDirectoryEntry {
                            name: name.into(),
                            depth_limited: path.split('/').count() >= 64,
                            path,
                            kind: "directory".into(),
                            matching_files: 0,
                            file: None,
                        });
                entry.matching_files += 1;
            } else {
                children.insert(
                    file.path.clone(),
                    DocumentDirectoryEntry {
                        path: file.path.clone(),
                        name: relative.into(),
                        kind: file.kind.clone(),
                        matching_files: 1,
                        depth_limited: false,
                        file: Some((*file).clone()),
                    },
                );
            }
        }
        let total_children = children.len();
        if offset > total_children {
            return Err(Error::Invalid(
                "document directory cursor outside result range".into(),
            ));
        }
        let end = offset.saturating_add(100).min(total_children);
        let next_cursor = (end < total_children)
            .then(|| {
                serde_json::to_string(&Cursor {
                    snapshot: inventory.snapshot_id.clone(),
                    filter,
                    offset: end,
                })
            })
            .transpose()?;
        let read = inventory.files.iter().filter(|f| f.state == "read").count();
        let source = self.current.sources.iter().find(|s| s.kind == "local");
        Ok(DocumentDirectoryPage {
            schema_version: 1,
            project_id: inventory.project_id,
            snapshot_id: inventory.snapshot_id,
            baseline_oid: inventory.baseline_oid,
            generated_at: inventory.generated_at,
            parent_path: parent.clone(),
            candidates: inventory.files.len(),
            read,
            omitted: inventory.files.len() - read,
            stale: source.is_none_or(|s| s.error.is_some()),
            error: source.and_then(|s| s.error.clone()),
            eligible_files: eligible.len(),
            matching_files: matching.len(),
            total_children,
            entries: children.into_values().skip(offset).take(100).collect(),
            next_cursor,
        })
    }
}
