use crate::{
    config::LocalConfig,
    error::{ContextError, Result},
    model::{SaveResult, WriteCondition},
    store::Store,
};
use std::{
    os::fd::AsFd,
    path::Path,
    process::{Command, Stdio},
};

pub fn edit(store: &Store, id: &str, actor: &str, config: &LocalConfig) -> Result<SaveResult> {
    let editor = config.editor.as_ref().ok_or_else(|| {
        ContextError::new(
            "EDITOR_NOT_CONFIGURED",
            "configure editor.program and editor.args in the local context configuration",
        )
    })?;
    let original = store.get(id)?;
    let draft_id = crate::fs::random_id();
    let drafts = store
        .directory()
        .create_dir(".context-local")?
        .create_dir("drafts")?;
    let draft_dir = drafts.create_dir_new(&draft_id)?;
    draft_dir.write_atomic("entry.md", &original.raw_bytes, false)?;
    let path = store
        .root
        .join(".context-local")
        .join("drafts")
        .join(&draft_id)
        .join("entry.md");
    if !store.directory().matches_path(&store.root)? {
        return Err(retained(
            "store directory moved before editor launch",
            &path,
        ));
    }
    let stderr = std::io::stderr().as_fd().try_clone_to_owned()?;
    let status = Command::new(&editor.program)
        .args(&editor.args)
        .arg(&path)
        .stdout(Stdio::from(stderr))
        .status()
        .map_err(|_| retained("editor could not be started", &path))?;
    if !status.success() {
        return Err(retained("editor exited unsuccessfully", &path));
    }
    let candidate = draft_dir
        .read("entry.md", crate::model::MAX_ENTRY_BYTES)
        .map_err(|mut error| {
            error.draft_path = Some(path.clone());
            error
        })?;
    let saved = store
        .put(
            id,
            &candidate,
            WriteCondition::Revision(original.revision),
            actor,
        )
        .map_err(|mut error| {
            error.draft_path = Some(path.clone());
            error
        })?;
    // A cleanup failure does not reverse an already durable save.
    if draft_dir.remove_file("entry.md").is_ok() {
        let _ = drafts.remove_dir(&draft_id);
    }
    Ok(saved)
}
fn retained(message: &str, path: &Path) -> ContextError {
    let mut error = ContextError::new("EDITOR_FAILED", message);
    error.draft_path = Some(path.to_path_buf());
    error
}
