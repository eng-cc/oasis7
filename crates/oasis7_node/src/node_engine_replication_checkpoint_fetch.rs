use super::*;

impl PosNodeEngine {
    pub(super) fn fetch_execution_checkpoint_bundle(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        world_id: &str,
        replication_runtime: &ReplicationRuntime,
        descriptor: &NodeExecutionCheckpointDescriptor,
        collect_fetch_observations: bool,
    ) -> Result<(NodeExecutionCheckpointBundle, Vec<serde_json::Value>), NodeError> {
        let mut fetch_observations = Vec::new();
        if collect_fetch_observations {
            fetch_observations.reserve(descriptor.blobs.len() + 1);
        }
        if let Some(observation) = self.ensure_execution_checkpoint_blob(
            endpoint,
            world_id,
            replication_runtime,
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
            collect_fetch_observations,
        )? {
            fetch_observations.push(observation);
        }
        for blob_ref in &descriptor.blobs {
            if let Some(observation) = self.ensure_execution_checkpoint_blob(
                endpoint,
                world_id,
                replication_runtime,
                blob_ref.content_hash.as_str(),
                blob_ref.size_bytes,
                collect_fetch_observations,
            )? {
                fetch_observations.push(observation);
            }
        }
        replication_runtime.pin_execution_checkpoint_descriptor(descriptor)?;
        Self::publish_execution_checkpoint_descriptor_providers(
            endpoint,
            world_id,
            replication_runtime,
            descriptor,
        )?;
        let bundle = replication_runtime
            .load_execution_checkpoint_bundle(descriptor)?
            .ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "execution checkpoint descriptor could not be materialized at height {}",
                    descriptor.height
                ),
            })?;
        Ok((bundle, fetch_observations))
    }

    fn ensure_execution_checkpoint_blob(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        world_id: &str,
        replication_runtime: &ReplicationRuntime,
        content_hash: &str,
        expected_size_bytes: u64,
        collect_fetch_observations: bool,
    ) -> Result<Option<serde_json::Value>, NodeError> {
        if let Some(bytes) = replication_runtime.load_blob_by_hash(content_hash)? {
            self.checkpoint_blob_fetch_progress.remove(content_hash);
            if bytes.len() as u64 != expected_size_bytes {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint local blob size mismatch hash={} expected={} actual={}",
                        content_hash,
                        expected_size_bytes,
                        bytes.len()
                    ),
                });
            }
            return Ok(collect_fetch_observations.then(|| {
                serde_json::json!({
                    "content_hash": content_hash,
                    "source": "local_cache",
                    "expected_size_bytes": expected_size_bytes,
                    "observed_size_bytes": bytes.len(),
                    "response_found": true,
                    "observed_content_hash": blake3_hex(bytes.as_slice()),
                })
            }));
        }
        let request = replication_runtime.build_fetch_blob_request(content_hash)?;
        let mut provider_lookup_failure = None;
        let provider_lookup = match endpoint
            .lookup_provider_ids_for_content_hash(world_id, content_hash)
        {
            Ok(provider_ids) => provider_ids,
            Err(err) => {
                provider_lookup_failure = Some(format!(
                    "provider lookup failed for execution checkpoint blob hash={content_hash}: {err:?}"
                ));
                None
            }
        };
        let response = {
            let progress = self
                .checkpoint_blob_fetch_progress
                .entry(content_hash.to_string())
                .or_default();
            request_fetch_blob_with_route_fallback_resuming_with_provenance(
                endpoint,
                world_id,
                content_hash,
                &request,
                provider_lookup.as_deref().filter(|ids| !ids.is_empty()),
                progress,
                expected_size_bytes,
            )
        };
        let (response, mut connected_candidates) = match response {
            Ok(response) => response,
            Err(err) => {
                if !crate::network_bridge::replication_network_error_is_rate_limited_protocol(
                    &err,
                    REPLICATION_FETCH_BLOB_PROTOCOL,
                ) {
                    self.checkpoint_blob_fetch_progress.remove(content_hash);
                }
                return Err(err);
            }
        };
        if !response.found {
            self.checkpoint_blob_fetch_progress.remove(content_hash);
            if let Some(provider_lookup_failure) = provider_lookup_failure {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint blob not found hash={content_hash}; {provider_lookup_failure}"
                    ),
                });
            }
            return Err(NodeError::Replication {
                reason: format!("execution checkpoint blob not found hash={content_hash}"),
            });
        }
        let blob = match response.blob {
            Some(blob) => blob,
            None => {
                self.checkpoint_blob_fetch_progress.remove(content_hash);
                return Err(NodeError::Replication {
                    reason: format!("execution checkpoint fetch missing blob hash={content_hash}"),
                });
            }
        };
        if blob.len() as u64 != expected_size_bytes {
            self.checkpoint_blob_fetch_progress.remove(content_hash);
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint fetched blob size mismatch hash={} expected={} actual={}",
                    content_hash,
                    expected_size_bytes,
                    blob.len()
                ),
            });
        }
        let actual = blake3_hex(blob.as_slice());
        if actual != content_hash {
            self.checkpoint_blob_fetch_progress.remove(content_hash);
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint fetched blob hash mismatch expected={} actual={}",
                    content_hash, actual
                ),
            });
        }
        replication_runtime.store_blob_by_hash(content_hash, blob.as_slice())?;
        self.checkpoint_blob_fetch_progress.remove(content_hash);
        if !collect_fetch_observations {
            return Ok(None);
        }
        let mut provider_candidates = provider_lookup.unwrap_or_default();
        provider_candidates.sort();
        provider_candidates.dedup();
        let mut connected_peer_ids = endpoint.connected_peer_ids();
        connected_peer_ids.sort();
        connected_peer_ids.dedup();
        connected_candidates.sort();
        connected_candidates.dedup();
        // A generic request has no single provider route to return, but it was
        // still served through the endpoint's live connected set. Preserve that
        // concrete set for the clean-room receipt instead of recording an
        // empty provenance list.
        if connected_candidates.is_empty() {
            connected_candidates.clone_from(&connected_peer_ids);
        }
        Ok(Some(serde_json::json!({
            "content_hash": content_hash,
            "source": "network_fetch",
            "provider_candidates": provider_candidates,
            "connected_peer_ids": connected_peer_ids,
            "connected_candidate_ids": connected_candidates,
            "signed_request": request.requester_signature_hex.is_some(),
            "response_found": true,
            "expected_size_bytes": expected_size_bytes,
            "observed_size_bytes": blob.len(),
            "observed_content_hash": actual,
        })))
    }
}
