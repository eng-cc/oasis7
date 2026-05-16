use oasis7_proto::distributed::{SnapshotManifest, StateChunkRef};
use oasis7_proto::distributed_storage::{JournalSegmentRef, SegmentConfig};
use oasis7_proto::world_error::WorldError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zstd::stream::{decode_all as zstd_decode_all, encode_all as zstd_encode_all};
const BLOBS_DIR: &str = "blobs";
const PINS_FILE: &str = "pins.json";
const FILES_INDEX_FILE: &str = "files_index.json";
const FILE_INDEX_VERSION: u64 = 1;
const COMPRESSED_BLOB_MAGIC: &[u8; 8] = b"O7CBLOB1";
const COMPRESSED_BLOB_HASH_HEX_LEN: usize = 64;
const COMPRESSED_BLOB_HEADER_LEN: usize = 16 + COMPRESSED_BLOB_HASH_HEX_LEN;
const COMPRESSIBLE_BLOB_MIN_BYTES: usize = 1024;
const COMPRESSED_BLOB_ZSTD_LEVEL: i32 = 3;
mod challenge;
mod challenge_scheduler;
mod feedback;
mod feedback_p2p;
mod manifest;
mod replication;

pub use challenge::*;
pub use challenge_scheduler::*;
pub use feedback::*;
pub use feedback_p2p::*;
pub use manifest::*;
pub use replication::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Blake3,
    Sha256,
}

pub trait BlobStore {
    fn put(&self, content_hash: &str, bytes: &[u8]) -> Result<(), WorldError>;
    fn get(&self, content_hash: &str) -> Result<Vec<u8>, WorldError>;
    fn has(&self, content_hash: &str) -> Result<bool, WorldError>;

    fn put_bytes(&self, bytes: &[u8]) -> Result<String, WorldError> {
        let content_hash = blake3_hex(bytes);
        self.put(&content_hash, bytes)?;
        Ok(content_hash)
    }
}

pub trait FileStore {
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<FileMetadata, WorldError>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, WorldError>;
    fn delete_file(&self, path: &str) -> Result<bool, WorldError>;
    fn stat_file(&self, path: &str) -> Result<Option<FileMetadata>, WorldError>;
    fn list_files(&self) -> Result<Vec<FileMetadata>, WorldError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileIndexAuditReport {
    pub total_indexed_files: usize,
    pub total_pins: usize,
    pub missing_file_blob_hashes: Vec<String>,
    pub dangling_pin_hashes: Vec<String>,
    pub orphan_blob_hashes: Vec<String>,
}

impl FileIndexAuditReport {
    pub fn is_clean(&self) -> bool {
        self.missing_file_blob_hashes.is_empty()
            && self.dangling_pin_hashes.is_empty()
            && self.orphan_blob_hashes.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct LocalCasStore {
    root: PathBuf,
    blobs_dir: PathBuf,
    pins_path: PathBuf,
    hash_algorithm: HashAlgorithm,
    files_index_path: PathBuf,
}

impl LocalCasStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self::new_with_hash_algorithm(root, HashAlgorithm::Blake3)
    }

    pub fn new_with_hash_algorithm(root: impl AsRef<Path>, hash_algorithm: HashAlgorithm) -> Self {
        let root = root.as_ref().to_path_buf();
        let blobs_dir = root.join(BLOBS_DIR);
        let pins_path = root.join(PINS_FILE);
        let files_index_path = root.join(FILES_INDEX_FILE);
        Self {
            root,
            blobs_dir,
            pins_path,
            hash_algorithm,
            files_index_path,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn blobs_dir(&self) -> &Path {
        &self.blobs_dir
    }

    pub fn pins_path(&self) -> &Path {
        &self.pins_path
    }

    pub fn hash_algorithm(&self) -> HashAlgorithm {
        self.hash_algorithm
    }

    pub fn get_verified(&self, content_hash: &str) -> Result<Vec<u8>, WorldError> {
        let bytes = self.get(content_hash)?;
        let actual_hash = self.hash_hex(&bytes);
        if actual_hash != content_hash {
            return Err(WorldError::BlobHashMismatch {
                expected: content_hash.to_string(),
                actual: actual_hash,
            });
        }
        Ok(bytes)
    }

    fn hash_hex(&self, bytes: &[u8]) -> String {
        match self.hash_algorithm {
            HashAlgorithm::Blake3 => blake3_hex(bytes),
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(bytes);
                format!("{:x}", hasher.finalize())
            }
        }
    }

    pub fn files_index_path(&self) -> &Path {
        &self.files_index_path
    }

    fn ensure_dirs(&self) -> Result<(), WorldError> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.blobs_dir)?;
        Ok(())
    }

    fn maybe_encode_blob_for_disk(
        &self,
        content_hash: &str,
        bytes: &[u8],
    ) -> Result<Vec<u8>, WorldError> {
        if bytes.len() < COMPRESSIBLE_BLOB_MIN_BYTES {
            return Ok(bytes.to_vec());
        }
        let compressed = zstd_encode_all(bytes, COMPRESSED_BLOB_ZSTD_LEVEL).map_err(|err| {
            WorldError::DistributedValidationFailed {
                reason: format!("compress blob for disk failed: {err}"),
            }
        })?;
        let encoded_len = COMPRESSED_BLOB_HEADER_LEN.saturating_add(compressed.len());
        if encoded_len >= bytes.len() {
            return Ok(bytes.to_vec());
        }

        let mut encoded = Vec::with_capacity(encoded_len);
        encoded.extend_from_slice(COMPRESSED_BLOB_MAGIC);
        encoded.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        encoded.extend_from_slice(content_hash.as_bytes());
        encoded.extend_from_slice(compressed.as_slice());
        Ok(encoded)
    }

    fn decode_blob_from_disk(&self, content_hash: &str, disk_bytes: Vec<u8>) -> Vec<u8> {
        if disk_bytes.len() < COMPRESSED_BLOB_HEADER_LEN
            || !disk_bytes.starts_with(COMPRESSED_BLOB_MAGIC)
        {
            return disk_bytes;
        }

        let Some(raw_len_bytes) = disk_bytes.get(8..16) else {
            return disk_bytes;
        };
        let expected_raw_len = u64::from_le_bytes(raw_len_bytes.try_into().unwrap_or([0; 8]));
        let Some(encoded_hash_bytes) = disk_bytes.get(16..COMPRESSED_BLOB_HEADER_LEN) else {
            return disk_bytes;
        };
        if encoded_hash_bytes != content_hash.as_bytes() {
            return disk_bytes;
        }
        match zstd_decode_all(&disk_bytes[COMPRESSED_BLOB_HEADER_LEN..]) {
            Ok(decoded) if decoded.len() as u64 == expected_raw_len => decoded,
            _ => disk_bytes,
        }
    }

    fn blob_path(&self, content_hash: &str) -> Result<PathBuf, WorldError> {
        validate_hash(content_hash)?;
        Ok(self.blobs_dir.join(format!("{content_hash}.blob")))
    }

    fn load_pins(&self) -> Result<PinFile, WorldError> {
        if !self.pins_path.exists() {
            return Ok(PinFile::default());
        }
        read_json_from_path(&self.pins_path)
    }

    fn save_pins(&self, pins: &PinFile) -> Result<(), WorldError> {
        self.ensure_dirs()?;
        write_json_atomic(pins, &self.pins_path)
    }

    fn load_file_index(&self) -> Result<FileIndexFile, WorldError> {
        if !self.files_index_path.exists() {
            return Ok(FileIndexFile::default());
        }
        let file_index: FileIndexFile = read_json_from_path(&self.files_index_path)?;
        if file_index.version != FILE_INDEX_VERSION {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsupported files index version: expected={}, actual={}",
                    FILE_INDEX_VERSION, file_index.version
                ),
            });
        }
        Ok(file_index)
    }

    fn save_file_index(&self, file_index: &FileIndexFile) -> Result<(), WorldError> {
        self.ensure_dirs()?;
        write_json_atomic(file_index, &self.files_index_path)
    }

    fn remove_blob_if_unreferenced_in_index(
        &self,
        file_index: &FileIndexFile,
        content_hash: &str,
    ) -> Result<bool, WorldError> {
        validate_hash(content_hash)?;
        if file_index
            .files
            .values()
            .any(|metadata| metadata.content_hash == content_hash)
        {
            return Ok(false);
        }
        if self.is_pinned(content_hash)? {
            return Ok(false);
        }
        let path = self.blob_path(content_hash)?;
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path)?;
        Ok(true)
    }

    pub fn pin(&self, content_hash: &str) -> Result<(), WorldError> {
        validate_hash(content_hash)?;
        if !self.has(content_hash)? {
            return Err(WorldError::BlobNotFound {
                content_hash: content_hash.to_string(),
            });
        }
        let mut pins = self.load_pins()?;
        pins.pins.insert(content_hash.to_string());
        self.save_pins(&pins)
    }

    pub fn unpin(&self, content_hash: &str) -> Result<bool, WorldError> {
        validate_hash(content_hash)?;
        let mut pins = self.load_pins()?;
        let removed = pins.pins.remove(content_hash);
        self.save_pins(&pins)?;
        Ok(removed)
    }

    pub fn list_pins(&self) -> Result<Vec<String>, WorldError> {
        let pins = self.load_pins()?;
        Ok(pins.pins.into_iter().collect())
    }

    pub fn is_pinned(&self, content_hash: &str) -> Result<bool, WorldError> {
        validate_hash(content_hash)?;
        let pins = self.load_pins()?;
        Ok(pins.pins.contains(content_hash))
    }

    pub fn prune_unpinned(&self, max_bytes: u64) -> Result<u64, WorldError> {
        self.ensure_dirs()?;
        let pins = self.load_pins()?.pins;
        let mut total_bytes = 0u64;
        let mut entries = Vec::new();

        if self.blobs_dir.exists() {
            for entry in fs::read_dir(&self.blobs_dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("blob") {
                    continue;
                }
                let metadata = entry.metadata()?;
                let size = metadata.len();
                total_bytes = total_bytes.saturating_add(size);
                let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
                let hash = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("")
                    .to_string();
                entries.push(BlobEntry {
                    hash,
                    path,
                    size,
                    modified,
                });
            }
        }

        if total_bytes <= max_bytes {
            return Ok(0);
        }

        entries.sort_by_key(|entry| entry.modified);
        let mut freed = 0u64;
        for entry in entries {
            if total_bytes <= max_bytes {
                break;
            }
            if pins.contains(&entry.hash) {
                continue;
            }
            fs::remove_file(&entry.path)?;
            total_bytes = total_bytes.saturating_sub(entry.size);
            freed = freed.saturating_add(entry.size);
        }
        Ok(freed)
    }

    pub fn write_file_if_match(
        &self,
        path: &str,
        expected_content_hash: Option<&str>,
        bytes: &[u8],
    ) -> Result<FileMetadata, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let expected_content_hash = normalize_expected_content_hash(expected_content_hash)?;
        let mut file_index = self.load_file_index()?;
        ensure_file_hash_precondition(
            &file_index,
            &normalized_path,
            expected_content_hash.as_deref(),
        )?;

        let content_hash = self.put_bytes(bytes)?;
        let metadata = FileMetadata {
            path: normalized_path.clone(),
            content_hash,
            size_bytes: bytes.len() as u64,
            updated_at_ms: now_unix_time_ms(),
        };
        let replaced_hash = file_index
            .files
            .insert(normalized_path, metadata.clone())
            .map(|previous| previous.content_hash);
        self.save_file_index(&file_index)?;
        if let Some(replaced_hash) = replaced_hash {
            if replaced_hash != metadata.content_hash {
                self.remove_blob_if_unreferenced_in_index(&file_index, replaced_hash.as_str())?;
            }
        }
        Ok(metadata)
    }

    pub fn delete_file_if_match(
        &self,
        path: &str,
        expected_content_hash: Option<&str>,
    ) -> Result<bool, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let expected_content_hash = normalize_expected_content_hash(expected_content_hash)?;
        let mut file_index = self.load_file_index()?;

        if let Some(expected_hash) = expected_content_hash.as_deref() {
            ensure_file_hash_precondition(&file_index, &normalized_path, Some(expected_hash))?;
        }

        let removed_hash = file_index
            .files
            .remove(&normalized_path)
            .map(|metadata| metadata.content_hash);
        if let Some(removed_hash) = removed_hash {
            self.save_file_index(&file_index)?;
            self.remove_blob_if_unreferenced_in_index(&file_index, removed_hash.as_str())?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn audit_file_index(&self) -> Result<FileIndexAuditReport, WorldError> {
        self.ensure_dirs()?;
        let file_index = self.load_file_index()?;
        let pins = self.load_pins()?.pins;
        let blob_hashes = self.scan_blob_hashes()?;

        let mut indexed_hashes = BTreeSet::new();
        for metadata in file_index.files.values() {
            validate_hash(metadata.content_hash.as_str())?;
            indexed_hashes.insert(metadata.content_hash.clone());
        }
        for pin in &pins {
            validate_hash(pin.as_str())?;
        }

        let missing_file_blob_hashes = indexed_hashes
            .iter()
            .filter(|hash| !blob_hashes.contains(*hash))
            .cloned()
            .collect::<Vec<_>>();
        let dangling_pin_hashes = pins
            .iter()
            .filter(|hash| !blob_hashes.contains(*hash))
            .cloned()
            .collect::<Vec<_>>();
        let orphan_blob_hashes = blob_hashes
            .iter()
            .filter(|hash| !indexed_hashes.contains(*hash) && !pins.contains(*hash))
            .cloned()
            .collect::<Vec<_>>();

        Ok(FileIndexAuditReport {
            total_indexed_files: file_index.files.len(),
            total_pins: pins.len(),
            missing_file_blob_hashes,
            dangling_pin_hashes,
            orphan_blob_hashes,
        })
    }

    pub fn prune_orphan_blobs(&self) -> Result<u64, WorldError> {
        let report = self.audit_file_index()?;
        let mut freed = 0_u64;
        for orphan_hash in report.orphan_blob_hashes {
            let path = self.blob_path(orphan_hash.as_str())?;
            if !path.exists() {
                continue;
            }
            let size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            fs::remove_file(&path)?;
            freed = freed.saturating_add(size);
        }
        Ok(freed)
    }

    fn scan_blob_hashes(&self) -> Result<BTreeSet<String>, WorldError> {
        self.ensure_dirs()?;
        let mut hashes = BTreeSet::new();
        if !self.blobs_dir.exists() {
            return Ok(hashes);
        }
        for entry in fs::read_dir(&self.blobs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("blob") {
                continue;
            }
            let hash = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| WorldError::DistributedValidationFailed {
                    reason: format!("invalid blob file name: {}", path.display()),
                })?;
            validate_hash(hash)?;
            hashes.insert(hash.to_string());
        }
        Ok(hashes)
    }
}

impl BlobStore for LocalCasStore {
    fn put(&self, content_hash: &str, bytes: &[u8]) -> Result<(), WorldError> {
        self.ensure_dirs()?;
        let actual_hash = self.hash_hex(bytes);
        if actual_hash != content_hash {
            return Err(WorldError::BlobHashMismatch {
                expected: content_hash.to_string(),
                actual: actual_hash,
            });
        }
        let path = self.blob_path(content_hash)?;
        if path.exists() {
            return Ok(());
        }
        let disk_bytes = self.maybe_encode_blob_for_disk(content_hash, bytes)?;
        write_bytes_atomic(&path, disk_bytes.as_slice())?;
        Ok(())
    }

    fn put_bytes(&self, bytes: &[u8]) -> Result<String, WorldError> {
        let content_hash = self.hash_hex(bytes);
        self.put(&content_hash, bytes)?;
        Ok(content_hash)
    }

    fn get(&self, content_hash: &str) -> Result<Vec<u8>, WorldError> {
        let path = self.blob_path(content_hash)?;
        if !path.exists() {
            return Err(WorldError::BlobNotFound {
                content_hash: content_hash.to_string(),
            });
        }
        let disk_bytes = fs::read(path)?;
        Ok(self.decode_blob_from_disk(content_hash, disk_bytes))
    }

    fn has(&self, content_hash: &str) -> Result<bool, WorldError> {
        let path = self.blob_path(content_hash)?;
        Ok(path.exists())
    }
}

impl FileStore for LocalCasStore {
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<FileMetadata, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let content_hash = self.put_bytes(bytes)?;
        let metadata = FileMetadata {
            path: normalized_path.clone(),
            content_hash,
            size_bytes: bytes.len() as u64,
            updated_at_ms: now_unix_time_ms(),
        };
        let mut file_index = self.load_file_index()?;
        let replaced_hash = file_index
            .files
            .insert(normalized_path, metadata.clone())
            .map(|previous| previous.content_hash);
        self.save_file_index(&file_index)?;
        if let Some(replaced_hash) = replaced_hash {
            if replaced_hash != metadata.content_hash {
                self.remove_blob_if_unreferenced_in_index(&file_index, replaced_hash.as_str())?;
            }
        }
        Ok(metadata)
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let file_index = self.load_file_index()?;
        let metadata = file_index.files.get(&normalized_path).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!("file not found: {normalized_path}"),
            }
        })?;
        self.get(&metadata.content_hash)
    }

    fn delete_file(&self, path: &str) -> Result<bool, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let mut file_index = self.load_file_index()?;
        let removed_hash = file_index
            .files
            .remove(&normalized_path)
            .map(|metadata| metadata.content_hash);
        if let Some(removed_hash) = removed_hash {
            self.save_file_index(&file_index)?;
            self.remove_blob_if_unreferenced_in_index(&file_index, removed_hash.as_str())?;
            return Ok(true);
        }
        Ok(false)
    }

    fn stat_file(&self, path: &str) -> Result<Option<FileMetadata>, WorldError> {
        let normalized_path = normalize_file_path(path)?;
        let file_index = self.load_file_index()?;
        Ok(file_index.files.get(&normalized_path).cloned())
    }

    fn list_files(&self) -> Result<Vec<FileMetadata>, WorldError> {
        let file_index = self.load_file_index()?;
        Ok(file_index.files.values().cloned().collect())
    }
}

pub fn segment_snapshot<T: Serialize>(
    snapshot: &T,
    world_id: &str,
    epoch: u64,
    store: &impl BlobStore,
    config: SegmentConfig,
) -> Result<SnapshotManifest, WorldError> {
    let bytes = to_canonical_cbor(snapshot)?;
    let state_root = blake3_hex(&bytes);
    let chunk_size = config.snapshot_chunk_bytes.max(1);
    let mut chunks = Vec::new();

    for (index, chunk) in bytes.chunks(chunk_size).enumerate() {
        let content_hash = store.put_bytes(chunk)?;
        chunks.push(StateChunkRef {
            chunk_id: format!("{epoch}-{index:04}"),
            content_hash,
            size_bytes: chunk.len() as u64,
        });
    }

    Ok(SnapshotManifest {
        world_id: world_id.to_string(),
        epoch,
        chunks,
        state_root,
    })
}

pub fn segment_journal<E: Serialize>(
    events: &[E],
    store: &impl BlobStore,
    config: SegmentConfig,
    event_id_of: impl Fn(&E) -> u64,
) -> Result<Vec<JournalSegmentRef>, WorldError> {
    if events.is_empty() {
        return Ok(Vec::new());
    }

    let max_events = config.journal_events_per_segment.max(1);
    let mut segments = Vec::new();

    for chunk in events.chunks(max_events) {
        let from_event_id = chunk.first().map(&event_id_of).unwrap_or(0);
        let to_event_id = chunk.last().map(&event_id_of).unwrap_or(0);
        let bytes = to_canonical_cbor(&chunk)?;
        let content_hash = store.put_bytes(&bytes)?;
        segments.push(JournalSegmentRef {
            from_event_id,
            to_event_id,
            content_hash,
            size_bytes: bytes.len() as u64,
        });
    }

    Ok(segments)
}

pub fn assemble_snapshot<T: DeserializeOwned>(
    manifest: &SnapshotManifest,
    store: &impl BlobStore,
) -> Result<T, WorldError> {
    let mut bytes = Vec::new();
    for chunk in &manifest.chunks {
        let chunk_bytes = store.get(&chunk.content_hash)?;
        let actual_hash = blake3_hex(&chunk_bytes);
        if actual_hash != chunk.content_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "snapshot chunk hash mismatch: expected={}, actual={}",
                    chunk.content_hash, actual_hash
                ),
            });
        }
        bytes.extend_from_slice(&chunk_bytes);
    }

    let actual_root = blake3_hex(&bytes);
    if actual_root != manifest.state_root {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "snapshot state_root mismatch: expected={}, actual={}",
                manifest.state_root, actual_root
            ),
        });
    }

    Ok(serde_cbor::from_slice(&bytes)?)
}

pub fn assemble_journal<E: DeserializeOwned>(
    segments: &[JournalSegmentRef],
    store: &impl BlobStore,
    event_id_of: impl Fn(&E) -> u64,
) -> Result<Vec<E>, WorldError> {
    let mut events = Vec::new();
    let mut expected_next: Option<u64> = None;

    for segment in segments {
        let segment_bytes = store.get(&segment.content_hash)?;
        let actual_hash = blake3_hex(&segment_bytes);
        if actual_hash != segment.content_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "journal segment hash mismatch: expected={}, actual={}",
                    segment.content_hash, actual_hash
                ),
            });
        }
        let segment_events: Vec<E> = serde_cbor::from_slice(&segment_bytes)?;
        let (first, last) = match (segment_events.first(), segment_events.last()) {
            (Some(first), Some(last)) => (first, last),
            _ => {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "journal segment empty".to_string(),
                });
            }
        };

        let first_id = event_id_of(first);
        let last_id = event_id_of(last);
        if first_id != segment.from_event_id || last_id != segment.to_event_id {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "journal segment range mismatch: segment={}..{}, events={}..{}",
                    segment.from_event_id, segment.to_event_id, first_id, last_id
                ),
            });
        }
        if let Some(expected) = expected_next {
            if first_id != expected {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "journal discontinuity: expected={}, got={}",
                        expected, first_id
                    ),
                });
            }
        }
        expected_next = last_id.checked_add(1);

        events.extend(segment_events);
    }

    Ok(events)
}

pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn validate_hash(content_hash: &str) -> Result<(), WorldError> {
    if content_hash.is_empty()
        || content_hash.contains('/')
        || content_hash.contains('\\')
        || content_hash.contains("..")
    {
        return Err(WorldError::BlobHashInvalid {
            content_hash: content_hash.to_string(),
        });
    }
    Ok(())
}

fn normalize_file_path(path: &str) -> Result<String, WorldError> {
    if path.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "invalid file path: empty path".to_string(),
        });
    }
    let raw_path = Path::new(path);
    if raw_path.is_absolute() {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("invalid file path: absolute path not allowed ({path})"),
        });
    }

    let mut normalized = Vec::new();
    for component in raw_path.components() {
        match component {
            std::path::Component::Normal(part) => {
                let segment =
                    part.to_str()
                        .ok_or_else(|| WorldError::DistributedValidationFailed {
                            reason: format!("invalid file path: non-utf8 segment ({path})"),
                        })?;
                normalized.push(segment.to_string());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!("invalid file path: parent traversal not allowed ({path})"),
                });
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!("invalid file path: absolute path not allowed ({path})"),
                });
            }
        }
    }
    if normalized.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("invalid file path: no path segment ({path})"),
        });
    }
    Ok(normalized.join("/"))
}

fn normalize_expected_content_hash(
    expected_content_hash: Option<&str>,
) -> Result<Option<String>, WorldError> {
    let Some(content_hash) = expected_content_hash else {
        return Ok(None);
    };
    validate_hash(content_hash)?;
    Ok(Some(content_hash.to_string()))
}

fn ensure_file_hash_precondition(
    file_index: &FileIndexFile,
    normalized_path: &str,
    expected_content_hash: Option<&str>,
) -> Result<(), WorldError> {
    let Some(expected_content_hash) = expected_content_hash else {
        return Ok(());
    };
    let current = file_index.files.get(normalized_path).ok_or_else(|| {
        WorldError::DistributedValidationFailed {
            reason: format!(
                "file precondition failed: path missing for expected hash path={} expected={}",
                normalized_path, expected_content_hash
            ),
        }
    })?;
    if current.content_hash != expected_content_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "file precondition failed: hash mismatch path={} expected={} actual={}",
                normalized_path, expected_content_hash, current.content_hash
            ),
        });
    }
    Ok(())
}

fn now_unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PinFile {
    #[serde(default)]
    pins: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileIndexFile {
    #[serde(default = "default_file_index_version")]
    version: u64,
    #[serde(default)]
    files: BTreeMap<String, FileMetadata>,
}

impl Default for FileIndexFile {
    fn default() -> Self {
        Self {
            version: FILE_INDEX_VERSION,
            files: BTreeMap::new(),
        }
    }
}

fn default_file_index_version() -> u64 {
    FILE_INDEX_VERSION
}

#[derive(Debug, Clone)]
struct BlobEntry {
    hash: String,
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn unique_atomic_temp_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!("tmp-{pid}-{nanos}"))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), WorldError> {
    let tmp = unique_atomic_temp_path(path);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn write_json_atomic<T: Serialize>(value: &T, path: &Path) -> Result<(), WorldError> {
    let tmp = unique_atomic_temp_path(path);
    let bytes = serde_json::to_vec(value)?;
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn read_json_from_path<T: DeserializeOwned>(path: &Path) -> Result<T, WorldError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn to_canonical_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, WorldError> {
    let mut buf = Vec::with_capacity(256);
    let canonical_value = serde_cbor::value::to_value(value)?;
    let mut serializer = serde_cbor::ser::Serializer::new(&mut buf);
    serializer.self_describe()?;
    canonical_value.serialize(&mut serializer)?;
    Ok(buf)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
