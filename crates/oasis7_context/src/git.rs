//! Offline reads of trusted local Git checkouts. No porcelain, filters or remote operations.
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
mod identity;
use identity::RepositoryIdentity;
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
fn read_output(mut input: impl Read, exceeded: &AtomicBool) -> Result<Vec<u8>, GitError> {
    let mut bytes = Vec::new();
    let mut buf = [0; 8192];
    loop {
        match input.read(&mut buf) {
            Ok(0) => return Ok(bytes),
            Ok(n) => {
                if bytes.len() + n > OUTPUT_LIMIT {
                    exceeded.store(true, Ordering::Relaxed);
                    return Err(error("HISTORY_INCOMPLETE", "Git output exceeds limit"));
                }
                bytes.extend_from_slice(&buf[..n]);
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(error("HISTORY_INCOMPLETE", "Git output read failed")),
        }
    }
}
// Prefer the OS binary when capable. These are fixed installation locations;
// neither PATH nor a caller/environment override participates in selection.
#[cfg(target_os = "macos")]
const GIT_PROGRAMS: &[&str] = &[
    "/usr/bin/git",
    "/opt/homebrew/bin/git",
    "/usr/local/bin/git",
];
#[cfg(not(target_os = "macos"))]
const GIT_PROGRAMS: &[&str] = &["/usr/bin/git", "/usr/local/bin/git"];
fn select_program<'a>(
    candidates: &[&'a str],
    mut probe: impl FnMut(&str) -> Result<(), GitError>,
) -> Result<&'a str, GitError> {
    for &program in candidates {
        if probe(program).is_ok() {
            return Ok(program);
        }
    }
    Err(error(
        "GIT_CAPABILITY_UNSUPPORTED",
        "No Git at the fixed installation paths accepts the required offline options",
    ))
}
fn selected_program() -> Result<&'static str, GitError> {
    static SELECTED: OnceLock<Result<&'static str, GitError>> = OnceLock::new();
    SELECTED
        .get_or_init(|| {
            select_program(GIT_PROGRAMS, |program| {
                run_program(program, None, &["--version"]).map(|_| ())
            })
        })
        .clone()
}
struct GitRoot<'a> {
    path: &'a Path,
    directory: Option<&'a crate::fs::Directory>,
    metadata: OnceLock<RepositoryIdentity>,
}
impl<'a> GitRoot<'a> {
    fn unpinned(path: &'a Path) -> Self {
        Self {
            path,
            directory: None,
            metadata: OnceLock::new(),
        }
    }
    fn pinned(path: &'a Path, directory: &'a crate::fs::Directory) -> Self {
        Self {
            path,
            directory: Some(directory),
            metadata: OnceLock::new(),
        }
    }
    fn capture_metadata(&self) -> Result<(), GitError> {
        if self.metadata.get().is_none() {
            let metadata = RepositoryIdentity::capture(self.path, self.directory)?;
            let _ = self.metadata.set(metadata);
        }
        self.verify()
    }
    fn verify(&self) -> Result<(), GitError> {
        if let Some(metadata) = self.metadata.get() {
            metadata.verify(self.path)?;
        }
        let Some(directory) = self.directory else {
            return Ok(());
        };
        if !directory.matches_path(self.path).map_err(|_| {
            error(
                "HISTORY_IDENTITY_CHANGED",
                "Pinned context directory is no longer available",
            )
        })? {
            return Err(error(
                "HISTORY_IDENTITY_CHANGED",
                "Pinned context directory identity changed",
            ));
        }
        Ok(())
    }
}
fn run(root: Option<&GitRoot>, args: &[&str]) -> Result<Vec<u8>, GitError> {
    run_program(selected_program()?, root, args)
}
fn run_program(program: &str, root: Option<&GitRoot>, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let mut command = Command::new(program);
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
        root.verify()?;
        if let Some(metadata) = root.metadata.get() {
            metadata.pin(&mut command)?;
        } else if let Some(directory) = root.directory {
            directory.pin_command_cwd(&mut command).map_err(|_| {
                error(
                    "HISTORY_UNAVAILABLE",
                    "Pinned Git working directory is unavailable",
                )
            })?;
        } else {
            command.current_dir(root.path);
        }
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
    let reader = thread::spawn(move || read_output(stdout, &flag));
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
    if let Some(root) = root {
        root.verify()?;
    }
    result?;
    if exceeded.load(Ordering::Relaxed) {
        return Err(error("HISTORY_INCOMPLETE", "Git output exceeds limit"));
    }
    bytes
}
fn text(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes).map_err(|_| error("HISTORY_INCOMPLETE", "Git metadata is not UTF-8"))
}
fn probe(root: &GitRoot) -> Result<(), GitError> {
    selected_program()?;
    root.capture_metadata()?;
    let top = text(
        run(Some(root), &["rev-parse", "--show-toplevel"]).map_err(|e| {
            if e.code == "HISTORY_IDENTITY_CHANGED" {
                e
            } else {
                error("HISTORY_UNAVAILABLE", "The store is not a Git checkout")
            }
        })?,
    )?;
    if std::fs::canonicalize(root.path).ok() != std::fs::canonicalize(top.trim()).ok() {
        return Err(error(
            "HISTORY_UNAVAILABLE",
            "Git checkout root must equal the context root",
        ));
    }
    Ok(())
}
fn head(root: &GitRoot) -> Result<Option<String>, GitError> {
    match run(Some(root), &["rev-parse", "--verify", "HEAD"]) {
        Ok(bytes) => Ok(Some(text(bytes)?.trim().to_owned())),
        Err(e) if e.code == "HISTORY_IDENTITY_CHANGED" => Err(e),
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
fn shallow(root: &GitRoot) -> Result<bool, GitError> {
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
fn blob(root: &GitRoot, oid: &str) -> Result<Vec<u8>, GitError> {
    run(Some(root), &["cat-file", "blob", oid])
}
fn tree(root: &GitRoot, commit: &str) -> Result<BTreeMap<String, String>, GitError> {
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
fn index(root: &GitRoot) -> Result<BTreeMap<String, String>, GitError> {
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
fn current(root: &GitRoot) -> Result<BTreeMap<String, Vec<u8>>, GitError> {
    let unavailable = || {
        error(
            "HISTORY_INCOMPLETE",
            "Current store files are unavailable or unsafe",
        )
    };
    let opened;
    let directory = if let Some(directory) = root.directory {
        directory
    } else {
        opened = crate::fs::Directory::open(root.path).map_err(|_| unavailable())?;
        &opened
    };
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
    status_inner(&GitRoot::unpinned(root))
}
pub fn status_pinned(root: &Path, directory: &crate::fs::Directory) -> Result<GitStatus, GitError> {
    status_inner(&GitRoot::pinned(root, directory))
}
fn status_inner(root: &GitRoot) -> Result<GitStatus, GitError> {
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
    history_inner(&GitRoot::unpinned(root), id, limit)
}
pub fn history_pinned(
    root: &Path,
    directory: &crate::fs::Directory,
    id: &str,
    limit: usize,
) -> Result<History, GitError> {
    history_inner(&GitRoot::pinned(root, directory), id, limit)
}
fn history_inner(root: &GitRoot, id: &str, limit: usize) -> Result<History, GitError> {
    history_after_probe(root, id, limit, || {})
}
fn history_after_probe(
    root: &GitRoot,
    id: &str,
    limit: usize,
    after_probe: impl FnOnce(),
) -> Result<History, GitError> {
    if !valid_id(id) || limit == 0 || limit > 1000 {
        return Err(error("INVALID_ARGUMENT", "Invalid history ID or limit"));
    }
    probe(root)?;
    after_probe();
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
    root.verify()?;
    Ok(History {
        head,
        complete,
        shallow,
        remote_contacted: false,
        commits,
    })
}
pub fn read_at_commit(root: &Path, id: &str, oid: &str) -> Result<HistoricalEntry, GitError> {
    read_at_commit_inner(&GitRoot::unpinned(root), id, oid)
}
pub fn read_at_commit_pinned(
    root: &Path,
    directory: &crate::fs::Directory,
    id: &str,
    oid: &str,
) -> Result<HistoricalEntry, GitError> {
    read_at_commit_inner(&GitRoot::pinned(root, directory), id, oid)
}
fn read_at_commit_inner(root: &GitRoot, id: &str, oid: &str) -> Result<HistoricalEntry, GitError> {
    read_after_probe(root, id, oid, || {})
}
fn read_after_probe(
    root: &GitRoot,
    id: &str,
    oid: &str,
    after_probe: impl FnOnce(),
) -> Result<HistoricalEntry, GitError> {
    if !valid_id(id) || !oid_valid(oid) {
        return Err(error(
            "INVALID_ARGUMENT",
            "A valid entry ID and full lowercase commit OID are required",
        ));
    }
    probe(root)?;
    after_probe();
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
    let result = HistoricalEntry {
        commit: oid.to_owned(),
        config_bytes: blob(root, config)?,
        entry_bytes: blob(root, entry)?,
    };
    root.verify()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    pub(super) fn git(root: &Path, args: &[&str]) {
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
    pub(super) fn fixture() -> TempDir {
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
    fn selection_falls_back_only_to_a_capable_fixed_candidate() {
        let candidates = ["/fixed/old-git", "/fixed/current-git", "/fixed/unused-git"];
        let mut attempted = Vec::new();
        let chosen = select_program(&candidates, |program| {
            attempted.push(program.to_owned());
            if program == candidates[1] {
                Ok(())
            } else {
                Err(error("HISTORY_INCOMPLETE", "unsupported option"))
            }
        })
        .unwrap();
        assert_eq!(chosen, candidates[1]);
        assert_eq!(attempted, candidates[..2]);
        let mut attempted = Vec::new();
        let failure = select_program(&candidates, |program| {
            attempted.push(program.to_owned());
            Err(error("HISTORY_UNAVAILABLE", "missing executable"))
        })
        .unwrap_err();
        assert_eq!(failure.code, "GIT_CAPABILITY_UNSUPPORTED");
        assert_eq!(attempted, candidates);
    }
    #[test]
    fn selection_probes_all_required_flags_without_repository_arguments() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let incapable = dir.path().join("incapable");
        let capable = dir.path().join("capable");
        std::fs::write(&incapable, "#!/bin/sh\nexit 129\n").unwrap();
        // The fixture accepts only the exact policy switches and a version query.
        std::fs::write(&capable, "#!/bin/sh\n[ \"$1\" = --no-lazy-fetch ] || exit 81\n[ \"$2\" = --no-replace-objects ] || exit 82\n[ \"$3\" = --no-pager ] || exit 83\n[ \"$4\" = --no-optional-locks ] || exit 84\nfor last; do :; done\n[ \"$last\" = --version ] || exit 85\nexit 0\n").unwrap();
        for path in [&incapable, &capable] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let programs = [incapable.to_str().unwrap(), capable.to_str().unwrap()];
        let chosen = select_program(&programs, |program| {
            run_program(program, None, &["--version"]).map(|_| ())
        })
        .unwrap();
        assert_eq!(chosen, programs[1]);
        let failure = select_program(&programs[..1], |program| {
            run_program(program, None, &["--version"]).map(|_| ())
        })
        .unwrap_err();
        assert_eq!(failure.code, "GIT_CAPABILITY_UNSUPPORTED");
    }
    #[test]
    fn interrupted_output_read_retries_but_io_failure_rejects_partial_bytes() {
        struct InjectedReader {
            call: usize,
            kind: std::io::ErrorKind,
        }
        impl Read for InjectedReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.call += 1;
                match self.call {
                    1 | 3 => {
                        buf[0] = b'x';
                        Ok(1)
                    }
                    2 => Err(std::io::Error::from(self.kind)),
                    _ => Ok(0),
                }
            }
        }
        let flag = AtomicBool::new(false);
        assert_eq!(
            read_output(
                InjectedReader {
                    call: 0,
                    kind: std::io::ErrorKind::Interrupted
                },
                &flag
            )
            .unwrap(),
            b"xx"
        );
        let failure = read_output(
            InjectedReader {
                call: 0,
                kind: std::io::ErrorKind::BrokenPipe,
            },
            &flag,
        )
        .unwrap_err();
        assert_eq!(failure.code, "HISTORY_INCOMPLETE");
        assert_eq!(failure.message, "Git output read failed");
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
        let objects = tree(&GitRoot::unpinned(root), &h.head).unwrap();
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

#[cfg(test)]
#[path = "git/metadata_tests.rs"]
mod metadata_tests;
