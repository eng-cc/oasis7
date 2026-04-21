use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::{
    normalized_schedule_key, MembershipRevocationDeadLetterReplayPolicyState,
    MembershipRevocationDeadLetterReplayPolicyStore,
    MembershipRevocationDeadLetterReplayScheduleState,
    MembershipRevocationDeadLetterReplayStateStore,
};
use crate::error::WorldError;

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayStateStore {
    states:
        Arc<Mutex<BTreeMap<(String, String), MembershipRevocationDeadLetterReplayScheduleState>>>,
}

impl InMemoryMembershipRevocationDeadLetterReplayStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayPolicyStore {
    states: Arc<Mutex<BTreeMap<(String, String), MembershipRevocationDeadLetterReplayPolicyState>>>,
}

impl InMemoryMembershipRevocationDeadLetterReplayPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayPolicyStore
    for InMemoryMembershipRevocationDeadLetterReplayPolicyStore
{
    fn load_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayPolicyState, WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay policy state lock poisoned".into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayPolicyState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay policy state lock poisoned".into(),
            )
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

impl MembershipRevocationDeadLetterReplayStateStore
    for InMemoryMembershipRevocationDeadLetterReplayStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayScheduleState, WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter replay state lock poisoned".into())
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayScheduleState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter replay state lock poisoned".into())
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-state.json"
        )))
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayPolicyStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayPolicyStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-policy-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayStateStore
    for FileMembershipRevocationDeadLetterReplayStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayScheduleState, WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(MembershipRevocationDeadLetterReplayScheduleState::default());
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayScheduleState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec(state)?)?;
        Ok(())
    }
}

impl MembershipRevocationDeadLetterReplayPolicyStore
    for FileMembershipRevocationDeadLetterReplayPolicyStore
{
    fn load_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayPolicyState, WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(MembershipRevocationDeadLetterReplayPolicyState::default());
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayPolicyState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec(state)?)?;
        Ok(())
    }
}
