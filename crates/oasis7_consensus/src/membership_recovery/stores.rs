use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::super::error::WorldError;

use super::super::membership_logic;

use super::types::{MembershipRevocationCoordinatorLeaseState, MembershipRevocationPendingAlert};

pub trait MembershipRevocationCoordinatorStateStore {
    fn load(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipRevocationCoordinatorLeaseState>, WorldError>;

    fn save(
        &self,
        world_id: &str,
        state: &MembershipRevocationCoordinatorLeaseState,
    ) -> Result<(), WorldError>;

    fn clear(&self, world_id: &str) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationCoordinatorStateStore {
    leases: Arc<Mutex<BTreeMap<String, MembershipRevocationCoordinatorLeaseState>>>,
}

impl InMemoryMembershipRevocationCoordinatorStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationCoordinatorStateStore
    for InMemoryMembershipRevocationCoordinatorStateStore
{
    fn load(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipRevocationCoordinatorLeaseState>, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator state lock poisoned".into())
        })?;
        Ok(guard.get(&world_id).cloned())
    }

    fn save(
        &self,
        world_id: &str,
        state: &MembershipRevocationCoordinatorLeaseState,
    ) -> Result<(), WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let mut guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator state lock poisoned".into())
        })?;
        guard.insert(world_id, state.clone());
        Ok(())
    }

    fn clear(&self, world_id: &str) -> Result<(), WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let mut guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator state lock poisoned".into())
        })?;
        guard.remove(&world_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationCoordinatorStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationCoordinatorStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn lease_path(&self, world_id: &str) -> Result<PathBuf, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        Ok(self
            .root_dir
            .join(format!("{world_id}.revocation-coordinator-lease.json")))
    }
}

impl MembershipRevocationCoordinatorStateStore for FileMembershipRevocationCoordinatorStateStore {
    fn load(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipRevocationCoordinatorLeaseState>, WorldError> {
        let path = self.lease_path(world_id)?;
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        let state = serde_json::from_slice(&bytes)?;
        Ok(Some(state))
    }

    fn save(
        &self,
        world_id: &str,
        state: &MembershipRevocationCoordinatorLeaseState,
    ) -> Result<(), WorldError> {
        let path = self.lease_path(world_id)?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(state)?;
        fs::write(path, bytes)?;
        Ok(())
    }

    fn clear(&self, world_id: &str) -> Result<(), WorldError> {
        let path = self.lease_path(world_id)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

pub trait MembershipRevocationAlertRecoveryStore {
    fn load_pending(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationPendingAlert>, WorldError>;

    fn save_pending(
        &self,
        world_id: &str,
        node_id: &str,
        alerts: &[MembershipRevocationPendingAlert],
    ) -> Result<(), WorldError>;
}

type PendingAlertRecords =
    Arc<Mutex<BTreeMap<(String, String), Vec<MembershipRevocationPendingAlert>>>>;

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationAlertRecoveryStore {
    pending: PendingAlertRecords,
}

impl InMemoryMembershipRevocationAlertRecoveryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationAlertRecoveryStore for InMemoryMembershipRevocationAlertRecoveryStore {
    fn load_pending(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationPendingAlert>, WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let guard = self.pending.lock().map_err(|_| {
            WorldError::Io("membership revocation recovery store lock poisoned".into())
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_pending(
        &self,
        world_id: &str,
        node_id: &str,
        alerts: &[MembershipRevocationPendingAlert],
    ) -> Result<(), WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.pending.lock().map_err(|_| {
            WorldError::Io("membership revocation recovery store lock poisoned".into())
        })?;
        if alerts.is_empty() {
            guard.remove(&key);
        } else {
            guard.insert(key, alerts.to_vec());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationAlertRecoveryStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationAlertRecoveryStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn pending_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = super::normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-alert-pending.json"
        )))
    }
}

impl MembershipRevocationAlertRecoveryStore for FileMembershipRevocationAlertRecoveryStore {
    fn load_pending(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationPendingAlert>, WorldError> {
        let path = self.pending_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(path)?;
        decode_pending_alerts(&bytes)
    }

    fn save_pending(
        &self,
        world_id: &str,
        node_id: &str,
        alerts: &[MembershipRevocationPendingAlert],
    ) -> Result<(), WorldError> {
        let path = self.pending_path(world_id, node_id)?;
        if alerts.is_empty() {
            if path.exists() {
                fs::remove_file(path)?;
            }
            return Ok(());
        }

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let bytes = serde_json::to_vec(alerts)?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct StoreBackedMembershipRevocationScheduleCoordinator {
    store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync>,
}

impl StoreBackedMembershipRevocationScheduleCoordinator {
    pub fn new(store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync>) -> Self {
        Self { store }
    }
}

impl super::super::membership_reconciliation::MembershipRevocationScheduleCoordinator
    for StoreBackedMembershipRevocationScheduleCoordinator
{
    fn acquire(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        lease_ttl_ms: i64,
    ) -> Result<bool, WorldError> {
        super::validate_coordinator_lease_ttl_ms(lease_ttl_ms)?;
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = super::normalized_node_id(node_id)?;

        if let Some(existing) = self.store.load(&world_id)? {
            let lease_active = now_ms < existing.expires_at_ms;
            if lease_active && existing.holder_node_id != node_id {
                return Ok(false);
            }
        }

        let expires_at_ms = super::checked_coordinator_lease_expiry(now_ms, lease_ttl_ms)?;

        self.store.save(
            &world_id,
            &MembershipRevocationCoordinatorLeaseState {
                holder_node_id: node_id,
                expires_at_ms,
            },
        )?;
        Ok(true)
    }

    fn release(&self, world_id: &str, node_id: &str) -> Result<(), WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = super::normalized_node_id(node_id)?;

        let should_clear = self
            .store
            .load(&world_id)?
            .map(|lease| lease.holder_node_id == node_id)
            .unwrap_or(false);
        if should_clear {
            self.store.clear(&world_id)?;
        }
        Ok(())
    }
}

fn decode_pending_alerts(
    bytes: &[u8],
) -> Result<Vec<MembershipRevocationPendingAlert>, WorldError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(current) = serde_json::from_slice::<Vec<MembershipRevocationPendingAlert>>(bytes) {
        return Ok(current);
    }

    let legacy = serde_json::from_slice::<
        Vec<super::super::membership_reconciliation::MembershipRevocationAnomalyAlert>,
    >(bytes)?;
    Ok(legacy
        .into_iter()
        .map(MembershipRevocationPendingAlert::from_legacy)
        .collect())
}
