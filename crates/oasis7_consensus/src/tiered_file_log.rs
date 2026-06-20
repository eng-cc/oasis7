use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::path::Path;

use oasis7_distfs::{BlobStore as _, LocalCasStore, blake3_hex};
use serde::{Deserialize, Serialize};

use crate::error::WorldError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TieredJsonlColdRef {
    content_hash: String,
    line_count: usize,
}

pub(crate) fn append_jsonl_line_with_cas_offload(
    hot_path: &Path,
    cold_refs_path: &Path,
    cas_store: &LocalCasStore,
    max_hot_lines: usize,
    cold_segment_max_lines: usize,
    line: &str,
) -> Result<(), WorldError> {
    append_jsonl_line(hot_path, line)?;
    compact_jsonl_with_cas_offload(
        hot_path,
        cold_refs_path,
        cas_store,
        max_hot_lines,
        cold_segment_max_lines,
    )
}

pub(crate) fn compact_jsonl_with_cas_offload(
    hot_path: &Path,
    cold_refs_path: &Path,
    cas_store: &LocalCasStore,
    max_hot_lines: usize,
    cold_segment_max_lines: usize,
) -> Result<(), WorldError> {
    if !hot_path.exists() {
        return Ok(());
    }

    let max_hot_lines = max_hot_lines.max(1);
    let cold_segment_max_lines = cold_segment_max_lines.max(1);
    let file = OpenOptions::new().read(true).open(hot_path)?;
    let reader = BufReader::new(file);

    let mut retained = VecDeque::with_capacity(max_hot_lines);
    let mut overflowed = false;
    let mut segment_lines = Vec::with_capacity(cold_segment_max_lines);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        retained.push_back(line);
        if retained.len() <= max_hot_lines {
            continue;
        }
        overflowed = true;
        let dropped = retained.pop_front().ok_or_else(|| {
            WorldError::Io("tiered jsonl compaction underflow while draining overflow".to_string())
        })?;
        segment_lines.push(dropped);
        if segment_lines.len() >= cold_segment_max_lines {
            persist_cold_segment(cold_refs_path, cas_store, &mut segment_lines)?;
        }
    }

    if !overflowed {
        return Ok(());
    }
    if !segment_lines.is_empty() {
        persist_cold_segment(cold_refs_path, cas_store, &mut segment_lines)?;
    }
    let retained = retained.into_iter().collect::<Vec<_>>();
    write_jsonl_lines(hot_path, retained.as_slice())
}

pub(crate) fn collect_jsonl_lines_with_cas_refs(
    hot_path: &Path,
    cold_refs_path: &Path,
    cas_store: &LocalCasStore,
) -> Result<Vec<String>, WorldError> {
    let mut lines = collect_cold_jsonl_lines(cold_refs_path, cas_store)?;
    let mut hot_lines = read_jsonl_lines(hot_path)?;
    lines.append(&mut hot_lines);
    Ok(lines)
}

pub(crate) fn collect_cold_jsonl_lines(
    cold_refs_path: &Path,
    cas_store: &LocalCasStore,
) -> Result<Vec<String>, WorldError> {
    if !cold_refs_path.exists() {
        return Ok(Vec::new());
    }
    let refs_file = OpenOptions::new().read(true).open(cold_refs_path)?;
    let refs_reader = BufReader::new(refs_file);
    let mut lines = Vec::new();
    for ref_line in refs_reader.lines() {
        let ref_line = ref_line?;
        if ref_line.trim().is_empty() {
            continue;
        }
        let cold_ref: TieredJsonlColdRef = serde_json::from_str(ref_line.as_str())?;
        if cold_ref.content_hash.trim().is_empty() || cold_ref.line_count == 0 {
            return Err(WorldError::Io(
                "tiered jsonl cold ref is missing content_hash or line_count".to_string(),
            ));
        }
        let bytes = cas_store.get_verified(cold_ref.content_hash.as_str())?;
        let before = lines.len();
        lines.extend(parse_jsonl_lines_from_bytes(bytes.as_slice())?);
        let appended = lines.len().saturating_sub(before);
        if appended != cold_ref.line_count {
            return Err(WorldError::Io(format!(
                "tiered jsonl cold ref line_count mismatch for hash {}: expected={} actual={}",
                cold_ref.content_hash, cold_ref.line_count, appended
            )));
        }
    }
    Ok(lines)
}

pub(crate) fn read_jsonl_lines(path: &Path) -> Result<Vec<String>, WorldError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        lines.push(line);
    }
    Ok(lines)
}

pub(crate) fn write_jsonl_lines(path: &Path, lines: &[String]) -> Result<(), WorldError> {
    if lines.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    for line in lines {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn append_jsonl_line(path: &Path, line: &str) -> Result<(), WorldError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

fn persist_cold_segment(
    cold_refs_path: &Path,
    cas_store: &LocalCasStore,
    segment_lines: &mut Vec<String>,
) -> Result<(), WorldError> {
    if segment_lines.is_empty() {
        return Ok(());
    }
    let mut payload = String::new();
    for line in segment_lines.iter() {
        payload.push_str(line);
        payload.push('\n');
    }
    let payload_bytes = payload.into_bytes();
    let content_hash = blake3_hex(payload_bytes.as_slice());
    cas_store.put(content_hash.as_str(), payload_bytes.as_slice())?;
    let cold_ref_line = serde_json::to_string(&TieredJsonlColdRef {
        content_hash,
        line_count: segment_lines.len(),
    })?;
    append_jsonl_line(cold_refs_path, cold_ref_line.as_str())?;
    segment_lines.clear();
    Ok(())
}

fn parse_jsonl_lines_from_bytes(bytes: &[u8]) -> Result<Vec<String>, WorldError> {
    let cursor = Cursor::new(bytes);
    let reader = BufReader::new(cursor);
    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        lines.push(line);
    }
    Ok(lines)
}
