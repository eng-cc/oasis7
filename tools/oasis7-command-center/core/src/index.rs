use crate::{Error, Result, domain::*, process::run};
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
pub fn read(project: &Project) -> Result<(Vec<DocumentSection>, GitSnapshot)> {
    let root = Path::new(&project.path);
    let files = run(
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
    )?;
    let changed = run("git", &["diff", "HEAD", "--name-only", "-z", "--"], root)?;
    let changed: std::collections::HashSet<&[u8]> = changed.split(|b| *b == 0).collect();
    let mut sections = Vec::new();
    let mut total = 0;
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
    for name in names.into_iter().take(3000) {
        let name = std::str::from_utf8(name)
            .map_err(|_| Error::Invalid("non UTF-8 document path".into()))?;
        if !name.ends_with(".md") {
            continue;
        }
        let path = match safe_path(root, name) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if std::fs::metadata(&path)?.len() > 1024 * 1024 {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        total += content.len();
        if total > 16 * 1024 * 1024 {
            break;
        }
        let dirty = changed.contains(name.as_bytes());
        let revision = digest(content.as_bytes());
        let headings: std::collections::HashSet<usize> = pulldown_cmark::Parser::new(&content)
            .into_offset_iter()
            .filter_map(|(event, range)| {
                matches!(
                    event,
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { .. })
                )
                .then(|| {
                    content[..range.start]
                        .bytes()
                        .filter(|b| *b == b'\n')
                        .count()
                })
            })
            .collect();
        let mut title = name.to_string();
        let mut start = 1;
        let mut block = String::new();
        let mut ordinal = 0;
        for (line, text) in content.lines().enumerate() {
            if headings.contains(&line) {
                if !block.is_empty() {
                    sections.push(section(
                        name,
                        &title,
                        &block,
                        start,
                        ordinal,
                        (&revision, dirty),
                        project,
                    ));
                    ordinal += 1;
                }
                title = text.trim_start_matches('#').trim().to_string();
                start = line + 1;
                block.clear();
            }
            block.push_str(text);
            block.push('\n');
        }
        if !block.is_empty() {
            sections.push(section(
                name,
                &title,
                &block,
                start,
                ordinal,
                (&revision, dirty),
                project,
            ));
        }
    }
    let current_head = git(root, &["rev-parse", "HEAD"])?;
    if current_head != project.head {
        return Err(Error::Invalid(
            "HEAD changed during indexing; refresh again".into(),
        ));
    }
    Ok((
        sections,
        GitSnapshot {
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
        },
    ))
}
fn section(
    path: &str,
    title: &str,
    content: &str,
    line: usize,
    ordinal: usize,
    version: (&str, bool),
    project: &Project,
) -> DocumentSection {
    let (revision, dirty) = version;
    DocumentSection {
        id: format!("{path}#{ordinal}"),
        path: path.into(),
        title: title.into(),
        content: content.into(),
        line,
        revision: revision.into(),
        committed_revision: project.head.clone(),
        dirty,
        fact_kind: "sourceFact".into(),
    }
}
