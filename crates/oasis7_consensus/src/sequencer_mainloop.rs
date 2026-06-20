use std::collections::HashSet;

use serde::Serialize;

use super::distributed::ActionEnvelope;
use super::distributed::WorldHeadAnnounce;
use super::distributed_dht::DistributedDht;
use super::ed25519_signer_policy::{
    normalize_ed25519_public_key_allowlist, normalize_ed25519_public_key_hex,
};
use super::error::WorldError;
use super::lease::{LeaseDecision, LeaseManager};
use super::mempool::{ActionBatchRules, ActionMempool, ActionMempoolConfig};
use super::pos::{
    PosConsensus, PosConsensusConfig, PosConsensusDecision, PosConsensusStatus,
    attest_world_head_with_pos, propose_world_head_with_pos,
};
use super::signature::{ED25519_SIGNATURE_V1_PREFIX, Ed25519SignatureSigner, HmacSha256Signer};

#[derive(Debug, Clone)]
pub struct SequencerMainloopConfig {
    pub world_id: String,
    pub node_id: String,
    pub lease_ttl_ms: i64,
    pub batch_rules: ActionBatchRules,
    pub mempool: ActionMempoolConfig,
    pub auto_attest_all_validators: bool,
    pub require_action_signature: bool,
    pub sign_head: bool,
    pub hmac_signer: Option<HmacSha256Signer>,
    pub ed25519_signer: Option<Ed25519SignatureSigner>,
    pub accepted_action_signer_public_keys: Vec<String>,
    pub initial_prev_block_hash: String,
}

impl Default for SequencerMainloopConfig {
    fn default() -> Self {
        Self {
            world_id: "w1".to_string(),
            node_id: "sequencer-1".to_string(),
            lease_ttl_ms: 5_000,
            batch_rules: ActionBatchRules::default(),
            mempool: ActionMempoolConfig::default(),
            auto_attest_all_validators: true,
            require_action_signature: false,
            sign_head: false,
            hmac_signer: None,
            ed25519_signer: None,
            accepted_action_signer_public_keys: Vec::new(),
            initial_prev_block_hash: "genesis".to_string(),
        }
    }
}

impl SequencerMainloopConfig {
    fn validate(&self) -> Result<(), WorldError> {
        if self.world_id.trim().is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sequencer world_id cannot be empty".to_string(),
            });
        }
        if self.node_id.trim().is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sequencer node_id cannot be empty".to_string(),
            });
        }
        if self.lease_ttl_ms <= 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sequencer lease_ttl_ms must be positive".to_string(),
            });
        }
        if self.batch_rules.max_actions == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sequencer batch_rules.max_actions must be positive".to_string(),
            });
        }
        if self.batch_rules.max_payload_bytes == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sequencer batch_rules.max_payload_bytes must be positive".to_string(),
            });
        }
        let accepted_action_signer_public_keys = normalize_ed25519_public_key_allowlist(
            &self.accepted_action_signer_public_keys,
            "accepted_action_signer_public_keys",
            "accepted_action_signer_public_keys",
        )?;
        if self.require_action_signature
            && self.hmac_signer.is_none()
            && accepted_action_signer_public_keys.is_none()
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: "require_action_signature requires hmac_signer or accepted_action_signer_public_keys"
                    .to_string(),
            });
        }
        if self.sign_head && self.hmac_signer.is_none() && self.ed25519_signer.is_none() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "sign_head requires hmac_signer or ed25519_signer".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerTickState {
    LeaseBlocked,
    Idle,
    Pending,
    Committed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencerTickReport {
    pub world_id: String,
    pub node_id: String,
    pub state: SequencerTickState,
    pub lease_granted: bool,
    pub height: Option<u64>,
    pub slot: Option<u64>,
    pub batch_id: Option<String>,
    pub block_hash: Option<String>,
    pub status: Option<PosConsensusStatus>,
}

pub struct SequencerMainloop {
    config: SequencerMainloopConfig,
    accepted_action_signer_public_keys: Option<HashSet<String>>,
    mempool: ActionMempool,
    consensus: PosConsensus,
    lease: LeaseManager,
    next_height: u64,
    next_slot: u64,
    prev_block_hash: String,
}

impl SequencerMainloop {
    pub fn new(
        config: SequencerMainloopConfig,
        pos_config: PosConsensusConfig,
    ) -> Result<Self, WorldError> {
        let accepted_action_signer_public_keys = normalize_ed25519_public_key_allowlist(
            &config.accepted_action_signer_public_keys,
            "accepted_action_signer_public_keys",
            "accepted_action_signer_public_keys",
        )?;
        config.validate()?;
        let consensus = PosConsensus::new(pos_config)?;
        Ok(Self {
            mempool: ActionMempool::new(config.mempool.clone()),
            lease: LeaseManager::new(),
            prev_block_hash: config.initial_prev_block_hash.clone(),
            accepted_action_signer_public_keys,
            config,
            consensus,
            next_height: 1,
            next_slot: 0,
        })
    }

    pub fn config(&self) -> &SequencerMainloopConfig {
        &self.config
    }

    pub fn next_height(&self) -> u64 {
        self.next_height
    }

    pub fn next_slot(&self) -> u64 {
        self.next_slot
    }

    pub fn pending_actions(&self) -> usize {
        self.mempool.len()
    }

    pub fn submit_action(&mut self, action: ActionEnvelope) -> bool {
        if action.world_id != self.config.world_id {
            return false;
        }
        if !self.verify_action_signature(&action) {
            return false;
        }
        self.mempool.add_action(action)
    }

    pub fn tick(
        &mut self,
        dht: &impl DistributedDht,
        now_ms: i64,
    ) -> Result<SequencerTickReport, WorldError> {
        let lease = self.ensure_lease(now_ms);
        if !lease.granted {
            return Ok(SequencerTickReport {
                world_id: self.config.world_id.clone(),
                node_id: self.config.node_id.clone(),
                state: SequencerTickState::LeaseBlocked,
                lease_granted: false,
                height: None,
                slot: None,
                batch_id: None,
                block_hash: None,
                status: None,
            });
        }

        if let Some(report) = self.drive_pending_head(dht, now_ms)? {
            return Ok(report);
        }

        let slot = self.next_slot;
        let height = self.next_height;
        let next_slot = checked_sequencer_counter_increment(self.next_slot, "next_slot")?;
        let Some(batch) = self.mempool.take_batch_with_rules(
            &self.config.world_id,
            &self.config.node_id,
            self.config.batch_rules,
            now_ms,
        )?
        else {
            return Ok(SequencerTickReport {
                world_id: self.config.world_id.clone(),
                node_id: self.config.node_id.clone(),
                state: SequencerTickState::Idle,
                lease_granted: true,
                height: None,
                slot: None,
                batch_id: None,
                block_hash: None,
                status: None,
            });
        };

        let state_root = state_root_for_actions(&batch.actions)?;
        let block_hash = block_hash_for_batch(
            &self.config.world_id,
            height,
            slot,
            &self.prev_block_hash,
            &batch.batch_id,
            &state_root,
        )?;

        let mut head = WorldHeadAnnounce {
            world_id: self.config.world_id.clone(),
            height,
            block_hash: block_hash.clone(),
            state_root,
            timestamp_ms: now_ms,
            signature: String::new(),
        };
        self.sign_head_if_needed(&mut head)?;

        let mut decision = propose_world_head_with_pos(
            dht,
            &mut self.consensus,
            &head,
            &self.config.node_id,
            slot,
            now_ms,
        )?;

        decision = self.drive_attestations_for_head(dht, &head, decision, now_ms)?;
        self.apply_finalized_status(&head.block_hash, decision.status)?;
        self.next_slot = next_slot;

        Ok(SequencerTickReport {
            world_id: self.config.world_id.clone(),
            node_id: self.config.node_id.clone(),
            state: tick_state_from_status(decision.status),
            lease_granted: true,
            height: Some(decision.height),
            slot: Some(decision.slot),
            batch_id: Some(batch.batch_id),
            block_hash: Some(head.block_hash),
            status: Some(decision.status),
        })
    }

    fn verify_action_signature(&self, action: &ActionEnvelope) -> bool {
        if action.signature.is_empty() {
            return !self.config.require_action_signature;
        }
        if action.signature.starts_with(ED25519_SIGNATURE_V1_PREFIX) {
            let Ok(signer_public_key) = Ed25519SignatureSigner::verify_action(action) else {
                return false;
            };
            let Ok(signer_public_key) = normalize_ed25519_public_key_hex(
                signer_public_key.as_str(),
                "action signature signer public key",
            ) else {
                return false;
            };
            let Some(accepted_action_signer_public_keys) =
                self.accepted_action_signer_public_keys.as_ref()
            else {
                return !self.config.require_action_signature;
            };
            return accepted_action_signer_public_keys.contains(&signer_public_key);
        }
        let Some(signer) = &self.config.hmac_signer else {
            return !self.config.require_action_signature;
        };
        signer.verify_action(action).is_ok()
    }

    fn sign_head_if_needed(&self, head: &mut WorldHeadAnnounce) -> Result<(), WorldError> {
        if !self.config.sign_head {
            return Ok(());
        }
        if let Some(signer) = self.config.ed25519_signer.as_ref() {
            head.signature = signer.sign_head(head)?;
            Ed25519SignatureSigner::verify_head(head)?;
            return Ok(());
        }
        let signer = self.config.hmac_signer.as_ref().ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: "sign_head requires hmac_signer or ed25519_signer".to_string(),
            }
        })?;
        head.signature = signer.sign_head(head)?;
        signer.verify_head(head)?;
        Ok(())
    }

    fn ensure_lease(&mut self, now_ms: i64) -> LeaseDecision {
        self.lease.expire_if_needed(now_ms);

        if let Some(current) = self.lease.current().cloned() {
            if current.holder_id == self.config.node_id && current.expires_at_ms > now_ms {
                return self
                    .lease
                    .renew(&current.lease_id, now_ms, self.config.lease_ttl_ms);
            }
        }

        self.lease
            .try_acquire(&self.config.node_id, now_ms, self.config.lease_ttl_ms)
    }

    fn drive_pending_head(
        &mut self,
        dht: &impl DistributedDht,
        now_ms: i64,
    ) -> Result<Option<SequencerTickReport>, WorldError> {
        let Some(record) = self
            .consensus
            .record(&self.config.world_id, self.next_height)
            .cloned()
        else {
            return Ok(None);
        };

        if !matches!(record.status, PosConsensusStatus::Pending) {
            return Ok(None);
        }

        let mut decision = self.decision_from_record(&record)?;
        decision = self.drive_attestations_for_head(dht, &record.head, decision, now_ms)?;
        self.apply_finalized_status(&record.head.block_hash, decision.status)?;

        Ok(Some(SequencerTickReport {
            world_id: self.config.world_id.clone(),
            node_id: self.config.node_id.clone(),
            state: tick_state_from_status(decision.status),
            lease_granted: true,
            height: Some(decision.height),
            slot: Some(decision.slot),
            batch_id: None,
            block_hash: Some(record.head.block_hash),
            status: Some(decision.status),
        }))
    }

    fn drive_attestations_for_head(
        &mut self,
        dht: &impl DistributedDht,
        head: &WorldHeadAnnounce,
        mut decision: PosConsensusDecision,
        now_ms: i64,
    ) -> Result<PosConsensusDecision, WorldError> {
        if !matches!(decision.status, PosConsensusStatus::Pending) {
            return Ok(decision);
        }

        let target_epoch = self.consensus.slot_epoch(head.height.saturating_sub(1));
        let source_epoch = target_epoch.saturating_sub(1);

        for validator in self.consensus.validators() {
            let validator_id = validator.validator_id;
            if validator_id == self.config.node_id {
                continue;
            }

            decision = attest_world_head_with_pos(
                dht,
                &mut self.consensus,
                &head.world_id,
                head.height,
                &head.block_hash,
                &validator_id,
                true,
                now_ms,
                source_epoch,
                target_epoch,
                Some("sequencer mainloop auto attestation".to_string()),
            )?;

            if !matches!(decision.status, PosConsensusStatus::Pending) {
                break;
            }
            if !self.config.auto_attest_all_validators {
                break;
            }
        }

        Ok(decision)
    }

    fn apply_finalized_status(
        &mut self,
        block_hash: &str,
        status: PosConsensusStatus,
    ) -> Result<(), WorldError> {
        match status {
            PosConsensusStatus::Pending => Ok(()),
            PosConsensusStatus::Committed => {
                let next_height =
                    checked_sequencer_counter_increment(self.next_height, "next_height")?;
                self.prev_block_hash = block_hash.to_string();
                self.next_height = next_height;
                Ok(())
            }
            PosConsensusStatus::Rejected => {
                self.next_height =
                    checked_sequencer_counter_increment(self.next_height, "next_height")?;
                Ok(())
            }
        }
    }

    fn decision_from_record(
        &self,
        record: &super::pos::PosHeadRecord,
    ) -> Result<PosConsensusDecision, WorldError> {
        Ok(PosConsensusDecision {
            world_id: record.head.world_id.clone(),
            height: record.head.height,
            block_hash: record.head.block_hash.clone(),
            slot: record.slot,
            epoch: record.epoch,
            status: record.status,
            approved_stake: record.approved_stake,
            rejected_stake: record.rejected_stake,
            total_stake: self.consensus.total_stake(),
            required_stake: self.consensus.required_stake(),
        })
    }
}

fn tick_state_from_status(status: PosConsensusStatus) -> SequencerTickState {
    match status {
        PosConsensusStatus::Pending => SequencerTickState::Pending,
        PosConsensusStatus::Committed => SequencerTickState::Committed,
        PosConsensusStatus::Rejected => SequencerTickState::Rejected,
    }
}

fn checked_sequencer_counter_increment(value: u64, field: &str) -> Result<u64, WorldError> {
    value
        .checked_add(1)
        .ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: format!("sequencer {field} overflow at {value}"),
        })
}

fn block_hash_for_batch(
    world_id: &str,
    height: u64,
    slot: u64,
    prev_block_hash: &str,
    batch_id: &str,
    state_root: &str,
) -> Result<String, WorldError> {
    let payload = BlockHashPayload {
        world_id,
        height,
        slot,
        prev_block_hash,
        batch_id,
        state_root,
    };
    let bytes = to_canonical_cbor(&payload)?;
    Ok(super::util::blake3_hex(&bytes))
}

fn state_root_for_actions(actions: &[ActionEnvelope]) -> Result<String, WorldError> {
    let summary: Vec<ActionStateSummary<'_>> = actions
        .iter()
        .map(|action| ActionStateSummary {
            action_id: &action.action_id,
            actor_id: &action.actor_id,
            payload_hash: &action.payload_hash,
            nonce: action.nonce,
            timestamp_ms: action.timestamp_ms,
            intent_batch_hash: &action.intent_batch_hash,
            idempotency_key: &action.idempotency_key,
            zone_id: &action.zone_id,
        })
        .collect();
    let bytes = to_canonical_cbor(&summary)?;
    Ok(super::util::blake3_hex(&bytes))
}

#[derive(Debug, Serialize)]
struct BlockHashPayload<'a> {
    world_id: &'a str,
    height: u64,
    slot: u64,
    prev_block_hash: &'a str,
    batch_id: &'a str,
    state_root: &'a str,
}

#[derive(Debug, Serialize)]
struct ActionStateSummary<'a> {
    action_id: &'a str,
    actor_id: &'a str,
    payload_hash: &'a str,
    nonce: u64,
    timestamp_ms: i64,
    intent_batch_hash: &'a str,
    idempotency_key: &'a str,
    zone_id: &'a str,
}

fn to_canonical_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, WorldError> {
    super::util::to_canonical_cbor(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::distributed::ActionEnvelope;
    use super::super::distributed_dht::InMemoryDht;
    use super::super::pos::PosValidator;
    use super::super::signature::{
        ED25519_SIGNATURE_V1_PREFIX, Ed25519SignatureSigner, HmacSha256Signer,
    };
    use oasis7_proto::distributed_dht::DistributedDht as _;

    fn action(id: &str, ts: i64) -> ActionEnvelope {
        ActionEnvelope {
            world_id: "w1".to_string(),
            action_id: id.to_string(),
            actor_id: "agent-1".to_string(),
            action_kind: "move".to_string(),
            payload_cbor: vec![1, 2, 3],
            payload_hash: format!("payload-{id}"),
            nonce: 1,
            timestamp_ms: ts,
            intent_batch_hash: String::new(),
            idempotency_key: String::new(),
            zone_id: String::new(),
            signature: String::new(),
        }
    }

    fn test_pos_config() -> PosConsensusConfig {
        PosConsensusConfig::ethereum_like(vec![PosValidator {
            validator_id: "sequencer-1".to_string(),
            stake: 100,
        }])
    }

    fn signer() -> HmacSha256Signer {
        HmacSha256Signer::new(b"sequencer-test-key".to_vec()).expect("signer")
    }

    fn ed25519_signer() -> Ed25519SignatureSigner {
        let private_key_hex = hex::encode([11_u8; 32]);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[11_u8; 32]);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        Ed25519SignatureSigner::new(private_key_hex.as_str(), public_key_hex.as_str())
            .expect("ed25519 signer")
    }

    #[test]
    fn sequencer_tick_commits_batch_and_publishes_head() {
        let mut loop_state =
            SequencerMainloop::new(SequencerMainloopConfig::default(), test_pos_config())
                .expect("create loop");
        let dht = InMemoryDht::new();

        assert!(loop_state.submit_action(action("a-1", 10)));

        let report = loop_state.tick(&dht, 100).expect("tick");
        assert_eq!(report.state, SequencerTickState::Committed);
        assert_eq!(report.height, Some(1));
        assert_eq!(report.slot, Some(0));
        assert!(report.batch_id.is_some());

        let head = dht
            .get_world_head("w1")
            .expect("head query")
            .expect("head exists");
        assert_eq!(head.height, 1);
        assert_eq!(loop_state.next_height(), 2);
        assert_eq!(loop_state.next_slot(), 1);
    }

    #[test]
    fn sequencer_tick_is_idle_without_actions() {
        let mut loop_state =
            SequencerMainloop::new(SequencerMainloopConfig::default(), test_pos_config())
                .expect("create loop");
        let dht = InMemoryDht::new();

        let report = loop_state.tick(&dht, 100).expect("tick");
        assert_eq!(report.state, SequencerTickState::Idle);
        assert_eq!(report.height, None);
        assert_eq!(loop_state.next_height(), 1);
        assert_eq!(loop_state.next_slot(), 0);
    }

    #[test]
    fn sequencer_tick_rejects_slot_overflow_without_partial_state() {
        let mut loop_state =
            SequencerMainloop::new(SequencerMainloopConfig::default(), test_pos_config())
                .expect("create loop");
        let dht = InMemoryDht::new();
        loop_state.next_slot = u64::MAX;
        assert!(loop_state.submit_action(action("a-slot-overflow", 20)));

        let err = loop_state
            .tick(&dht, 100)
            .expect_err("slot overflow must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { reason }
                if reason.contains("next_slot overflow")
        ));
        assert_eq!(loop_state.next_slot(), u64::MAX);
        assert_eq!(loop_state.next_height(), 1);
        assert_eq!(
            dht.get_world_head("w1").expect("head query"),
            None,
            "slot overflow should fail before proposal publish"
        );
    }

    #[test]
    fn sequencer_tick_rejects_height_overflow_without_partial_state() {
        let mut loop_state =
            SequencerMainloop::new(SequencerMainloopConfig::default(), test_pos_config())
                .expect("create loop");
        let dht = InMemoryDht::new();
        loop_state.next_height = u64::MAX;
        loop_state.next_slot = 7;
        loop_state.prev_block_hash = "prev-hash".to_string();
        assert!(loop_state.submit_action(action("a-height-overflow", 21)));

        let err = loop_state
            .tick(&dht, 100)
            .expect_err("height overflow must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { reason }
                if reason.contains("next_height overflow")
        ));
        assert_eq!(loop_state.next_height(), u64::MAX);
        assert_eq!(loop_state.next_slot(), 7);
        assert_eq!(loop_state.prev_block_hash, "prev-hash");
    }

    #[test]
    fn submit_action_rejects_world_mismatch() {
        let mut loop_state =
            SequencerMainloop::new(SequencerMainloopConfig::default(), test_pos_config())
                .expect("create loop");

        let mut invalid = action("a-x", 1);
        invalid.world_id = "w2".to_string();

        assert!(!loop_state.submit_action(invalid));
        assert_eq!(loop_state.pending_actions(), 0);
    }

    #[test]
    fn config_rejects_non_positive_lease_ttl() {
        let config = SequencerMainloopConfig {
            lease_ttl_ms: 0,
            ..SequencerMainloopConfig::default()
        };
        let result = SequencerMainloop::new(config, test_pos_config());
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    #[test]
    fn submit_action_rejects_unsigned_when_signature_required() {
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            hmac_signer: Some(signer()),
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let unsigned = action("a-unsigned", 11);
        assert!(!loop_state.submit_action(unsigned));
        assert_eq!(loop_state.pending_actions(), 0);
    }

    #[test]
    fn submit_action_accepts_signed_when_signature_required() {
        let signer = signer();
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            hmac_signer: Some(signer.clone()),
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let mut signed = action("a-signed", 12);
        signed.signature = signer.sign_action(&signed).expect("sign action");
        assert!(loop_state.submit_action(signed));
        assert_eq!(loop_state.pending_actions(), 1);
    }

    #[test]
    fn submit_action_accepts_ed25519_signed_when_signature_required() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            accepted_action_signer_public_keys: vec![signer.public_key_hex().to_string()],
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let mut signed = action("a-signed-ed25519", 13);
        signed.signature = signer.sign_action(&signed).expect("sign action");
        assert!(loop_state.submit_action(signed));
        assert_eq!(loop_state.pending_actions(), 1);
    }

    #[test]
    fn submit_action_rejects_ed25519_signed_when_signer_not_allowed() {
        let signer = ed25519_signer();
        let unaccepted_signer_public_key = hex::encode([0x77_u8; 32]);
        assert_ne!(unaccepted_signer_public_key, signer.public_key_hex());
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            accepted_action_signer_public_keys: vec![unaccepted_signer_public_key],
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let mut signed = action("a-signed-ed25519", 14);
        signed.signature = signer.sign_action(&signed).expect("sign action");
        assert!(!loop_state.submit_action(signed));
        assert_eq!(loop_state.pending_actions(), 0);
    }

    #[test]
    fn submit_action_accepts_ed25519_signed_when_allowlist_key_is_uppercase() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            accepted_action_signer_public_keys: vec![signer.public_key_hex().to_uppercase()],
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let mut signed = action("a-signed-ed25519-upper-allowlist", 15);
        signed.signature = signer.sign_action(&signed).expect("sign action");
        assert!(loop_state.submit_action(signed));
        assert_eq!(loop_state.pending_actions(), 1);
    }

    #[test]
    fn submit_action_accepts_ed25519_signed_when_signature_public_key_is_uppercase() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            accepted_action_signer_public_keys: vec![signer.public_key_hex().to_string()],
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");

        let mut signed = action("a-signed-ed25519-upper-signature", 16);
        signed.signature = signer.sign_action(&signed).expect("sign action");
        let encoded = signed
            .signature
            .strip_prefix(ED25519_SIGNATURE_V1_PREFIX)
            .expect("ed25519 signature prefix");
        let (public_key_hex, signature_hex) = encoded
            .split_once(':')
            .expect("ed25519 signer and signature hex");
        signed.signature = format!(
            "{ED25519_SIGNATURE_V1_PREFIX}{}:{signature_hex}",
            public_key_hex.to_uppercase()
        );

        assert!(loop_state.submit_action(signed));
        assert_eq!(loop_state.pending_actions(), 1);
    }

    #[test]
    fn sequencer_tick_signs_head_when_enabled() {
        let signer = signer();
        let config = SequencerMainloopConfig {
            sign_head: true,
            hmac_signer: Some(signer.clone()),
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");
        let dht = InMemoryDht::new();

        assert!(loop_state.submit_action(action("a-1", 10)));
        let report = loop_state.tick(&dht, 100).expect("tick");
        assert_eq!(report.state, SequencerTickState::Committed);

        let head = dht
            .get_world_head("w1")
            .expect("head query")
            .expect("head exists");
        assert!(!head.signature.is_empty());
        signer.verify_head(&head).expect("verify signed head");
    }

    #[test]
    fn sequencer_tick_signs_head_with_ed25519_when_enabled() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            sign_head: true,
            ed25519_signer: Some(signer.clone()),
            ..SequencerMainloopConfig::default()
        };
        let mut loop_state = SequencerMainloop::new(config, test_pos_config()).expect("loop");
        let dht = InMemoryDht::new();

        assert!(loop_state.submit_action(action("a-1", 10)));
        let report = loop_state.tick(&dht, 100).expect("tick");
        assert_eq!(report.state, SequencerTickState::Committed);

        let head = dht
            .get_world_head("w1")
            .expect("head query")
            .expect("head exists");
        assert!(head.signature.starts_with(ED25519_SIGNATURE_V1_PREFIX));
        let signer_public_key = Ed25519SignatureSigner::verify_head(&head).expect("verify head");
        assert_eq!(signer_public_key, signer.public_key_hex());
    }

    #[test]
    fn config_rejects_signature_requirements_without_signer() {
        let missing_action_signer = SequencerMainloopConfig {
            require_action_signature: true,
            hmac_signer: None,
            ..SequencerMainloopConfig::default()
        };
        assert!(matches!(
            SequencerMainloop::new(missing_action_signer, test_pos_config()),
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let missing_head_signer = SequencerMainloopConfig {
            sign_head: true,
            hmac_signer: None,
            ..SequencerMainloopConfig::default()
        };
        assert!(matches!(
            SequencerMainloop::new(missing_head_signer, test_pos_config()),
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    #[test]
    fn config_accepts_signature_requirements_with_ed25519_allowlist_only() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            require_action_signature: true,
            accepted_action_signer_public_keys: vec![signer.public_key_hex().to_string()],
            hmac_signer: None,
            ..SequencerMainloopConfig::default()
        };
        let loop_state = SequencerMainloop::new(config, test_pos_config());
        assert!(loop_state.is_ok());
    }

    #[test]
    fn config_rejects_invalid_action_signer_public_key_allowlist_entry() {
        let config = SequencerMainloopConfig {
            accepted_action_signer_public_keys: vec!["not-hex".to_string()],
            ..SequencerMainloopConfig::default()
        };
        let result = SequencerMainloop::new(config, test_pos_config());
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { reason })
                if reason.contains("accepted_action_signer_public_keys")
        ));
    }

    #[test]
    fn config_rejects_duplicate_normalized_action_signer_public_keys() {
        let signer = ed25519_signer();
        let config = SequencerMainloopConfig {
            accepted_action_signer_public_keys: vec![
                signer.public_key_hex().to_string(),
                signer.public_key_hex().to_uppercase(),
            ],
            ..SequencerMainloopConfig::default()
        };
        let result = SequencerMainloop::new(config, test_pos_config());
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { reason })
                if reason.contains("duplicate signer public key")
        ));
    }
}
