use crate::{
    error::{ContextError, Result},
    format,
    fs::{Directory, WriteLock},
    model::*,
};
use std::path::{Path, PathBuf};
pub struct Store {
    pub root: PathBuf,
    pub config: StoreConfig,
    pub lock_timeout_ms: u64,
    pub verification_max_age_days: u32,
    dir: Directory,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_directory(root, Directory::open(root)?)
    }
    /// Open metadata from the original held directory, never reopen its pathname.
    pub fn open_directory(root: &Path, dir: Directory) -> Result<Self> {
        let config: StoreConfig =
            format::strict_json(&dir.read("context.json", MAX_METADATA_BYTES)?)?;
        config.validate()?;
        Ok(Self {
            root: root.into(),
            config,
            lock_timeout_ms: 0,
            verification_max_age_days: 30,
            dir,
        })
    }
    pub fn config_raw(&self) -> Result<Vec<u8>> {
        self.dir.read("context.json", MAX_METADATA_BYTES)
    }
    pub fn lock(&self, timeout_ms: u64) -> Result<WriteLock> {
        self.dir.lock(timeout_ms)
    }
    pub fn directory(&self) -> &Directory {
        &self.dir
    }
    pub fn init(root: &Path, project: &str, display_name: &str) -> Result<Self> {
        if root.exists() {
            Self::init_directory(root, Directory::open(root)?, project, display_name)
        } else {
            let parent = root
                .parent()
                .ok_or_else(|| ContextError::new("UNSAFE_PATH", "root needs parent"))?;
            Self::init_in_parent(root, &Directory::open(parent)?, project, display_name)
        }
    }
    fn new_config(root: &Path, project: &str, display_name: &str) -> Result<StoreConfig> {
        if !root.is_absolute() {
            return Err(ContextError::new("UNSAFE_PATH", "root must be absolute"));
        }
        validate_id(project)?;
        let config = StoreConfig {
            schema_version: "oasis7.context_store.v1".into(),
            store_id: crate::fs::random_id(),
            project_id: project.into(),
            name: display_name.into(),
        };
        config.validate()?;
        Ok(config)
    }
    pub fn init_directory(
        root: &Path,
        dir: Directory,
        project: &str,
        display_name: &str,
    ) -> Result<Self> {
        Self::init_directory_expected(root, dir, project, display_name, None)
    }
    /// `None` means the prepared directory had no store; never adopt one initialized later.
    pub fn init_directory_frozen(
        root: &Path,
        dir: Directory,
        project: &str,
        display_name: &str,
        expected_store_id: Option<&str>,
    ) -> Result<Self> {
        Self::init_directory_expected(root, dir, project, display_name, Some(expected_store_id))
    }
    fn init_directory_expected(
        root: &Path,
        dir: Directory,
        project: &str,
        display_name: &str,
        expected_store_id: Option<Option<&str>>,
    ) -> Result<Self> {
        let config = Self::new_config(root, project, display_name)?;
        match dir.read("context.json", MAX_METADATA_BYTES) {
            Ok(_) => {
                let store = Self::open_directory(root, dir)?;
                if expected_store_id
                    .is_some_and(|expected| expected != Some(store.config.store_id.as_str()))
                {
                    return Err(ContextError::new(
                        "REVISION_CONFLICT",
                        "store initialized or changed after preparation",
                    ));
                }
                if store.config.project_id != project {
                    return Err(ContextError::new(
                        "INVALID_INPUT",
                        "project differs from existing store",
                    ));
                }
                store.dir.create_dir(".context-local")?;
                return Ok(store);
            }
            Err(e) if e.code == "ENTRY_NOT_FOUND" => (),
            Err(e) => return Err(e),
        }
        if expected_store_id.flatten().is_some() {
            return Err(ContextError::new(
                "REVISION_CONFLICT",
                "prepared store configuration disappeared",
            ));
        }
        validate_initialization(&dir)?;
        let lock = dir.lock(0)?;
        validate_initialization(&dir).map_err(|mut e| {
            e.code = "INIT_INCOMPLETE".into();
            e.effect = "applied".into();
            e
        })?;
        Self::initialize(&dir, &config).map_err(|mut e| {
            if e.code != "PERSISTENCE_UNCERTAIN" {
                e.code = "INIT_INCOMPLETE".into();
                e.effect = "applied".into();
            }
            e
        })?;
        drop(lock);
        Self::open_directory(root, dir)
    }
    pub fn init_in_parent(
        root: &Path,
        parent: &Directory,
        project: &str,
        display_name: &str,
    ) -> Result<Self> {
        let config = Self::new_config(root, project, display_name)?;
        let target = root
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| ContextError::new("UNSAFE_PATH", "invalid root filename"))?;
        let stage = format!(".context-stage-{}", crate::fs::random_id());
        let dir = parent.create_dir_new(&stage)?;
        Self::initialize(&dir, &config)?;
        dir.sync()?;
        parent.publish_new_directory(&stage, target)?;
        Self::open_directory(root, dir)
    }
    fn initialize(d: &Directory, c: &StoreConfig) -> Result<()> {
        Self::initialize_with(d, c, |_| Ok(()))
    }
    fn initialize_with(
        d: &Directory,
        c: &StoreConfig,
        mut checkpoint: impl FnMut(&str) -> Result<()>,
    ) -> Result<()> {
        let e = d.create_dir("entries")?;
        if let Err(err) = e.read(".gitkeep", 1) {
            if err.code != "ENTRY_NOT_FOUND" {
                return Err(err);
            }
            e.write_atomic(".gitkeep", b"", false)?;
        }
        d.create_dir(".context-local")?;
        for (file, rules) in [
            (
                ".gitignore",
                vec!["/.context-local/", "/entries/.context-stage-*"],
            ),
            (".gitattributes", vec!["entries/*.md text eol=lf"]),
        ] {
            let old = match d.read(file, MAX_METADATA_BYTES) {
                Ok(b) => b,
                Err(e) if e.code == "ENTRY_NOT_FOUND" => Vec::new(),
                Err(e) => return Err(e),
            };
            let mut s = String::from_utf8(old.clone())
                .map_err(|_| ContextError::new("INVALID_FORMAT", "invalid auxiliary file"))?;
            for rule in rules {
                if !s.lines().any(|x| x == rule) {
                    if !s.is_empty() && !s.ends_with('\n') {
                        s.push('\n')
                    }
                    s.push_str(rule);
                    s.push('\n');
                }
            }
            d.write_atomic(
                file,
                s.as_bytes(),
                !old.is_empty() || d.names()?.contains(&file.to_string()),
            )?;
        }
        checkpoint("before_configuration")?;
        let b = serde_json::to_vec_pretty(c).unwrap();
        d.write_atomic("context.json", &b, false)?;
        d.sync()
    }
    pub fn get(&self, id: &str) -> Result<EntrySnapshot> {
        validate_id(id)?;
        let bytes = self
            .dir
            .child("entries")?
            .read(&format!("{id}.md"), MAX_ENTRY_BYTES)?;
        self.snapshot(id, bytes)
    }
    fn snapshot(&self, id: &str, bytes: Vec<u8>) -> Result<EntrySnapshot> {
        let entry = format::parse(&bytes)?;
        if entry.metadata.id != id || entry.metadata.project_id != self.config.project_id {
            return Err(ContextError::new(
                "INVALID_FORMAT",
                "entry identity differs from store",
            ));
        }
        let verification_state = format::verification_state(&entry, self.verification_max_age_days);
        Ok(EntrySnapshot {
            revision: format::revision(&bytes),
            raw_bytes: bytes,
            entry,
            verification_state,
        })
    }
    pub fn scan(&self) -> Result<(Vec<EntrySnapshot>, Vec<String>)> {
        let e = self.dir.child("entries")?;
        let mut results = Vec::new();
        let mut warnings = Vec::new();
        let mut total = 0usize;
        let mut count = 0usize;
        for name in e.names()? {
            if name == ".gitkeep" || name.starts_with(".context-stage-") {
                continue;
            }
            let Some(id) = name.strip_suffix(".md") else {
                warnings.push("unexpected entry directory object".into());
                continue;
            };
            if validate_id(id).is_err() {
                warnings.push("invalid entry filename".into());
                continue;
            }
            count += 1;
            if count > MAX_ENTRIES {
                return Err(ContextError::new(
                    "LIMIT_EXCEEDED",
                    "store entry count exceeds limit",
                ));
            }
            match e.read(&name, MAX_ENTRY_BYTES) {
                Ok(bytes) => {
                    total = total.checked_add(bytes.len()).ok_or_else(|| {
                        ContextError::new("LIMIT_EXCEEDED", "store exceeds limit")
                    })?;
                    if total > MAX_STORE_BYTES {
                        return Err(ContextError::new(
                            "LIMIT_EXCEEDED",
                            "store bytes exceed limit",
                        ));
                    }
                    match self.snapshot(id, bytes) {
                        Ok(x) => results.push(x),
                        Err(_) => warnings.push(format!("invalid entry: {id}")),
                    }
                }
                Err(err) if err.code == "CAPACITY_EXCEEDED" || err.code == "LIMIT_EXCEEDED" => {
                    return Err(err);
                }
                Err(_) => warnings.push(format!("unreadable entry: {id}")),
            }
        }
        Ok((results, warnings))
    }
    pub fn read_all(&self) -> Result<Vec<EntrySnapshot>> {
        let (entries, warnings) = self.scan()?;
        if !warnings.is_empty() {
            return Err(ContextError::new(
                "INVALID_FORMAT",
                "store contains invalid entries",
            ));
        }
        Ok(entries)
    }
    pub fn put(
        &self,
        id: &str,
        candidate: &[u8],
        condition: WriteCondition,
        actor: &str,
    ) -> Result<SaveResult> {
        validate_id(id)?;
        let entry = format::parse(candidate)?;
        if entry.metadata.id != id || entry.metadata.project_id != self.config.project_id {
            return Err(ContextError::new(
                "INVALID_INPUT",
                "candidate identity differs from store",
            ));
        }
        let _lock = self.lock(self.lock_timeout_ms)?;
        self.save_locked(id, entry, condition, actor)
    }
    fn save_locked(
        &self,
        id: &str,
        mut entry: Entry,
        condition: WriteCondition,
        actor: &str,
    ) -> Result<SaveResult> {
        if actor.is_empty() || actor.len() > 128 || actor.contains('\0') {
            return Err(ContextError::new("INVALID_INPUT", "invalid actor label"));
        }
        let current = match self.get(id) {
            Ok(c) => Some(c),
            Err(e) if e.code == "ENTRY_NOT_FOUND" => None,
            Err(e) => return Err(e),
        };
        let conflict = match (&condition, &current) {
            (WriteCondition::Absent, None) => false,
            (WriteCondition::Revision(r), Some(c)) if r == &c.revision => false,
            _ => true,
        };
        if conflict {
            let mut e = ContextError::new(
                "REVISION_CONFLICT",
                "write precondition does not match current entry",
            );
            e.current_revision = current.as_ref().map(|x| x.revision.clone());
            return Err(e);
        }
        if let Some(c) = &current {
            if format::content_equal_for_save(&c.entry, &entry) {
                return Ok(SaveResult {
                    revision: c.revision.clone(),
                    persistence: "unchanged".into(),
                    changed: false,
                });
            }
        }
        entry.metadata.updated_at = format::now();
        entry.metadata.updated_by = actor.into();
        let bytes = format::encode(&entry)?;
        let entries = self.read_all()?;
        let total = entries
            .iter()
            .try_fold(0usize, |n, e| n.checked_add(e.raw_bytes.len()))
            .ok_or_else(|| ContextError::new("LIMIT_EXCEEDED", "store exceeds limit"))?;
        let final_total = total
            .checked_sub(current.as_ref().map_or(0, |e| e.raw_bytes.len()))
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or_else(|| ContextError::new("LIMIT_EXCEEDED", "store exceeds limit"))?;
        if final_total > MAX_STORE_BYTES
            || entries.len() + usize::from(current.is_none()) > MAX_ENTRIES
        {
            return Err(ContextError::new(
                "LIMIT_EXCEEDED",
                "store exceeds capacity",
            ));
        }
        self.dir
            .child("entries")?
            .write_atomic(&format!("{id}.md"), &bytes, current.is_some())?;
        Ok(SaveResult {
            revision: format::revision(&bytes),
            persistence: "saved".into(),
            changed: true,
        })
    }
    pub fn archive(&self, id: &str, revision: &str, actor: &str) -> Result<SaveResult> {
        let _lock = self.lock(self.lock_timeout_ms)?;
        let mut e = self.get(id)?.entry;
        e.metadata.state = "archived".into();
        self.save_locked(id, e, WriteCondition::Revision(revision.into()), actor)
    }
    pub fn verify(
        &self,
        id: &str,
        revision: &str,
        method: &str,
        evidence: &str,
        observed_at: Option<&str>,
        actor: &str,
    ) -> Result<SaveResult> {
        let _lock = self.lock(self.lock_timeout_ms)?;
        let mut e = self.get(id)?.entry;
        let subject = format::subject_sha256(&e);
        e.metadata.verification = Some(Verification {
            method: method.into(),
            observed_at: observed_at.map(str::to_owned).unwrap_or_else(format::now),
            observer: actor.into(),
            evidence_ref: evidence.into(),
            subject_sha256: subject,
        });
        e.metadata.validate()?;
        self.save_locked(id, e, WriteCondition::Revision(revision.into()), actor)
    }
    pub fn doctor(&self) -> Result<serde_json::Value> {
        let (entries, mut warnings) = self.scan()?;
        if !self.dir.private_permissions()? {
            warnings.push("store directory permits group or other access".into());
        }
        if !self.dir.file_private_permissions("context.json")? {
            warnings.push("store configuration permits group or other access".into());
        }
        let ed = self.dir.child("entries")?;
        if !ed.private_permissions()? {
            warnings.push("entries directory permits group or other access".into());
        }
        for name in ed.names()? {
            if name.ends_with(".md") && ed.file_private_permissions(&name).is_ok_and(|p| !p) {
                warnings.push("entry file permits group or other access".into());
            }
            if name.starts_with(".context-stage-") {
                warnings.push("residual staged entry file".into());
            }
        }
        if self.dir.names()?.iter().any(|n| n == ".context-local") {
            match self.dir.child(".context-local") {
                Ok(local) => {
                    if !local.private_permissions()? {
                        warnings.push("local state directory permits group or other access".into());
                    }
                    for name in local.names()? {
                        match name.as_str() {
                            "write.lock" => {
                                if local.read(&name, 1).is_err() {
                                    warnings.push("invalid write lock".into());
                                }
                            }
                            "drafts" => match local.child(&name) {
                                Ok(d) => {
                                    if !d.names()?.is_empty() {
                                        warnings.push("residual edit drafts".into());
                                    }
                                }
                                Err(_) => warnings.push("invalid drafts directory".into()),
                            },
                            _ => warnings.push("unexpected local state object".into()),
                        }
                    }
                }
                Err(_) => warnings.push("invalid local state directory".into()),
            }
        }
        let history = match crate::git::status_pinned(&self.root, self.directory()) {
            Ok(s) => serde_json::to_value(s).unwrap(),
            Err(e) => serde_json::json!({"available":false,"code":e.code}),
        };
        let ids: std::collections::HashSet<_> = entries
            .iter()
            .map(|e| e.entry.metadata.id.as_str())
            .collect();
        for e in &entries {
            for l in &e.entry.metadata.links {
                if !ids.contains(l.entry_id.as_str()) {
                    warnings.push(format!("dangling link in {}", e.entry.metadata.id))
                }
            }
            if e.entry.body.lines().any(|x| {
                x.starts_with("<<<<<<<") || x.starts_with("=======") || x.starts_with(">>>>>>>")
            }) {
                warnings.push(format!("conflict marker in {}", e.entry.metadata.id));
            }
        }
        Ok(
            serde_json::json!({"healthy":warnings.is_empty(),"entry_count":entries.len(),"warnings":warnings,"history":history}),
        )
    }
}

fn validate_initialization(d: &Directory) -> Result<()> {
    for n in d.names()? {
        if ![
            ".git",
            ".gitignore",
            ".gitattributes",
            "entries",
            ".context-local",
        ]
        .contains(&n.as_str())
        {
            return Err(ContextError::new(
                "INVALID_INPUT",
                "initialization directory contains unexpected files",
            ));
        }
    }
    if d.names()?.iter().any(|n| n == "entries") {
        let e = d.child("entries")?;
        for n in e.names()? {
            if n != ".gitkeep" || !e.read(&n, 1)?.is_empty() {
                return Err(ContextError::new(
                    "INVALID_INPUT",
                    "entries directory is not empty",
                ));
            }
        }
    }
    if d.names()?.iter().any(|n| n == ".git") && d.child(".git").is_err() {
        let git = d.read(".git", MAX_METADATA_BYTES)?;
        if !git.starts_with(b"gitdir: ") {
            return Err(ContextError::new(
                "INVALID_INPUT",
                "invalid Git management entry",
            ));
        }
    }

    if !d.names()?.iter().any(|n| n == ".context-local") {
        return Ok(());
    }
    let local = d.child(".context-local")?;
    for n in local.names()? {
        match n.as_str() {
            "write.lock" => {
                if !local.read(&n, 1)?.is_empty() {
                    return Err(ContextError::new(
                        "INVALID_INPUT",
                        "invalid local lock file",
                    ));
                }
            }
            "drafts" => {
                let drafts = local.child(&n)?;
                for draft in drafts.names()? {
                    if draft.len() != 32
                        || !draft
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        return Err(ContextError::new(
                            "INVALID_INPUT",
                            "invalid draft directory",
                        ));
                    }
                    let dd = drafts.child(&draft)?;
                    for file in dd.names()? {
                        if file != "entry.md" {
                            return Err(ContextError::new(
                                "INVALID_INPUT",
                                "unexpected draft object",
                            ));
                        }
                        dd.read(&file, MAX_ENTRY_BYTES)?;
                    }
                }
            }
            _ => {
                return Err(ContextError::new(
                    "INVALID_INPUT",
                    "unexpected local state object",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn interrupted_initialization_retries_and_preserves_auxiliary_content() {
        let temp = tempfile::tempdir().unwrap();
        let d = Directory::open(temp.path()).unwrap();
        std::fs::write(
            temp.path().join(".gitignore"),
            "# existing\n/private-note\n",
        )
        .unwrap();
        std::fs::write(
            temp.path().join(".gitattributes"),
            "# existing attributes\n*.txt text\n",
        )
        .unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o750)).unwrap();
        let c = StoreConfig {
            schema_version: "oasis7.context_store.v1".into(),
            store_id: crate::fs::random_id(),
            project_id: "test".into(),
            name: "Test".into(),
        };
        assert!(
            Store::initialize_with(&d, &c, |_| Err(ContextError::new(
                "IO_ERROR",
                "injected interruption"
            )))
            .is_err()
        );
        assert!(!temp.path().join("context.json").exists());
        let before = d.read(".gitignore", 16384).unwrap();
        Store::init(temp.path(), "test", "Test").unwrap();
        assert_eq!(d.read(".gitignore", 16384).unwrap(), before);
        assert!(
            String::from_utf8(d.read(".gitattributes", 16384).unwrap())
                .unwrap()
                .starts_with("# existing attributes\n*.txt text\n")
        );
        assert_eq!(
            std::fs::metadata(temp.path()).unwrap().permissions().mode() & 0o777,
            0o750
        );
        assert_eq!(
            std::fs::metadata(temp.path().join("entries"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(temp.path().join("context.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
