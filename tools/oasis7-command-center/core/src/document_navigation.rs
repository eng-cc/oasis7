//! Explicit navigation sections only; ordinary prose and integration links are excluded.
use crate::document_inventory::{DocumentNavigation, DocumentPairing, DocumentPairings};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

pub(super) fn parse(path: &str, content: &str) -> DocumentNavigation {
    let mut result = DocumentNavigation {
        relationships: DocumentPairings {
            targets: vec![],
            issue: None,
        },
        labels: vec![],
    };
    let product_root = path == "doc/product/README.md";
    let product = path.starts_with("doc/product/");
    let module_root = [
        "world-rules-core-gameplay",
        "world-infrastructure",
        "agents-world-simulation",
        "player-entry-distribution",
    ]
    .iter()
    .any(|slug| path == format!("doc/product/{slug}/prd.md"));
    let matrix = path == "doc/README.md";
    let index = path.ends_with("/prd.index.md");
    let landing = path.ends_with("/README.md");
    let design = path.ends_with("/design.md");
    let mut active = false;
    let mut active_level = 0;
    let mut in_link = false;
    let mut blocked = 0;
    let mut list_depth = 0;
    for (event, range) in Parser::new_ext(content, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::HtmlBlock) => blocked += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::BlockQuote(_) | TagEnd::HtmlBlock) => {
                blocked -= 1
            }
            Event::Start(Tag::Heading { level, .. }) if blocked == 0 && list_depth == 0 => {
                let title = Parser::new(&content[range.clone()])
                    .filter_map(|event| match event {
                        Event::Text(t) | Event::Code(t) => Some(t.into_string()),
                        _ => None,
                    })
                    .collect::<String>();
                let title = title.trim();
                let selected = if product_root {
                    title == "四大产品模块"
                } else if product {
                    module_root && title == "活跃产品专题"
                } else if matrix {
                    title == "工程模块入口矩阵"
                } else if index {
                    true
                } else if landing {
                    [
                        "入口",
                        "从这里开始",
                        "按主题进入",
                        "专题导航",
                        "目录导航",
                        "文档导航",
                        "目录",
                        "导航",
                        "子目录",
                    ]
                    .contains(&title)
                } else if design {
                    title.contains("专题导航")
                        || title.contains("阅读顺序")
                        || title == "关键接口 / 入口"
                } else {
                    false
                };
                if selected {
                    active = true;
                    active_level = level as usize;
                } else if level as usize <= active_level {
                    active = false;
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) if active && blocked == 0 => {
                in_link = true;
                let raw = &content[range.clone()];
                let label = raw
                    .strip_prefix('[')
                    .and_then(|s| s.split_once(']'))
                    .map_or(dest_url.as_ref(), |(l, _)| l)
                    .trim_matches('`')
                    .to_owned();
                push(
                    &mut result,
                    path,
                    content,
                    range.start,
                    dest_url.as_ref(),
                    &label,
                    false,
                );
            }
            Event::End(TagEnd::Link) => in_link = false,
            Event::Code(value)
                if active
                    && blocked == 0
                    && !in_link
                    && (value.ends_with(".md")
                        || value.contains(".md#")
                        || value.ends_with('/')) =>
            {
                push(
                    &mut result,
                    path,
                    content,
                    range.start,
                    &value,
                    &value,
                    true,
                );
            }
            _ => {}
        }
    }
    if product_root {
        let unique = result
            .relationships
            .targets
            .iter()
            .filter_map(|edge| edge.target.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .len();
        if unique != 4 {
            result.relationships.issue = Some(format!(
                "product module table has {unique} supported module roots; expected exactly four"
            ));
        }
    }
    result
}
fn push(
    result: &mut DocumentNavigation,
    path: &str,
    content: &str,
    offset: usize,
    destination: &str,
    label: &str,
    code: bool,
) {
    if result.relationships.targets.len() >= 256 || destination.len() > 4096 {
        result.relationships.issue =
            Some("navigation bound exceeded; projection incomplete".into());
        return;
    }
    if path == "doc/product/README.md" || path == "doc/README.md" {
        let start = content[..offset].rfind('\n').map_or(0, |i| i + 1);
        let line = &content[start
            ..content[offset..]
                .find('\n')
                .map_or(content.len(), |i| offset + i)];
        let column = content[start..offset]
            .bytes()
            .filter(|b| *b == b'|')
            .count();
        if !line.trim_start().starts_with('|')
            || if path == "doc/product/README.md" {
                column != 2
            } else {
                column != 2 && column != 3
            }
        {
            return;
        }
    }
    // A directory spelling addresses only its explicit README candidate, never all descendants.
    let candidate = if destination.ends_with('/') {
        format!("{destination}README.md")
    } else {
        destination.to_owned()
    };
    let target = super::lifecycle_metadata::normalize(path, &candidate, code).ok();
    if path == "doc/product/README.md"
        && !target.as_deref().is_some_and(|p| {
            [
                "world-rules-core-gameplay",
                "world-infrastructure",
                "agents-world-simulation",
                "player-entry-distribution",
            ]
            .iter()
            .any(|slug| p == format!("doc/product/{slug}/prd.md"))
        })
    {
        result.relationships.issue = Some(
            "product module table contains an unsupported fifth or invalid module target".into(),
        );
        return;
    }
    if path.starts_with("doc/product/")
        && path != "doc/product/README.md"
        && !target.as_deref().is_some_and(|p| {
            p.split('#')
                .next()
                .is_some_and(|p| p.ends_with("/prd.md") || p.ends_with(".prd.md"))
        })
    {
        return;
    }
    if path == "doc/product/README.md"
        && result
            .relationships
            .targets
            .iter()
            .any(|edge| edge.target == target)
    {
        result.relationships.issue =
            Some("duplicate product module declaration; duplicate occurrence omitted".into());
        return;
    }
    let line = content[..offset].bytes().filter(|b| *b == b'\n').count() + 1;
    if result
        .relationships
        .targets
        .iter()
        .any(|e| e.declaration_line == line && e.destination == destination)
    {
        return;
    }
    result.relationships.targets.push(DocumentPairing {
        declaration_line: line,
        destination: destination.into(),
        target,
        resolution: "outside_scope".into(),
        target_section: None,
        target_revision: None,
        target_kind: None,
    });
    let source_line = content.lines().nth(line - 1).unwrap_or("");
    let label = if path == "doc/README.md" || path == "doc/product/README.md" {
        source_line
            .trim_start()
            .strip_prefix('|')
            .and_then(|s| s.split('|').next())
            .map(str::trim)
            .unwrap_or(label)
    } else {
        label
    };
    result.labels.push(label.chars().take(512).collect());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_root_table_columns_exclude_prose_fifth_and_duplicates() {
        let content = "# Product\n## 四大产品模块\n| 产品模块 | 唯一入口 | 职责 |\n| --- | --- | --- |\n| One | [root](world-rules-core-gameplay/prd.md) | [not root](../../core/prd.md) |\n| Again | [root](world-rules-core-gameplay/prd.md) | ordinary |\n| Fifth | [fifth](fifth/prd.md) | ordinary |\n\nExternal [runtime](agents-world-simulation/external.prd.md)\n";
        let nav = parse("doc/product/README.md", content);
        assert_eq!(nav.relationships.targets.len(), 1);
        assert_eq!(nav.relationships.targets[0].declaration_line, 5);
        assert_eq!(nav.labels, ["One"]);
        assert!(
            nav.relationships
                .issue
                .unwrap()
                .contains("expected exactly four")
        );
    }
    #[test]
    fn topics_h3_stop_at_sibling_exclude_examples_and_secondary_design() {
        let content = "# Root\n> ### 活跃产品专题\n> [fake](fake.prd.md)\n\n### **活跃产品专题**\n- [topic](topic.prd.md) and [design](topic.design.md)\n### 非权威迁移索引\n- [excluded](old.prd.md)\n";
        let nav = parse("doc/product/world-infrastructure/prd.md", content);
        assert_eq!(nav.relationships.targets.len(), 1);
        assert_eq!(
            nav.relationships.targets[0].target.as_deref(),
            Some("doc/product/world-infrastructure/topic.prd.md")
        );
        assert_eq!(nav.relationships.targets[0].declaration_line, 6);
        assert!(
            parse("doc/product/world-infrastructure/topic.prd.md", content)
                .relationships
                .targets
                .is_empty()
        );
    }
    #[test]
    fn subsection_headings_preserve_scope_until_sibling_or_higher_heading() {
        let product = "# Root\n### 活跃产品专题\n#### 生产\n- [production](production.prd.md)\n- # Nested list heading must not end the outer scope\n\n- [second](second.prd.md)\n### 非权威迁移索引\n- [excluded](excluded.prd.md)\n";
        let nav = parse("doc/product/world-infrastructure/prd.md", product);
        assert_eq!(
            nav.relationships
                .targets
                .iter()
                .map(|e| e.destination.as_str())
                .collect::<Vec<_>>(),
            ["production.prd.md", "second.prd.md"]
        );
        let professional = "# Design\n## 专题导航\n### 内核\n- [kernel](kernel.design.md)\n## 集成点\n- [excluded](excluded.design.md)\n";
        let nav = parse("doc/engineering/design.md", professional);
        assert_eq!(nav.relationships.targets.len(), 1);
        assert_eq!(nav.relationships.targets[0].destination, "kernel.design.md");
    }
    #[test]
    fn professional_code_paths_directory_candidates_and_setext_heading() {
        let nav = parse(
            "doc/world-simulator/design.md",
            "# Design\n专题导航\n--------\n- `viewer/` and `doc/world-simulator/prd.index.md`\n## 集成点\n[other](../testing/prd.md)\n",
        );
        assert_eq!(nav.relationships.targets.len(), 2);
        assert_eq!(
            nav.relationships.targets[0].target.as_deref(),
            Some("doc/world-simulator/viewer/README.md")
        );
        assert_eq!(
            nav.relationships.targets[1].target.as_deref(),
            Some("doc/world-simulator/prd.index.md")
        );
        assert_eq!(nav.relationships.targets[0].declaration_line, 4);
        let nav = parse(
            "doc/README.md",
            "## 工程模块入口矩阵\n| 模块 | PRD 主文档 | 设计主文档 | 描述 |\n| --- | --- | --- | --- |\n| core | `doc/core/prd.md` | `doc/core/design.md` | [excluded](core/other.design.md) |\n",
        );
        assert_eq!(nav.relationships.targets.len(), 2);
        assert_eq!(nav.labels, ["core", "core"]);
    }
}
