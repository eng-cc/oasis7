//! Crash-safe immutable publication and completion of durable selector batches.
use super::*;
const STAGING: &str = ".physical-staging-v1";
pub(super) fn publish(root: &Path, target: &Path, bytes: &[u8]) -> Result<(), FormalError> {
    let staging = root.join(STAGING);
    directory(&staging)?;
    let temporary = staging.join(super::super::fencing::fresh_challenge().map_err(io)?);
    create(&temporary, bytes)?;
    if target.parent() == Some(root.join(FRAMES).as_path()) {
        fault(1)?;
    }
    match fs::hard_link(&temporary, target) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if read(target, LIMIT)? != bytes {
                return Err(invalid("immutable physical publication differs"));
            }
            sync(target)?;
        }
        Err(e) => return Err(io(e)),
    }
    sync(
        target
            .parent()
            .ok_or_else(|| invalid("publication parent"))?,
    )?;
    if target.parent() == Some(root.join(SELECTORS).as_path()) {
        fault(2)?;
    }
    fs::remove_file(&temporary).map_err(io)?;
    sync(&staging)?;
    Ok(())
}
/// Caller holds the stable root's exclusive kernel fence and endpoint lease.
/// A durable selector binds a complete immutable recipe; finish all members,
/// including members not present in a retry's smaller object input set.
pub(super) fn recover(root: &Path) -> Result<(), FormalError> {
    #[cfg(unix)]
    let mut verified = std::collections::BTreeSet::new();
    for entry in fs::read_dir(root.join(SELECTORS)).map_err(io)? {
        let entry = entry.map_err(io)?;
        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("selector UTF8"))?;
        hash_hex(&id)?;
        let logical = root.join(&id);
        if selected_path(&logical, &id)?.parent() != Some(root) {
            continue;
        }
        #[cfg(unix)]
        {
            let metadata = entry.metadata().map_err(io)?;
            // Memoization is an optimization, never a whole-history inventory.
            // Eviction may repeat validation but cannot skip a required recipe.
            if verified.len() >= 4096 {
                verified.clear();
            }
            if !verified.insert((metadata.dev(), metadata.ino())) {
                continue;
            }
        }
        recover_names(root, vec![id])?;
    }
    Ok(())
}

pub(super) fn recover_objects(
    root: &Path,
    objects: &[(String, Vec<u8>)],
) -> Result<(), FormalError> {
    let mut names = Vec::new();
    for (id, _) in objects {
        hash_hex(id)?;
        if root.join(SELECTORS).join(id).try_exists().map_err(io)? {
            names.push(id.clone());
        }
    }
    recover_names(root, names)
}
fn recover_names(root: &Path, mut names: Vec<String>) -> Result<(), FormalError> {
    names.sort();
    #[cfg(unix)]
    let mut verified = std::collections::BTreeSet::new();
    for id in names {
        let selected = root.join(SELECTORS).join(&id);
        let metadata = fs::symlink_metadata(&selected).map_err(io)?;
        if !metadata.is_file() {
            return Err(invalid("selector recovery regular file"));
        }
        #[cfg(unix)]
        if !verified.insert((metadata.dev(), metadata.ino())) {
            continue;
        }
        let recipe = decode_recipe(&read(&selected, META_LIMIT)?)?;
        if recipe
            .objects
            .binary_search_by(|entry| entry.id.cmp(&id))
            .is_err()
        {
            return Err(invalid("selector recovery member mismatch"));
        }
        let mut incomplete = false;
        for object in &recipe.objects {
            for (path, required) in [
                (root.join(SELECTORS).join(&object.id), true),
                (root.join(&object.id), false),
            ] {
                match fs::symlink_metadata(path) {
                    Ok(m) => {
                        if !m.is_file() {
                            return Err(invalid("physical recovery alias type"));
                        }
                        #[cfg(unix)]
                        if (m.dev(), m.ino()) != (metadata.dev(), metadata.ino()) {
                            return Err(invalid("physical recovery alias identity"));
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => incomplete |= required,
                    Err(e) => return Err(io(e)),
                }
            }
        }
        // Complete immutable batches need no history-wide payload replay.
        // Ordinary logical reads still verify frames and full object hashes.
        if !incomplete {
            continue;
        }
        for object in &recipe.objects {
            let (_, bytes) = range(root, &recipe, &object.id, 0, LIMIT as u32)?;
            if blake3::hash(&bytes).to_hex().as_str() != object.id {
                return Err(invalid("selector recovery logical checksum"));
            }
        }
        sync(&selected)?;
        for frame in &recipe.frames {
            sync(&root.join(FRAMES).join(&frame.stored))?;
        }
        sync(&root.join(FRAMES))?;
        for object in &recipe.objects {
            for alias in [root.join(SELECTORS).join(&object.id)] {
                match fs::symlink_metadata(&alias) {
                    Ok(m) => {
                        if !m.is_file() {
                            return Err(invalid("selector recovery alias type"));
                        }
                        #[cfg(unix)]
                        if (m.dev(), m.ino()) != (metadata.dev(), metadata.ino()) {
                            return Err(invalid("selector recovery immutable alias changed"));
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        fs::hard_link(&selected, &alias).map_err(io)?;
                    }
                    Err(e) => return Err(io(e)),
                }
            }
        }
        sync(&root.join(SELECTORS))?;
        sync(root)?;
    }
    Ok(())
}

/// A run-local marker amortizes fsync only. Every requested object is still
/// decoded and hash checked before this function can observe a marker.
pub(super) fn sync_recipe_once(
    root: &Path,
    id: &str,
    recipe: &Recipe,
    markers: Option<&Path>,
) -> Result<(), FormalError> {
    let selected = root.join(SELECTORS).join(id);
    let marker = if let Some(markers) = markers {
        let metadata = fs::symlink_metadata(&selected).map_err(io)?;
        #[cfg(unix)]
        let key = format!(
            "physical-recipe-{}-{}-{}",
            blake3::hash(root.as_os_str().as_encoded_bytes()).to_hex(),
            metadata.dev(),
            metadata.ino()
        );
        #[cfg(not(unix))]
        let key = format!("physical-recipe-{id}");
        let marker = markers.join(key);
        match fs::symlink_metadata(&marker) {
            Ok(meta) if meta.is_file() => return Ok(()),
            Ok(_) => return Err(invalid("physical sync marker type")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io(e)),
        }
        Some(marker)
    } else {
        None
    };
    #[cfg(test)]
    RECIPE_SYNCS.with(|count| count.set(count.get() + 1));
    sync(&selected)?;
    for frame in &recipe.frames {
        sync(&root.join(FRAMES).join(&frame.stored))?;
    }
    sync(&root.join(FRAMES))?;
    sync(&root.join(SELECTORS))?;
    sync(root)?;
    if let Some(marker) = marker {
        create(&marker, b"")?;
    }
    Ok(())
}

#[cfg(test)]
thread_local! {static FAILURE:std::cell::Cell<u8>=const {std::cell::Cell::new(0)};}
pub(super) fn fault(_stage: u8) -> Result<(), FormalError> {
    #[cfg(test)]
    if FAILURE.with(|failure| {
        if failure.get() == _stage {
            failure.set(0);
            true
        } else {
            false
        }
    }) {
        return Err(FormalError::Io(
            "injected physical publication interruption".into(),
        ));
    }
    Ok(())
}
#[cfg(test)]
pub(super) fn fail_at(stage: u8) {
    FAILURE.with(|failure| failure.set(stage));
}

/// One publication guard invocation owns this private ephemeral scratch space.
/// It contains only fsync deduplication identities, never bytes or durable proof.
/// A crash may leave a system temporary directory; a later operation never reuses it.
pub(super) fn with_readback_markers<T>(
    run: impl FnOnce(&Path) -> Result<T, FormalError>,
) -> Result<T, FormalError> {
    let parent = std::env::temp_dir().canonicalize().map_err(io)?;
    if !fs::symlink_metadata(&parent).map_err(io)?.is_dir() {
        return Err(invalid("physical readback temporary parent"));
    }
    let path = parent.join(format!(
        "oasis7-physical-readback-{}",
        super::super::fencing::fresh_challenge().map_err(io)?
    ));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(&path).map_err(io)?;
    struct Owned(PathBuf);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let owned = Owned(path);
    let result = run(&owned.0);
    // Drop only the fresh directory this invocation successfully created.
    drop(owned);
    result
}
#[cfg(test)]
thread_local! {static RECIPE_SYNCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };}
#[cfg(test)]
pub(super) fn reset_recipe_syncs() {
    RECIPE_SYNCS.with(|count| count.set(0));
}
#[cfg(test)]
pub(super) fn recipe_syncs() -> usize {
    RECIPE_SYNCS.with(|count| count.get())
}
