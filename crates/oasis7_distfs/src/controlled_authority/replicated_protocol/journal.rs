//! Legacy v1 bounded immutable delta frames. Sealed segments remain the permanent request
//! and artifact archive; checkpoints authenticate segment boundaries, never GC.
//! Legacy seals are not full state checkpoints. Production recovery migrates
//! these frames into the v2 archive/checkpoint backend without retaining all
//! artifact bodies; legacy replay helpers below are test-only.
use super::*;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use storage::{Fault, Phase};
const FRAMES_PER_SEGMENT: u64 = 64;
const MAX_FRAME: usize = storage::MAX_FILE;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Delta {
    identity: Value,
    prepared: Value,
    decisions: Vec<(usize, Value)>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    version: u32,
    sequence: u64,
    previous: String,
    delta: Delta,
    digest: String,
}
fn digest(sequence: u64, previous: &str, delta: &Delta) -> Result<String, ProtocolError> {
    let bytes = serde_json::to_vec(&(1u32, sequence, previous, delta))
        .map_err(|e| invalid(e.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
fn bounded_read(path: &Path) -> Result<Vec<u8>, ProtocolError> {
    storage::no_symlink(path)?;
    let meta = fs::symlink_metadata(path).map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_FRAME as u64 {
        return Err(invalid("journal frame regular file/size limit"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io)?
        .take((MAX_FRAME + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_FRAME {
        return Err(invalid("journal frame size limit"));
    }
    Ok(bytes)
}
#[cfg(test)]
fn replay(
    path: &Path,
    legacy: Option<Vec<u8>>,
) -> Result<(Option<Value>, u64, String), ProtocolError> {
    let mut state = legacy
        .map(|b| serde_json::from_slice::<Value>(&b).map_err(|e| invalid(e.to_string())))
        .transpose()?;
    let root = path.join("journal");
    storage::no_symlink(&root)?;
    if !root.exists() {
        return Ok((state, 0, String::new()));
    }
    let mut frames = Vec::new();
    let mut checkpoints = std::collections::BTreeMap::new();
    for segment in fs::read_dir(&root).map_err(io)? {
        let segment = segment.map_err(io)?.path();
        storage::no_symlink(&segment)?;
        if !segment.is_dir() {
            return Err(invalid("journal segment not directory"));
        }
        let segment_number = segment
            .file_name()
            .and_then(|v| v.to_str())
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| invalid("journal segment name"))?;
        if segment.file_name().and_then(|v| v.to_str())
            != Some(format!("{segment_number:020}").as_str())
        {
            return Err(invalid("journal segment name"));
        }
        for entry in fs::read_dir(&segment).map_err(io)? {
            let file = entry.map_err(io)?.path();
            let name = file
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or_else(|| invalid("journal filename"))?;
            if name.starts_with(".replicated-pending-") {
                continue;
            }
            if name == "checkpoint.json" {
                let sealed: (u64, String) = serde_json::from_slice(&bounded_read(&file)?)
                    .map_err(|e| invalid(e.to_string()))?;
                let sequence = segment_number
                    .checked_add(1)
                    .and_then(|v| v.checked_mul(FRAMES_PER_SEGMENT))
                    .ok_or_else(|| invalid("checkpoint sequence overflow"))?;
                if sealed.0 != sequence {
                    return Err(invalid("checkpoint sequence"));
                }
                checkpoints.insert(sequence, sealed.1);
                continue;
            }
            let sequence = name
                .strip_suffix(".json")
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| invalid("unknown journal file"))?;
            let expected_segment =
                format!("{:020}", sequence.saturating_sub(1) / FRAMES_PER_SEGMENT);
            if sequence == 0
                || segment.file_name().and_then(|v| v.to_str()) != Some(&expected_segment)
            {
                return Err(invalid("journal segment sequence"));
            }
            frames.push((sequence, file));
        }
    }
    frames.sort_by_key(|v| v.0);
    let mut count = 0;
    let mut previous = String::new();
    for (sequence, file) in frames {
        let frame: Frame =
            serde_json::from_slice(&bounded_read(&file)?).map_err(|e| invalid(e.to_string()))?;
        if sequence != count + 1
            || frame.sequence != sequence
            || frame.version != 1
            || frame.previous != previous
            || frame.digest != digest(sequence, &previous, &frame.delta)?
        {
            return Err(invalid("journal sequence/checksum chain"));
        }
        let mut next = frame.delta.identity;
        next["prepared"] = frame.delta.prepared;
        let mut decisions = match state
            .as_mut()
            .and_then(|s| s.as_object_mut())
            .and_then(|s| s.remove("decisions"))
        {
            Some(Value::Array(entries)) => entries,
            None => Vec::new(),
            Some(_) => return Err(invalid("journal decision array")),
        };
        for (index, entry) in frame.delta.decisions {
            if index > decisions.len() {
                return Err(invalid("journal decision gap"));
            }
            if index == decisions.len() {
                decisions.push(entry);
            } else {
                decisions[index] = entry;
            }
        }
        next["decisions"] = Value::Array(decisions);
        state = Some(next);
        count = sequence;
        previous = frame.digest;
        if let Some(sealed) = checkpoints.remove(&sequence)
            && sealed != previous
        {
            return Err(invalid("journal checkpoint mismatch"));
        }
    }
    if !checkpoints.is_empty() {
        return Err(invalid("checkpoint missing journal boundary"));
    }
    Ok((state, count, previous))
}
#[cfg(test)]
pub(super) fn read(path: &Path, legacy: Option<Vec<u8>>) -> Result<Option<Vec<u8>>, ProtocolError> {
    replay(path, legacy)?
        .0
        .map(|state| serde_json::to_vec(&state).map_err(|e| invalid(e.to_string())))
        .transpose()
}
pub(super) fn atomic(
    path: &Path,
    target: &Path,
    bytes: &[u8],
    fault: Fault,
) -> Result<(), ProtocolError> {
    if bytes.len() > MAX_FRAME {
        return Err(invalid("journal frame limit"));
    }
    storage::no_symlink(target)?;
    let (temp, mut file) = storage::temp(path)?;
    let result = (|| {
        fault.check(Phase::TempCreated)?;
        file.write_all(bytes).map_err(io)?;
        fault.check(Phase::Written)?;
        file.sync_all().map_err(io)?;
        fault.check(Phase::Synced)?;
        fs::rename(&temp, target).map_err(io)?;
        fault.check(Phase::Renamed)?;
        storage::sync_dir(path)?;
        fault.check(Phase::DirectorySynced)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
#[cfg(test)]
pub(super) fn persist(path: &Path, bytes: &[u8], fault: Fault) -> Result<(), ProtocolError> {
    // Legacy snapshots are immutable after migration, so restart always replays
    // the same base plus durable frames, including a torn migration attempt.
    let legacy_path = path.join("replicated-endpoint.json");
    let legacy = if legacy_path.exists() {
        Some(bounded_read(&legacy_path)?)
    } else {
        None
    };
    let (old, count, previous) = replay(path, legacy)?;
    let mut next: Value = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    let prepared = next
        .as_object_mut()
        .ok_or_else(|| invalid("journal state object"))?
        .remove("prepared")
        .ok_or_else(|| invalid("journal prepared"))?;
    let decisions = next
        .as_object_mut()
        .expect("object")
        .remove("decisions")
        .and_then(|v| v.as_array().cloned())
        .ok_or_else(|| invalid("journal decisions"))?;
    let old_entries = old.as_ref().and_then(|v| v["decisions"].as_array());
    if old_entries.is_some_and(|v| v.len() > decisions.len()) {
        return Err(invalid("journal history truncation"));
    }
    // Restore is intentionally split into individual bounded frames.
    let changed: Vec<_> = decisions
        .into_iter()
        .enumerate()
        .filter(|(i, e)| old_entries.and_then(|v| v.get(*i)) != Some(e))
        .collect();
    let batches: Vec<_> = if changed.is_empty() {
        vec![Vec::new()]
    } else {
        changed.into_iter().map(|entry| vec![entry]).collect()
    };
    let root = path.join("journal");
    storage::no_symlink(&root)?;
    storage::private_dir(&root)?;
    storage::sync_dir(path)?;
    let mut previous = previous;
    for (offset, batch) in batches.into_iter().enumerate() {
        let sequence = count + offset as u64 + 1;
        let segment = root.join(format!("{:020}", (sequence - 1) / FRAMES_PER_SEGMENT));
        storage::no_symlink(&segment)?;
        storage::private_dir(&segment)?;
        storage::sync_dir(&root)?;
        let delta = Delta {
            identity: next.clone(),
            prepared: prepared.clone(),
            decisions: batch,
        };
        let hash = digest(sequence, &previous, &delta)?;
        let frame = Frame {
            version: 1,
            sequence,
            previous,
            delta,
            digest: hash.clone(),
        };
        let bytes = serde_json::to_vec(&frame).map_err(|e| invalid(e.to_string()))?;
        atomic(
            &segment,
            &segment.join(format!("{sequence:020}.json")),
            &bytes,
            fault,
        )?;
        if sequence.is_multiple_of(FRAMES_PER_SEGMENT) {
            let checkpoint = serde_json::to_vec(&(sequence, hash.clone()))
                .map_err(|e| invalid(e.to_string()))?;
            atomic(
                &segment,
                &segment.join("checkpoint.json"),
                &checkpoint,
                Fault::None,
            )?;
        }
        previous = hash;
    }
    Ok(())
}

/// One-time migration visitor: retain one bounded artifact-bearing frame, never
/// reconstruct all historical decisions in RAM.
pub(super) fn visit(
    path: &Path,
    mut apply: impl FnMut(&Value, &Value, &[(usize, Value)]) -> Result<(), ProtocolError>,
) -> Result<(), ProtocolError> {
    let root = path.join("journal");
    storage::no_symlink(&root)?;
    if !root.exists() {
        return Ok(());
    }
    // Count directories without retaining an unbounded path list. Canonical
    // contiguous segment names let us replay with one bounded frame in memory.
    let mut segments = 0u64;
    for dir in fs::read_dir(&root).map_err(io)? {
        let dir = dir.map_err(io)?.path();
        storage::no_symlink(&dir)?;
        let name = dir
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| invalid("migration segment name"))?;
        let number = name
            .parse::<u64>()
            .map_err(|_| invalid("migration segment name"))?;
        if name != format!("{number:020}") || !dir.is_dir() {
            return Err(invalid("migration segment"));
        }
        segments = segments
            .checked_add(1)
            .ok_or_else(|| invalid("migration segment overflow"))?;
    }
    let mut previous = String::new();
    let mut count = 0u64;
    for segment_number in 0..segments {
        let dir = root.join(format!("{segment_number:020}"));
        storage::no_symlink(&dir)?;
        let mut frames = 0u64;
        let mut seal = None;
        for entry in fs::read_dir(&dir).map_err(io)? {
            let file = entry.map_err(io)?.path();
            let name = file
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or_else(|| invalid("migration filename"))?;
            if name.starts_with(".replicated-pending-") {
                continue;
            }
            if name == "checkpoint.json" {
                seal = Some(
                    serde_json::from_slice::<(u64, String)>(&bounded_read(&file)?)
                        .map_err(|e| invalid(e.to_string()))?,
                );
                continue;
            }
            let sequence = name
                .strip_suffix(".json")
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| invalid("migration frame name"))?;
            if sequence == 0
                || name != format!("{sequence:020}.json")
                || (sequence - 1) / FRAMES_PER_SEGMENT != segment_number
            {
                return Err(invalid("migration frame segment"));
            }
            frames = frames
                .checked_add(1)
                .ok_or_else(|| invalid("migration frame count"))?;
            if frames > FRAMES_PER_SEGMENT {
                return Err(invalid("migration segment frame limit"));
            }
        }
        if segment_number + 1 < segments && frames != FRAMES_PER_SEGMENT {
            return Err(invalid("migration missing segment suffix"));
        }
        for _ in 0..frames {
            let sequence = count
                .checked_add(1)
                .ok_or_else(|| invalid("migration sequence exhausted"))?;
            let file = dir.join(format!("{sequence:020}.json"));
            let frame: Frame = serde_json::from_slice(&bounded_read(&file)?)
                .map_err(|e| invalid(e.to_string()))?;
            if frame.sequence != sequence
                || frame.version != 1
                || frame.previous != previous
                || frame.digest != digest(sequence, &previous, &frame.delta)?
            {
                return Err(invalid("migration checksum/sequence chain"));
            }
            apply(
                &frame.delta.identity,
                &frame.delta.prepared,
                &frame.delta.decisions,
            )?;
            previous = frame.digest;
            count = sequence;
        }
        if let Some((sequence, hash)) = seal
            && (frames != FRAMES_PER_SEGMENT || sequence != count || hash != previous)
        {
            return Err(invalid("migration checkpoint boundary"));
        }
    }
    Ok(())
}
