use super::distributed_client::DistributedClient;
use super::distributed_dht::DistributedDht;
use super::distributed_head_follow::HeadFollower;
use super::error::WorldError;
use super::head_sync::follow_head_sync;
use super::observer::{
    HeadFollowReport, HeadSyncModeReport, HeadSyncModeWithDhtReport, HeadSyncSourceMode,
    HeadSyncSourceModeWithDht, ObserverClient, ObserverSubscription,
};
use oasis7_distfs::{BlobStore, FileStore};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObserverModeCounters {
    pub total: u64,
    pub applied: u64,
    pub fallback: u64,
}

impl ObserverModeCounters {
    fn record(&mut self, applied: bool, fallback_used: bool) {
        self.total += 1;
        if applied {
            self.applied += 1;
        }
        if fallback_used {
            self.fallback += 1;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObserverModeRuntimeMetricsSnapshot {
    pub network_only: ObserverModeCounters,
    pub path_index_only: ObserverModeCounters,
    pub network_then_path_index: ObserverModeCounters,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObserverModeWithDhtRuntimeMetricsSnapshot {
    pub network_with_dht_only: ObserverModeCounters,
    pub path_index_only: ObserverModeCounters,
    pub network_with_dht_then_path_index: ObserverModeCounters,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObserverRuntimeMetricsSnapshot {
    pub mode: ObserverModeRuntimeMetricsSnapshot,
    pub mode_with_dht: ObserverModeWithDhtRuntimeMetricsSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct ObserverRuntimeMetrics {
    snapshot: ObserverRuntimeMetricsSnapshot,
}

impl ObserverRuntimeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_mode_report(&mut self, report: &HeadSyncModeReport) {
        let counters = self.select_mode_counters_mut(report.mode);
        counters.record(report.report.applied.is_some(), report.fallback_used);
    }

    pub fn record_mode_with_dht_report(&mut self, report: &HeadSyncModeWithDhtReport) {
        let counters = self.select_mode_with_dht_counters_mut(report.mode);
        counters.record(report.report.applied.is_some(), report.fallback_used);
    }

    pub fn snapshot(&self) -> ObserverRuntimeMetricsSnapshot {
        self.snapshot.clone()
    }

    pub fn reset(&mut self) {
        self.snapshot = ObserverRuntimeMetricsSnapshot::default();
    }

    fn select_mode_counters_mut(&mut self, mode: HeadSyncSourceMode) -> &mut ObserverModeCounters {
        match mode {
            HeadSyncSourceMode::NetworkOnly => &mut self.snapshot.mode.network_only,
            HeadSyncSourceMode::PathIndexOnly => &mut self.snapshot.mode.path_index_only,
            HeadSyncSourceMode::NetworkThenPathIndex => {
                &mut self.snapshot.mode.network_then_path_index
            }
        }
    }

    fn select_mode_with_dht_counters_mut(
        &mut self,
        mode: HeadSyncSourceModeWithDht,
    ) -> &mut ObserverModeCounters {
        match mode {
            HeadSyncSourceModeWithDht::NetworkWithDhtOnly => {
                &mut self.snapshot.mode_with_dht.network_with_dht_only
            }
            HeadSyncSourceModeWithDht::PathIndexOnly => {
                &mut self.snapshot.mode_with_dht.path_index_only
            }
            HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex => {
                &mut self.snapshot.mode_with_dht.network_with_dht_then_path_index
            }
        }
    }
}

impl ObserverClient {
    pub fn sync_heads_with_mode_observed_report_and_record(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        metrics: &mut ObserverRuntimeMetrics,
    ) -> Result<HeadSyncModeReport, WorldError> {
        let observed =
            self.sync_heads_with_mode_observed_report(mode, subscription, follower, client, store)?;
        metrics.record_mode_report(&observed);
        Ok(observed)
    }

    pub fn sync_heads_with_dht_mode_observed_report_and_record(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        metrics: &mut ObserverRuntimeMetrics,
    ) -> Result<HeadSyncModeWithDhtReport, WorldError> {
        let observed = self.sync_heads_with_dht_mode_observed_report(
            mode,
            subscription,
            follower,
            dht,
            client,
            store,
        )?;
        metrics.record_mode_with_dht_report(&observed);
        Ok(observed)
    }

    pub fn follow_heads_with_mode_and_metrics(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        metrics: &mut ObserverRuntimeMetrics,
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            let observed = self.sync_heads_with_mode_observed_report(
                mode,
                subscription,
                follower,
                client,
                store,
            )?;
            metrics.record_mode_report(&observed);
            Ok(observed.report)
        })
    }

    pub fn follow_heads_with_dht_mode_and_metrics(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        metrics: &mut ObserverRuntimeMetrics,
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            let observed = self.sync_heads_with_dht_mode_observed_report(
                mode,
                subscription,
                follower,
                dht,
                client,
                store,
            )?;
            metrics.record_mode_with_dht_report(&observed);
            Ok(observed.report)
        })
    }
}

#[cfg(all(test, feature = "self_tests"))]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::{Action, World};
    use oasis7::GeoPos;
    use oasis7_distfs::LocalCasStore;

    use super::super::distributed::topic_head;
    use super::super::distributed::WorldHeadAnnounce;
    use super::super::distributed_client::DistributedClient;
    use super::super::distributed_dht::InMemoryDht;
    use super::super::distributed_head_follow::HeadFollower;
    use super::super::distributed_net::{DistributedNetwork, InMemoryNetwork};
    use super::super::distributed_storage::{
        store_execution_result_with_path_index, ExecutionWriteConfig, ExecutionWriteResult,
    };
    use super::super::observer::HeadSyncReport;
    use super::super::util::to_canonical_cbor;
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-net-{prefix}-{unique}"))
    }

    fn write_world_fixture(world_id: &str, store: &LocalCasStore) -> ExecutionWriteResult {
        let mut world = World::new();
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();
        store_execution_result_with_path_index(
            world_id,
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            store,
            ExecutionWriteConfig::default(),
        )
        .expect("write fixture")
    }

    fn publish_head(network: &Arc<dyn DistributedNetwork + Send + Sync>, head: &WorldHeadAnnounce) {
        let payload = to_canonical_cbor(head).expect("head cbor");
        network
            .publish(&topic_head(&head.world_id), &payload)
            .expect("publish");
    }

    fn sample_head(height: u64) -> WorldHeadAnnounce {
        WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height,
            block_hash: format!("block-{height}"),
            state_root: format!("state-{height}"),
            timestamp_ms: 1,
            signature: "sig".to_string(),
        }
    }

    fn build_mode_report(
        mode: HeadSyncSourceMode,
        applied: bool,
        fallback_used: bool,
    ) -> HeadSyncModeReport {
        let report = HeadSyncReport {
            drained: 1,
            applied: applied.then(|| super::super::observer::HeadSyncResult {
                head: sample_head(1),
                world: World::new(),
            }),
        };
        HeadSyncModeReport {
            mode,
            report,
            fallback_used,
        }
    }

    fn build_mode_with_dht_report(
        mode: HeadSyncSourceModeWithDht,
        applied: bool,
        fallback_used: bool,
    ) -> HeadSyncModeWithDhtReport {
        let report = HeadSyncReport {
            drained: 1,
            applied: applied.then(|| super::super::observer::HeadSyncResult {
                head: sample_head(2),
                world: World::new(),
            }),
        };
        HeadSyncModeWithDhtReport {
            mode,
            report,
            fallback_used,
        }
    }

    #[test]
    fn observer_runtime_metrics_records_mode_report_counters() {
        let mut metrics = ObserverRuntimeMetrics::new();

        metrics.record_mode_report(&build_mode_report(
            HeadSyncSourceMode::NetworkOnly,
            false,
            false,
        ));
        metrics.record_mode_report(&build_mode_report(
            HeadSyncSourceMode::PathIndexOnly,
            true,
            false,
        ));
        metrics.record_mode_report(&build_mode_report(
            HeadSyncSourceMode::NetworkThenPathIndex,
            true,
            true,
        ));

        let snapshot = metrics.snapshot();
        assert_eq!(
            snapshot.mode.network_only,
            ObserverModeCounters {
                total: 1,
                applied: 0,
                fallback: 0,
            }
        );
        assert_eq!(
            snapshot.mode.path_index_only,
            ObserverModeCounters {
                total: 1,
                applied: 1,
                fallback: 0,
            }
        );
        assert_eq!(
            snapshot.mode.network_then_path_index,
            ObserverModeCounters {
                total: 1,
                applied: 1,
                fallback: 1,
            }
        );
    }

    #[test]
    fn observer_runtime_metrics_records_dht_mode_counters_and_supports_reset() {
        let mut metrics = ObserverRuntimeMetrics::new();

        metrics.record_mode_with_dht_report(&build_mode_with_dht_report(
            HeadSyncSourceModeWithDht::NetworkWithDhtOnly,
            true,
            false,
        ));
        metrics.record_mode_with_dht_report(&build_mode_with_dht_report(
            HeadSyncSourceModeWithDht::PathIndexOnly,
            false,
            false,
        ));
        metrics.record_mode_with_dht_report(&build_mode_with_dht_report(
            HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex,
            true,
            true,
        ));
        metrics.record_mode_with_dht_report(&build_mode_with_dht_report(
            HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex,
            false,
            false,
        ));

        let snapshot = metrics.snapshot();
        assert_eq!(
            snapshot.mode_with_dht.network_with_dht_only,
            ObserverModeCounters {
                total: 1,
                applied: 1,
                fallback: 0,
            }
        );
        assert_eq!(
            snapshot.mode_with_dht.path_index_only,
            ObserverModeCounters {
                total: 1,
                applied: 0,
                fallback: 0,
            }
        );
        assert_eq!(
            snapshot.mode_with_dht.network_with_dht_then_path_index,
            ObserverModeCounters {
                total: 2,
                applied: 1,
                fallback: 1,
            }
        );

        metrics.reset();
        assert_eq!(
            metrics.snapshot(),
            ObserverRuntimeMetricsSnapshot::default()
        );
    }

    #[test]
    fn observer_bridge_sync_mode_records_metrics_on_fallback() {
        let dir = temp_dir("observer-bridge-sync-mode");
        let store = LocalCasStore::new(&dir);
        let write = write_world_fixture("w1", &store);
        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let mut metrics = ObserverRuntimeMetrics::new();
        let observed = observer
            .sync_heads_with_mode_observed_report_and_record(
                HeadSyncSourceMode::NetworkThenPathIndex,
                &subscription,
                &mut follower,
                &client,
                &store,
                &mut metrics,
            )
            .expect("bridge sync");

        assert!(observed.fallback_used);
        assert_eq!(
            metrics.snapshot().mode.network_then_path_index,
            ObserverModeCounters {
                total: 1,
                applied: 1,
                fallback: 1,
            }
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_bridge_follow_mode_records_metrics_per_round() {
        let dir = temp_dir("observer-bridge-follow-mode");
        let store = LocalCasStore::new(&dir);
        let write = write_world_fixture("w1", &store);
        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let mut metrics = ObserverRuntimeMetrics::new();
        let follow = observer
            .follow_heads_with_mode_and_metrics(
                HeadSyncSourceMode::PathIndexOnly,
                &subscription,
                &mut follower,
                &client,
                &store,
                &mut metrics,
                4,
            )
            .expect("bridge follow");

        assert_eq!(follow.rounds, 2);
        assert_eq!(follow.drained, 1);
        assert!(follow.applied.is_some());
        assert_eq!(
            metrics.snapshot().mode.path_index_only,
            ObserverModeCounters {
                total: 2,
                applied: 1,
                fallback: 0,
            }
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_bridge_sync_dht_mode_records_metrics_on_fallback() {
        let dir = temp_dir("observer-bridge-sync-dht-mode");
        let store = LocalCasStore::new(&dir);
        let write = write_world_fixture("w1", &store);
        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let mut metrics = ObserverRuntimeMetrics::new();
        let observed = observer
            .sync_heads_with_dht_mode_observed_report_and_record(
                HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
                &mut metrics,
            )
            .expect("bridge sync dht");

        assert!(observed.fallback_used);
        assert_eq!(
            metrics
                .snapshot()
                .mode_with_dht
                .network_with_dht_then_path_index,
            ObserverModeCounters {
                total: 1,
                applied: 1,
                fallback: 1,
            }
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_bridge_follow_dht_mode_records_metrics_per_round() {
        let dir = temp_dir("observer-bridge-follow-dht-mode");
        let store = LocalCasStore::new(&dir);
        let write = write_world_fixture("w1", &store);
        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let mut metrics = ObserverRuntimeMetrics::new();
        let follow = observer
            .follow_heads_with_dht_mode_and_metrics(
                HeadSyncSourceModeWithDht::PathIndexOnly,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
                &mut metrics,
                4,
            )
            .expect("bridge follow dht");

        assert_eq!(follow.rounds, 2);
        assert_eq!(follow.drained, 1);
        assert!(follow.applied.is_some());
        assert_eq!(
            metrics.snapshot().mode_with_dht.path_index_only,
            ObserverModeCounters {
                total: 2,
                applied: 1,
                fallback: 0,
            }
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
