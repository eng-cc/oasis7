//! Bounded source retirement after the new immutable closure is selected.
use super::*;
#[path = "physical_generation_cleanup.rs"]
mod cleanup;
pub(super) fn retire(
    root: &Path,
    state: &mut Build,
    selected: &str,
) -> Result<FormalGenerationProgress, FormalError> {
    let _fence = pin(root, true)?;
    if current(root)?.as_deref() != Some(selected) {
        return Err(invalid("generation retire lost selector"));
    }
    let target = manifest(root, selected)?;
    if target.nonce != state.nonce
        || target.catalogs != state.completed
        || state.objects != state.completed
    {
        return Err(invalid("generation retire target coverage"));
    }
    let dir = root.join(GENERATIONS).join(selected);
    let ids = catalog::page(&dir, &target.catalogs, state.retire_after.as_deref(), 1024)?;
    let mut size = 0usize;
    let mut done = 0;
    for id in &ids {
        let target_bytes = crate::controlled_authority::formal::business_pack::read_object_durable(
            &dir.join(id),
            id,
        )?
        .ok_or_else(|| invalid("generation retirement target missing"))?;
        if done > 0 && size.saturating_add(target_bytes.len()) > BATCH {
            break;
        }
        size = size
            .checked_add(target_bytes.len())
            .ok_or_else(|| invalid("generation retirement byte overflow"))?;
        for path in [root.join(SELECTORS).join(id), root.join(id)] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(e)),
            }
        }
        publication::fault(6)?;
        state.retire_after = Some(id.clone());
        done += 1;
    }
    sync(&root.join(SELECTORS))?;
    sync(root)?;
    save(root, state)?;
    if done < ids.len() || ids.len() == 1024 {
        return progress(root, state);
    }
    if let Some(previous) = &state.previous {
        if previous == selected {
            return Err(invalid("cannot retire selected generation"));
        }
        let old = state
            .previous_manifest
            .as_ref()
            .ok_or_else(|| invalid("generation original manifest absent"))?;
        if blake3::hash(&serde_cbor::to_vec(old).map_err(io)?)
            .to_hex()
            .as_str()
            != previous
        {
            return Err(invalid("generation original manifest checksum"));
        }
        let olddir = root.join(GENERATIONS).join(previous);
        if !state.old_verified {
            if let Some((key, reference)) = old.catalogs.iter().find(|(key, _)| {
                state
                    .old_checked
                    .as_ref()
                    .is_none_or(|checked| *key > checked)
            }) {
                let new = target
                    .catalogs
                    .get(key)
                    .ok_or_else(|| invalid("generation old shard not covered"))?;
                if !catalog::load(&olddir, key, reference)?
                    .is_subset(&catalog::load(&dir, key, new)?)
                {
                    return Err(invalid("generation old closure not covered"));
                }
                state.old_checked = Some(key.clone());
                save(root, state)?;
                return progress(root, state);
            }
            state.old_verified = true;
            save(root, state)?;
        }
        if !cleanup::old_generation_batch(&olddir)? {
            return progress(root, state);
        }
    }
    if !super::super::gc::retire_overlay_frames(root)? {
        save(root, state)?;
        return progress(root, state);
    }
    publication::fault(7)?;
    fs::remove_file(root.join(BUILD)).map_err(io)?;
    sync(root)?;
    let count = catalog::count(&target.catalogs)?;
    Ok(FormalGenerationProgress {
        building: false,
        selected: Some(selected.into()),
        copied_objects: count,
        total_objects: count,
    })
}
