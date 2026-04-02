use std::collections::{BTreeMap, BTreeSet};

use super::distributed_dht::{DistributedDht, ProviderRecord};
use super::error::WorldError;
use super::provider_selection::ProviderSelectionPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaMaintenancePolicy {
    pub target_replicas_per_blob: usize,
    pub max_repairs_per_round: usize,
    pub max_rebalances_per_round: usize,
    pub rebalance_source_load_min_per_mille: u16,
    pub rebalance_target_load_max_per_mille: u16,
}

impl Default for ReplicaMaintenancePolicy {
    fn default() -> Self {
        Self {
            target_replicas_per_blob: 3,
            max_repairs_per_round: 32,
            max_rebalances_per_round: 32,
            rebalance_source_load_min_per_mille: 850,
            rebalance_target_load_max_per_mille: 450,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaTransferKind {
    Repair,
    Rebalance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaTransferTask {
    pub kind: ReplicaTransferKind,
    pub content_hash: String,
    pub source_provider_id: String,
    pub target_provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplicaMaintenancePlan {
    pub repair_tasks: Vec<ReplicaTransferTask>,
    pub rebalance_tasks: Vec<ReplicaTransferTask>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaMaintenanceFailedTask {
    pub task: ReplicaTransferTask,
    pub error: WorldError,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplicaMaintenanceReport {
    pub attempted_tasks: usize,
    pub succeeded_tasks: usize,
    pub failed_tasks: Vec<ReplicaMaintenanceFailedTask>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaMaintenancePollingPolicy {
    pub poll_interval_ms: i64,
}

impl Default for ReplicaMaintenancePollingPolicy {
    fn default() -> Self {
        Self {
            poll_interval_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplicaMaintenancePollingState {
    pub last_polled_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaMaintenanceRoundResult {
    pub polled_at_ms: i64,
    pub plan: ReplicaMaintenancePlan,
    pub report: ReplicaMaintenanceReport,
}

pub trait ReplicaTransferExecutor {
    fn execute_transfer(
        &self,
        world_id: &str,
        task: &ReplicaTransferTask,
    ) -> Result<(), WorldError>;
}

pub fn execute_replica_maintenance_plan(
    dht: &impl DistributedDht,
    executor: &impl ReplicaTransferExecutor,
    world_id: &str,
    plan: &ReplicaMaintenancePlan,
) -> ReplicaMaintenanceReport {
    let mut report = ReplicaMaintenanceReport::default();

    for task in plan.repair_tasks.iter().chain(plan.rebalance_tasks.iter()) {
        report.attempted_tasks = report.attempted_tasks.saturating_add(1);
        match execute_replica_task(dht, executor, world_id, task) {
            Ok(()) => {
                report.succeeded_tasks = report.succeeded_tasks.saturating_add(1);
            }
            Err(error) => {
                report.failed_tasks.push(ReplicaMaintenanceFailedTask {
                    task: task.clone(),
                    error,
                });
            }
        }
    }

    report
}

pub fn run_replica_maintenance_poll(
    dht: &impl DistributedDht,
    executor: &impl ReplicaTransferExecutor,
    world_id: &str,
    content_hashes: &[String],
    maintenance_policy: ReplicaMaintenancePolicy,
    polling_policy: ReplicaMaintenancePollingPolicy,
    state: &mut ReplicaMaintenancePollingState,
    now_ms: i64,
) -> Result<Option<ReplicaMaintenanceRoundResult>, WorldError> {
    validate_polling_policy(polling_policy)?;
    if !should_run_poll(
        state.last_polled_at_ms,
        now_ms,
        polling_policy.poll_interval_ms,
    ) {
        return Ok(None);
    }

    let plan = plan_replica_maintenance(dht, world_id, content_hashes, maintenance_policy)?;
    let report = execute_replica_maintenance_plan(dht, executor, world_id, &plan);
    state.last_polled_at_ms = Some(now_ms);
    Ok(Some(ReplicaMaintenanceRoundResult {
        polled_at_ms: now_ms,
        plan,
        report,
    }))
}

pub fn plan_replica_maintenance(
    dht: &impl DistributedDht,
    world_id: &str,
    content_hashes: &[String],
    policy: ReplicaMaintenancePolicy,
) -> Result<ReplicaMaintenancePlan, WorldError> {
    validate_policy(policy)?;

    let required_hashes = normalize_hashes(content_hashes);
    if required_hashes.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "replica maintenance requires at least one content hash".to_string(),
        });
    }

    let mut providers_by_hash: BTreeMap<String, Vec<ProviderRecord>> = BTreeMap::new();
    for content_hash in &required_hashes {
        let providers = dedupe_providers(dht.get_providers(world_id, content_hash)?);
        providers_by_hash.insert(content_hash.clone(), providers);
    }

    let mut plan = ReplicaMaintenancePlan::default();
    plan_repair_tasks(&providers_by_hash, policy, &mut plan);
    plan_rebalance_tasks(&providers_by_hash, policy, &mut plan);
    Ok(plan)
}

fn execute_replica_task(
    dht: &impl DistributedDht,
    executor: &impl ReplicaTransferExecutor,
    world_id: &str,
    task: &ReplicaTransferTask,
) -> Result<(), WorldError> {
    executor.execute_transfer(world_id, task)?;
    dht.publish_provider(world_id, &task.content_hash, &task.target_provider_id)?;
    Ok(())
}

fn validate_policy(policy: ReplicaMaintenancePolicy) -> Result<(), WorldError> {
    if policy.target_replicas_per_blob == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "replica maintenance policy requires target_replicas_per_blob > 0".to_string(),
        });
    }
    Ok(())
}

fn validate_polling_policy(policy: ReplicaMaintenancePollingPolicy) -> Result<(), WorldError> {
    if policy.poll_interval_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "replica maintenance polling policy requires poll_interval_ms > 0".to_string(),
        });
    }
    Ok(())
}

fn should_run_poll(last_polled_at_ms: Option<i64>, now_ms: i64, poll_interval_ms: i64) -> bool {
    match last_polled_at_ms {
        Some(last_polled_at_ms) => now_ms.saturating_sub(last_polled_at_ms) >= poll_interval_ms,
        None => true,
    }
}

fn plan_repair_tasks(
    providers_by_hash: &BTreeMap<String, Vec<ProviderRecord>>,
    policy: ReplicaMaintenancePolicy,
    plan: &mut ReplicaMaintenancePlan,
) {
    if policy.max_repairs_per_round == 0 {
        return;
    }

    let all_candidates = collect_global_candidates(providers_by_hash);
    let selector = ProviderSelectionPolicy::default();

    for (content_hash, providers) in providers_by_hash {
        if plan.repair_tasks.len() >= policy.max_repairs_per_round {
            return;
        }

        let current_replica_count = providers.len();
        if current_replica_count >= policy.target_replicas_per_blob {
            continue;
        }

        let Some(source) = selector
            .rank_providers(providers, selection_now_ms(providers))
            .into_iter()
            .next()
        else {
            plan.warnings.push(format!(
                "repair planning skipped for content_hash={content_hash}: no source provider"
            ));
            continue;
        };

        let mut selected_targets: BTreeSet<String> =
            providers.iter().map(|p| p.provider_id.clone()).collect();
        let target_candidates = selector.rank_providers(
            &all_candidates
                .iter()
                .filter(|candidate| !selected_targets.contains(&candidate.provider_id))
                .cloned()
                .collect::<Vec<_>>(),
            selection_now_ms(&all_candidates),
        );

        let needed = policy
            .target_replicas_per_blob
            .saturating_sub(current_replica_count);
        let mut produced = 0usize;
        for target in target_candidates {
            if produced >= needed || plan.repair_tasks.len() >= policy.max_repairs_per_round {
                break;
            }
            if !selected_targets.insert(target.provider_id.clone()) {
                continue;
            }
            plan.repair_tasks.push(ReplicaTransferTask {
                kind: ReplicaTransferKind::Repair,
                content_hash: content_hash.clone(),
                source_provider_id: source.provider_id.clone(),
                target_provider_id: target.provider_id,
            });
            produced = produced.saturating_add(1);
        }

        if produced < needed {
            plan.warnings.push(format!(
                "repair planning insufficient targets for content_hash={content_hash}: needed={needed}, planned={produced}"
            ));
        }
    }
}

fn plan_rebalance_tasks(
    providers_by_hash: &BTreeMap<String, Vec<ProviderRecord>>,
    policy: ReplicaMaintenancePolicy,
    plan: &mut ReplicaMaintenancePlan,
) {
    if policy.max_rebalances_per_round == 0 {
        return;
    }

    let all_candidates = collect_global_candidates(providers_by_hash);
    let underloaded: Vec<ProviderRecord> = all_candidates
        .iter()
        .filter(|record| {
            record
                .load_ratio_per_mille
                .map(|load| load <= policy.rebalance_target_load_max_per_mille)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let mut existing_tasks: BTreeSet<(String, String)> = plan
        .repair_tasks
        .iter()
        .map(|task| (task.content_hash.clone(), task.target_provider_id.clone()))
        .collect();

    for (content_hash, providers) in providers_by_hash {
        if plan.rebalance_tasks.len() >= policy.max_rebalances_per_round {
            return;
        }

        let source = providers
            .iter()
            .filter(|record| {
                record
                    .load_ratio_per_mille
                    .map(|load| load >= policy.rebalance_source_load_min_per_mille)
                    .unwrap_or(false)
            })
            .cloned()
            .max_by_key(|record| {
                (
                    record.load_ratio_per_mille.unwrap_or(0),
                    record.last_seen_ms,
                    std::cmp::Reverse(record.provider_id.clone()),
                )
            });
        let Some(source) = source else {
            continue;
        };

        let occupied: BTreeSet<String> = providers.iter().map(|p| p.provider_id.clone()).collect();
        let target = underloaded
            .iter()
            .filter(|candidate| !occupied.contains(&candidate.provider_id))
            .cloned()
            .min_by_key(|record| {
                (
                    record.load_ratio_per_mille.unwrap_or(u16::MAX),
                    std::cmp::Reverse(record.last_seen_ms),
                    record.provider_id.clone(),
                )
            });
        let Some(target) = target else {
            continue;
        };

        let task_key = (content_hash.clone(), target.provider_id.clone());
        if existing_tasks.contains(&task_key) {
            continue;
        }

        existing_tasks.insert(task_key);
        plan.rebalance_tasks.push(ReplicaTransferTask {
            kind: ReplicaTransferKind::Rebalance,
            content_hash: content_hash.clone(),
            source_provider_id: source.provider_id,
            target_provider_id: target.provider_id,
        });
    }
}

fn collect_global_candidates(
    providers_by_hash: &BTreeMap<String, Vec<ProviderRecord>>,
) -> Vec<ProviderRecord> {
    let mut by_id: BTreeMap<String, ProviderRecord> = BTreeMap::new();
    for providers in providers_by_hash.values() {
        for record in providers {
            by_id
                .entry(record.provider_id.clone())
                .and_modify(|existing| {
                    if record.last_seen_ms > existing.last_seen_ms {
                        *existing = record.clone();
                    }
                })
                .or_insert_with(|| record.clone());
        }
    }
    by_id.into_values().collect()
}

fn dedupe_providers(providers: Vec<ProviderRecord>) -> Vec<ProviderRecord> {
    let mut by_id: BTreeMap<String, ProviderRecord> = BTreeMap::new();
    for record in providers {
        by_id
            .entry(record.provider_id.clone())
            .and_modify(|existing| {
                if record.last_seen_ms > existing.last_seen_ms {
                    *existing = record.clone();
                }
            })
            .or_insert(record);
    }
    by_id.into_values().collect()
}

fn normalize_hashes(content_hashes: &[String]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for content_hash in content_hashes {
        set.insert(content_hash.clone());
    }
    set.into_iter().collect()
}

fn selection_now_ms(providers: &[ProviderRecord]) -> i64 {
    providers
        .iter()
        .map(|record| record.last_seen_ms)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    use oasis7_proto::distributed as proto_distributed;

    use super::*;
    use crate::proto_dht;

    #[derive(Clone, Default)]
    struct StaticProvidersDht {
        providers_by_hash: Arc<Mutex<HashMap<String, Vec<ProviderRecord>>>>,
        published: Arc<Mutex<Vec<(String, String, String)>>>,
    }

    impl StaticProvidersDht {
        fn with_providers_by_hash(providers_by_hash: HashMap<String, Vec<ProviderRecord>>) -> Self {
            Self {
                providers_by_hash: Arc::new(Mutex::new(providers_by_hash)),
                published: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn published(&self) -> Vec<(String, String, String)> {
            self.published.lock().expect("lock published").clone()
        }
    }

    impl proto_dht::DistributedDht<WorldError> for StaticProvidersDht {
        fn publish_provider(
            &self,
            world_id: &str,
            content_hash: &str,
            provider_id: &str,
        ) -> Result<(), WorldError> {
            self.published.lock().expect("lock published").push((
                world_id.to_string(),
                content_hash.to_string(),
                provider_id.to_string(),
            ));

            let mut providers_by_hash = self.providers_by_hash.lock().expect("lock providers");
            let providers = providers_by_hash
                .entry(content_hash.to_string())
                .or_default();
            if providers
                .iter()
                .all(|record| record.provider_id != provider_id)
            {
                providers.push(provider(provider_id, Some(300)));
            }
            Ok(())
        }

        fn get_providers(
            &self,
            _world_id: &str,
            content_hash: &str,
        ) -> Result<Vec<ProviderRecord>, WorldError> {
            Ok(self
                .providers_by_hash
                .lock()
                .expect("lock providers")
                .get(content_hash)
                .cloned()
                .unwrap_or_default())
        }

        fn put_world_head(
            &self,
            _world_id: &str,
            _head: &proto_distributed::WorldHeadAnnounce,
        ) -> Result<(), WorldError> {
            Ok(())
        }

        fn get_world_head(
            &self,
            _world_id: &str,
        ) -> Result<Option<proto_distributed::WorldHeadAnnounce>, WorldError> {
            Ok(None)
        }

        fn put_membership_directory(
            &self,
            _world_id: &str,
            _snapshot: &super::super::distributed_dht::MembershipDirectorySnapshot,
        ) -> Result<(), WorldError> {
            Ok(())
        }

        fn get_membership_directory(
            &self,
            _world_id: &str,
        ) -> Result<Option<super::super::distributed_dht::MembershipDirectorySnapshot>, WorldError>
        {
            Ok(None)
        }

        fn put_peer_record(
            &self,
            _world_id: &str,
            _record: &super::super::distributed_dht::SignedPeerRecord,
        ) -> Result<(), WorldError> {
            Ok(())
        }

        fn get_peer_record(
            &self,
            _world_id: &str,
            _peer_id: &str,
        ) -> Result<Option<super::super::distributed_dht::SignedPeerRecord>, WorldError> {
            Ok(None)
        }
    }

    #[derive(Clone, Default)]
    struct ScriptedTransferExecutor {
        failed_hashes: HashSet<String>,
    }

    impl ScriptedTransferExecutor {
        fn fail_on_hashes(content_hashes: &[&str]) -> Self {
            Self {
                failed_hashes: content_hashes
                    .iter()
                    .map(|content_hash| (*content_hash).to_string())
                    .collect(),
            }
        }
    }

    impl ReplicaTransferExecutor for ScriptedTransferExecutor {
        fn execute_transfer(
            &self,
            _world_id: &str,
            task: &ReplicaTransferTask,
        ) -> Result<(), WorldError> {
            if self.failed_hashes.contains(&task.content_hash) {
                return Err(WorldError::NetworkRequestFailed {
                    code: proto_distributed::DistributedErrorCode::ErrNotAvailable,
                    message: "transfer failed".to_string(),
                    retryable: true,
                });
            }
            Ok(())
        }
    }

    fn provider(provider_id: &str, load_ratio_per_mille: Option<u16>) -> ProviderRecord {
        ProviderRecord {
            provider_id: provider_id.to_string(),
            last_seen_ms: 1_000,
            storage_total_bytes: Some(1_000),
            storage_available_bytes: Some(500),
            uptime_ratio_per_mille: Some(990),
            challenge_pass_ratio_per_mille: Some(980),
            load_ratio_per_mille,
            p50_read_latency_ms: Some(20),
        }
    }

    fn map(entries: &[(&str, Vec<ProviderRecord>)]) -> HashMap<String, Vec<ProviderRecord>> {
        let mut out = HashMap::new();
        for (hash, providers) in entries {
            out.insert((*hash).to_string(), providers.clone());
        }
        out
    }

    #[test]
    fn plan_replica_maintenance_creates_repair_tasks_for_under_replicated_blob() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[
            (
                "hash-a",
                vec![provider("peer-1", Some(300)), provider("peer-2", Some(400))],
            ),
            ("hash-b", vec![provider("peer-1", Some(300))]),
        ]));
        let hashes = vec!["hash-a".to_string(), "hash-b".to_string()];

        let plan = plan_replica_maintenance(
            &dht,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy {
                target_replicas_per_blob: 2,
                max_repairs_per_round: 8,
                max_rebalances_per_round: 0,
                ..ReplicaMaintenancePolicy::default()
            },
        )
        .expect("plan");

        assert!(!plan.repair_tasks.is_empty());
        assert!(plan
            .repair_tasks
            .iter()
            .any(|task| task.content_hash == "hash-b"));
        assert!(plan.rebalance_tasks.is_empty());
    }

    #[test]
    fn plan_replica_maintenance_creates_rebalance_tasks_for_overloaded_provider() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[
            (
                "hash-a",
                vec![
                    provider("peer-hot", Some(950)),
                    provider("peer-cool", Some(200)),
                ],
            ),
            (
                "hash-b",
                vec![
                    provider("peer-hot", Some(940)),
                    provider("peer-warm", Some(300)),
                ],
            ),
            (
                "hash-c",
                vec![
                    provider("peer-hot", Some(930)),
                    provider("peer-cool", Some(220)),
                ],
            ),
        ]));
        let hashes = vec![
            "hash-a".to_string(),
            "hash-b".to_string(),
            "hash-c".to_string(),
        ];

        let plan = plan_replica_maintenance(
            &dht,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy {
                target_replicas_per_blob: 2,
                max_repairs_per_round: 0,
                max_rebalances_per_round: 8,
                rebalance_source_load_min_per_mille: 900,
                rebalance_target_load_max_per_mille: 350,
            },
        )
        .expect("plan");

        assert!(plan.repair_tasks.is_empty());
        assert!(!plan.rebalance_tasks.is_empty());
        assert!(plan
            .rebalance_tasks
            .iter()
            .all(|task| task.kind == ReplicaTransferKind::Rebalance));
    }

    #[test]
    fn plan_replica_maintenance_writes_warning_when_no_target_candidate() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[(
            "hash-a",
            vec![provider("peer-only", Some(500))],
        )]));
        let hashes = vec!["hash-a".to_string()];

        let plan = plan_replica_maintenance(
            &dht,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy {
                target_replicas_per_blob: 3,
                max_repairs_per_round: 8,
                max_rebalances_per_round: 0,
                ..ReplicaMaintenancePolicy::default()
            },
        )
        .expect("plan");

        assert!(plan.repair_tasks.is_empty());
        assert!(!plan.warnings.is_empty());
    }

    #[test]
    fn execute_replica_maintenance_plan_publishes_target_provider_on_success() {
        let dht = StaticProvidersDht::default();
        let executor = ScriptedTransferExecutor::default();
        let plan = ReplicaMaintenancePlan {
            repair_tasks: vec![ReplicaTransferTask {
                kind: ReplicaTransferKind::Repair,
                content_hash: "hash-a".to_string(),
                source_provider_id: "peer-1".to_string(),
                target_provider_id: "peer-2".to_string(),
            }],
            ..ReplicaMaintenancePlan::default()
        };

        let report = execute_replica_maintenance_plan(&dht, &executor, "w1", &plan);

        assert_eq!(report.attempted_tasks, 1);
        assert_eq!(report.succeeded_tasks, 1);
        assert!(report.failed_tasks.is_empty());
        assert_eq!(
            dht.published(),
            vec![("w1".to_string(), "hash-a".to_string(), "peer-2".to_string(),)]
        );
    }

    #[test]
    fn execute_replica_maintenance_plan_does_not_publish_on_transfer_failure() {
        let dht = StaticProvidersDht::default();
        let executor = ScriptedTransferExecutor::fail_on_hashes(&["hash-a"]);
        let plan = ReplicaMaintenancePlan {
            repair_tasks: vec![ReplicaTransferTask {
                kind: ReplicaTransferKind::Repair,
                content_hash: "hash-a".to_string(),
                source_provider_id: "peer-1".to_string(),
                target_provider_id: "peer-2".to_string(),
            }],
            ..ReplicaMaintenancePlan::default()
        };

        let report = execute_replica_maintenance_plan(&dht, &executor, "w1", &plan);

        assert_eq!(report.attempted_tasks, 1);
        assert_eq!(report.succeeded_tasks, 0);
        assert_eq!(report.failed_tasks.len(), 1);
        assert!(matches!(
            report.failed_tasks[0].error,
            WorldError::NetworkRequestFailed { .. }
        ));
        assert!(dht.published().is_empty());
    }

    #[test]
    fn run_replica_maintenance_poll_runs_first_round_and_updates_state() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[
            ("hash-a", vec![provider("peer-1", Some(300))]),
            ("hash-b", vec![provider("peer-2", Some(320))]),
        ]));
        let executor = ScriptedTransferExecutor::default();
        let hashes = vec!["hash-a".to_string(), "hash-b".to_string()];
        let mut state = ReplicaMaintenancePollingState::default();

        let result = run_replica_maintenance_poll(
            &dht,
            &executor,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy {
                target_replicas_per_blob: 2,
                max_repairs_per_round: 8,
                max_rebalances_per_round: 0,
                ..ReplicaMaintenancePolicy::default()
            },
            ReplicaMaintenancePollingPolicy {
                poll_interval_ms: 100,
            },
            &mut state,
            1_000,
        )
        .expect("poll result");

        let round = result.expect("first round should run");
        assert_eq!(round.polled_at_ms, 1_000);
        assert!(round.report.succeeded_tasks >= 1);
        assert_eq!(state.last_polled_at_ms, Some(1_000));
        assert!(!dht.published().is_empty());
    }

    #[test]
    fn run_replica_maintenance_poll_skips_when_interval_not_elapsed() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[
            ("hash-a", vec![provider("peer-1", Some(300))]),
            ("hash-b", vec![provider("peer-2", Some(320))]),
        ]));
        let executor = ScriptedTransferExecutor::default();
        let hashes = vec!["hash-a".to_string(), "hash-b".to_string()];
        let mut state = ReplicaMaintenancePollingState {
            last_polled_at_ms: Some(1_000),
        };

        let result = run_replica_maintenance_poll(
            &dht,
            &executor,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy::default(),
            ReplicaMaintenancePollingPolicy {
                poll_interval_ms: 100,
            },
            &mut state,
            1_050,
        )
        .expect("poll result");

        assert!(result.is_none());
        assert_eq!(state.last_polled_at_ms, Some(1_000));
        assert!(dht.published().is_empty());
    }

    #[test]
    fn run_replica_maintenance_poll_rejects_non_positive_interval() {
        let dht = StaticProvidersDht::with_providers_by_hash(map(&[
            ("hash-a", vec![provider("peer-1", Some(300))]),
            ("hash-b", vec![provider("peer-2", Some(320))]),
        ]));
        let executor = ScriptedTransferExecutor::default();
        let hashes = vec!["hash-a".to_string(), "hash-b".to_string()];
        let mut state = ReplicaMaintenancePollingState::default();

        let err = run_replica_maintenance_poll(
            &dht,
            &executor,
            "w1",
            &hashes,
            ReplicaMaintenancePolicy::default(),
            ReplicaMaintenancePollingPolicy {
                poll_interval_ms: 0,
            },
            &mut state,
            1_000,
        )
        .expect_err("interval=0 should fail");

        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { .. }
        ));
        assert_eq!(state.last_polled_at_ms, None);
        assert!(dht.published().is_empty());
    }
}
