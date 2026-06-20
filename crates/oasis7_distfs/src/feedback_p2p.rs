use std::sync::Arc;

use oasis7_proto::distributed::gossipsub_topic;
use oasis7_proto::distributed_net::{DistributedNetwork, NetworkSubscription};
use oasis7_proto::world_error::WorldError;
use serde::{Deserialize, Serialize};

use super::{
    FeedbackActionKind, FeedbackEventRecord, FeedbackMutationReceipt, FeedbackRootRecord,
    FeedbackStore, blake3_hex,
};

pub const FEEDBACK_ANNOUNCE_TOPIC_SUFFIX: &str = "feedback.announce";
const FEEDBACK_ANNOUNCE_VERSION: u8 = 1;

pub fn feedback_announce_topic(world_id: &str) -> String {
    gossipsub_topic(world_id, FEEDBACK_ANNOUNCE_TOPIC_SUFFIX)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackAnnounce {
    pub version: u8,
    pub world_id: String,
    pub feedback_id: String,
    pub action: FeedbackActionKind,
    pub event_id: String,
    pub actor_public_key_hex: String,
    pub blob_ref: super::FeedbackBlobRef,
    pub emitted_at_ms: i64,
}

pub struct FeedbackAnnounceBridge {
    world_id: String,
    topic: String,
    network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>,
    subscription: NetworkSubscription,
}

impl FeedbackAnnounceBridge {
    pub fn new(
        world_id: &str,
        network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>,
    ) -> Result<Self, WorldError> {
        let topic = feedback_announce_topic(world_id);
        let subscription = network.subscribe(topic.as_str())?;
        Ok(Self {
            world_id: world_id.to_string(),
            topic,
            network,
            subscription,
        })
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn publish(&self, announce: &FeedbackAnnounce) -> Result<(), WorldError> {
        if announce.world_id != self.world_id {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "feedback announce world mismatch: expected={} actual={}",
                    self.world_id, announce.world_id
                ),
            });
        }
        let payload = encode_feedback_announce(announce)?;
        self.network
            .publish(self.topic.as_str(), payload.as_slice())
    }

    pub fn drain(&self) -> Vec<Result<FeedbackAnnounce, WorldError>> {
        self.subscription
            .drain()
            .into_iter()
            .map(|payload| decode_feedback_announce(payload.as_slice()))
            .collect()
    }
}

pub fn encode_feedback_announce(announce: &FeedbackAnnounce) -> Result<Vec<u8>, WorldError> {
    serde_json::to_vec(announce).map_err(WorldError::from)
}

pub fn decode_feedback_announce(payload: &[u8]) -> Result<FeedbackAnnounce, WorldError> {
    Ok(serde_json::from_slice(payload)?)
}

pub fn build_feedback_announce_from_receipt(
    store: &FeedbackStore,
    world_id: &str,
    receipt: &FeedbackMutationReceipt,
    emitted_at_ms: i64,
) -> Result<FeedbackAnnounce, WorldError> {
    let Some(blob_ref) = store.blob_ref_for_receipt(receipt)? else {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "missing blob ref for feedback receipt feedback_id={} action={}",
                receipt.feedback_id,
                feedback_action_label(receipt.action)
            ),
        });
    };
    Ok(FeedbackAnnounce {
        version: FEEDBACK_ANNOUNCE_VERSION,
        world_id: world_id.to_string(),
        feedback_id: receipt.feedback_id.clone(),
        action: receipt.action,
        event_id: receipt.event_id.clone(),
        actor_public_key_hex: blob_ref.actor_public_key_hex.clone(),
        blob_ref,
        emitted_at_ms,
    })
}

pub fn ingest_feedback_announce_with_fetcher(
    store: &FeedbackStore,
    announce: &FeedbackAnnounce,
    fetch_blob_by_hash: impl Fn(&str) -> Result<Vec<u8>, WorldError>,
) -> Result<FeedbackMutationReceipt, WorldError> {
    validate_feedback_announce(announce)?;
    let blob_bytes = fetch_blob_by_hash(announce.blob_ref.content_hash.as_str())?;
    let actual_hash = blake3_hex(blob_bytes.as_slice());
    if actual_hash != announce.blob_ref.content_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "feedback announce blob hash mismatch expected={} actual={}",
                announce.blob_ref.content_hash, actual_hash
            ),
        });
    }
    if blob_bytes.len() as u64 != announce.blob_ref.size_bytes {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "feedback announce blob size mismatch expected={} actual={}",
                announce.blob_ref.size_bytes,
                blob_bytes.len()
            ),
        });
    }
    match announce.action {
        FeedbackActionKind::Create => {
            let root_record: FeedbackRootRecord = serde_json::from_slice(&blob_bytes)?;
            if root_record.feedback_id != announce.feedback_id {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "feedback announce root feedback_id mismatch expected={} actual={}",
                        announce.feedback_id, root_record.feedback_id
                    ),
                });
            }
            if root_record.author_public_key_hex != announce.actor_public_key_hex {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "feedback announce root actor mismatch expected={} actual={}",
                        announce.actor_public_key_hex, root_record.author_public_key_hex
                    ),
                });
            }
            store.ingest_replicated_root_record(root_record)
        }
        FeedbackActionKind::Append | FeedbackActionKind::Tombstone => {
            let event_record: FeedbackEventRecord = serde_json::from_slice(&blob_bytes)?;
            if event_record.feedback_id != announce.feedback_id {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "feedback announce event feedback_id mismatch expected={} actual={}",
                        announce.feedback_id, event_record.feedback_id
                    ),
                });
            }
            if event_record.event_id != announce.event_id {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "feedback announce event_id mismatch expected={} actual={}",
                        announce.event_id, event_record.event_id
                    ),
                });
            }
            if event_record.actor_public_key_hex != announce.actor_public_key_hex {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "feedback announce event actor mismatch expected={} actual={}",
                        announce.actor_public_key_hex, event_record.actor_public_key_hex
                    ),
                });
            }
            store.ingest_replicated_event_record(event_record)
        }
    }
}

fn validate_feedback_announce(announce: &FeedbackAnnounce) -> Result<(), WorldError> {
    if announce.version != FEEDBACK_ANNOUNCE_VERSION {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "unsupported feedback announce version: {}",
                announce.version
            ),
        });
    }
    if announce.world_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "feedback announce world_id cannot be empty".to_string(),
        });
    }
    if announce.feedback_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "feedback announce feedback_id cannot be empty".to_string(),
        });
    }
    if announce.event_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "feedback announce event_id cannot be empty".to_string(),
        });
    }
    if announce.blob_ref.path.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "feedback announce blob path cannot be empty".to_string(),
        });
    }
    super::validate_hash(announce.blob_ref.content_hash.as_str())?;
    Ok(())
}

fn feedback_action_label(action: FeedbackActionKind) -> &'static str {
    match action {
        FeedbackActionKind::Create => "create",
        FeedbackActionKind::Append => "append",
        FeedbackActionKind::Tombstone => "tombstone",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::{
        BlobStore, FeedbackAppendRequest, FeedbackCreateRequest, FeedbackStoreConfig,
        FeedbackTombstoneRequest, LocalCasStore, sign_feedback_append_request,
        sign_feedback_create_request, sign_feedback_tombstone_request,
    };

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-feedback-p2p-{prefix}-{unique}"))
    }

    fn now_plus(delta_ms: i64) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_millis() as i64;
        now.saturating_add(delta_ms)
    }

    #[test]
    fn feedback_announce_build_encode_decode_ingest_roundtrip() {
        let source_dir = temp_dir("source");
        let target_dir = temp_dir("target");
        let source_cas = LocalCasStore::new(&source_dir);
        let target_cas = LocalCasStore::new(&target_dir);
        let source = FeedbackStore::new(source_cas.clone(), FeedbackStoreConfig::default());
        let target = FeedbackStore::new(target_cas, FeedbackStoreConfig::default());
        let signing_key_hex =
            "1111111111111111111111111111111111111111111111111111111111111111".to_string();
        let actor_public_key_hex =
            crate::public_key_hex_from_signing_key_hex(&signing_key_hex).expect("derive pubkey");
        let base_ts = now_plus(0);

        let mut create_request = FeedbackCreateRequest {
            feedback_id: "fb-p2p-1".to_string(),
            author_public_key_hex: actor_public_key_hex.clone(),
            submit_ip: "127.0.0.10".to_string(),
            category: "bug".to_string(),
            platform: "web".to_string(),
            game_version: "0.4.0".to_string(),
            content: "p2p root".to_string(),
            attachments: vec![],
            nonce: "n-p2p-root".to_string(),
            timestamp_ms: base_ts,
            expires_at_ms: base_ts + 120_000,
            signature_hex: String::new(),
        };
        create_request.signature_hex =
            sign_feedback_create_request(&create_request, signing_key_hex.as_str())
                .expect("sign create");
        let create_receipt = source.submit_feedback(create_request).expect("submit");
        let create_announce =
            build_feedback_announce_from_receipt(&source, "w1", &create_receipt, base_ts + 1)
                .expect("build announce");
        let encoded = encode_feedback_announce(&create_announce).expect("encode");
        let decoded = decode_feedback_announce(encoded.as_slice()).expect("decode");
        let create_ingest =
            ingest_feedback_announce_with_fetcher(&target, &decoded, |hash| source_cas.get(hash))
                .expect("ingest create");
        assert_eq!(create_ingest.action, FeedbackActionKind::Create);

        let mut append_request = FeedbackAppendRequest {
            feedback_id: "fb-p2p-1".to_string(),
            actor_public_key_hex,
            submit_ip: "127.0.0.10".to_string(),
            content: "p2p append".to_string(),
            nonce: "n-p2p-append".to_string(),
            timestamp_ms: base_ts + 2,
            expires_at_ms: base_ts + 120_000,
            signature_hex: String::new(),
        };
        append_request.signature_hex =
            sign_feedback_append_request(&append_request, signing_key_hex.as_str())
                .expect("sign append");
        let append_receipt = source.append_feedback(append_request).expect("append");
        let append_announce =
            build_feedback_announce_from_receipt(&source, "w1", &append_receipt, base_ts + 3)
                .expect("build announce");
        ingest_feedback_announce_with_fetcher(&target, &append_announce, |hash| {
            source_cas.get(hash)
        })
        .expect("ingest append");
        ingest_feedback_announce_with_fetcher(&target, &append_announce, |hash| {
            source_cas.get(hash)
        })
        .expect("ingest append idempotent");

        let mut tombstone_request = FeedbackTombstoneRequest {
            feedback_id: "fb-p2p-1".to_string(),
            actor_public_key_hex: crate::public_key_hex_from_signing_key_hex(&signing_key_hex)
                .expect("pubkey"),
            submit_ip: "127.0.0.10".to_string(),
            reason: "done".to_string(),
            nonce: "n-p2p-tombstone".to_string(),
            timestamp_ms: base_ts + 4,
            expires_at_ms: base_ts + 120_000,
            signature_hex: String::new(),
        };
        tombstone_request.signature_hex =
            sign_feedback_tombstone_request(&tombstone_request, signing_key_hex.as_str())
                .expect("sign tombstone");
        let tombstone_receipt = source
            .tombstone_feedback(tombstone_request)
            .expect("tombstone");
        let tombstone_announce =
            build_feedback_announce_from_receipt(&source, "w1", &tombstone_receipt, base_ts + 5)
                .expect("build tombstone announce");
        ingest_feedback_announce_with_fetcher(&target, &tombstone_announce, |hash| {
            source_cas.get(hash)
        })
        .expect("ingest tombstone");

        let view = target
            .read_feedback_public("fb-p2p-1")
            .expect("read target")
            .expect("target exists");
        assert_eq!(view.append_events.len(), 1);
        assert!(view.tombstoned);
        assert_eq!(view.tombstone_reason, Some("done".to_string()));

        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(target_dir);
    }

    #[test]
    fn feedback_announce_ingest_rejects_blob_hash_mismatch() {
        let source_dir = temp_dir("source-mismatch");
        let target_dir = temp_dir("target-mismatch");
        let source_cas = LocalCasStore::new(&source_dir);
        let source = FeedbackStore::new(source_cas.clone(), FeedbackStoreConfig::default());
        let target = FeedbackStore::new(
            LocalCasStore::new(&target_dir),
            FeedbackStoreConfig::default(),
        );
        let signing_key_hex =
            "2222222222222222222222222222222222222222222222222222222222222222".to_string();
        let actor_public_key_hex =
            crate::public_key_hex_from_signing_key_hex(&signing_key_hex).expect("derive pubkey");
        let base_ts = now_plus(0);
        let mut create_request = FeedbackCreateRequest {
            feedback_id: "fb-p2p-2".to_string(),
            author_public_key_hex: actor_public_key_hex,
            submit_ip: "127.0.0.11".to_string(),
            category: "bug".to_string(),
            platform: "web".to_string(),
            game_version: "0.4.0".to_string(),
            content: "payload".to_string(),
            attachments: vec![],
            nonce: "n-mm".to_string(),
            timestamp_ms: base_ts,
            expires_at_ms: base_ts + 120_000,
            signature_hex: String::new(),
        };
        create_request.signature_hex =
            sign_feedback_create_request(&create_request, signing_key_hex.as_str())
                .expect("sign create");
        let receipt = source.submit_feedback(create_request).expect("submit");
        let announce = build_feedback_announce_from_receipt(&source, "w1", &receipt, base_ts + 1)
            .expect("announce");
        let result = ingest_feedback_announce_with_fetcher(&target, &announce, |_hash| {
            Ok(b"tampered".to_vec())
        });
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(target_dir);
    }
}
