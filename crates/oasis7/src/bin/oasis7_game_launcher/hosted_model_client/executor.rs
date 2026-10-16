//! Operator-provisioned SDK authority, checked independently of model credentials.
use super::{BindingTarget, Identity, Principal, config::ExecutorConfig};
use oasis7_agent_api::game::{GameTaskFenceV1, GameTaskStatusV1};
use oasis7_game_client::device_client::Client;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub(super) struct Executor {
    pub account_id: String,
    pub client: Arc<Client>,
    pub fence: GameTaskFenceV1,
}
pub(super) struct Executors {
    identity: Identity,
    entries: Vec<Arc<Executor>>,
    running: Arc<Mutex<HashSet<String>>>,
}
impl Executors {
    pub fn load(config: &[ExecutorConfig], identity: Identity) -> Result<Self, String> {
        let mut entries = Vec::new();
        let mut devices = HashSet::new();
        for configured in config {
            let client = Arc::new(
                Client::open(&configured.device_config_path)
                    .map_err(|_| "configured executor device unavailable")?,
            );
            let fence = read_fence(&client)?;
            let owner = identity
                .lock()
                .map_err(|_| "executor identity unavailable")?
                .as_mut()
                .is_some_and(|s| {
                    s.model_account_owns_player(&configured.account_id, &fence.owner_binding)
                });
            if !owner
                || !devices.insert((
                    fence.world_id.clone(),
                    fence.agent_id.clone(),
                    client.public_key.clone(),
                ))
            {
                return Err("configured executor ownership unavailable or duplicate".into());
            }
            entries.push(Arc::new(Executor {
                account_id: configured.account_id.clone(),
                client,
                fence,
            }));
        }
        Ok(Self {
            identity,
            entries,
            running: Arc::new(Mutex::new(HashSet::new())),
        })
    }
    pub fn available(&self, principal: &Principal) -> bool {
        self.entries
            .iter()
            .any(|e| e.account_id == principal.account_id && self.current(e, principal))
    }
    pub fn resolve(&self, principal: &Principal, agent: &str) -> Option<Arc<Executor>> {
        self.entries
            .iter()
            .find(|e| {
                e.account_id == principal.account_id
                    && e.fence.agent_id == agent
                    && self.current(e, principal)
            })
            .cloned()
    }
    pub fn authorize(&self, principal: &Principal, target: &BindingTarget) -> bool {
        self.entries.iter().any(|e| {
            target_matches(&e.fence, target)
                && e.account_id == principal.account_id
                && self.current(e, principal)
        })
    }
    fn current(&self, executor: &Executor, principal: &Principal) -> bool {
        if executor.account_id != principal.account_id {
            return false;
        }
        // Network is outside both identity and ConnectionManager locks.
        let Ok(fresh) = read_fence(&executor.client) else {
            return false;
        };
        if fresh != executor.fence {
            return false;
        }
        self.identity
            .lock()
            .ok()
            .and_then(|mut guard| {
                guard.as_mut().map(|service| {
                    service.model_session_current(principal)
                        && service
                            .model_account_owns_player(&principal.account_id, &fresh.owner_binding)
                })
            })
            .unwrap_or(false)
    }
    pub fn claim(&self, executor: &Executor, connection: &str) -> Result<RunClaim, ()> {
        let device = format!(
            "device:{}:{}:{}",
            executor.fence.world_id, executor.fence.agent_id, executor.client.public_key
        );
        let connection = format!("connection:{connection}");
        let mut running = self.running.lock().map_err(|_| ())?;
        if running.contains(&device) || running.contains(&connection) {
            return Err(());
        }
        running.insert(device.clone());
        running.insert(connection.clone());
        Ok(RunClaim {
            running: Arc::clone(&self.running),
            keys: [device, connection],
        })
    }
}
pub(super) struct RunClaim {
    running: Arc<Mutex<HashSet<String>>>,
    keys: [String; 2],
}
impl Drop for RunClaim {
    fn drop(&mut self) {
        if let Ok(mut running) = self.running.lock() {
            for key in &self.keys {
                running.remove(key);
            }
        }
    }
}
fn read_fence(client: &Client) -> Result<GameTaskFenceV1, String> {
    let view = client.public_view()?;
    let fence = view
        .task_fence()
        .ok_or("executor task fence unavailable")?
        .clone();
    validate_device_fence(
        &fence,
        &client.world.world_id,
        &client.world.genesis_digest,
        &client.agent,
        &client.public_key,
    )?;
    Ok(fence)
}
fn validate_device_fence(
    fence: &GameTaskFenceV1,
    world: &str,
    genesis: &str,
    agent: &str,
    public_key: &str,
) -> Result<(), String> {
    fence.validate()?;
    if fence.world_id != world
        || fence.genesis_digest != genesis
        || fence.agent_id != agent
        || fence.task_status != GameTaskStatusV1::Active
        || fence.active_goal_reference.is_none()
        || !fence
            .executor_binding
            .as_ref()
            .is_some_and(|b| b.executor_public_key == public_key)
    {
        return Err("configured executor Runtime authority mismatch".into());
    }
    Ok(())
}
pub(super) fn target(fence: &GameTaskFenceV1) -> BindingTarget {
    BindingTarget {
        world_id: fence.world_id.clone(),
        agent_id: fence.agent_id.clone(),
        task_id: fence.task_id.clone(),
        goal_revision: fence.active_goal_revision.0,
        executor_id: fence
            .executor_binding
            .as_ref()
            .map(|b| b.executor_id.clone())
            .unwrap_or_default(),
        executor_epoch: fence.executor_epoch.0,
    }
}
fn target_matches(fence: &GameTaskFenceV1, candidate: &BindingTarget) -> bool {
    target(fence) == *candidate
}

pub(super) fn run_config(
    executor: &Executor,
    preset: &oasis7_model_provider::preset::ProviderPreset,
    limits: &oasis7_model_provider::connection::Limits,
) -> oasis7_agent_harness::NativeTaskRunConfig {
    // Stable across connections, sessions, generation changes and process
    // restarts: opening a new model connection cannot bypass an old original.
    let run_id = run_identity(&executor.client.public_key, &executor.fence);
    oasis7_agent_harness::NativeTaskRunConfig {
        run_id,
        model: preset.model.clone(),
        system_prompt: format!(
            "Operate the complete authorized native industry task. Model preset {} revision {}. Completion of this bounded run does not prove completion of the player goal.",
            preset.id, preset.revision
        ),
        max_turns: 4,
        max_model_calls: limits.max_calls.min(1024) as u32,
        max_tool_calls_per_call: 8,
        model_timeout_ms: preset.limits.timeout_ms.clamp(10, 120_000),
        control_poll_ms: 10,
    }
}

fn run_identity(public_key: &str, fence: &GameTaskFenceV1) -> String {
    let digest = oasis7_agent_api::h_v1("oasis7.byok.native-run.v1", &(public_key, fence));
    format!(
        "byok:{}",
        digest
            .as_str()
            .strip_prefix("blake3:")
            .unwrap_or(digest.as_str())
    )
}
#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
