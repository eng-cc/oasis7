//! Explicit top-level pairing fields. Resolution uses only the candidate manifest.
use crate::document_inventory::{DocumentPairing, DocumentPairings, IndexedDocument};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;

pub(super) fn parse(path: &str, content: &str) -> DocumentPairings {
    let mut result = DocumentPairings {
        targets: Vec::new(),
        issue: None,
    };
    let mut depth = 0;
    let mut quotes = 0;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) if level != HeadingLevel::H1 => break,
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
                let Some((key, value)) = field.split_once('：').or_else(|| field.split_once(':'))
                else {
                    continue;
                };
                if ["示例", "例如", "Example", "Examples", "以下是示例", "举例"]
                    .contains(&key.trim().trim_matches('*'))
                {
                    break;
                }
                if key.trim().trim_matches('*') != "配对系统设计" {
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

pub(super) fn resolve(files: &mut [IndexedDocument]) {
    let states: HashMap<_, _> = files
        .iter()
        .map(|file| (file.path.clone(), file.state.clone()))
        .collect();
    for file in files {
        if let Some(pairings) = &mut file.pairings {
            for edge in &mut pairings.targets {
                let Some(target) = &edge.target else { continue };
                let (path, fragment) = target
                    .split_once('#')
                    .map_or((target.as_str(), false), |(p, _)| (p, true));
                edge.resolution = match states.get(path).map(String::as_str) {
                    Some("read") if fragment => "fragment_unchecked",
                    Some("read") => "resolved",
                    Some("deleted") => "missing",
                    None => "outside_scope",
                    Some(_) => "unread",
                }
                .into();
            }
        }
    }
}
