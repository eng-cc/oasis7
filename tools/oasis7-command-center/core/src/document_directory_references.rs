//! Same-inventory explicit references only; never follows or reads a source link.
use crate::document_inventory::IndexedDocument;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn is_history(file: &IndexedDocument) -> bool {
    matches!(
        file.lifecycle.as_ref().and_then(|l| l.category.as_deref()),
        Some("superseded" | "retired")
    )
}

pub(crate) fn historical_references(
    files: &[&IndexedDocument],
) -> BTreeMap<String, BTreeSet<String>> {
    let historical: BTreeMap<_, _> = files
        .iter()
        .copied()
        .filter(|f| f.state == "read" && is_history(f))
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut references: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in files
        .iter()
        .copied()
        .filter(|f| f.state == "read" && !is_history(f))
    {
        let header = source.pairings.iter().chain(source.upstream.iter());
        let rows = source
            .allocations
            .iter()
            .flat_map(|a| &a.rows)
            .flat_map(|r| [&r.upstream, &r.design]);
        for link in header.chain(rows).flat_map(|p| &p.targets) {
            if link.resolution != "resolved" {
                continue;
            }
            let Some(target) = link
                .target
                .as_deref()
                .and_then(|p| historical.get(p.split('#').next().unwrap_or(p)))
            else {
                continue;
            };
            // Legacy locators or foreign revisions cannot justify a current placeholder.
            if target.revision.is_none() || link.target_revision != target.revision {
                continue;
            }
            references
                .entry(target.path.clone())
                .or_default()
                .insert(source.path.clone());
        }
    }
    references
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_inventory::{DocumentPairing, DocumentPairings};
    #[test]
    fn stale_or_legacy_links_never_become_history_placeholders() {
        let mut target = IndexedDocument::new("project", "doc/old.design.md", false);
        target.revision = Some("actual".into());
        target.lifecycle = Some(serde_json::from_value(serde_json::json!({
            "status":"retired", "rawStatus":"retired", "category":"retired",
            "successorDeclaration":null,"successorTargets":[],"successorReason":null,"issue":null
        })).unwrap());
        let mut source = IndexedDocument::new("project", "doc/current.design.md", false);
        for (resolution, revision) in [
            ("fragment_unchecked", Some("actual")),
            ("resolved", None),
            ("resolved", Some("foreign")),
            ("ambiguous", Some("actual")),
        ] {
            source.upstream = Some(DocumentPairings {
                issue: None,
                targets: vec![DocumentPairing {
                    declaration_line: 3,
                    destination: "old.design.md".into(),
                    target: Some(target.path.clone()),
                    resolution: resolution.into(),
                    target_section: None,
                    target_revision: revision.map(str::to_owned),
                    target_kind: Some("design".into()),
                }],
            });
            assert!(historical_references(&[&source, &target]).is_empty());
        }
    }
}
