//! Ephemeral fragment locations from already-read Markdown. No source IO.
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::HashMap;

pub(super) struct Location {
    pub line: usize,
    pub standalone: bool,
}

pub(super) fn parse(content: &str) -> HashMap<String, Vec<Location>> {
    let mut result: HashMap<String, Vec<Location>> = HashMap::new();
    let mut heading = None;
    let mut html_block = None;
    let mut raw_inline: Option<String> = None;
    // Offset-to-line mapping avoids repeated scans of the whole prefix.
    let starts: Vec<_> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    for (event, range) in Parser::new(content).into_offset_iter() {
        if let Some(tag) = &raw_inline {
            if let Event::InlineHtml(value) = &event
                && closes(value.trim(), tag)
            {
                raw_inline = None;
            }
            continue;
        }
        match event {
            Event::Start(Tag::Heading { .. }) => heading = Some((range.start, String::new())),
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, title)) = &mut heading {
                    title.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, title)) = &mut heading {
                    title.push(' ');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((offset, title)) = heading.take() {
                    let slug = slug(&title);
                    if !slug.is_empty() {
                        result.entry(slug).or_default().push(Location {
                            line: starts.partition_point(|start| *start <= offset),
                            standalone: false,
                        });
                    }
                }
            }
            Event::Start(Tag::HtmlBlock) => html_block = Some(range),
            Event::End(TagEnd::HtmlBlock) => {
                if let Some(range) = html_block.take() {
                    html(
                        &content[range.clone()],
                        range.start,
                        content,
                        &starts,
                        &mut result,
                    );
                }
            }
            Event::InlineHtml(value) => {
                if let Some(tag) = value
                    .strip_prefix('<')
                    .and_then(|v| v.split([' ', '>', '\t', '\n']).next())
                    .filter(|t| raw_text(t))
                {
                    raw_inline = Some(tag.to_ascii_lowercase());
                } else {
                    html(&value, range.start, content, &starts, &mut result);
                }
            }
            _ => {}
        }
    }
    result
}

fn slug(title: &str) -> String {
    // The governance heading convention removes formatting, punctuation and
    // keeps BMP non-ASCII text. Duplicate slugs remain ambiguous, without suffixes.
    let normalized: String = title
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| {
            !"`*_~".contains(*c)
                && (c.is_alphanumeric()
                    || *c == '-'
                    || *c == ' '
                    || ('\u{0080}'..='\u{ffff}').contains(c))
        })
        .collect();
    normalized.split_whitespace().collect::<Vec<_>>().join("-")
}

fn html(
    value: &str,
    offset: usize,
    content: &str,
    starts: &[usize],
    result: &mut HashMap<String, Vec<Location>>,
) {
    let mut remaining = value;
    while let Some(open) = remaining.find('<') {
        let local = value.len() - remaining.len() + open;
        remaining = &remaining[open..];
        if remaining.starts_with("<!--") {
            let Some(end) = remaining.find("-->") else {
                break;
            };
            remaining = &remaining[end + 3..];
            continue;
        }
        let Some(end) = tag_end(remaining) else {
            break;
        };
        let tag = &remaining[1..end];
        // HTML raw-text elements do not create document anchors from their text.
        if let Some(name) = tag.split_whitespace().next().filter(|name| raw_text(name)) {
            let Some(close) = raw_end(remaining, name) else {
                break;
            };
            remaining = &remaining[close..];
            continue;
        }
        if let Some(attrs) = tag.strip_prefix('a').or_else(|| tag.strip_prefix('A'))
            && attrs.starts_with(char::is_whitespace)
            && let Some(id) = attribute_id(attrs)
        {
            let absolute = offset + local;
            let line = starts.partition_point(|start| *start <= absolute);
            let source_line = content
                [starts[line - 1]..starts.get(line).copied().unwrap_or(content.len())]
                .trim();
            let standalone = anchor_only(source_line);
            result
                .entry(id.to_lowercase())
                .or_default()
                .push(Location { line, standalone });
        }
        remaining = &remaining[end + 1..];
    }
}

fn attribute_id(mut attrs: &str) -> Option<String> {
    while !attrs.trim_start().is_empty() {
        attrs = attrs.trim_start();
        let end = attrs
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(attrs.len());
        if end == 0 {
            return None;
        }
        let key = &attrs[..end];
        attrs = attrs[end..].trim_start();
        if !attrs.starts_with('=') {
            continue;
        }
        attrs = attrs[1..].trim_start();
        let quote = attrs.chars().next()?;
        if quote != '\'' && quote != '"' {
            let end = attrs.find(char::is_whitespace).unwrap_or(attrs.len());
            attrs = &attrs[end..];
            continue;
        }
        attrs = &attrs[1..];
        let end = attrs.find(quote)?;
        let value = &attrs[..end];
        attrs = &attrs[end + 1..];
        if key.eq_ignore_ascii_case("id") {
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }
    None
}

fn tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (position, c) in tag.char_indices().skip(1) {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c == '>' => return Some(position),
            None => {}
        }
    }
    None
}

/// Whether a line consists solely of explicit anchor tags (possibly aliases).
pub(super) fn anchor_only(mut line: &str) -> bool {
    let mut count = 0;
    loop {
        line = line.trim();
        if line.is_empty() {
            return count > 0;
        }
        let Some(end) = tag_end(line) else {
            return false;
        };
        if !line.starts_with('<') {
            return false;
        }
        let tag = &line[1..end];
        if tag.eq_ignore_ascii_case("/a") {
            line = &line[end + 1..];
            continue;
        }
        let Some(attrs) = tag.strip_prefix('a').or_else(|| tag.strip_prefix('A')) else {
            return false;
        };
        if !attrs.starts_with(char::is_whitespace) || attribute_id(attrs).is_none() {
            return false;
        }
        count += 1;
        line = &line[end + 1..];
    }
}

fn raw_text(tag: &str) -> bool {
    [
        "script",
        "style",
        "pre",
        "textarea",
        "title",
        "xmp",
        "iframe",
        "noembed",
        "noframes",
        "plaintext",
    ]
    .contains(&tag.to_ascii_lowercase().as_str())
}

fn closes(value: &str, name: &str) -> bool {
    value
        .strip_prefix("</")
        .and_then(|v| v.strip_suffix('>'))
        .is_some_and(|v| v.trim_end().eq_ignore_ascii_case(name))
}

fn raw_end(mut value: &str, name: &str) -> Option<usize> {
    let original_len = value.len();
    while let Some(start) = value.find("</") {
        value = &value[start..];
        let end = tag_end(value)?;
        if closes(&value[..end + 1], name) {
            return Some(original_len - value.len() + end + 1);
        }
        value = &value[end + 1..];
    }
    None
}
