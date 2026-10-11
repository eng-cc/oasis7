use crate::domain::DocumentLifecycle;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashSet;

const FIELD_LIMIT: usize = 4096;
fn bounded(value: &str) -> String {
    if value.len() <= FIELD_LIMIT {
        return value.to_owned();
    }
    let mut end = FIELD_LIMIT - 40;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [metadata exceeds 4096 bytes]", &value[..end])
}
fn append_issue(issue: &mut Option<String>, message: &str) {
    if let Some(old) = issue {
        if !old.contains(message) {
            *old = bounded(&format!("{old}; {message}"));
        }
    } else {
        *issue = Some(message.into());
    }
}
fn store(values: &mut Vec<String>, value: &str, issue: &mut Option<String>) {
    if value.len() > FIELD_LIMIT {
        append_issue(issue, "metadata exceeds 4096 bytes; projection incomplete");
    }
    let value = bounded(value);
    if !values.contains(&value) {
        if values.len() < 2 {
            values.push(value);
        } else {
            append_issue(issue, "too many conflicting metadata declarations");
        }
    }
}
fn raw(values: &[String]) -> Option<String> {
    (!values.is_empty()).then(|| bounded(&values.join(" | ")))
}

pub fn parse(path: &str, content: &str) -> Option<DocumentLifecycle> {
    let events: Vec<_> = Parser::new(content).into_offset_iter().collect();
    let headings: Vec<_> = events
        .iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Heading { level, .. }) => Some((*level, range.clone())),
            _ => None,
        })
        .collect();
    let h2: Vec<_> = headings
        .iter()
        .filter(|(level, _)| *level == HeadingLevel::H2)
        .collect();
    let (start, end) = if let Some((_, range)) = h2.first() {
        let title = content[range.clone()].trim().trim_start_matches('#').trim();
        if ["文档身份", "Document identity"].contains(&title) {
            (range.end, h2.get(1).map_or(content.len(), |(_, r)| r.start))
        } else {
            (0, range.start)
        }
    } else {
        (0, content.len())
    };
    let blocked: Vec<_> = events
        .iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::HtmlBlock) => {
                Some(range.clone())
            }
            _ => None,
        })
        .collect();
    let mut statuses = Vec::new();
    let mut successors = Vec::new();
    let mut reasons = Vec::new();
    let mut issue = None;
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        let position = offset;
        offset += line.len();
        if position < start || position >= end || blocked.iter().any(|r| r.contains(&position)) {
            continue;
        }
        // Identity rows are top-level source declarations; nested example lists
        // cannot supply lifecycle authority.
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let line = line.trim();
        let example = line
            .trim_start_matches('#')
            .trim_start_matches("- ")
            .trim_start_matches("* ")
            .trim();
        if example.to_ascii_lowercase().starts_with("example")
            || ["示例", "例如", "举例", "以下是示例", "Example", "Examples"]
                .iter()
                .any(|label| example.starts_with(label))
            || headings
                .iter()
                .any(|(level, range)| *level != HeadingLevel::H1 && range.start == position)
        {
            break;
        }
        if line.starts_with('>') {
            continue;
        }
        let field = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .unwrap_or(line);
        let Some((key, value)) = field.split_once('：').or_else(|| field.split_once(':')) else {
            continue;
        };
        let key = key.trim().trim_matches('*');
        let value = value.trim();
        if ["示例", "例如", "Example", "Examples", "以下是示例", "举例"].contains(&key) {
            break;
        }
        match key {
            "生命周期" | "状态" => {
                store(&mut statuses, value.trim_matches('`').trim(), &mut issue)
            }
            "后继文档" | "superseded_by" => store(&mut successors, value, &mut issue),
            "替代理由" | "后继理由" => store(&mut reasons, value, &mut issue),
            _ => {}
        }
    }
    if statuses.is_empty() && successors.is_empty() && reasons.is_empty() {
        return None;
    }
    let conflict = statuses.len() > 1 || successors.len() > 1 || reasons.len() > 1;
    if conflict {
        append_issue(
            &mut issue,
            "conflicting explicit metadata declarations; no successor projection",
        );
    }
    let status = if statuses.len() > 1 {
        Some("unknown".into())
    } else {
        raw(&statuses)
    };
    let successor_declaration = raw(&successors);
    let successor_reason = raw(&reasons);
    let mut successor_targets = Vec::new();
    if !conflict
        && issue.is_none()
        && let Some(value) = successors.first()
    {
        match targets(path, value) {
            Ok(targets) => successor_targets = targets,
            Err(error) => append_issue(&mut issue, &error),
        }
    }
    Some(DocumentLifecycle {
        status,
        successor_declaration,
        successor_targets,
        successor_reason,
        issue,
    })
}

fn targets(source: &str, value: &str) -> Result<Vec<String>, String> {
    if ["无", "none", "None"].contains(&value.trim().trim_matches('`')) {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let mut links = 0;
    for (event, range) in Parser::new(value).into_offset_iter() {
        let candidate = match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links += 1;
                Some((dest_url.into_string(), false))
            }
            Event::End(TagEnd::Link) => {
                links -= 1;
                None
            }
            Event::Code(path) if links == 0 => {
                let syntax = &value[range];
                if syntax.starts_with("``") {
                    return Err("successor paths require single backticks".into());
                }
                Some((path.into_string(), true))
            }
            _ => None,
        };
        if let Some((target, code)) = candidate {
            let target = normalize(source, &target, code)?;
            if !result.contains(&target) {
                result.push(target);
            }
            if result.len() > 8 {
                return Err("successor target bound exceeds 8; no partial projection".into());
            }
        }
    }
    if result.is_empty() {
        return Err(
            "successor declaration has no explicit local Markdown link or single-backtick path"
                .into(),
        );
    }
    Ok(result)
}
fn decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = value
                .get(i + 1..i + 3)
                .ok_or("invalid percent-encoded successor path")?;
            result.push(
                u8::from_str_radix(hex, 16)
                    .map_err(|_| "invalid percent-encoded successor path")?,
            );
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result).map_err(|_| "invalid UTF-8 successor path".into())
}
fn normalize(source: &str, target: &str, code: bool) -> Result<String, String> {
    let (path, fragment) = target
        .split_once('#')
        .map_or((target, None), |(p, f)| (p, Some(f)));
    let path = decode(path)?;
    if path.starts_with('/')
        || path.contains(':')
        || path.contains('\\')
        || path.contains('#')
        || path.chars().any(char::is_control)
    {
        return Err("successor target is external, absolute, or unsafe".into());
    }
    if fragment.is_some_and(|f| f.chars().any(|c| c.is_control() || c.is_whitespace())) {
        return Err("unsafe successor fragment".into());
    }
    let parent = source.rsplit_once('/').map_or("", |(p, _)| p);
    let mut parts: Vec<&str> = if path.is_empty() {
        source.split('/').collect()
    } else if code && (path.starts_with("doc/") || path == "AGENTS.md") {
        Vec::new()
    } else {
        parent.split('/').filter(|p| !p.is_empty()).collect()
    };
    if !path.is_empty() {
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop().ok_or("successor path escapes repository")?;
                }
                _ => parts.push(part),
            }
        }
    }
    let path = parts.join("/");
    if !(path.starts_with("doc/") || path == "AGENTS.md") || !path.ends_with(".md") {
        return Err("successor target is outside indexed document scope".into());
    }
    Ok(match fragment.filter(|f| !f.is_empty()) {
        Some(f) => format!("{path}#{f}"),
        None => path,
    })
}
pub fn validate_indexed(metadata: &mut DocumentLifecycle, indexed: &HashSet<String>) {
    if metadata
        .successor_targets
        .iter()
        .any(|target| !indexed.contains(target.split('#').next().unwrap_or(target)))
    {
        metadata.successor_targets.clear();
        append_issue(
            &mut metadata.issue,
            "successor target is missing or outside current tracked index coverage",
        );
    }
}
