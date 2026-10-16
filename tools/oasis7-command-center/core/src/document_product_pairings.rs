//! Exact product identity declarations, separate from prose and tree navigation.
use crate::document_inventory::DocumentPairings;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
pub(super) fn append(result: &mut DocumentPairings, path: &str, content: &str) {
    let mut active = false;
    let mut direct = false;
    let mut depth = 0;
    let mut blocked = 0;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::HtmlBlock) => blocked += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::BlockQuote(_) | TagEnd::HtmlBlock) => {
                blocked -= 1
            }
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Heading { level, .. }) if depth == 0 && blocked == 0 => {
                if active && level <= HeadingLevel::H2 {
                    break;
                }
                let title = Parser::new(&content[range])
                    .filter_map(|event| match event {
                        Event::Text(t) | Event::Code(t) => Some(t.into_string()),
                        _ => None,
                    })
                    .collect::<String>();
                if level == HeadingLevel::H2 && title.trim() == "文档身份" {
                    active = true;
                    direct = true;
                } else if active {
                    direct = false;
                }
            }
            Event::Start(Tag::Item) if active && direct && depth == 1 && blocked == 0 => {
                let raw = &content[range.clone()];
                let field = raw
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim_start()
                    .trim_start_matches("- ")
                    .trim_start_matches("* ");
                let Some((key, _)) = field.split_once(['：', ':']) else {
                    continue;
                };
                let key = key.trim().trim_matches('*');
                if ["示例", "例如", "Example", "Examples", "以下是示例", "举例"].contains(&key)
                {
                    break;
                }
                if !["配对产品 design", "配对产品设计"].contains(&key) {
                    continue;
                }
                let mut parsed = super::parse_header(path, raw, key);
                let line_offset = content[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count();
                for edge in &mut parsed.targets {
                    edge.declaration_line += line_offset;
                }
                if let Some(issue) = parsed.issue {
                    append_issue(result, &issue);
                }
                let remaining = 32usize.saturating_sub(result.targets.len());
                if parsed.targets.len() > remaining {
                    append_issue(
                        result,
                        "pairing target limit 32 exceeded; projection incomplete",
                    );
                }
                result
                    .targets
                    .extend(parsed.targets.into_iter().take(remaining));
            }
            _ => {}
        }
    }
}
fn append_issue(result: &mut DocumentPairings, issue: &str) {
    if let Some(old) = &mut result.issue {
        if !old.contains(issue) {
            old.push_str("; ");
            old.push_str(issue);
        }
    } else {
        result.issue = Some(issue.into());
    }
}
#[cfg(test)]
mod tests {
    use super::super::parse;
    const PATH: &str = "doc/product/agents-world-simulation/external-agent-runtime-play.prd.md";
    #[test]
    fn actual_product_identity_fields_merge_header_and_preserve_source_lines() {
        let source = "# 外部 Agent Runtime 接入与持续游玩\n- 配对系统设计：[legacy](legacy.design.md)\n\n## 文档身份\n\n- 产品模块：智能体、世界模拟与交互\n- 配对产品 design：[external-agent-runtime-play.design.md](external-agent-runtime-play.design.md)\n- 配对产品设计：[alternate](alternate.design.md)\n\n## 产品目标\n- 配对产品 design：[body](body.design.md)\n";
        let result = parse(PATH, source, "配对系统设计");
        assert!(result.issue.is_none());
        assert_eq!(
            result
                .targets
                .iter()
                .map(|e| e.declaration_line)
                .collect::<Vec<_>>(),
            [2, 7, 8]
        );
        assert_eq!(
            result.targets[1].target.as_deref(),
            Some("doc/product/agents-world-simulation/external-agent-runtime-play.design.md")
        );
        assert!(
            parse("doc/engineering/topic.prd.md", source, "配对系统设计")
                .targets
                .len()
                == 1
        );
        assert!(
            parse(
                "doc/product/agents-world-simulation/topic.design.md",
                source,
                "配对系统设计"
            )
            .targets
            .len()
                == 1
        );
    }
    #[test]
    fn example_quote_code_nested_list_and_body_never_declare_product_pairing() {
        let source = "# PRD\n> ## 文档身份\n> - 配对产品 design：[quoted](quoted.design.md)\n\n```md\n## 文档身份\n- 配对产品 design：[code](code.design.md)\n```\n\n## 文档身份\n- parent\n  - 配对产品 design：[nested](nested.design.md)\n- 示例：example follows\n- 配对产品 design：[example](example.design.md)\n## Body\n- 配对产品 design：[body](body.design.md)\n";
        assert!(parse(PATH, source, "配对系统设计").targets.is_empty());
        let subsection =
            "# PRD\n## 文档身份\n### 示例\n- 配对产品 design：[example](example.design.md)\n";
        assert!(parse(PATH, subsection, "配对系统设计").targets.is_empty());
    }
    #[test]
    fn merged_pairing_budget_and_metadata_value_bound_are_shared() {
        let header = (0..31)
            .map(|i| format!("[d{i}](d{i}.design.md)"))
            .collect::<Vec<_>>()
            .join(" ");
        let source = format!(
            "# PRD\n- 配对系统设计：{header}\n## 文档身份\n- 配对产品 design：[a](a.design.md) [b](b.design.md)\n"
        );
        let result = parse(PATH, &source, "配对系统设计");
        assert_eq!(result.targets.len(), 32);
        assert!(result.issue.unwrap().contains("limit 32"));
        let source = format!(
            "# PRD\n## 文档身份\n- 配对产品 design：{} [a](a.design.md)\n",
            "x".repeat(4096)
        );
        let result = parse(PATH, &source, "配对系统设计");
        assert!(result.targets.is_empty());
        assert!(result.issue.unwrap().contains("4096"));
    }
}
