//! Filters explicit pairing diagnostics; never infers absence from old or unread data.
use crate::document_inventory::IndexedDocument;

pub(crate) fn valid(filter: &str) -> bool {
    matches!(
        filter,
        "all"
            | "unpaired"
            | "unparsed"
            | "missing"
            | "unread"
            | "ambiguous"
            | "outside_scope"
            | "incomplete"
    )
}
pub(crate) fn matches(file: &IndexedDocument, filter: &str) -> bool {
    if filter == "all" {
        return true;
    }
    if file.state != "read" || file.pairings.is_none() {
        return filter == "unparsed";
    }
    let group = file.pairings.as_ref().unwrap();
    match filter {
        "unpaired" => group.targets.is_empty() && group.issue.is_none(),
        "incomplete" => {
            group.issue.is_some()
                || group.targets.iter().any(|t| {
                    ![
                        "resolved",
                        "missing",
                        "missing_fragment",
                        "unread",
                        "ambiguous",
                        "outside_scope",
                    ]
                    .contains(&t.resolution.as_str())
                        || (t.resolution == "resolved"
                            && (t.target.is_none() || t.target_revision.is_none()))
                })
        }
        "missing" => group
            .targets
            .iter()
            .any(|t| matches!(t.resolution.as_str(), "missing" | "missing_fragment")),
        "unread" | "ambiguous" | "outside_scope" => {
            group.targets.iter().any(|t| t.resolution == filter)
        }
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_inventory::{DocumentPairing, DocumentPairings};
    #[test]
    fn diagnostic_classes_never_turn_unknown_or_incomplete_into_absence() {
        let mut file = IndexedDocument::new("project", "doc/prd.md", false);
        assert!(matches(&file, "unparsed"));
        assert!(!matches(&file, "unpaired"));
        file.pairings = Some(DocumentPairings {
            targets: vec![],
            issue: None,
        });
        assert!(matches(&file, "unpaired"));
        file.pairings.as_mut().unwrap().issue = Some("incomplete".into());
        assert!(matches(&file, "incomplete"));
        assert!(!matches(&file, "unpaired"));
        for (resolution, filter) in [
            ("missing", "missing"),
            ("missing_fragment", "missing"),
            ("unread", "unread"),
            ("ambiguous", "ambiguous"),
            ("outside_scope", "outside_scope"),
            ("fragment_unchecked", "incomplete"),
            ("future", "incomplete"),
        ] {
            file.pairings = Some(DocumentPairings {
                issue: None,
                targets: vec![DocumentPairing {
                    declaration_line: 3,
                    destination: "design.md".into(),
                    target: Some("doc/design.md".into()),
                    resolution: resolution.into(),
                    target_section: None,
                    target_revision: None,
                    target_kind: Some("design".into()),
                }],
            });
            assert!(matches(&file, filter));
            assert!(!matches(&file, "unpaired"));
        }
        file.pairings.as_mut().unwrap().targets[0].resolution = "resolved".into();
        assert!(matches(&file, "incomplete"));
        file.pairings.as_mut().unwrap().targets[0].target_revision = Some("rev".into());
        assert!(!matches(&file, "incomplete"));
        file.state = "deleted".into();
        assert!(matches(&file, "unparsed"));
        assert!(!matches(&file, "unpaired"));
    }
}
