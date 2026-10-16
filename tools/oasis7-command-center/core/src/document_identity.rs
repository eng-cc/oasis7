//! Explicit header identity and bounded diagnostics, independent of file keys.
use crate::document_inventory::{DocumentIdentity, IndexedDocument};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;

pub(super) fn parse(content: &str) -> DocumentIdentity {
    let mut result = DocumentIdentity {
        raw: None,
        declared_id: None,
        declaration_line: None,
        issue: None,
        duplicate_files: 0,
        duplicate_paths: Vec::new(),
    };
    let (mut depth, mut quotes) = (0, 0);
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
                let item = &content[range.clone()];
                let line = item.lines().next().unwrap_or("");
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
                if key.trim().trim_matches('*') != "文档 ID" {
                    continue;
                }
                if result.declaration_line.is_some() {
                    result.declared_id = None;
                    result.issue = Some("multiple document ID declarations".into());
                    continue;
                }
                result.declaration_line = Some(
                    content[..range.start]
                        .bytes()
                        .filter(|b| *b == b'\n')
                        .count()
                        + 1,
                );
                // Preserve a bounded raw value even when it cannot be interpreted.
                result.raw = Some(value.chars().take(512).collect());
                if value.len() > 512 {
                    result.issue = Some("document ID declaration exceeds 512 bytes".into());
                    continue;
                }
                if item.lines().skip(1).any(|line| !line.trim().is_empty()) {
                    result.issue = Some("multiline document ID declaration".into());
                    continue;
                }
                let value = value.trim();
                let id = value
                    .strip_prefix('`')
                    .and_then(|s| s.strip_suffix('`'))
                    .unwrap_or(value);
                if id.is_empty()
                    || id.len() > 256
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
                {
                    result.issue = Some("invalid document ID declaration".into());
                    continue;
                }
                result.declared_id = Some(id.into());
            }
            _ => {}
        }
    }
    result
}

pub(super) fn duplicates(files: &mut [IndexedDocument]) {
    let mut paths: HashMap<String, Vec<String>> = HashMap::new();
    for file in files.iter() {
        if let Some(id) = file.identity.as_ref().and_then(|i| i.declared_id.as_ref()) {
            paths.entry(id.clone()).or_default().push(file.path.clone());
        }
    }
    for group in paths.values_mut() {
        group.sort();
    }
    for file in files {
        if let Some(identity) = &mut file.identity
            && let Some(id) = &identity.declared_id
        {
            let group = &paths[id];
            identity.duplicate_files = group.len() - 1;
            identity.duplicate_paths = group
                .iter()
                .filter(|p| *p != &file.path)
                .take(20)
                .cloned()
                .collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignores_examples_body_nested_lists_code_and_quoted_headings() {
        let input = "# Title\n\n> ## Quoted\n> - 文档 ID：QUOTED\n\n```md\n- 文档 ID：CODE\n```\n\n- Other:\n  - 文档 ID：NESTED\n- **文档 ID**: `REAL-1`\n\n## Body\n- 文档 ID：BODY\n";
        let result = parse(input);
        assert_eq!(result.declared_id.as_deref(), Some("REAL-1"));
        assert_eq!(result.declaration_line, Some(12));
        assert!(result.issue.is_none());
        assert!(
            parse("# Title\n- Example:\n- 文档 ID：EXAMPLE\n")
                .declared_id
                .is_none()
        );
    }
    #[test]
    fn rejects_ambiguous_empty_multiline_formatted_and_oversized_ids() {
        for declaration in [
            "",
            "`bad id`",
            "**ID**",
            "[ID](path)",
            "`ID` suffix",
            "ID\n  continued",
        ] {
            let result = parse(&format!("# Title\n- 文档 ID：{declaration}\n"));
            assert!(result.declared_id.is_none(), "{declaration}");
            assert!(result.issue.is_some(), "{declaration}");
        }
        let result = parse("# Title\n- 文档 ID：ONE\n- 文档 ID：TWO\n");
        assert_eq!(result.raw.as_deref(), Some("ONE"));
        assert!(result.declared_id.is_none());
        assert_eq!(
            result.issue.as_deref(),
            Some("multiple document ID declarations")
        );
        let result = parse(&format!("# Title\n- 文档 ID：{}\n", "界".repeat(600)));
        assert_eq!(result.raw.unwrap().chars().count(), 512);
        assert!(result.declared_id.is_none());
        assert!(parse("# No ID\n").raw.is_none());
    }
}
