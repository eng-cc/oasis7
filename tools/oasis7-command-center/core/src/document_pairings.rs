//! Explicit top-level document relationship fields. Resolution uses only the candidate manifest.
use crate::document_inventory::{DocumentPairing, DocumentPairings, IndexedDocument};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;

pub(super) fn parse(path: &str, content: &str, field_name: &str) -> DocumentPairings {
    let mut result = DocumentPairings {
        targets: Vec::new(),
        issue: None,
    };
    let mut depth = 0;
    let mut quotes = 0;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if level != HeadingLevel::H1 && depth == 0 && quotes == 0 =>
            {
                break;
            }
            Event::Start(Tag::BlockQuote(_)) => quotes += 1,
            Event::End(TagEnd::BlockQuote(_)) => quotes -= 1,
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Item) if depth == 1 && quotes == 0 => {
                let line = content[range.clone()].lines().next().unwrap_or("");
                let field = line
                    .trim_start()
                    .trim_start_matches("- ")
                    .trim_start_matches("* ");
                let Some((key, value)) = field.split_once(['：', ':']) else {
                    continue;
                };
                if ["示例", "例如", "Example", "Examples", "以下是示例", "举例"]
                    .contains(&key.trim().trim_matches('*'))
                {
                    break;
                }
                if key.trim().trim_matches('*') != field_name {
                    continue;
                }
                if content[range.clone()]
                    .lines()
                    .skip(1)
                    .any(|line| !line.trim().is_empty())
                {
                    result.issue =
                        Some("multiline pairing declaration; projection incomplete".into());
                }
                let line_number = content[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1;
                if value.len() > 4096 {
                    result.issue = Some(
                        "pairing declaration exceeds 4096 bytes; projection incomplete".into(),
                    );
                    continue;
                }
                let mut found = false;
                for event in Parser::new(value) {
                    if let Event::Start(Tag::Link { dest_url, .. }) = event {
                        found = true;
                        if result.targets.len() == 32 {
                            result.issue = Some(
                                "pairing target limit 32 exceeded; projection incomplete".into(),
                            );
                            continue;
                        }
                        let destination = dest_url.into_string();
                        let target =
                            super::lifecycle_metadata::normalize(path, &destination, false).ok();
                        result.targets.push(DocumentPairing {
                            declaration_line: line_number,
                            destination,
                            target,
                            resolution: "outside_scope".into(),
                            target_section: None,
                            target_revision: None,
                            target_kind: None,
                        });
                    }
                }
                if !found && !["无", "none", "None"].contains(&value.trim().trim_matches('`')) {
                    result.issue = Some("pairing declaration has no explicit Markdown link".into());
                }
            }
            _ => {}
        }
    }
    result
}

pub(super) fn resolve(
    files: &mut [IndexedDocument],
    sections: &[crate::domain::DocumentSection],
    contents: &HashMap<String, String>,
) {
    let states: HashMap<_, _> = files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                (file.state.clone(), file.revision.clone(), file.kind.clone()),
            )
        })
        .collect();
    let mut chapters: HashMap<&str, Vec<&crate::domain::DocumentSection>> = HashMap::new();
    for section in sections {
        chapters.entry(&section.path).or_default().push(section);
    }
    let mut fragments = HashMap::new();
    for file in files {
        let allocation_groups = file.allocations.iter_mut().flat_map(|table| {
            table
                .rows
                .iter_mut()
                .flat_map(|row| [&mut row.upstream, &mut row.design])
        });
        for pairings in [&mut file.pairings, &mut file.upstream]
            .into_iter()
            .flatten()
            .chain(allocation_groups)
        {
            for edge in &mut pairings.targets {
                let Some(target) = &edge.target else { continue };
                let (path, fragment) = target
                    .split_once('#')
                    .map_or((target.as_str(), None), |(p, f)| (p, Some(f)));
                edge.target_kind = states.get(path).map(|(_, _, kind)| kind.clone());
                edge.resolution = match states.get(path).map(|(state, _, _)| state.as_str()) {
                    Some("read") => "resolved",
                    Some("deleted") => "missing",
                    None => "outside_scope",
                    Some(_) => "unread",
                }
                .into();
                if edge.resolution != "resolved" {
                    continue;
                }
                edge.target_revision = states[path].1.clone();
                let candidates = chapters.get(path).map(Vec::as_slice).unwrap_or(&[]);
                let selected = if let Some(fragment) = fragment {
                    let catalog = fragments
                        .entry(path.to_owned())
                        .or_insert_with(|| super::document_fragments::parse(&contents[path]));
                    match catalog.get(fragment).map(Vec::as_slice).unwrap_or(&[]) {
                        [] => {
                            edge.resolution = "missing_fragment".into();
                            None
                        }
                        [location] => {
                            // A standalone anchor adjacent to a heading addresses that heading,
                            // while inline/table anchors address their containing chapter.
                            let next = candidates.iter().copied().find(|s| s.line > location.line);
                            next.filter(|s| {
                                location.standalone
                                    && contents[path]
                                        .lines()
                                        .skip(location.line)
                                        .take(s.line - location.line - 1)
                                        .all(|l| {
                                            l.trim().is_empty()
                                                || l.trim().eq_ignore_ascii_case("</a>")
                                                || super::document_fragments::anchor_only(l)
                                        })
                            })
                            .or_else(|| {
                                candidates
                                    .iter()
                                    .copied()
                                    .rev()
                                    .find(|s| s.line <= location.line)
                            })
                        }
                        _ => {
                            edge.resolution = "ambiguous".into();
                            None
                        }
                    }
                } else {
                    candidates.first().copied()
                };
                if let Some(section) = selected {
                    edge.target_section = Some(section.id.clone());
                    edge.target_revision = Some(section.revision.clone());
                }
            }
        }
    }
}
