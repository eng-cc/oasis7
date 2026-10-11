//! Offline reads of trusted local Git checkouts. No porcelain, filters or remote operations.
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, Serialize)]
pub struct GitError {
    pub code: &'static str,
    pub message: &'static str,
}
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for GitError {}
fn error(code: &'static str, message: &'static str) -> GitError {
    GitError { code, message }
}
#[derive(Debug, Serialize)]
pub struct GitStatus {
    pub available: bool,
    pub head: Option<String>,
    pub shallow: bool,
    pub remote_contacted: bool,
    pub comparison: &'static str,
    pub working_tree_dirty: bool,
    pub changes_from_head: Vec<String>,
    pub index_changes: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct HistoryCommit {
    pub commit: String,
    pub committed_at: String,
    pub author: String,
    pub title: String,
}
#[derive(Debug, Serialize)]
pub struct History {
    pub head: String,
    pub complete: bool,
    pub shallow: bool,
    pub remote_contacted: bool,
    pub commits: Vec<HistoryCommit>,
}
pub struct HistoricalEntry {
    pub commit: String,
    pub config_bytes: Vec<u8>,
    pub entry_bytes: Vec<u8>,
}
fn run(root: Option<&Path>, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let mut command = Command::new("/usr/bin/git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    command.args([
        "--no-lazy-fetch",
        "--no-replace-objects",
        "--no-pager",
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.pager=cat",
        "-c",
        "log.showSignature=false",
        "-c",
        "log.mailmap=false",
        "-c",
        "i18n.logOutputEncoding=UTF-8",
    ]);
    if let Some(root) = root {
        command.current_dir(root);
    }
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|_| error("HISTORY_UNAVAILABLE", "Git could not be started"))?;
    let stdout = child.stdout.take().unwrap();
    let exceeded = Arc::new(AtomicBool::new(false));
    let flag = exceeded.clone();
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut input = stdout;
        let mut buf = [0; 8192];
        loop {
            match input.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if bytes.len() + n > OUTPUT_LIMIT {
                        flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                }
            }
        }
        bytes
    });
    let started = Instant::now();
    let result = loop {
        if exceeded.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            break Err(error(
                "HISTORY_INCOMPLETE",
                "Git read exceeded its output or time limit",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(error(
                        "HISTORY_INCOMPLETE",
                        "Required local Git object or repository data is unavailable",
                    ))
                };
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error("HISTORY_INCOMPLETE", "Git read failed"));
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| error("HISTORY_INCOMPLETE", "Git output read failed"))?;
    result?;
    if exceeded.load(Ordering::Relaxed) {
        return Err(error("HISTORY_INCOMPLETE", "Git output exceeds limit"));
    }
    Ok(bytes)
}
fn text(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes).map_err(|_| error("HISTORY_INCOMPLETE", "Git metadata is not UTF-8"))
}
fn probe(root: &Path) -> Result<(), GitError> {
    run(None, &["--version"]).map_err(|_| {
        error(
            "GIT_CAPABILITY_UNSUPPORTED",
            "Git lacks the required offline command options",
        )
    })?;
    let top = text(
        run(Some(root), &["rev-parse", "--show-toplevel"])
            .map_err(|_| error("HISTORY_UNAVAILABLE", "The store is not a Git checkout"))?,
    )?;
    if std::fs::canonicalize(root).ok() != std::fs::canonicalize(top.trim()).ok() {
        return Err(error(
            "HISTORY_UNAVAILABLE",
            "Git checkout root must equal the context root",
        ));
    }
    Ok(())
}
fn head(root: &Path) -> Result<Option<String>, GitError> {
    match run(Some(root), &["rev-parse", "--verify", "HEAD"]) {
        Ok(bytes) => Ok(Some(text(bytes)?.trim().to_owned())),
        Err(_) => {
            // An unborn branch has no refs, rather than a missing promised commit.
            let refs = run(Some(root), &["rev-parse", "--all"])?;
            if refs.is_empty() {
                Ok(None)
            } else {
                Err(error("HISTORY_INCOMPLETE", "HEAD cannot be read"))
            }
        }
    }
}
fn shallow(root: &Path) -> Result<bool, GitError> {
    Ok(text(run(Some(root), &["rev-parse", "--is-shallow-repository"])?)?.trim() == "true")
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || i > 0 && (b == b'-' || b == b'_')
        })
}
fn oid_valid(oid: &str) -> bool {
    (oid.len() == 40 || oid.len() == 64)
        && oid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn managed(path: &str) -> bool {
    path == "context.json"
        || path
            .strip_prefix("entries/")
            .and_then(|s| s.strip_suffix(".md"))
            .is_some_and(valid_id)
}
fn blob(root: &Path, oid: &str) -> Result<Vec<u8>, GitError> {
    run(Some(root), &["cat-file", "blob", oid])
}
fn tree(root: &Path, commit: &str) -> Result<BTreeMap<String, String>, GitError> {
    let data = run(
        Some(root),
        &[
            "ls-tree",
            "-r",
            "-z",
            commit,
            "--",
            "context.json",
            "entries",
        ],
    )?;
    let mut map = BTreeMap::new();
    for record in data.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record = std::str::from_utf8(record)
            .map_err(|_| error("HISTORY_INCOMPLETE", "Git tree has invalid paths"))?;
        let (meta, path) = record
            .split_once('\t')
            .ok_or_else(|| error("HISTORY_INCOMPLETE", "Git tree is malformed"))?;
        let fields: Vec<_> = meta.split(' ').collect();
        if managed(path) {
            if fields.len() != 3 || fields[0] != "100644" || fields[1] != "blob" {
                return Err(error(
                    "HISTORY_INCOMPLETE",
                    "History contains an unsupported entry type",
                ));
            }
            map.insert(path.to_owned(), fields[2].to_owned());
        }
    }
    Ok(map)
}
fn index(root: &Path) -> Result<BTreeMap<String, String>, GitError> {
    let data = run(
        Some(root),
        &["ls-files", "--stage", "-z", "--", "context.json", "entries"],
    )?;
    let mut map = BTreeMap::new();
    for record in data.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record = std::str::from_utf8(record)
            .map_err(|_| error("HISTORY_INCOMPLETE", "Git index has invalid paths"))?;
        let (meta, path) = record
            .split_once('\t')
            .ok_or_else(|| error("HISTORY_INCOMPLETE", "Git index is malformed"))?;
        if managed(path) {
            let fields: Vec<_> = meta.split(' ').collect();
            if fields.len() != 3 || fields[0] != "100644" || fields[2] != "0" {
                return Err(error(
                    "HISTORY_INCOMPLETE",
                    "Git index has unresolved or unsupported entries",
                ));
            }
            map.insert(path.to_owned(), fields[1].to_owned());
        }
    }
    Ok(map)
}
fn current(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, GitError> {
    let unavailable = || {
        error(
            "HISTORY_INCOMPLETE",
            "Current store files are unavailable or unsafe",
        )
    };
    let directory = crate::fs::Directory::open(root).map_err(|_| unavailable())?;
    let entries = directory.child("entries").map_err(|_| unavailable())?;
    let mut map = BTreeMap::new();
    let config = directory
        .read("context.json", 16 * 1024)
        .map_err(|_| unavailable())?;
    let mut total = config.len();
    map.insert("context.json".to_owned(), config);
    for name in entries.names().map_err(|_| unavailable())? {
        let path = format!("entries/{name}");
        if managed(&path) {
            let bytes = entries
                .read(&name, 1024 * 1024)
                .map_err(|_| unavailable())?;
            total += bytes.len();
            if total > 64 * 1024 * 1024 {
                return Err(error(
                    "HISTORY_INCOMPLETE",
                    "Current store exceeds its byte limit",
                ));
            }
            map.insert(path, bytes);
        }
    }
    Ok(map)
}
pub fn status(root: &Path) -> Result<GitStatus, GitError> {
    if let Err(err) = probe(root) {
        if err.code == "HISTORY_UNAVAILABLE" {
            return Ok(GitStatus {
                available: false,
                head: None,
                shallow: false,
                remote_contacted: false,
                comparison: "raw_bytes",
                working_tree_dirty: false,
                changes_from_head: vec![],
                index_changes: vec![],
            });
        }
        return Err(err);
    }
    let head = head(root)?;
    let baseline = match &head {
        Some(oid) => tree(root, oid)?,
        None => BTreeMap::new(),
    };
    let staged = index(root)?;
    let current = current(root)?;
    let mut paths: std::collections::BTreeSet<_> = current
        .keys()
        .chain(baseline.keys())
        .chain(staged.keys())
        .cloned()
        .collect();
    let mut changes = Vec::new();
    let mut index_changes = Vec::new();
    for path in std::mem::take(&mut paths) {
        let prior = baseline.get(&path).map(|oid| blob(root, oid)).transpose()?;
        if current.get(&path) != prior.as_ref() {
            changes.push(path.clone());
        }
        let index_bytes = staged.get(&path).map(|oid| blob(root, oid)).transpose()?;
        if index_bytes != prior {
            index_changes.push(path);
        }
    }
    Ok(GitStatus {
        available: true,
        head,
        shallow: shallow(root)?,
        remote_contacted: false,
        comparison: "raw_bytes",
        working_tree_dirty: !changes.is_empty() || !index_changes.is_empty(),
        changes_from_head: changes,
        index_changes,
    })
}
pub fn history(root: &Path, id: &str, limit: usize) -> Result<History, GitError> {
    if !valid_id(id) || limit == 0 || limit > 1000 {
        return Err(error("INVALID_ARGUMENT", "Invalid history ID or limit"));
    }
    probe(root)?;
    let head =
        head(root)?.ok_or_else(|| error("HISTORY_UNAVAILABLE", "There is no committed history"))?;
    let path = format!("entries/{id}.md");
    let max = format!("-{}", limit + 1);
    let data = text(run(
        Some(root),
        &[
            "log",
            &max,
            "--format=%H%x00%cI%x00%an%x00%s%x00",
            &head,
            "--",
            &path,
        ],
    )?)?;
    let mut commits = Vec::new();
    let fields: Vec<_> = data.split('\0').collect();
    for fields in fields.chunks(4) {
        if fields.len() < 4 {
            break;
        }
        commits.push(HistoryCommit {
            commit: fields[0].trim().to_owned(),
            committed_at: fields[1].to_owned(),
            author: fields[2].to_owned(),
            title: fields[3].to_owned(),
        });
    }
    let shallow = shallow(root)?;
    let complete = !shallow && commits.len() <= limit;
    commits.truncate(limit);
    Ok(History {
        head,
        complete,
        shallow,
        remote_contacted: false,
        commits,
    })
}
pub fn read_at_commit(root: &Path, id: &str, oid: &str) -> Result<HistoricalEntry, GitError> {
    if !valid_id(id) || !oid_valid(oid) {
        return Err(error(
            "INVALID_ARGUMENT",
            "A valid entry ID and full lowercase commit OID are required",
        ));
    }
    probe(root)?;
    if text(run(Some(root), &["cat-file", "-t", oid])?)?.trim() != "commit" {
        return Err(error("INVALID_ARGUMENT", "The object is not a commit"));
    }
    let objects = tree(root, oid)?;
    let path = format!("entries/{id}.md");
    let config = objects.get("context.json").ok_or_else(|| {
        error(
            "HISTORY_INCOMPLETE",
            "Historical store identity is unavailable",
        )
    })?;
    let entry = objects
        .get(&path)
        .ok_or_else(|| error("HISTORY_INCOMPLETE", "Historical entry is unavailable"))?;
    Ok(HistoricalEntry {
        commit: oid.to_owned(),
        config_bytes: blob(root, config)?,
        entry_bytes: blob(root, entry)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("/usr/bin/git")
            .current_dir(root)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "fixture command failed");
    }
    fn fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        git(root, &["init"]);
        git(root, &["config", "user.name", "Fixture"]);
        git(root, &["config", "user.email", "fixture@example.invalid"]);
        std::fs::create_dir(root.join("entries")).unwrap();
        std::fs::write(root.join("context.json"), b"fixture identity").unwrap();
        std::fs::write(root.join("entries/machine-test.md"), b"first\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "First"]);
        dir
    }
    #[test]
    fn status_compares_raw_head_and_index_and_ignores_other_files() {
        let dir = fixture();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        assert!(!status(root).unwrap().working_tree_dirty);
        std::fs::write(root.join("unrelated"), b"ignored").unwrap();
        assert!(!status(root).unwrap().working_tree_dirty);
        std::fs::write(root.join("entries/machine-test.md"), b"second\n").unwrap();
        let result = status(root).unwrap();
        assert_eq!(result.changes_from_head, vec!["entries/machine-test.md"]);
        assert!(result.index_changes.is_empty());
        assert!(!result.remote_contacted);
        git(root, &["add", "entries/machine-test.md"]);
        assert_eq!(
            status(root).unwrap().index_changes,
            vec!["entries/machine-test.md"]
        );
    }
    #[test]
    fn historical_blob_is_exact_and_replace_refs_are_ignored() {
        let dir = fixture();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        let first = history(root, "machine-test", 10).unwrap().head;
        std::fs::write(root.join("entries/machine-test.md"), b"second\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "Second"]);
        let second = history(root, "machine-test", 10).unwrap().head;
        git(root, &["replace", &first, &second]);
        let old = read_at_commit(root, "machine-test", &first).unwrap();
        assert_eq!(old.entry_bytes, b"first\n");
        assert_eq!(old.config_bytes, b"fixture identity");
        assert_eq!(history(root, "machine-test", 1).unwrap().commits.len(), 1);
        assert!(!history(root, "machine-test", 1).unwrap().complete);
        assert_eq!(
            read_at_commit(root, "../escape", &first)
                .err()
                .unwrap()
                .code,
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            read_at_commit(root, "machine-test", "HEAD")
                .err()
                .unwrap()
                .code,
            "INVALID_ARGUMENT"
        );
    }
    #[test]
    fn query_does_not_run_worktree_filters_or_fsmonitor() {
        let dir = fixture();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        let marker = root.join("executed");
        let command = format!("touch {}", marker.display());
        git(root, &["config", "filter.side.clean", &command]);
        git(root, &["config", "filter.side.smudge", &command]);
        git(root, &["config", "core.fsmonitor", &command]);
        std::fs::write(root.join(".gitattributes"), b"entries/*.md filter=side\n").unwrap();
        assert!(!status(root).unwrap().working_tree_dirty);
        let h = history(root, "machine-test", 10).unwrap();
        assert_eq!(
            read_at_commit(root, "machine-test", &h.head)
                .unwrap()
                .entry_bytes,
            b"first\n"
        );
        assert!(!marker.exists());
    }
    #[test]
    fn plain_directory_and_unborn_history_are_explicit() {
        let dir = TempDir::new().unwrap();
        assert!(!status(dir.path()).unwrap().available);
        assert_eq!(
            history(dir.path(), "test", 10).err().unwrap().code,
            "HISTORY_UNAVAILABLE"
        );
        git(dir.path(), &["init"]);
        assert_eq!(
            history(dir.path(), "test", 10).err().unwrap().code,
            "HISTORY_UNAVAILABLE"
        );
    }
    #[test]
    fn shallow_history_is_explicitly_incomplete() {
        let dir = fixture();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        let original = history(root, "machine-test", 10).unwrap();
        std::fs::write(root.join(".git/shallow"), format!("{}\n", original.head)).unwrap();
        let limited = history(root, "machine-test", 10).unwrap();
        assert!(limited.shallow);
        assert!(!limited.complete);
    }
    #[test]
    fn missing_blob_reports_incomplete_without_network() {
        let dir = fixture();
        let canonical_root = dir.path().canonicalize().unwrap();
        let root = canonical_root.as_path();
        let h = history(root, "machine-test", 10).unwrap();
        let objects = tree(root, &h.head).unwrap();
        let oid = &objects["entries/machine-test.md"];
        // A promised object must not invoke this configured transport during a query.
        let marker = root.join("remote-contacted");
        git(root, &["config", "remote.origin.promisor", "true"]);
        git(
            root,
            &[
                "config",
                "remote.origin.url",
                &format!("ext::touch {}", marker.display()),
            ],
        );
        git(root, &["config", "protocol.ext.allow", "always"]);
        std::fs::remove_file(root.join(".git/objects").join(&oid[..2]).join(&oid[2..])).unwrap();
        assert_eq!(
            read_at_commit(root, "machine-test", &h.head)
                .err()
                .unwrap()
                .code,
            "HISTORY_INCOMPLETE"
        );
        assert!(!marker.exists());
    }
}
