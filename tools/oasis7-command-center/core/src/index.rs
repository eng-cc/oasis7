#[path = "index_coverage.rs"]
mod coverage;
#[path = "document_fragments.rs"]
mod document_fragments;
#[path = "document_pairings.rs"]
mod document_pairings;
#[path = "document_read.rs"]
mod document_read;
#[path = "lifecycle_metadata.rs"]
mod lifecycle_metadata;
use crate::{
    Error, Result,
    document_inventory::{IndexedDocument, Inventory, sections_digest},
    domain::*,
    process::run,
};
use coverage::Coverage;
use sha2::{Digest, Sha256};
use std::path::Path;
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn git(path: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&run("git", args, path)?)
        .trim_end_matches('\n')
        .to_string())
}
pub fn project(path: &Path) -> Result<Project> {
    let path = path.canonicalize()?;
    let root = git(&path, &["rev-parse", "--show-toplevel"])?;
    let root = Path::new(&root).canonicalize()?;
    let common = git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let common = Path::new(&common).canonicalize()?;
    Ok(Project {
        id: digest(format!("{}:{}", common.display(), root.display()).as_bytes()),
        path: root.to_string_lossy().into(),
        git_common_dir: common.to_string_lossy().into(),
        branch: git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).ok(),
        head: git(&root, &["rev-parse", "HEAD"])?,
        environment: "local".into(),
    })
}
pub fn safe_path(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::Invalid("document path must be relative".into()));
    }
    let mut candidate = root.to_path_buf();
    for component in rel.components() {
        candidate.push(component);
        if std::fs::symlink_metadata(&candidate)?
            .file_type()
            .is_symlink()
        {
            return Err(Error::Invalid("document path contains a symlink".into()));
        }
    }
    let path = candidate.canonicalize()?;
    if !path.starts_with(root) {
        return Err(Error::Invalid("document escapes project".into()));
    }
    Ok(path)
}
#[cfg(test)]
pub fn read(project: &Project) -> Result<(Vec<DocumentSection>, GitSnapshot, String, bool)> {
    read_candidate(project, || {})
}
pub(crate) struct IndexCandidate {
    pub sections: Vec<DocumentSection>,
    pub git: GitSnapshot,
    pub coverage: String,
    pub omitted: bool,
    pub inventory: Inventory,
}
pub(crate) fn read_with_inventory(project: &Project) -> Result<IndexCandidate> {
    scan(project, || {})
}
#[cfg(test)]
fn read_candidate(
    project: &Project,
    after_read: impl FnOnce(),
) -> Result<(Vec<DocumentSection>, GitSnapshot, String, bool)> {
    let candidate = scan(project, after_read)?;
    Ok((
        candidate.sections,
        candidate.git,
        candidate.coverage,
        candidate.omitted,
    ))
}
fn scan(project: &Project, after_read: impl FnOnce()) -> Result<IndexCandidate> {
    let root = Path::new(&project.path);
    let files = document_list(root)?;
    let changed = dirty_paths(root)?;
    let changed: std::collections::HashSet<&[u8]> = changed.split(|b| *b == 0).collect();
    let mut sections = Vec::new();
    let mut inventory_files = Vec::new();
    let mut pairing_contents = std::collections::HashMap::new();
    let mut metadata = std::collections::HashMap::new();
    let mut total = 0;
    let mut observed = Vec::new();
    let mut omissions = Vec::new();
    let mut names: Vec<&[u8]> = files
        .split(|b| *b == 0)
        .filter(|b| b.ends_with(b".md"))
        .collect();
    names.sort_unstable();
    names.dedup();
    names.sort_by_key(|name| {
        let name = String::from_utf8_lossy(name);
        let priority = if name == "doc/core/prd.md" {
            0
        } else if name == "doc/engineering/workflow/source-of-truth.md" {
            1
        } else if name == "AGENTS.md"
            || name == "skills/oasis7-command-center/references/project-map.md"
            || (name.starts_with("doc/") && name.ends_with("/README.md"))
        {
            // Keep structural navigation available when body documents exhaust
            // either the path count or total byte budget.
            2
        } else if name.starts_with("doc/product/") || name.ends_with("/prd.md") {
            3
        } else {
            4
        };
        (priority, name.into_owned())
    });
    let mut coverage = Coverage::new(names.len());
    for (position, name) in names.into_iter().enumerate() {
        let name = std::str::from_utf8(name)
            .map_err(|_| Error::Invalid("non UTF-8 document path".into()))?;
        if !name.ends_with(".md") {
            continue;
        }
        let mut file = IndexedDocument::new(&project.id, name, changed.contains(name.as_bytes()));
        if position >= 3000 {
            file.omit("file_count_budget");
            inventory_files.push(file);
            coverage.omit(name, "file_count_budget");
            continue;
        }
        let remaining = 16 * 1024 * 1024 - total;
        let content = match read_source(root, name, remaining) {
            Ok(content) => content,
            Err(reason) => {
                file.omit(reason);
                inventory_files.push(file);
                coverage.omit(name, reason);
                omissions.push((name.to_owned(), reason, remaining));
                continue;
            }
        };
        total += content.len();
        coverage.read += 1;
        let dirty = changed.contains(name.as_bytes());
        let revision = digest(content.as_bytes());
        observed.push((name.to_owned(), revision.clone()));
        metadata.insert(name.to_owned(), lifecycle_metadata::parse(name, &content));
        let parsed = document_sections(name, &content, &revision, dirty, project);
        file.title = parsed
            .iter()
            .find(|section| section.title != name)
            .map_or_else(|| name.to_owned(), |section| section.title.clone());
        file.revision = Some(revision);
        file.bytes = Some(content.len());
        file.pairings = Some(document_pairings::parse(name, &content, "配对系统设计"));
        if file.kind == "design" {
            file.upstream = Some(document_pairings::parse(name, &content, "上游"));
        }
        inventory_files.push(file);
        sections.extend(parsed);
        pairing_contents.insert(name.to_owned(), content);
    }
    let indexed: std::collections::HashSet<String> =
        sections.iter().map(|s| s.path.clone()).collect();
    for value in metadata.values_mut().flatten() {
        lifecycle_metadata::validate_indexed(value, &indexed);
    }
    document_pairings::resolve(&mut inventory_files, &sections, &pairing_contents);
    for file in &mut inventory_files {
        file.lifecycle = metadata.get(&file.path).cloned().flatten();
    }
    for section in &mut sections {
        section.lifecycle = metadata.get(&section.path).cloned().flatten();
    }
    after_read();
    let snapshot = GitSnapshot {
        status: String::from_utf8_lossy(&run(
            "git",
            &["status", "--porcelain=v2", "-z", "--branch"],
            root,
        )?)
        .into(),
        worktrees: String::from_utf8_lossy(&run(
            "git",
            &["worktree", "list", "--porcelain", "-z"],
            root,
        )?)
        .into(),
    };
    // HEAD alone cannot detect edits to already-dirty files or index-only additions.
    // Reject the candidate instead of publishing sections from different observations.
    let current_changed = dirty_paths(root)?;
    let current_changed: std::collections::HashSet<&[u8]> =
        current_changed.split(|b| *b == 0).collect();
    if document_list(root)? != files || current_changed != changed {
        return Err(Error::Invalid(
            "document list or dirty state changed during indexing; refresh again".into(),
        ));
    }
    for (name, revision) in observed {
        let matches = read_source(root, &name, 1024 * 1024)
            .is_ok_and(|content| digest(content.as_bytes()) == revision);
        if !matches {
            return Err(Error::Invalid(
                "document content changed during indexing; refresh again".into(),
            ));
        }
    }
    for (name, reason, remaining) in omissions {
        if read_source(root, &name, remaining).err() != Some(reason) {
            return Err(Error::Invalid(
                "document omission changed during indexing; refresh again".into(),
            ));
        }
    }
    if git(root, &["rev-parse", "HEAD"])? != project.head {
        return Err(Error::Invalid(
            "HEAD changed during indexing; refresh again".into(),
        ));
    }
    inventory_files.sort_by(|a, b| a.path.cmp(&b.path));
    let snapshot_id = digest(&serde_json::to_vec(&(
        &project.id,
        &project.head,
        &inventory_files,
    ))?);
    let inventory = Inventory {
        schema_version: 1,
        project_id: project.id.clone(),
        baseline_oid: project.head.clone(),
        snapshot_id,
        generated_at: crate::now(),
        source_generation: String::new(),
        sections_digest: sections_digest(&sections),
        files: inventory_files,
    };
    Ok(IndexCandidate {
        sections,
        git: snapshot,
        coverage: coverage.summary(total),
        omitted: coverage.has_omissions(),
        inventory,
    })
}
fn read_source(
    root: &Path,
    name: &str,
    remaining: usize,
) -> std::result::Result<String, &'static str> {
    document_read::read(root, name, remaining)
}
fn dirty_paths(root: &Path) -> Result<Vec<u8>> {
    run(
        "git",
        &[
            "diff",
            "HEAD",
            "--name-only",
            "-z",
            "--",
            "doc",
            "AGENTS.md",
            "skills/oasis7-command-center/references/project-map.md",
        ],
        root,
    )
    .map(markdown_paths)
}
fn document_list(root: &Path) -> Result<Vec<u8>> {
    run(
        "git",
        &[
            "ls-files",
            "-z",
            "--",
            "doc",
            "AGENTS.md",
            "skills/oasis7-command-center/references/project-map.md",
        ],
        root,
    )
    .map(markdown_paths)
}
fn markdown_paths(paths: Vec<u8>) -> Vec<u8> {
    let mut result = Vec::new();
    for path in paths
        .split(|b| *b == 0)
        .filter(|path| path.ends_with(b".md"))
    {
        result.extend_from_slice(path);
        result.push(0);
    }
    result
}
fn document_sections(
    path: &str,
    content: &str,
    revision: &str,
    dirty: bool,
    project: &Project,
) -> Vec<DocumentSection> {
    use std::collections::HashMap;
    let lines: Vec<&str> = content.lines().collect();
    // Use rendered inline text; source lines retain Markdown syntax and miss
    // multiline Setext headings. Keep the original lines for source display.
    let mut headings = HashMap::new();
    let mut heading: Option<(usize, usize, String)> = None;
    for (event, range) in pulldown_cmark::Parser::new(content).into_offset_iter() {
        use pulldown_cmark::{Event, Tag, TagEnd};
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let line = content[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count();
                heading = Some((line, level as usize, String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, title)) = &mut heading {
                    title.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, title)) = &mut heading {
                    title.push(' ');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((line, level, title)) = heading.take() {
                    headings.insert(line, (level, title.trim().to_owned()));
                }
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    let mut title = path.to_string();
    let mut key = format!("{path}#preamble");
    let mut start = 0;
    for line in 0..lines.len() {
        let Some((level, heading_title)) = headings.get(&line) else {
            continue;
        };
        if line > start {
            result.push(make_section(
                path,
                &title,
                &key,
                &lines[start..line],
                start,
                (revision, dirty),
                project,
            ));
        }
        title = heading_title.clone();
        // Keep the existing source-based key contract for personal records.
        // Display normalization must not silently rebind an old reading position.
        let identity_title = lines[line].trim_start_matches('#').trim().to_owned();
        while parents.last().is_some_and(|(old, _)| old >= level) {
            parents.pop();
        }
        let adjacent_anchor = lines[..line]
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .and_then(|l| anchor(l));
        let explicit = adjacent_anchor.or_else(|| clause_id(&identity_title));
        key = match explicit {
            Some(id) => format!("{path}#{id}"),
            None => {
                let context: Vec<&str> = parents
                    .iter()
                    .map(|(_, t)| t.as_str())
                    .chain(std::iter::once(identity_title.as_str()))
                    .collect();
                format!(
                    "{path}#heading-{}",
                    digest(&serde_json::to_vec(&context).expect("string encoding"))
                )
            }
        };
        parents.push((*level, identity_title));
        start = line;
    }
    if start < lines.len() {
        result.push(make_section(
            path,
            &title,
            &key,
            &lines[start..],
            start,
            (revision, dirty),
            project,
        ));
    }
    let mut counts = HashMap::new();
    for section in &result {
        *counts.entry(section.id.clone()).or_insert(0) += 1;
    }
    // Ambiguous duplicates have no stable cross-version identity. Their version-scoped IDs
    // are intentionally unresolved after any document change, preserving historical references.
    for section in &mut result {
        if counts[&section.id] > 1 {
            section.id = format!("{}@{}:line{}", section.id, revision, section.line);
        }
    }
    result
}
fn anchor(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line
        .strip_prefix("<a id=\"")
        .or_else(|| line.strip_prefix("<a id='"))?;
    let id = rest.split(['\"', '\'']).next()?;
    (!id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:".contains(c)))
    .then(|| id.to_string())
}
fn clause_id(title: &str) -> Option<String> {
    let token = title
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .next()?;
    (["REQ-", "AC-", "PRD-", "DES-"]
        .iter()
        .any(|prefix| token.starts_with(prefix))
        && token.chars().any(|c| c.is_ascii_digit()))
    .then(|| token.to_string())
}
fn make_section(
    path: &str,
    title: &str,
    id: &str,
    lines: &[&str],
    start: usize,
    version: (&str, bool),
    project: &Project,
) -> DocumentSection {
    let (revision, dirty) = version;
    DocumentSection {
        id: id.into(),
        path: path.into(),
        title: title.into(),
        content: format!("{}\n", lines.join("\n")),
        line: start + 1,
        revision: revision.into(),
        committed_revision: project.head.clone(),
        dirty,
        fact_kind: "sourceFact".into(),
        lifecycle: None,
    }
}

#[cfg(test)]
#[path = "index_consistency_tests.rs"]
mod consistency_tests;
