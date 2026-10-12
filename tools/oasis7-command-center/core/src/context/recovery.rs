//! Exact, nonsecret recovery evidence. Reconciliation observes facts and never repeats writes.
use super::*;
use oasis7_context::{format, model::Entry};

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextRecoveryEvidence {
    Entry {
        binding: ContextBinding,
        entry_id: String,
        before_revision: Option<String>,
        desired_digest: String,
    },
    Binding {
        root: String,
        registered_project_id: String,
        expected_store_id: Option<String>,
        expected_project_id: String,
        root_identity: Option<String>,
        parent_identity: String,
    },
    SnapshotExport {
        binding: ContextBinding,
        output_path: String,
        parent_identity: String,
        selection: Vec<String>,
        revisions: Vec<(String, String)>,
        source_digest: String,
    },
    SnapshotRestore {
        binding: ContextRestoreIdentity,
        target_path: String,
        parent_identity: String,
        input_sha256: String,
        #[serde(default)]
        staging_name: Option<String>,
        revisions: Vec<(String, String)>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ContextReconciliation {
    pub state: String,
    pub reason: String,
}
fn conclusion(state: &str, reason: &str) -> ContextReconciliation {
    ContextReconciliation {
        state: state.into(),
        reason: reason.into(),
    }
}
fn absent(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(Error::Io(e)),
    }
}
fn semantic_digest(entry: &Entry) -> Result<String> {
    let mut metadata = serde_json::to_value(&entry.metadata)?;
    let object = metadata.as_object_mut().expect("metadata object");
    object.remove("updated_at");
    object.remove("updated_by");
    Ok(crate::index::digest(&format::canonical_json(
        &serde_json::json!({"metadata":metadata,"body":entry.body.replace("\r\n","\n")}),
    )))
}
fn revisions(
    entries: &[oasis7_context::model::EntrySnapshot],
    selection: &[String],
) -> Vec<(String, String)> {
    let mut result: Vec<_> = entries
        .iter()
        .filter(|e| selection.is_empty() || selection.contains(&e.entry.metadata.id))
        .map(|e| (e.entry.metadata.id.clone(), e.revision.clone()))
        .collect();
    result.sort();
    result
}
pub fn file_digest(path: &Path) -> Result<String> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    let before = file.metadata()?;
    const LIMIT: u64 = 128 * 1024 * 1024;
    if !path.is_absolute() || !before.is_file() || before.len() > LIMIT {
        return Err(failure("bounded ordinary snapshot file required"));
    }
    let mut bytes = Vec::new();
    file.by_ref().take(LIMIT + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 > LIMIT
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err(failure("snapshot input changed while reading"));
    }
    Ok(crate::index::digest(&bytes))
}
impl ContextService {
    pub fn save_evidence(&self, request: &ContextSaveRequest) -> Result<ContextRecoveryEvidence> {
        let binding = self.validate_binding(&request.binding_id, request.binding_version)?;
        let candidate = format::parse(request.raw_text.as_bytes()).map_err(Error::Context)?;
        if candidate.metadata.id != request.id
            || candidate.metadata.project_id != binding.project_id
        {
            return Err(failure("candidate identity differs from binding"));
        }
        Ok(ContextRecoveryEvidence::Entry {
            binding,
            entry_id: request.id.clone(),
            before_revision: request.expected_revision.clone(),
            desired_digest: semantic_digest(&candidate)?,
        })
    }
    pub fn archive_evidence(
        &self,
        id: &str,
        version: u64,
        entry_id: &str,
        revision: &str,
    ) -> Result<ContextRecoveryEvidence> {
        let binding = self.validate_binding(id, version)?;
        let mut entry = self
            .store(id, version)?
            .get(entry_id)
            .map_err(Error::Context)?
            .entry;
        entry.metadata.state = "archived".into();
        Ok(ContextRecoveryEvidence::Entry {
            binding,
            entry_id: entry_id.into(),
            before_revision: Some(revision.into()),
            desired_digest: semantic_digest(&entry)?,
        })
    }
    pub fn restore_entry_evidence(
        &self,
        id: &str,
        version: u64,
        entry_id: &str,
        commit: &str,
        revision: Option<&str>,
    ) -> Result<ContextRecoveryEvidence> {
        let binding = self.validate_binding(id, version)?;
        let store = self.store(id, version)?;
        let old = oasis7_context::git::read_at_commit_pinned(
            &store.root,
            store.directory(),
            entry_id,
            commit,
        )
        .map_err(failure)?;
        let config: oasis7_context::model::StoreConfig =
            format::strict_json(&old.config_bytes).map_err(Error::Context)?;
        if config.store_id != binding.store_id || config.project_id != binding.project_id {
            return Err(failure("historical store identity differs"));
        }
        let entry = format::parse(&old.entry_bytes).map_err(Error::Context)?;
        Ok(ContextRecoveryEvidence::Entry {
            binding,
            entry_id: entry_id.into(),
            before_revision: revision.map(str::to_owned),
            desired_digest: semantic_digest(&entry)?,
        })
    }
    pub fn binding_evidence(
        &self,
        path: &Path,
        registered_project_id: &str,
        create: bool,
    ) -> Result<ContextRecoveryEvidence> {
        let root = canonical_candidate(path)?;
        check_unwritten_ordinary_overlap(&self.db, &root)?;
        let parent = root
            .parent()
            .ok_or_else(|| failure("binding root needs parent"))?;
        let parent_dir = oasis7_context::fs::Directory::open(parent).map_err(Error::Context)?;
        let parent_identity = held_directory_identity(&parent_dir)?;
        let (config, root_identity) = if root.exists() {
            let dir = parent_dir
                .child(
                    root.file_name()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| failure("invalid binding root"))?,
                )
                .map_err(Error::Context)?;
            let identity = held_directory_identity(&dir)?;
            let config = match dir.read("context.json", oasis7_context::model::MAX_METADATA_BYTES) {
                Ok(bytes) => {
                    let config: oasis7_context::model::StoreConfig =
                        format::strict_json(&bytes).map_err(Error::Context)?;
                    config.validate().map_err(Error::Context)?;
                    Some(config)
                }
                Err(e) if create && e.code == "ENTRY_NOT_FOUND" => None,
                Err(e) => return Err(Error::Context(e)),
            };
            if !dir.matches_path(&root).map_err(Error::Context)? {
                return Err(failure("binding root changed during preparation"));
            }
            (config, Some(identity))
        } else {
            (None, None)
        };
        if !parent_dir.matches_path(parent).map_err(Error::Context)? {
            return Err(failure("binding parent changed during preparation"));
        }
        Ok(ContextRecoveryEvidence::Binding {
            root: root.display().to_string(),
            registered_project_id: registered_project_id.into(),
            expected_store_id: config.as_ref().map(|c| c.store_id.clone()),
            root_identity,
            parent_identity,
            expected_project_id: config
                .map(|c| c.project_id)
                .unwrap_or_else(|| registered_project_id.into()),
        })
    }
    pub fn snapshot_export_evidence(
        &self,
        id: &str,
        version: u64,
        output: &Path,
        selection: &[String],
    ) -> Result<ContextRecoveryEvidence> {
        let binding = self.validate_binding(id, version)?;
        let store = self.store(id, version)?;
        let entries = store.read_all().map_err(Error::Context)?;
        if selection
            .iter()
            .any(|id| !entries.iter().any(|e| &e.entry.metadata.id == id))
        {
            return Err(failure("snapshot selection missing"));
        }
        let output = canonical_candidate(output)?;
        if output.exists() {
            return Err(failure("snapshot output already exists"));
        }
        Ok(ContextRecoveryEvidence::SnapshotExport {
            binding,
            output_path: output.display().to_string(),
            parent_identity: directory_identity(
                output
                    .parent()
                    .ok_or_else(|| failure("output needs parent"))?,
            )?,
            selection: selection.to_vec(),
            revisions: revisions(&entries, selection),
            source_digest: oasis7_context::snapshot::source_digest(
                &store.config,
                &entries,
                selection,
            ),
        })
    }
    pub fn snapshot_input_identity(
        &self,
        id: &str,
        version: u64,
        selection: &[String],
    ) -> Result<String> {
        let store = self.store(id, version)?;
        Ok(oasis7_context::snapshot::source_digest(
            &store.config,
            &store.read_all().map_err(Error::Context)?,
            selection,
        ))
    }
    pub fn snapshot_export_frozen(
        &self,
        id: &str,
        version: u64,
        output: &Path,
        selection: &[String],
        passphrase: &[u8],
        expected_digest: &str,
    ) -> Result<serde_json::Value> {
        let parent = output
            .parent()
            .ok_or_else(|| failure("output needs parent"))?;
        let expected_parent = directory_identity(parent)?;
        self.snapshot_export_frozen_parent(
            id,
            version,
            output,
            selection,
            passphrase,
            expected_digest,
            &expected_parent,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn snapshot_export_frozen_parent(
        &self,
        id: &str,
        version: u64,
        output: &Path,
        selection: &[String],
        passphrase: &[u8],
        expected_digest: &str,
        expected_parent: &str,
    ) -> Result<serde_json::Value> {
        let parent = output
            .parent()
            .ok_or_else(|| failure("output needs parent"))?;
        let dir = oasis7_context::fs::Directory::open(parent).map_err(Error::Context)?;
        if held_directory_identity(&dir)? != expected_parent
            || !dir.matches_path(parent).map_err(Error::Context)?
        {
            return Err(failure("snapshot output parent changed after preparation"));
        }
        let store = self.store(id, version)?;
        Ok(serde_json::to_value(
            oasis7_context::snapshot::export_to_directory(
                &store,
                output,
                selection,
                passphrase,
                0,
                Some(expected_digest),
                &dir,
            )
            .map_err(Error::Context)?,
        )?)
    }
    pub fn snapshot_restore_evidence(
        &self,
        id: &str,
        version: u64,
        input: &Path,
        target: &Path,
        passphrase: &[u8],
    ) -> Result<ContextRecoveryEvidence> {
        let binding = self.binding_record(id, version)?;
        let source = ContextSnapshotRestoreSource {
            binding_id: Some(id.into()),
            binding_version: Some(version),
            registered_project_id: binding.registered_project_id,
        };
        let digest = file_digest(input)?;
        self.restore_snapshot_evidence(&source, input, target, passphrase, &digest)
    }
    pub fn restore_snapshot_evidence(
        &self,
        source: &ContextSnapshotRestoreSource,
        input: &Path,
        target: &Path,
        passphrase: &[u8],
        expected_input_sha256: &str,
    ) -> Result<ContextRecoveryEvidence> {
        let digest = expected_input_sha256.to_owned();
        let identity = self.restore_identity(source, input, passphrase, &digest)?;
        let (_, mut revisions) =
            oasis7_context::snapshot::inspect_revisions_frozen(input, passphrase, &digest)
                .map_err(Error::Context)?;
        let target = canonical_candidate(target)?;
        check_unwritten_ordinary_overlap(&self.db, &target)?;
        if target.exists() {
            return Err(failure("restore target already exists"));
        }
        revisions.sort();
        Ok(ContextRecoveryEvidence::SnapshotRestore {
            binding: identity,
            target_path: target.display().to_string(),
            parent_identity: directory_identity(
                target
                    .parent()
                    .ok_or_else(|| failure("restore target needs parent"))?,
            )?,
            input_sha256: digest,
            staging_name: Some(oasis7_context::snapshot::new_restore_stage()),
            revisions,
        })
    }
    pub fn reconcile(
        &self,
        evidence: &ContextRecoveryEvidence,
        passphrase: Option<&[u8]>,
    ) -> Result<ContextReconciliation> {
        match evidence {
            ContextRecoveryEvidence::Entry {
                binding,
                entry_id,
                before_revision,
                desired_digest,
            } => {
                let store = self.store(&binding.id, binding.binding_version)?;
                match store.get(entry_id) {
                    Ok(current) if semantic_digest(&current.entry)? == *desired_digest => {
                        Ok(conclusion(
                            "succeeded",
                            "Current entry exactly matches intended content and metadata",
                        ))
                    }
                    Ok(current) if before_revision.as_ref() == Some(&current.revision) => {
                        Ok(conclusion(
                            "failed",
                            "Original revision remains; intended change is absent",
                        ))
                    }
                    Err(e) if e.code == "ENTRY_NOT_FOUND" && before_revision.is_none() => {
                        Ok(conclusion("failed", "Create target remains absent"))
                    }
                    _ => Ok(conclusion(
                        "unknown",
                        "Current revision cannot establish original outcome",
                    )),
                }
            }
            ContextRecoveryEvidence::Binding {
                root,
                registered_project_id,
                expected_store_id,
                expected_project_id,
                root_identity,
                parent_identity,
            } => {
                let target = Path::new(root);
                if directory_identity(
                    target
                        .parent()
                        .ok_or_else(|| failure("binding needs parent"))?,
                )? != *parent_identity
                    || root_identity.as_ref().is_some_and(|identity| {
                        directory_identity(target).is_ok_and(|actual| &actual != identity)
                    })
                {
                    return Ok(conclusion(
                        "unknown",
                        "Original selected directory identity changed",
                    ));
                }
                let candidate = self.bindings()?.into_iter().find(|b| {
                    b.root == *root
                        && b.registered_project_id == *registered_project_id
                        && b.project_id == *expected_project_id
                        && expected_store_id
                            .as_ref()
                            .is_none_or(|id| &b.store_id == id)
                });
                if let Some(binding) = candidate {
                    self.validate_binding(&binding.id, binding.binding_version)?;
                    return Ok(conclusion(
                        "succeeded",
                        "Exact library identity and project relation are registered",
                    ));
                }
                if absent(&Path::new(root).join("context.json"))? {
                    return Ok(conclusion(
                        "unknown",
                        &format!(
                            "原资料库未完成初始化或绑定；无法确认是否保留初始化暂存目录。原目标：{}；原父目录：{}。请人工核对该父目录中的 .context-stage-*；不会自动打开、清理或重放。",
                            root,
                            Path::new(root)
                                .parent()
                                .ok_or_else(|| failure("binding root needs parent"))?
                                .display()
                        ),
                    ));
                }
                let store = application::open(Some(Path::new(root))).map_err(Error::Context)?;
                if store.config.project_id == *expected_project_id
                    && expected_store_id
                        .as_ref()
                        .is_none_or(|id| &store.config.store_id == id)
                {
                    return Ok(conclusion(
                        "partial",
                        "资料库已初始化，绑定未完成；明确连接现有目录完成绑定",
                    ));
                }
                Ok(conclusion(
                    "unknown",
                    "Library identity differs; original binding outcome is unconfirmed",
                ))
            }
            ContextRecoveryEvidence::SnapshotExport {
                binding,
                output_path,
                parent_identity,
                selection,
                revisions: expected,
                ..
            } => {
                if directory_identity(
                    Path::new(output_path)
                        .parent()
                        .ok_or_else(|| failure("output needs parent"))?,
                )? != *parent_identity
                {
                    return Ok(conclusion(
                        "unknown",
                        "Original output parent identity changed",
                    ));
                }
                if absent(Path::new(output_path))? {
                    return Ok(conclusion("failed", "Exact output file is absent"));
                }
                let Some(passphrase) = passphrase else {
                    return Ok(conclusion(
                        "unknown",
                        "A fresh passphrase is required to verify the original encrypted output",
                    ));
                };
                let (summary, mut actual) =
                    oasis7_context::snapshot::inspect_revisions(Path::new(output_path), passphrase)
                        .map_err(Error::Context)?;
                actual.sort();
                if summary.store_id == binding.store_id
                    && summary.project_id == binding.project_id
                    && summary.scope
                        == if selection.is_empty() {
                            "all"
                        } else {
                            "selection"
                        }
                    && actual == *expected
                {
                    return Ok(conclusion(
                        "succeeded",
                        "Encrypted output validates exact source revisions and scope",
                    ));
                }
                Ok(conclusion(
                    "unknown",
                    "Encrypted output does not match frozen source revisions and scope",
                ))
            }
            ContextRecoveryEvidence::SnapshotRestore {
                binding: identity,
                target_path,
                parent_identity,
                staging_name,
                revisions: expected,
                ..
            } => {
                if let Some(stage) = staging_name {
                    oasis7_context::snapshot::validate_restore_stage(stage)
                        .map_err(Error::Context)?;
                }
                let target = Path::new(target_path);
                if directory_identity(
                    target
                        .parent()
                        .ok_or_else(|| failure("restore target needs parent"))?,
                )? != *parent_identity
                {
                    return Ok(conclusion(
                        "unknown",
                        "Original restore parent identity changed",
                    ));
                }
                let parent = target
                    .parent()
                    .ok_or_else(|| failure("restore target needs parent"))?;
                if let Some(stage) = staging_name {
                    let staging_path = parent.join(stage);
                    if !absent(&staging_path)? {
                        return Ok(conclusion(
                            "unknown",
                            &format!(
                                "原恢复暂存目录仍在，可能含私有明文：{}。原目标：{}。请人工核对原目录；不会自动打开、清理或重放。",
                                staging_path.display(),
                                target.display()
                            ),
                        ));
                    }
                }
                if absent(target)? {
                    let reference = staging_name
                        .as_ref()
                        .map(|stage| parent.join(stage).display().to_string())
                        .unwrap_or_else(|| "旧记录没有暂存目录身份".into());
                    return Ok(conclusion(
                        "unknown",
                        &format!(
                            "原恢复目标未发布，无法证明暂存明文未移动或未移除。原目标：{}；原暂存引用：{}；原父目录：{}。请人工核对；不会重放。",
                            target.display(),
                            reference,
                            parent.display()
                        ),
                    ));
                }
                let store = application::open(Some(target)).map_err(Error::Context)?;
                if store.config.store_id != identity.store_id
                    || store.config.project_id != identity.project_id
                {
                    return Ok(conclusion("unknown", "Restored store identity differs"));
                }
                if revisions(&store.read_all().map_err(Error::Context)?, &[]) == *expected {
                    let registered = self.bindings()?.into_iter().find(|candidate| {
                        candidate.root == *target_path
                            && candidate.store_id == identity.store_id
                            && candidate.project_id == identity.project_id
                            && candidate.registered_project_id == identity.registered_project_id
                    });
                    if let Some(registered) = registered {
                        self.validate_binding(&registered.id, registered.binding_version)?;
                        return Ok(conclusion(
                            "succeeded",
                            "New directory contains exact snapshot revisions and registered project relation",
                        ));
                    }
                    return Ok(conclusion(
                        "partial",
                        "资料文件已准确恢复，绑定未完成；明确连接原恢复目录完成绑定",
                    ));
                }
                Ok(conclusion(
                    "unknown",
                    "New directory content does not match original snapshot",
                ))
            }
        }
    }
}
