use super::*;

impl ReplicationRuntime {
    pub(crate) fn checkpoint_probe_nonce_is_valid(probe_nonce: &str) -> bool {
        probe_nonce.len() >= 32
            && probe_nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    }

    /// Persisted only after the engine has fetched, hash/size verified, and
    /// installed a checkpoint closure. Probe tooling reads this from its own
    /// fresh replication root rather than accepting operator JSON.
    pub(crate) fn persist_checkpoint_verification_receipt(
        &self,
        world_id: &str,
        probe_nonce: Option<&str>,
        descriptor: &NodeExecutionCheckpointDescriptor,
        bundle: Option<&NodeExecutionCheckpointBundle>,
        fetch_observations: &[serde_json::Value],
    ) -> Result<(), NodeError> {
        let Some(probe_nonce) = probe_nonce else {
            return Ok(());
        };
        let Some(bundle) = bundle else {
            return Err(NodeError::Replication {
                reason: "checkpoint verification receipt probe lost its installed closure"
                    .to_string(),
            });
        };
        if !Self::checkpoint_probe_nonce_is_valid(probe_nonce) {
            return Err(NodeError::Replication {
                reason: "checkpoint verification receipt probe nonce must be at least 32 URL-safe characters".to_string(),
            });
        }
        if world_id.trim().is_empty() {
            return Err(NodeError::Replication {
                reason: "checkpoint verification receipt requires a non-empty world id".to_string(),
            });
        }
        if bundle.height != descriptor.height
            || bundle.execution_block_hash != descriptor.execution_block_hash
            || bundle.execution_state_root != descriptor.execution_state_root
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint verification receipt binding mismatch at height {}",
                    descriptor.height
                ),
            });
        }
        if bundle.blobs.len() != descriptor.blobs.len() {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint verification receipt closure count mismatch expected={} actual={}",
                    descriptor.blobs.len(),
                    bundle.blobs.len()
                ),
            });
        }
        if fetch_observations.len() != descriptor.blobs.len() + 1 {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint verification receipt observation count mismatch expected={} actual={}",
                    descriptor.blobs.len() + 1,
                    fetch_observations.len()
                ),
            });
        }
        let mut expected_observations = Vec::with_capacity(descriptor.blobs.len() + 1);
        expected_observations.push((
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
        ));
        expected_observations.extend(
            descriptor
                .blobs
                .iter()
                .map(|blob| (blob.content_hash.as_str(), blob.size_bytes)),
        );
        for ((expected_hash, expected_size), observation) in
            expected_observations.iter().zip(fetch_observations.iter())
        {
            let connected_candidates = observation
                .get("connected_candidate_ids")
                .and_then(serde_json::Value::as_array)
                .filter(|candidates| !candidates.is_empty());
            let observed_size = observation
                .get("observed_size_bytes")
                .and_then(serde_json::Value::as_u64);
            if observation
                .get("source")
                .and_then(serde_json::Value::as_str)
                != Some("network_fetch")
                || observation
                    .get("content_hash")
                    .and_then(serde_json::Value::as_str)
                    != Some(*expected_hash)
                || observation
                    .get("observed_content_hash")
                    .and_then(serde_json::Value::as_str)
                    != Some(*expected_hash)
                || observed_size != Some(*expected_size)
                || observation
                    .get("response_found")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
                || observation
                    .get("signed_request")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
                || connected_candidates.is_none()
            {
                return Err(NodeError::Replication {
                    reason: format!(
                        "checkpoint verification receipt requires a signed network fetch through a connected candidate for hash={expected_hash}"
                    ),
                });
            }
        }
        let observed_manifest_hash = blake3_hex(bundle.manifest_json.as_slice());
        if observed_manifest_hash != descriptor.manifest_ref
            || bundle.manifest_json.len() as u64 != descriptor.manifest_size_bytes
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint verification receipt manifest binding mismatch expected_hash={} observed_hash={} expected_size={} observed_size={}",
                    descriptor.manifest_ref,
                    observed_manifest_hash,
                    descriptor.manifest_size_bytes,
                    bundle.manifest_json.len()
                ),
            });
        }
        let mut objects = Vec::with_capacity(descriptor.blobs.len() + 1);
        objects.push(serde_json::json!({
            "expected_content_hash": descriptor.manifest_ref.clone(),
            "observed_content_hash": observed_manifest_hash,
            "expected_size_bytes": descriptor.manifest_size_bytes,
            "observed_size_bytes": bundle.manifest_json.len(),
        }));
        for (expected, actual) in descriptor.blobs.iter().zip(bundle.blobs.iter()) {
            let observed_hash = blake3_hex(actual.bytes.as_slice());
            if actual.content_hash != expected.content_hash
                || observed_hash != expected.content_hash
                || actual.bytes.len() as u64 != expected.size_bytes
            {
                return Err(NodeError::Replication {
                    reason: format!(
                        "checkpoint verification receipt blob binding mismatch expected_hash={} actual_declared_hash={} observed_hash={} expected_size={} observed_size={}",
                        expected.content_hash,
                        actual.content_hash,
                        observed_hash,
                        expected.size_bytes,
                        actual.bytes.len()
                    ),
                });
            }
            objects.push(serde_json::json!({
                "expected_content_hash": expected.content_hash.clone(),
                "observed_content_hash": observed_hash,
                "expected_size_bytes": expected.size_bytes,
                "observed_size_bytes": actual.bytes.len(),
            }));
        }
        let value = serde_json::json!({
            "schema_version": "oasis7.checkpoint_closure_verification_receipt.v1",
            "world_id": world_id,
            "probe_nonce": probe_nonce,
            "height": descriptor.height,
            "execution_block_hash": descriptor.execution_block_hash.clone(),
            "execution_state_root": descriptor.execution_state_root.clone(),
            "manifest_hash": descriptor.manifest_ref.clone(),
            "objects": objects,
            "fetch_observations": fetch_observations,
        });
        let receipt_dir = self.config.root_dir.join("checkpoint-verification");
        std::fs::create_dir_all(&receipt_dir).map_err(|err| NodeError::Replication {
            reason: format!("create checkpoint verification receipt directory: {err}"),
        })?;
        let final_path = receipt_dir.join(format!("{}.json", descriptor.height));
        let temp_path =
            receipt_dir.join(format!("{}.{}.tmp", descriptor.height, std::process::id()));
        let bytes = serde_json::to_vec(&value).map_err(|err| NodeError::Replication {
            reason: format!("serialize checkpoint verification receipt: {err}"),
        })?;
        let write_result = (|| -> Result<(), std::io::Error> {
            let mut temp = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            temp.write_all(bytes.as_slice())?;
            temp.sync_all()
        })();
        if let Err(err) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(NodeError::Replication {
                reason: format!("write checkpoint verification receipt: {err}"),
            });
        }
        match std::fs::hard_link(&temp_path, &final_path) {
            Ok(()) => std::fs::remove_file(&temp_path).map_err(|err| NodeError::Replication {
                reason: format!("finalize checkpoint verification receipt: {err}"),
            }),
            Err(err) => {
                let _ = std::fs::remove_file(&temp_path);
                Err(NodeError::Replication {
                    reason: format!(
                        "publish checkpoint verification receipt without overwrite: {err}"
                    ),
                })
            }
        }
    }
}
