#[path = "index_coverage.rs"]
mod coverage;
#[path = "lifecycle_metadata.rs"]
mod lifecycle_metadata;
use crate::{Error, Result, domain::*, process::run};
use coverage::{Coverage, read_document};
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
    let path = root.join(rel).canonicalize()?;
    if !path.starts_with(root) {
        return Err(Error::Invalid("document escapes project".into()));
    }
    Ok(path)
}
pub fn read(project: &Project) -> Result<(Vec<DocumentSection>, GitSnapshot, String, bool)> {
    read_candidate(project, || {})
}
fn read_candidate(
    project: &Project,
    after_read: impl FnOnce(),
) -> Result<(Vec<DocumentSection>, GitSnapshot, String, bool)> {
    let root = Path::new(&project.path);
    let files = document_list(root)?;
    let changed = dirty_paths(root)?;
    let changed: std::collections::HashSet<&[u8]> = changed.split(|b| *b == 0).collect();
    let mut sections = Vec::new();
    let mut metadata = std::collections::HashMap::new();
    let mut total = 0;
    let mut observed = Vec::new();
    let mut names: Vec<&[u8]> = files
        .split(|b| *b == 0)
        .filter(|b| b.ends_with(b".md"))
        .collect();
    names.sort_by_key(|name| {
        let name = String::from_utf8_lossy(name);
        let priority = if name == "doc/core/prd.md" {
            0
        } else if name == "doc/engineering/workflow/source-of-truth.md" {
            1
        } else if name.starts_with("doc/product/")
            || name.ends_with("/prd.md")
            || name == "AGENTS.md"
        {
            2
        } else {
            3
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
        if position >= 3000 {
            coverage.omit(name, "file_count_budget");
            continue;
        }
        let path = match safe_path(root, name) {
            Ok(path) => path,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                coverage.omit(name, "deleted");
                continue;
            }
            Err(_) => {
                coverage.omit(name, "path_unavailable");
                continue;
            }
        };
        let content = match read_document(&path, 16 * 1024 * 1024 - total) {
            Ok(content) => content,
            Err(reason) => {
                coverage.omit(name, reason);
                continue;
            }
        };
        total += content.len();
        coverage.read += 1;
        let dirty = changed.contains(name.as_bytes());
        let revision = digest(content.as_bytes());
        observed.push((name.to_owned(), revision.clone()));
        metadata.insert(name.to_owned(), lifecycle_metadata::parse(name, &content));
        sections.extend(document_sections(name, &content, &revision, dirty, project));
    }
    let indexed: std::collections::HashSet<String> =
        sections.iter().map(|s| s.path.clone()).collect();
    for value in metadata.values_mut().flatten() {
        lifecycle_metadata::validate_indexed(value, &indexed);
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
        let matches = safe_path(root, &name)
            .and_then(|path| {
                // Keep verification within the same per-file budget as source reading.
                if std::fs::metadata(&path)?.len() > 1024 * 1024 {
                    return Ok(false);
                }
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                Ok(bytes.len() <= 1024 * 1024 && digest(&bytes) == revision)
            })
            .unwrap_or(false);
        if !matches {
            return Err(Error::Invalid(
                "document content changed during indexing; refresh again".into(),
            ));
        }
    }
    if git(root, &["rev-parse", "HEAD"])? != project.head {
        return Err(Error::Invalid(
            "HEAD changed during indexing; refresh again".into(),
        ));
    }
    Ok((
        sections,
        snapshot,
        coverage.summary(total),
        coverage.has_omissions(),
    ))
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
    let headings: HashMap<usize, usize> = pulldown_cmark::Parser::new(content)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => Some((
                content[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count(),
                level as usize,
            )),
            _ => None,
        })
        .collect();
    let mut result = Vec::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    let mut title = path.to_string();
    let mut key = format!("{path}#preamble");
    let mut start = 0;
    for (line, text) in lines.iter().enumerate() {
        let Some(level) = headings.get(&line) else {
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
        title = text.trim_start_matches('#').trim().to_string();
        while parents.last().is_some_and(|(old, _)| old >= level) {
            parents.pop();
        }
        let adjacent_anchor = lines[..line]
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .and_then(|l| anchor(l));
        let explicit = adjacent_anchor.or_else(|| clause_id(&title));
        key = match explicit {
            Some(id) => format!("{path}#{id}"),
            None => {
                let context: Vec<&str> = parents
                    .iter()
                    .map(|(_, t)| t.as_str())
                    .chain(std::iter::once(title.as_str()))
                    .collect();
                format!(
                    "{path}#heading-{}",
                    digest(&serde_json::to_vec(&context).expect("string encoding"))
                )
            }
        };
        parents.push((*level, title.clone()));
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
