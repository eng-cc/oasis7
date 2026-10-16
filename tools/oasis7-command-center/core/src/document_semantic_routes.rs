//! Stable explicit-edge identity. Source line, title and numeric occurrence are excluded.
use super::{Node, Projection};
use crate::document_tree_reading::DocumentTreeRouteStep;
use std::collections::{BTreeSet, HashMap, HashSet};
pub(super) fn steps(nodes: &[Node]) -> Vec<DocumentTreeRouteStep> {
    let bases: Vec<_> = nodes
        .iter()
        .map(|node| DocumentTreeRouteStep {
            path: node.path.clone(),
            source_path: node.source.clone(),
            destination: node.edge.as_ref().map(|edge| edge.destination.clone()),
            group: node.group_key.clone(),
            duplicate_index: 0,
            duplicate_count: 0,
        })
        .collect();
    let mut counts = HashMap::new();
    for base in &bases {
        *counts.entry(base.clone()).or_insert(0) += 1;
    }
    let mut occurrences = HashMap::new();
    bases
        .into_iter()
        .map(|base| {
            let occurrence = occurrences.entry(base.clone()).or_insert(0);
            let mut step = base.clone();
            step.duplicate_index = *occurrence;
            step.duplicate_count = counts[&base];
            *occurrence += 1;
            step
        })
        .collect()
}
impl Projection<'_> {
    pub fn resolve_route(
        &self,
        route: &[DocumentTreeRouteStep],
        snapshot: &str,
        view: &str,
        matching: &BTreeSet<String>,
    ) -> (String, Option<String>) {
        let mut nodes = self.roots.clone();
        let mut indices = Vec::new();
        let mut ancestors = HashSet::new();
        let mut notice = None;
        let mut selected = None;
        for requested in route {
            if requested.duplicate_count > 1 {
                notice = Some("saved navigation declaration is ambiguous; restored nearest confirmed ancestor without guessing".into());
                break;
            }
            let identities = steps(&nodes);
            let Some(index) = identities.iter().position(|step| step == requested) else {
                notice = Some("saved navigation target was removed or its explicit route changed; restored nearest confirmed ancestor without title matching".into());
                break;
            };
            let node = &nodes[index];
            if node.path.as_ref().is_some_and(|path| {
                self.files
                    .get(path.as_str())
                    .is_none_or(|file| file.state == "deleted")
            }) || node.edge.as_ref().is_some_and(|edge| {
                matches!(
                    edge.resolution.as_str(),
                    "missing" | "missing_fragment" | "ambiguous" | "outside_scope"
                )
            }) {
                notice = Some("saved navigation target is deleted, unresolved or ambiguous; restored nearest confirmed ancestor without redirecting to another file".into());
                break;
            }
            if node
                .path
                .as_ref()
                .is_some_and(|path| !ancestors.insert(path.clone()))
            {
                notice = Some(
                    "saved navigation reaches a cycle; restored nearest confirmed ancestor".into(),
                );
                break;
            }
            if indices.len() >= 63 {
                notice = Some(
                    "saved navigation reached depth budget; restored nearest confirmed ancestor"
                        .into(),
                );
                break;
            }
            indices.push(index.to_string());
            selected = Some(node.clone());
            nodes = self.next(node);
        }
        if notice.is_none()
            && selected.as_ref().is_some_and(|node| {
                node.boundary.is_none()
                    && self
                        .reachable(std::slice::from_ref(node))
                        .is_disjoint(matching)
            })
        {
            notice = Some("saved navigation still exists but is hidden by the applied filters; this is not a deleted document".into());
        }
        let occurrence = if indices.is_empty() {
            String::new()
        } else {
            format!("{snapshot}-{view}:{}", indices.join("/"))
        };
        (occurrence, notice)
    }
}
