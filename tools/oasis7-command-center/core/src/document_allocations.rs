//! Only explicitly named top-level allocation sections, not arbitrary link tables.
use crate::document_inventory::{
    DocumentAllocation, DocumentAllocations, DocumentPairing, DocumentPairings,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

fn group() -> DocumentPairings {
    DocumentPairings {
        targets: Vec::new(),
        issue: None,
    }
}

pub(super) fn parse(path: &str, content: &str) -> DocumentAllocations {
    let mut result = DocumentAllocations {
        rows: Vec::new(),
        issue: None,
    };
    let (mut depth, mut quotes) = (0, 0);
    let (mut heading, mut title, mut scope) = (false, String::new(), false);
    let (mut declared, mut tables, mut links) = (false, 0, 0);
    let events: Vec<_> = Parser::new_ext(content, Options::ENABLE_TABLES)
        .into_offset_iter()
        .collect();
    for (index, (event, _range)) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => quotes += 1,
            Event::End(TagEnd::BlockQuote(_)) => quotes -= 1,
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Heading { .. }) if depth == 0 && quotes == 0 => {
                heading = true;
                title.clear();
                scope = false;
            }
            Event::Text(text) | Event::Code(text) if heading => title.push_str(text),
            Event::SoftBreak | Event::HardBreak if heading => title.push(' '),
            Event::End(TagEnd::Heading(_)) if heading => {
                heading = false;
                scope = title
                    .trim()
                    .strip_suffix("需求承接与分配表")
                    .is_some_and(|prefix| {
                        (prefix.trim().is_empty() || prefix.chars().any(|c| c.is_ascii_digit()))
                            && prefix
                                .chars()
                                .all(|c| c.is_ascii_digit() || c == '.' || c.is_whitespace())
                    });
                declared |= scope;
            }
            Event::Start(Tag::Table(_)) if scope && depth == 0 && quotes == 0 => {
                tables += 1;
                let end = events[index..]
                    .iter()
                    .position(|(event, _)| matches!(event, Event::End(TagEnd::Table)))
                    .unwrap_or(events.len() - index - 1)
                    + index;
                parse_table(path, content, &events[index..=end], &mut result, &mut links);
            }
            _ => {}
        }
    }
    if declared && tables == 0 {
        result.issue = Some("allocation section has no table".into());
    }
    result
}

fn parse_table(
    path: &str,
    content: &str,
    events: &[(Event<'_>, std::ops::Range<usize>)],
    result: &mut DocumentAllocations,
    links: &mut usize,
) {
    let mut headers = Vec::new();
    let (mut head, mut column) = (false, 0usize);
    let (mut upstream, mut design) = (None, None);
    let mut current: Option<DocumentAllocation> = None;
    let mut cell_text = String::new();
    let mut oversized = false;
    // Reuse full-source events so reference links retain definitions without reparsing.
    for (event, offset) in events.iter().cloned() {
        match event {
            Event::Start(Tag::TableHead) => {
                head = true;
                column = 0;
            }
            Event::End(TagEnd::TableHead) => {
                head = false;
                let indices = |name: &str| {
                    headers
                        .iter()
                        .enumerate()
                        .filter_map(|(i, h): (usize, &String)| (h.trim() == name).then_some(i))
                        .collect::<Vec<_>>()
                };
                let u = indices("上游条款");
                let d = indices("本设计条款");
                if u.len() != 1 || d.len() != 1 {
                    result.issue =
                        Some("allocation table requires unique upstream and design columns".into());
                    return;
                }
                upstream = Some(u[0]);
                design = Some(d[0]);
            }
            Event::Start(Tag::TableRow) => {
                column = 0;
                if result.rows.len() == 32 {
                    result.issue = Some("allocation row limit 32 exceeded".into());
                    return;
                }
                oversized = offset.len() > 4096;
                let row = &content[offset.clone()];
                current = Some(DocumentAllocation {
                    declaration_line: content[..offset.start]
                        .bytes()
                        .filter(|b| *b == b'\n')
                        .count()
                        + 1,
                    raw_sample: row.chars().take(512).collect(),
                    upstream: group(),
                    design: group(),
                });
                if oversized {
                    result.issue = Some("allocation row exceeds 4096 bytes".into());
                }
            }
            Event::Start(Tag::TableCell) => cell_text.clear(),
            Event::Text(text) | Event::Code(text) => cell_text.push_str(&text),
            Event::Start(Tag::Link { dest_url, .. }) if !head && !oversized => {
                if let Some(row) = &mut current {
                    let group = if Some(column) == upstream {
                        Some(&mut row.upstream)
                    } else if Some(column) == design {
                        Some(&mut row.design)
                    } else {
                        None
                    };
                    if let Some(group) = group {
                        if *links == 64 {
                            result.issue = Some("allocation link limit 64 exceeded".into());
                            group.issue = Some("allocation link limit 64 exceeded".into());
                            continue;
                        }
                        *links += 1;
                        let destination = dest_url.into_string();
                        let target =
                            super::lifecycle_metadata::normalize(path, &destination, false).ok();
                        group.targets.push(DocumentPairing {
                            declaration_line: row.declaration_line,
                            destination,
                            target,
                            resolution: "outside_scope".into(),
                            target_section: None,
                            target_revision: None,
                            target_kind: None,
                        });
                    }
                }
            }
            Event::End(TagEnd::TableCell) => {
                if head {
                    headers.push(cell_text.clone());
                } else if let Some(row) = &mut current {
                    let group = if Some(column) == upstream {
                        Some(&mut row.upstream)
                    } else if Some(column) == design {
                        Some(&mut row.design)
                    } else {
                        None
                    };
                    if let Some(group) = group {
                        if oversized {
                            group.issue = Some("allocation row exceeds 4096 bytes".into());
                        } else if group.targets.is_empty()
                            && group.issue.is_none()
                            && !["无", "none", "None"].contains(&cell_text.trim())
                        {
                            group.issue =
                                Some("allocation cell has no explicit Markdown link".into());
                        }
                    }
                }
                column += 1;
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(row) = current.take() {
                    result.rows.push(row);
                }
            }
            _ => {}
        }
    }
}
