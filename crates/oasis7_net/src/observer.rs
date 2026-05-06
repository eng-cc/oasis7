use std::sync::Arc;

use super::distributed::{topic_event, topic_head, WorldHeadAnnounce};
use super::distributed_client::DistributedClient;
use super::distributed_dht::DistributedDht;
use super::distributed_head_follow::HeadFollower;
use super::distributed_net::{DistributedNetwork, NetworkSubscription};
use super::error::WorldError;
use super::head_sync::{
    compose_head_sync_report, follow_head_sync, HeadFollowReport as GenericHeadFollowReport,
    HeadSyncReport as GenericHeadSyncReport, HeadSyncResult as GenericHeadSyncResult,
};
use oasis7::runtime::World;
use oasis7_distfs::{BlobStore, FileStore};

#[derive(Debug, Clone)]
pub struct ObserverSubscription {
    pub event_sub: NetworkSubscription,
    pub head_sub: NetworkSubscription,
}

pub type HeadSyncResult = GenericHeadSyncResult<World>;
pub type HeadSyncReport = GenericHeadSyncReport<World>;
pub type HeadFollowReport = GenericHeadFollowReport<World>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadSyncSourceMode {
    NetworkOnly,
    PathIndexOnly,
    NetworkThenPathIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadSyncSourceModeWithDht {
    NetworkWithDhtOnly,
    PathIndexOnly,
    NetworkWithDhtThenPathIndex,
}

#[derive(Debug)]
pub struct HeadSyncModeReport {
    pub mode: HeadSyncSourceMode,
    pub report: HeadSyncReport,
    pub fallback_used: bool,
}

#[derive(Debug)]
pub struct HeadSyncModeWithDhtReport {
    pub mode: HeadSyncSourceModeWithDht,
    pub report: HeadSyncReport,
    pub fallback_used: bool,
}

#[derive(Debug)]
struct ModeSyncOutcome {
    world: Option<World>,
    fallback_used: bool,
}

#[derive(Clone)]
pub struct ObserverClient {
    network: Arc<dyn DistributedNetwork + Send + Sync>,
}

impl ObserverClient {
    pub fn new(network: Arc<dyn DistributedNetwork + Send + Sync>) -> Self {
        Self { network }
    }

    pub fn subscribe(&self, world_id: &str) -> Result<ObserverSubscription, WorldError> {
        let event_topic = topic_event(world_id);
        let head_topic = topic_head(world_id);
        let event_sub = self.network.subscribe(&event_topic)?;
        let head_sub = self.network.subscribe(&head_topic)?;
        Ok(ObserverSubscription {
            event_sub,
            head_sub,
        })
    }

    pub fn drain_events(
        &self,
        subscription: &ObserverSubscription,
    ) -> Result<Vec<Vec<u8>>, WorldError> {
        Ok(subscription.event_sub.drain())
    }

    pub fn drain_heads(
        &self,
        subscription: &ObserverSubscription,
    ) -> Result<Vec<WorldHeadAnnounce>, WorldError> {
        let raw = subscription.head_sub.drain();
        let mut heads = Vec::new();
        for bytes in raw {
            heads.push(serde_cbor::from_slice(&bytes)?);
        }
        Ok(heads)
    }

    pub fn sync_heads(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<Option<World>, WorldError> {
        let heads = self.drain_heads(subscription)?;
        follower.sync_from_heads(&heads, client, store)
    }

    pub fn sync_heads_report(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<HeadSyncReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let world = follower.sync_from_heads(&heads, client, store)?;
        compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
            WorldError::DistributedValidationFailed {
                reason: "head follower did not record applied head".to_string(),
            }
        })
    }

    pub fn sync_heads_with_result(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<Option<HeadSyncResult>, WorldError> {
        let world = self.sync_heads(subscription, follower, client, store)?;
        match world {
            Some(world) => {
                let head = follower.current_head().cloned().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "head follower did not record applied head".to_string(),
                    }
                })?;
                Ok(Some(HeadSyncResult { head, world }))
            }
            None => Ok(None),
        }
    }

    pub fn sync_heads_with_dht(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<Option<World>, WorldError> {
        let heads = self.drain_heads(subscription)?;
        follower.sync_from_heads_with_dht(&heads, dht, client, store)
    }

    pub fn sync_heads_with_dht_report(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<HeadSyncReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let world = follower.sync_from_heads_with_dht(&heads, dht, client, store)?;
        compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
            WorldError::DistributedValidationFailed {
                reason: "head follower did not record applied head".to_string(),
            }
        })
    }

    pub fn sync_heads_with_dht_result(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &impl BlobStore,
    ) -> Result<Option<HeadSyncResult>, WorldError> {
        let world = self.sync_heads_with_dht(subscription, follower, dht, client, store)?;
        match world {
            Some(world) => {
                let head = follower.current_head().cloned().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "head follower did not record applied head".to_string(),
                    }
                })?;
                Ok(Some(HeadSyncResult { head, world }))
            }
            None => Ok(None),
        }
    }

    pub fn follow_heads(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &impl BlobStore,
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            self.sync_heads_report(subscription, follower, client, store)
        })
    }

    pub fn follow_heads_with_dht(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &impl BlobStore,
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            self.sync_heads_with_dht_report(subscription, follower, dht, client, store)
        })
    }

    pub fn sync_heads_with_path_index(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<World>, WorldError> {
        let heads = self.drain_heads(subscription)?;
        follower.sync_from_heads_with_path_index(&heads, store)
    }

    pub fn sync_heads_with_path_index_report(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        store: &(impl BlobStore + FileStore),
    ) -> Result<HeadSyncReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let world = follower.sync_from_heads_with_path_index(&heads, store)?;
        compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
            WorldError::DistributedValidationFailed {
                reason: "head follower did not record applied head".to_string(),
            }
        })
    }

    pub fn sync_heads_with_path_index_result(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<HeadSyncResult>, WorldError> {
        let world = self.sync_heads_with_path_index(subscription, follower, store)?;
        match world {
            Some(world) => {
                let head = follower.current_head().cloned().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "head follower did not record applied head".to_string(),
                    }
                })?;
                Ok(Some(HeadSyncResult { head, world }))
            }
            None => Ok(None),
        }
    }

    pub fn follow_heads_with_path_index(
        &self,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        store: &(impl BlobStore + FileStore),
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            self.sync_heads_with_path_index_report(subscription, follower, store)
        })
    }

    pub fn sync_heads_with_mode(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<World>, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let outcome =
            self.sync_heads_with_mode_observed_from_heads(mode, &heads, follower, client, store)?;
        Ok(outcome.world)
    }

    pub fn sync_heads_with_mode_report(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<HeadSyncReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let world = self
            .sync_heads_with_mode_observed_from_heads(mode, &heads, follower, client, store)?
            .world;
        compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
            WorldError::DistributedValidationFailed {
                reason: "head follower did not record applied head".to_string(),
            }
        })
    }

    pub fn sync_heads_with_mode_result(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<HeadSyncResult>, WorldError> {
        let world = self.sync_heads_with_mode(mode, subscription, follower, client, store)?;
        match world {
            Some(world) => {
                let head = follower.current_head().cloned().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "head follower did not record applied head".to_string(),
                    }
                })?;
                Ok(Some(HeadSyncResult { head, world }))
            }
            None => Ok(None),
        }
    }

    pub fn follow_heads_with_mode(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            self.sync_heads_with_mode_report(mode, subscription, follower, client, store)
        })
    }

    pub fn sync_heads_with_mode_observed_report(
        &self,
        mode: HeadSyncSourceMode,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<HeadSyncModeReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let ModeSyncOutcome {
            world,
            fallback_used,
        } = self.sync_heads_with_mode_observed_from_heads(mode, &heads, follower, client, store)?;
        let report =
            compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
                WorldError::DistributedValidationFailed {
                    reason: "head follower did not record applied head".to_string(),
                }
            })?;
        Ok(HeadSyncModeReport {
            mode,
            report,
            fallback_used,
        })
    }

    fn sync_heads_with_mode_observed_from_heads(
        &self,
        mode: HeadSyncSourceMode,
        heads: &[WorldHeadAnnounce],
        follower: &mut HeadFollower,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<ModeSyncOutcome, WorldError> {
        match mode {
            HeadSyncSourceMode::NetworkOnly => Ok(ModeSyncOutcome {
                world: follower.sync_from_heads(heads, client, store)?,
                fallback_used: false,
            }),
            HeadSyncSourceMode::PathIndexOnly => Ok(ModeSyncOutcome {
                world: follower.sync_from_heads_with_path_index(heads, store)?,
                fallback_used: false,
            }),
            HeadSyncSourceMode::NetworkThenPathIndex => {
                match follower.sync_from_heads(heads, client, store) {
                    Ok(world) => Ok(ModeSyncOutcome {
                        world,
                        fallback_used: false,
                    }),
                    Err(network_error) => {
                        match follower.sync_from_heads_with_path_index(heads, store) {
                            Ok(world) => Ok(ModeSyncOutcome {
                                world,
                                fallback_used: true,
                            }),
                            Err(path_index_error) => Err(WorldError::DistributedValidationFailed {
                                reason: format!(
                                    "head sync fallback failed: mode={mode:?}, network_error={network_error:?}, path_index_error={path_index_error:?}",
                                ),
                            }),
                        }
                    }
                }
            }
        }
    }

    pub fn sync_heads_with_dht_mode(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<World>, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let outcome = self.sync_heads_with_dht_mode_observed_from_heads(
            mode, &heads, follower, dht, client, store,
        )?;
        Ok(outcome.world)
    }

    pub fn sync_heads_with_dht_mode_report(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<HeadSyncReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let world = self
            .sync_heads_with_dht_mode_observed_from_heads(
                mode, &heads, follower, dht, client, store,
            )?
            .world;
        compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
            WorldError::DistributedValidationFailed {
                reason: "head follower did not record applied head".to_string(),
            }
        })
    }

    pub fn sync_heads_with_dht_mode_result(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<Option<HeadSyncResult>, WorldError> {
        let world =
            self.sync_heads_with_dht_mode(mode, subscription, follower, dht, client, store)?;
        match world {
            Some(world) => {
                let head = follower.current_head().cloned().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "head follower did not record applied head".to_string(),
                    }
                })?;
                Ok(Some(HeadSyncResult { head, world }))
            }
            None => Ok(None),
        }
    }

    pub fn follow_heads_with_dht_mode(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
        max_rounds: usize,
    ) -> Result<HeadFollowReport, WorldError> {
        follow_head_sync(max_rounds, || {
            self.sync_heads_with_dht_mode_report(mode, subscription, follower, dht, client, store)
        })
    }

    pub fn sync_heads_with_dht_mode_observed_report(
        &self,
        mode: HeadSyncSourceModeWithDht,
        subscription: &ObserverSubscription,
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<HeadSyncModeWithDhtReport, WorldError> {
        let heads = self.drain_heads(subscription)?;
        let drained = heads.len();
        let ModeSyncOutcome {
            world,
            fallback_used,
        } = self.sync_heads_with_dht_mode_observed_from_heads(
            mode, &heads, follower, dht, client, store,
        )?;
        let report =
            compose_head_sync_report(drained, world, follower.current_head().cloned(), || {
                WorldError::DistributedValidationFailed {
                    reason: "head follower did not record applied head".to_string(),
                }
            })?;
        Ok(HeadSyncModeWithDhtReport {
            mode,
            report,
            fallback_used,
        })
    }

    fn sync_heads_with_dht_mode_observed_from_heads(
        &self,
        mode: HeadSyncSourceModeWithDht,
        heads: &[WorldHeadAnnounce],
        follower: &mut HeadFollower,
        dht: &impl DistributedDht,
        client: &DistributedClient,
        store: &(impl BlobStore + FileStore),
    ) -> Result<ModeSyncOutcome, WorldError> {
        match mode {
            HeadSyncSourceModeWithDht::NetworkWithDhtOnly => Ok(ModeSyncOutcome {
                world: follower.sync_from_heads_with_dht(heads, dht, client, store)?,
                fallback_used: false,
            }),
            HeadSyncSourceModeWithDht::PathIndexOnly => Ok(ModeSyncOutcome {
                world: follower.sync_from_heads_with_path_index(heads, store)?,
                fallback_used: false,
            }),
            HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex => {
                match follower.sync_from_heads_with_dht(heads, dht, client, store) {
                    Ok(world) => Ok(ModeSyncOutcome {
                        world,
                        fallback_used: false,
                    }),
                    Err(network_error) => {
                        match follower.sync_from_heads_with_path_index(heads, store) {
                            Ok(world) => Ok(ModeSyncOutcome {
                                world,
                                fallback_used: true,
                            }),
                            Err(path_index_error) => Err(WorldError::DistributedValidationFailed {
                                reason: format!(
                                    "head sync fallback failed: mode={mode:?}, network_error={network_error:?}, path_index_error={path_index_error:?}",
                                ),
                            }),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "self_tests"))]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::{Action, World};
    use oasis7::GeoPos;
    use oasis7_distfs::{BlobStore as _, LocalCasStore};
    use oasis7_proto::distributed::{
        FetchBlobRequest, FetchBlobResponse, GetBlockRequest, GetBlockResponse, RR_FETCH_BLOB,
        RR_GET_BLOCK,
    };

    use super::super::distributed_dht::InMemoryDht;
    use super::super::distributed_head_follow::HeadFollower;
    use super::super::distributed_net::InMemoryNetwork;
    use super::super::distributed_storage::{
        store_execution_result_with_path_index, ExecutionWriteConfig, ExecutionWriteResult,
    };
    use super::super::util::to_canonical_cbor;
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-net-{prefix}-{unique}"))
    }

    fn write_world_fixture(world_id: &str, store: &LocalCasStore) -> (ExecutionWriteResult, usize) {
        let mut world = World::new();
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();
        let write = store_execution_result_with_path_index(
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
        .expect("write");
        (write, journal.len())
    }

    fn publish_head(network: &Arc<dyn DistributedNetwork + Send + Sync>, head: &WorldHeadAnnounce) {
        let payload = to_canonical_cbor(head).expect("head cbor");
        network
            .publish(&topic_head(&head.world_id), &payload)
            .expect("publish");
    }

    fn register_block_fetch_handlers(
        network: &Arc<dyn DistributedNetwork + Send + Sync>,
        world_id: &'static str,
        store: &LocalCasStore,
        write: &ExecutionWriteResult,
    ) {
        let block = write.block.clone();
        let snapshot_ref = write.snapshot_manifest_ref.content_hash.clone();
        let journal_ref = write.journal_segments_ref.content_hash.clone();

        network
            .register_handler(
                RR_GET_BLOCK,
                Box::new(move |payload| {
                    let request: GetBlockRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    assert_eq!(request.world_id, world_id);
                    let response = GetBlockResponse {
                        block: block.clone(),
                        journal_ref: journal_ref.clone(),
                        snapshot_ref: snapshot_ref.clone(),
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register block");

        let store_clone = store.clone();
        network
            .register_handler(
                RR_FETCH_BLOB,
                Box::new(move |payload| {
                    let request: FetchBlobRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    let bytes = store_clone.get(&request.content_hash).expect("load blob");
                    let response = FetchBlobResponse {
                        blob: bytes,
                        content_hash: request.content_hash,
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register fetch");
    }

    #[test]
    fn observer_subscribes_and_drains_head_updates() {
        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");

        let head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 2,
            block_hash: "b1".to_string(),
            state_root: "s1".to_string(),
            timestamp_ms: 1,
            signature: "sig".to_string(),
        };
        let payload = to_canonical_cbor(&head).expect("cbor");
        network
            .publish(&topic_head("w1"), &payload)
            .expect("publish");

        let heads = observer.drain_heads(&subscription).expect("drain");
        assert_eq!(heads.len(), 1);
        assert_eq!(heads[0], head);
    }

    #[test]
    fn observer_sync_heads_with_mode_network_only_applies_world() {
        const WORLD_ID: &str = "w1";
        let dir = temp_dir("observer-mode-network");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture(WORLD_ID, &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        register_block_fetch_handlers(&network, WORLD_ID, &store, &write);
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe(WORLD_ID).expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_mode(
                HeadSyncSourceMode::NetworkOnly,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("sync");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sync_heads_with_mode_path_index_only_applies_world() {
        let dir = temp_dir("observer-mode-path-index");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_mode(
                HeadSyncSourceMode::PathIndexOnly,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("sync");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sync_heads_with_mode_falls_back_to_path_index() {
        let dir = temp_dir("observer-mode-fallback");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));

        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);
        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_mode(
                HeadSyncSourceMode::NetworkThenPathIndex,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("sync with fallback");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let network_only_sub = observer.subscribe("w1").expect("second subscribe");
        publish_head(&network, &write.head_announce);
        let mut network_only_follower = HeadFollower::new("w1");
        let network_only_error = observer
            .sync_heads_with_mode(
                HeadSyncSourceMode::NetworkOnly,
                &network_only_sub,
                &mut network_only_follower,
                &client,
                &store,
            )
            .expect_err("network-only should fail without handlers");
        assert!(matches!(
            network_only_error,
            WorldError::NetworkProtocolUnavailable { .. }
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sync_heads_with_dht_mode_network_with_dht_only_applies_world() {
        const WORLD_ID: &str = "w1";
        let dir = temp_dir("observer-dht-mode-network");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture(WORLD_ID, &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        register_block_fetch_handlers(&network, WORLD_ID, &store, &write);
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe(WORLD_ID).expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_dht_mode(
                HeadSyncSourceModeWithDht::NetworkWithDhtOnly,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("sync");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sync_heads_with_dht_mode_path_index_only_applies_world() {
        let dir = temp_dir("observer-dht-mode-path-index");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_dht_mode(
                HeadSyncSourceModeWithDht::PathIndexOnly,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("sync");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sync_heads_with_dht_mode_falls_back_to_path_index() {
        let dir = temp_dir("observer-dht-mode-fallback");
        let store = LocalCasStore::new(&dir);
        let (write, journal_len) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();

        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);
        let mut follower = HeadFollower::new("w1");
        let result = observer
            .sync_heads_with_dht_mode(
                HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("sync with fallback");
        let applied = result.expect("applied world");
        assert_eq!(applied.journal().len(), journal_len);
        assert_eq!(follower.current_head(), Some(&write.head_announce));

        let network_only_sub = observer.subscribe("w1").expect("second subscribe");
        publish_head(&network, &write.head_announce);
        let mut network_only_follower = HeadFollower::new("w1");
        let network_only_error = observer
            .sync_heads_with_dht_mode(
                HeadSyncSourceModeWithDht::NetworkWithDhtOnly,
                &network_only_sub,
                &mut network_only_follower,
                &dht,
                &client,
                &store,
            )
            .expect_err("network+dht-only should fail without handlers");
        assert!(matches!(
            network_only_error,
            WorldError::NetworkProtocolUnavailable { .. }
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_mode_observed_report_marks_no_fallback() {
        const WORLD_ID: &str = "w1";
        let dir = temp_dir("observer-mode-observed-no-fallback");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture(WORLD_ID, &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        register_block_fetch_handlers(&network, WORLD_ID, &store, &write);
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe(WORLD_ID).expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new(WORLD_ID);
        let observed = observer
            .sync_heads_with_mode_observed_report(
                HeadSyncSourceMode::NetworkOnly,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("observed report");
        assert_eq!(observed.mode, HeadSyncSourceMode::NetworkOnly);
        assert!(!observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_mode_observed_report_marks_fallback() {
        let dir = temp_dir("observer-mode-observed-fallback");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let observed = observer
            .sync_heads_with_mode_observed_report(
                HeadSyncSourceMode::NetworkThenPathIndex,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("observed report");
        assert_eq!(observed.mode, HeadSyncSourceMode::NetworkThenPathIndex);
        assert!(observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_dht_mode_observed_report_marks_fallback() {
        let dir = temp_dir("observer-dht-mode-observed-fallback");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let observed = observer
            .sync_heads_with_dht_mode_observed_report(
                HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("observed report");
        assert_eq!(
            observed.mode,
            HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex
        );
        assert!(observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_mode_observed_report_marks_no_fallback_for_path_index_only() {
        let dir = temp_dir("observer-mode-observed-path-index");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let observed = observer
            .sync_heads_with_mode_observed_report(
                HeadSyncSourceMode::PathIndexOnly,
                &subscription,
                &mut follower,
                &client,
                &store,
            )
            .expect("observed report");
        assert!(!observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_dht_mode_observed_report_marks_no_fallback_for_network_with_dht() {
        const WORLD_ID: &str = "w1";
        let dir = temp_dir("observer-dht-mode-observed-network");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture(WORLD_ID, &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        register_block_fetch_handlers(&network, WORLD_ID, &store, &write);
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe(WORLD_ID).expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new(WORLD_ID);
        let observed = observer
            .sync_heads_with_dht_mode_observed_report(
                HeadSyncSourceModeWithDht::NetworkWithDhtOnly,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("observed report");
        assert!(!observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_dht_mode_observed_report_marks_no_fallback_for_path_index_only() {
        let dir = temp_dir("observer-dht-mode-observed-path-index");
        let store = LocalCasStore::new(&dir);
        let (write, _) = write_world_fixture("w1", &store);

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let observer = ObserverClient::new(Arc::clone(&network));
        let client = DistributedClient::new(Arc::clone(&network));
        let dht = InMemoryDht::new();
        let subscription = observer.subscribe("w1").expect("subscribe");
        publish_head(&network, &write.head_announce);

        let mut follower = HeadFollower::new("w1");
        let observed = observer
            .sync_heads_with_dht_mode_observed_report(
                HeadSyncSourceModeWithDht::PathIndexOnly,
                &subscription,
                &mut follower,
                &dht,
                &client,
                &store,
            )
            .expect("observed report");
        assert!(!observed.fallback_used);
        assert_eq!(observed.report.drained, 1);
        assert!(observed.report.applied.is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
