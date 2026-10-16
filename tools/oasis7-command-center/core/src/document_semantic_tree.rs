//! Snapshot-bound explicit navigation projections. No filesystem reads or authority inference.
use crate::document_tree_reading::DocumentTreeRouteStep;
use crate::{
    CommandCenter, Error, Result,
    document_inventory::{DocumentInventoryQuery, DocumentPairing, IndexedDocument},
    index,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[path = "document_semantic_graph.rs"]
mod graph;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSemanticTreeQuery {
    pub inventory: DocumentInventoryQuery,
    pub view: String,
    pub parent_occurrence: String,
    #[serde(default)]
    #[ts(optional)]
    pub parent_route: Option<Vec<DocumentTreeRouteStep>>,
    #[serde(default)]
    #[ts(optional)]
    pub lifecycle_filter: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub kind_filter: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub pairing_filter: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSemanticBreadcrumb {
    pub occurrence_id: String,
    pub stable_route: Vec<DocumentTreeRouteStep>,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSemanticTreeEntry {
    pub occurrence_id: String,
    pub stable_route: Vec<DocumentTreeRouteStep>,
    pub label: String,
    pub path: Option<String>,
    pub kind: String,
    pub matching_files: usize,
    /// Current-view historical placeholder evidence; at most 20 sorted source paths.
    pub historical_reference_files: usize,
    pub historical_reference_paths: Vec<String>,
    pub cycle: bool,
    /// Cross-domain navigation is a reference leaf, never ownership.
    pub boundary: Option<String>,
    pub depth_limited: bool,
    pub source_path: Option<String>,
    pub source_line: Option<usize>,
    pub relationship: Option<DocumentPairing>,
    pub file: Option<IndexedDocument>,
    pub has_children: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSemanticTreePage {
    pub schema_version: u32,
    pub project_id: String,
    pub snapshot_id: String,
    pub baseline_oid: String,
    pub generated_at: String,
    pub view: String,
    pub parent_occurrence: String,
    pub breadcrumbs: Vec<DocumentSemanticBreadcrumb>,
    pub parent_route: Vec<DocumentTreeRouteStep>,
    pub route_notice: Option<String>,
    pub candidates: usize,
    pub read: usize,
    pub omitted: usize,
    pub stale: bool,
    pub error: Option<String>,
    pub eligible_files: usize,
    pub matching_files: usize,
    pub hidden_history_files: usize,
    pub lifecycle_filter: String,
    pub kind_filter: String,
    pub pairing_filter: String,
    pub total_children: usize,
    pub entries: Vec<DocumentSemanticTreeEntry>,
    pub next_cursor: Option<String>,
    pub issues: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    snapshot: String,
    filter: String,
    offset: usize,
}
impl CommandCenter {
    pub fn document_semantic_tree(
        &self,
        request: DocumentSemanticTreeQuery,
    ) -> Result<DocumentSemanticTreePage> {
        if let Some(route) = &request.parent_route {
            crate::document_tree_reading::validate_route(route)?;
        }
        if !matches!(request.view.as_str(), "product" | "professional") {
            return Err(Error::Invalid("invalid semantic tree view".into()));
        }
        let lifecycle = request.lifecycle_filter.as_deref().unwrap_or("all");
        if lifecycle.len() > 1024
            || !(matches!(
                lifecycle,
                "all"
                    | "current"
                    | "proposed"
                    | "draft"
                    | "active"
                    | "superseded"
                    | "retired"
                    | "other"
                    | "unknown"
            ) || lifecycle
                .strip_prefix("raw:")
                .is_some_and(|s| !s.is_empty()))
        {
            return Err(Error::Invalid("invalid document lifecycle filter".into()));
        }
        let kind = request.kind_filter.as_deref().unwrap_or("all");
        let pairing = request.pairing_filter.as_deref().unwrap_or("all");
        if !matches!(kind, "all" | "prd" | "design" | "entry")
            || !crate::document_directory_pairings::valid(pairing)
        {
            return Err(Error::Invalid("invalid semantic tree filter".into()));
        }
        let inventory = self.load_document_inventory(&request.inventory)?;
        let projection = graph::Projection::new(&inventory.files, &request.view);
        let text = request.inventory.query.trim().to_lowercase();
        let filtered: std::collections::BTreeSet<String> = projection
            .eligible
            .iter()
            .filter(|path| {
                let f = projection.files[path.as_str()];
                (kind == "all" || f.kind == kind)
                    && f.matches_filter(&text, &request.inventory.read_filter)
                    && crate::document_directory_pairings::matches(f, pairing)
            })
            .cloned()
            .collect();
        let matching: std::collections::BTreeSet<String> = filtered
            .iter()
            .filter(|path| {
                let f = projection.files[path.as_str()];
                crate::document_directory::matches_lifecycle(f, lifecycle)
                    || (lifecycle == "current" && projection.references.contains_key(path.as_str()))
            })
            .cloned()
            .collect();
        let hidden_history_files = if lifecycle == "current" {
            filtered
                .difference(&matching)
                .filter(|path| {
                    crate::document_directory_references::is_history(
                        projection.files[path.as_str()],
                    )
                })
                .count()
        } else {
            0
        };
        let (resolved_parent, route_notice) = if let Some(route) = &request.parent_route {
            projection.resolve_route(route, &inventory.snapshot_id, &request.view, &matching)
        } else {
            (request.parent_occurrence.clone(), None)
        };
        let (mut entries, breadcrumbs) = projection.children(
            &resolved_parent,
            &matching,
            &inventory.snapshot_id,
            &request.view,
            lifecycle == "current",
        )?;
        let filter = index::digest(&serde_json::to_vec(&(
            "semantic-tree-v1",
            &request.view,
            &resolved_parent,
            &text,
            &request.inventory.read_filter,
            lifecycle,
            kind,
            pairing,
        ))?);
        let offset = if let Some(cursor) = &request.inventory.cursor {
            let cursor: Cursor = serde_json::from_str(cursor)
                .map_err(|_| Error::Invalid("invalid semantic tree cursor".into()))?;
            if cursor.snapshot != inventory.snapshot_id || cursor.filter != filter {
                return Err(Error::Invalid(
                    "semantic tree cursor expired; reload this level".into(),
                ));
            }
            cursor.offset
        } else {
            0
        };
        let total_children = entries.len();
        if offset > total_children {
            return Err(Error::Invalid(
                "semantic tree cursor outside result range".into(),
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
        entries = entries.into_iter().skip(offset).take(100).collect();
        let read = inventory.files.iter().filter(|f| f.state == "read").count();
        let source = self.current.sources.iter().find(|s| s.kind == "local");
        Ok(DocumentSemanticTreePage {
            schema_version: 1,
            project_id: inventory.project_id.clone(),
            snapshot_id: inventory.snapshot_id.clone(),
            baseline_oid: inventory.baseline_oid.clone(),
            generated_at: inventory.generated_at.clone(),
            view: request.view,
            parent_occurrence: resolved_parent,
            parent_route: breadcrumbs
                .last()
                .map(|b| b.stable_route.clone())
                .unwrap_or_default(),
            route_notice,
            breadcrumbs,
            candidates: inventory.files.len(),
            read,
            omitted: inventory.files.len() - read,
            stale: source.is_none_or(|s| s.error.is_some()),
            error: source.and_then(|s| s.error.clone()),
            eligible_files: projection.eligible.len(),
            matching_files: matching.len(),
            hidden_history_files,
            lifecycle_filter: lifecycle.into(),
            kind_filter: kind.into(),
            pairing_filter: pairing.into(),
            total_children,
            entries,
            next_cursor,
            issues: projection.issues,
        })
    }
}
