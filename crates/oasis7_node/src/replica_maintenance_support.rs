use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn maybe_run_runtime_replica_maintenance_poll(
    config: Option<NodeReplicaMaintenanceConfig>,
    node_id: &str,
    world_id: &str,
    now_ms: i64,
    last_polled_at_ms: Option<i64>,
    dht: Option<&(dyn proto_dht::DistributedDht<ProtoWorldError> + Send + Sync)>,
    replication_network: Option<&ReplicationNetworkEndpoint>,
    replication: Option<&ReplicationRuntime>,
) -> Result<Option<i64>, NodeError> {
    let Some(config) = config else {
        return Ok(last_polled_at_ms);
    };
    if !config.enabled {
        return Ok(last_polled_at_ms);
    }

    let Some(dht) = dht else {
        return Ok(last_polled_at_ms);
    };
    let Some(replication_network) = replication_network else {
        return Ok(last_polled_at_ms);
    };
    let Some(replication) = replication else {
        return Ok(last_polled_at_ms);
    };

    let content_hashes = replication
        .recent_replicated_content_hashes(world_id, config.max_content_hash_samples_per_round)?;
    if content_hashes.is_empty() {
        return Ok(last_polled_at_ms);
    }

    let mut polling_state = ReplicaMaintenancePollingState { last_polled_at_ms };
    let polling_policy = ReplicaMaintenancePollingPolicy {
        poll_interval_ms: config.poll_interval_ms,
    };
    let maintenance_policy = ReplicaMaintenancePolicy {
        target_replicas_per_blob: config.target_replicas_per_blob,
        max_repairs_per_round: config.max_repairs_per_round,
        max_rebalances_per_round: config.max_rebalances_per_round,
        rebalance_source_load_min_per_mille: config.rebalance_source_load_min_per_mille,
        rebalance_target_load_max_per_mille: config.rebalance_target_load_max_per_mille,
    };
    let dht_adapter = RuntimeReplicaMaintenanceDht { inner: dht };
    let executor = RuntimeReplicaMaintenanceTransferExecutor {
        node_id,
        replication,
        replication_network,
    };
    let round = run_replica_maintenance_poll(
        &dht_adapter,
        &executor,
        world_id,
        &content_hashes,
        maintenance_policy,
        polling_policy,
        &mut polling_state,
        now_ms,
    )
    .map_err(node_replica_maintenance_error)?;

    Ok(round
        .map(|summary| summary.polled_at_ms)
        .or(polling_state.last_polled_at_ms))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn maybe_run_runtime_replica_maintenance_poll(
    _config: Option<NodeReplicaMaintenanceConfig>,
    _node_id: &str,
    _world_id: &str,
    _now_ms: i64,
    last_polled_at_ms: Option<i64>,
    _dht: Option<&(dyn proto_dht::DistributedDht<ProtoWorldError> + Send + Sync)>,
    _replication_network: Option<&ReplicationNetworkEndpoint>,
    _replication: Option<&ReplicationRuntime>,
) -> Result<Option<i64>, NodeError> {
    Ok(last_polled_at_ms)
}

#[cfg(not(target_arch = "wasm32"))]
fn node_replica_maintenance_error(err: ProtoWorldError) -> NodeError {
    NodeError::Replication {
        reason: format!("replica maintenance error: {err:?}"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn node_error_to_world_validation(err: NodeError) -> ProtoWorldError {
    ProtoWorldError::DistributedValidationFailed {
        reason: err.to_string(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct RuntimeReplicaMaintenanceDht<'a> {
    inner: &'a (dyn proto_dht::DistributedDht<ProtoWorldError> + Send + Sync),
}

#[cfg(not(target_arch = "wasm32"))]
impl proto_dht::DistributedDht<ProtoWorldError> for RuntimeReplicaMaintenanceDht<'_> {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        provider_id: &str,
    ) -> Result<(), ProtoWorldError> {
        self.inner
            .publish_provider(world_id, content_hash, provider_id)
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<proto_dht::ProviderRecord>, ProtoWorldError> {
        self.inner.get_providers(world_id, content_hash)
    }

    fn put_world_head(
        &self,
        world_id: &str,
        head: &oasis7_proto::distributed::WorldHeadAnnounce,
    ) -> Result<(), ProtoWorldError> {
        self.inner.put_world_head(world_id, head)
    }

    fn get_world_head(
        &self,
        world_id: &str,
    ) -> Result<Option<oasis7_proto::distributed::WorldHeadAnnounce>, ProtoWorldError> {
        self.inner.get_world_head(world_id)
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &proto_dht::MembershipDirectorySnapshot,
    ) -> Result<(), ProtoWorldError> {
        self.inner.put_membership_directory(world_id, snapshot)
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<proto_dht::MembershipDirectorySnapshot>, ProtoWorldError> {
        self.inner.get_membership_directory(world_id)
    }

    fn put_peer_record(
        &self,
        world_id: &str,
        record: &proto_dht::SignedPeerRecord,
    ) -> Result<(), ProtoWorldError> {
        self.inner.put_peer_record(world_id, record)
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<proto_dht::SignedPeerRecord>, ProtoWorldError> {
        self.inner.get_peer_record(world_id, peer_id)
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct RuntimeReplicaMaintenanceTransferExecutor<'a> {
    node_id: &'a str,
    replication: &'a ReplicationRuntime,
    replication_network: &'a ReplicationNetworkEndpoint,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReplicaTransferExecutor for RuntimeReplicaMaintenanceTransferExecutor<'_> {
    fn execute_transfer(
        &self,
        _world_id: &str,
        task: &ReplicaTransferTask,
    ) -> Result<(), ProtoWorldError> {
        if task.target_provider_id != self.node_id {
            return Err(ProtoWorldError::DistributedValidationFailed {
                reason: format!(
                    "replica maintenance task target mismatch expected={} actual={}",
                    self.node_id, task.target_provider_id
                ),
            });
        }

        let request = self
            .replication
            .build_fetch_blob_request(task.content_hash.as_str())
            .map_err(node_error_to_world_validation)?;
        let providers = vec![task.source_provider_id.clone()];
        let response = self
            .replication_network
            .request_json_with_providers::<FetchBlobRequest, FetchBlobResponse>(
                REPLICATION_FETCH_BLOB_PROTOCOL,
                &request,
                providers.as_slice(),
            )
            .map_err(node_error_to_world_validation)?;
        if !response.found {
            return Err(ProtoWorldError::BlobNotFound {
                content_hash: task.content_hash.clone(),
            });
        }
        let blob = response
            .blob
            .ok_or_else(|| ProtoWorldError::DistributedValidationFailed {
                reason: format!(
                    "replica maintenance transfer missing blob payload for hash={}",
                    task.content_hash
                ),
            })?;
        let actual_hash = blake3_hex(blob.as_slice());
        if actual_hash != task.content_hash {
            return Err(ProtoWorldError::BlobHashMismatch {
                expected: task.content_hash.clone(),
                actual: actual_hash,
            });
        }
        self.replication
            .store_blob_by_hash(task.content_hash.as_str(), blob.as_slice())
            .map_err(node_error_to_world_validation)
    }
}
