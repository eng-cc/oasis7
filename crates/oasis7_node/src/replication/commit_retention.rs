use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use oasis7_proto::storage_cold_index::{
    storage_cold_index_dir_name, StorageColdIndexManifest, StorageColdIndexRange,
    StorageColdIndexRangeAnchor, STORAGE_COLD_INDEX_KEY_KIND_HEIGHT,
    STORAGE_COLD_INDEX_MANIFEST_FILE, STORAGE_COLD_INDEX_VALUE_KIND_CONTENT_HASH,
};
use serde::{Deserialize, Serialize};

use crate::NodeError;

use super::{write_json_compact, COMMIT_MESSAGE_DIR};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CommitMessageColdIndex {
    #[serde(flatten, default)]
    pub(super) manifest: StorageColdIndexManifest,
    #[serde(default)]
    pub(super) by_height: BTreeMap<u64, String>,
}

impl Default for CommitMessageColdIndex {
    fn default() -> Self {
        Self {
            manifest: StorageColdIndexManifest::new(
                COMMIT_MESSAGE_DIR,
                STORAGE_COLD_INDEX_KEY_KIND_HEIGHT,
                STORAGE_COLD_INDEX_VALUE_KIND_CONTENT_HASH,
            ),
            by_height: BTreeMap::new(),
        }
    }
}

impl CommitMessageColdIndex {
    pub(super) fn refresh_metadata(&mut self, hot_window: &CommitMessageHotWindow) {
        self.manifest.namespace = COMMIT_MESSAGE_DIR.to_string();
        self.manifest.key_kind = STORAGE_COLD_INDEX_KEY_KIND_HEIGHT.to_string();
        self.manifest.value_kind = STORAGE_COLD_INDEX_VALUE_KIND_CONTENT_HASH.to_string();
        self.manifest.hot_range =
            match (hot_window.hot_window_start_height, hot_window.latest_height) {
                (Some(from_key), Some(to_key)) if from_key <= to_key => {
                    Some(StorageColdIndexRange { from_key, to_key })
                }
                _ => None,
            };
        self.manifest.cold_range_anchor = build_cold_range_anchor(&self.by_height);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommitMessageHotWindow {
    pub(super) latest_height: Option<u64>,
    pub(super) hot_window_start_height: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommitMessageRetentionPlan {
    pub(super) hot_window: CommitMessageHotWindow,
    pub(super) offload_candidates: Vec<CommitMessageOffloadCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommitMessageOffloadCandidate {
    pub(super) height: u64,
    pub(super) path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CommitMessageReadbackSource {
    HotMirror { path: PathBuf },
    ColdArchive { content_hash: String },
}

pub(super) fn build_commit_message_retention_plan(
    root_dir: &Path,
    hot_window_heights: usize,
) -> Result<CommitMessageRetentionPlan, NodeError> {
    let hot_commit_files = list_hot_commit_message_files(root_dir)?;
    let Some(latest_height) = hot_commit_files.last().map(|(height, _)| *height) else {
        return Ok(CommitMessageRetentionPlan {
            hot_window: CommitMessageHotWindow {
                latest_height: None,
                hot_window_start_height: None,
            },
            offload_candidates: Vec::new(),
        });
    };

    let retained_hot_window = u64::try_from(hot_window_heights.max(1)).unwrap_or(u64::MAX);
    let hot_window_start_height =
        latest_height.saturating_sub(retained_hot_window.saturating_sub(1));
    let offload_candidates = hot_commit_files
        .into_iter()
        .filter(|(height, _)| *height < hot_window_start_height)
        .map(|(height, path)| CommitMessageOffloadCandidate { height, path })
        .collect();

    Ok(CommitMessageRetentionPlan {
        hot_window: CommitMessageHotWindow {
            latest_height: Some(latest_height),
            hot_window_start_height: Some(hot_window_start_height),
        },
        offload_candidates,
    })
}

pub(super) fn resolve_commit_message_readback_source(
    root_dir: &Path,
    height: u64,
) -> Result<Option<CommitMessageReadbackSource>, NodeError> {
    let hot_path = commit_message_path_from_root(root_dir, height);
    if hot_path.exists() {
        return Ok(Some(CommitMessageReadbackSource::HotMirror {
            path: hot_path,
        }));
    }

    let Some(content_hash) = load_commit_message_cold_index_from_root(root_dir)?
        .by_height
        .get(&height)
        .cloned()
    else {
        return Ok(None);
    };

    Ok(Some(CommitMessageReadbackSource::ColdArchive {
        content_hash,
    }))
}

pub(super) fn load_commit_message_cold_index_from_root(
    root_dir: &Path,
) -> Result<CommitMessageColdIndex, NodeError> {
    let canonical_path = commit_message_cold_index_manifest_path_from_root(root_dir);
    let compat_alias_path = commit_message_cold_index_compat_alias_path_from_root(root_dir);
    if canonical_path.exists() {
        let loaded = load_json_or_default::<CommitMessageColdIndex>(canonical_path.as_path())?;
        let cold_index = normalize_commit_message_cold_index(loaded.clone());
        if !compat_alias_path.exists() || cold_index != loaded {
            write_commit_message_cold_index_to_root(root_dir, &cold_index)?;
        }
        return Ok(cold_index);
    }

    if compat_alias_path.exists() {
        let cold_index =
            normalize_commit_message_cold_index(load_json_or_default::<CommitMessageColdIndex>(
                compat_alias_path.as_path(),
            )?);
        write_commit_message_cold_index_to_root(root_dir, &cold_index)?;
        return Ok(cold_index);
    }

    Ok(CommitMessageColdIndex::default())
}

pub(super) fn has_commit_message_cold_index(root_dir: &Path) -> bool {
    commit_message_cold_index_manifest_path_from_root(root_dir).exists()
        || commit_message_cold_index_compat_alias_path_from_root(root_dir).exists()
}

pub(super) fn write_commit_message_cold_index_to_root(
    root_dir: &Path,
    cold_index: &CommitMessageColdIndex,
) -> Result<(), NodeError> {
    write_json_compact(
        commit_message_cold_index_manifest_path_from_root(root_dir).as_path(),
        cold_index,
    )?;
    write_json_compact(
        commit_message_cold_index_compat_alias_path_from_root(root_dir).as_path(),
        cold_index,
    )
}

fn list_hot_commit_message_files(root_dir: &Path) -> Result<Vec<(u64, PathBuf)>, NodeError> {
    let commit_dir = root_dir.join(COMMIT_MESSAGE_DIR);
    if !commit_dir.exists() {
        return Ok(Vec::new());
    }

    let mut commit_files = Vec::new();
    let entries = fs::read_dir(&commit_dir).map_err(|err| NodeError::Replication {
        reason: format!("list commit dir {} failed: {}", commit_dir.display(), err),
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| NodeError::Replication {
            reason: format!("read commit dir entry failed: {}", err),
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(height_text) = file_name.strip_suffix(".json") else {
            continue;
        };
        let Ok(height) = height_text.parse::<u64>() else {
            continue;
        };
        if height == 0 {
            continue;
        }
        commit_files.push((height, path));
    }

    commit_files.sort_by_key(|(height, _)| *height);
    Ok(commit_files)
}

fn commit_message_path_from_root(root_dir: &Path, height: u64) -> PathBuf {
    root_dir
        .join(COMMIT_MESSAGE_DIR)
        .join(format!("{:020}.json", height))
}

fn commit_message_cold_index_root_dir(root_dir: &Path) -> PathBuf {
    root_dir.join(storage_cold_index_dir_name(COMMIT_MESSAGE_DIR))
}

fn commit_message_cold_index_manifest_path_from_root(root_dir: &Path) -> PathBuf {
    commit_message_cold_index_root_dir(root_dir).join(STORAGE_COLD_INDEX_MANIFEST_FILE)
}

fn commit_message_cold_index_compat_alias_path_from_root(root_dir: &Path) -> PathBuf {
    root_dir.join("replication_commit_messages_cold_index.json")
}

fn normalize_commit_message_cold_index(
    mut cold_index: CommitMessageColdIndex,
) -> CommitMessageColdIndex {
    if cold_index.manifest.namespace.trim().is_empty() {
        cold_index.manifest.namespace = COMMIT_MESSAGE_DIR.to_string();
    }
    if cold_index.manifest.key_kind.trim().is_empty() {
        cold_index.manifest.key_kind = STORAGE_COLD_INDEX_KEY_KIND_HEIGHT.to_string();
    }
    if cold_index.manifest.value_kind.trim().is_empty() {
        cold_index.manifest.value_kind = STORAGE_COLD_INDEX_VALUE_KIND_CONTENT_HASH.to_string();
    }
    if cold_index.manifest.cold_range_anchor.is_none() {
        cold_index.manifest.cold_range_anchor = build_cold_range_anchor(&cold_index.by_height);
    }
    cold_index
}

fn build_cold_range_anchor(
    by_height: &BTreeMap<u64, String>,
) -> Option<StorageColdIndexRangeAnchor> {
    let ((from_key, first_content_hash), (to_key, last_content_hash)) =
        (by_height.iter().next()?, by_height.iter().next_back()?);
    Some(StorageColdIndexRangeAnchor {
        from_key: *from_key,
        to_key: *to_key,
        first_content_hash: first_content_hash.clone(),
        last_content_hash: last_content_hash.clone(),
        entry_count: by_height.len(),
    })
}

fn load_json_or_default<T>(path: &Path) -> Result<T, NodeError>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let bytes = fs::read(path).map_err(|err| NodeError::Replication {
        reason: format!("read {} failed: {}", path.display(), err),
    })?;
    serde_json::from_slice::<T>(&bytes).map_err(|err| NodeError::Replication {
        reason: format!("parse {} failed: {}", path.display(), err),
    })
}
