use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::model::{PersistedBridgeState, BRIDGE_STATE_SCHEMA_V1};

#[derive(Debug)]
pub(super) enum StoreMutateError<E> {
    Domain(E),
    Persist(String),
}

#[derive(Debug)]
pub(super) struct BridgeStateStore {
    path: PathBuf,
    state: Mutex<PersistedBridgeState>,
}

impl BridgeStateStore {
    pub(super) fn new(path: PathBuf) -> Result<Self, String> {
        let state = load_state(path.as_path())?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub(super) fn snapshot(&self) -> PersistedBridgeState {
        self.state.lock().expect("bridge state lock").clone()
    }

    pub(super) fn mutate<T, E, F>(&self, op: F) -> Result<T, StoreMutateError<E>>
    where
        F: FnOnce(&mut PersistedBridgeState) -> Result<T, E>,
    {
        let mut state = self.state.lock().expect("bridge state lock");
        let mut working = state.clone();
        let output = op(&mut working).map_err(StoreMutateError::Domain)?;
        persist_state(self.path.as_path(), &working).map_err(StoreMutateError::Persist)?;
        *state = working;
        Ok(output)
    }
}

fn load_state(path: &Path) -> Result<PersistedBridgeState, String> {
    if !path.exists() {
        return Ok(PersistedBridgeState::default());
    }
    let bytes = fs::read(path)
        .map_err(|err| format!("read bridge state {} failed: {err}", path.display()))?;
    let state: PersistedBridgeState = serde_json::from_slice(bytes.as_slice())
        .map_err(|err| format!("parse bridge state {} failed: {err}", path.display()))?;
    if state.schema_version != BRIDGE_STATE_SCHEMA_V1 {
        return Err(format!(
            "unsupported bridge state schema_version {} in {}",
            state.schema_version,
            path.display()
        ));
    }
    Ok(state)
}

fn persist_state(path: &Path, state: &PersistedBridgeState) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|err| format!("serialize bridge state failed: {err}"))?;
    write_bytes_atomic(path, bytes.as_slice())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("create bridge state dir {} failed: {err}", parent.display())
            })?;
        }
    }
    let temp_path = path.with_extension("json.tmp");
    fs::write(temp_path.as_path(), bytes).map_err(|err| {
        format!(
            "write bridge state temp {} failed: {err}",
            temp_path.display()
        )
    })?;
    fs::rename(temp_path.as_path(), path).map_err(|err| {
        format!(
            "rename bridge state temp {} -> {} failed: {err}",
            temp_path.display(),
            path.display()
        )
    })
}
