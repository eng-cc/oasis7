//! The World struct - core runtime implementation.

mod actions;
mod agent_claims;
mod audit;
mod base_layer;
mod body;
mod bootstrap_economy;
mod bootstrap_gameplay;
mod bootstrap_power;
mod economy;
mod effects;
mod event_processing;
mod gameplay_layer;
mod gameplay_loop;
mod governance;
mod governance_identity_penalty;
mod logistics;
mod main_token_economy_audit;
mod module_actions;
mod module_runtime;
mod module_runtime_labels;
mod module_runtime_metering;
mod module_tick_runtime;
mod operability_release_gate;
mod persistence;
mod policy;
mod release_manifest;
mod resources;
mod restricted_claim_grants;
mod rules;
mod scheduling;
mod snapshot;
mod step;
mod tick_consensus;

#[cfg(all(test, feature = "wasmtime", feature = "test_tier_full"))]
pub(crate) use bootstrap_economy::m4_bootstrap_module_ids;
pub use bootstrap_power::M1ScenarioBootstrapConfig;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use oasis7_wasm_router::PreparedSubscription;

use super::consensus::{TickConsensusRecord, TickConsensusRejectionAuditEvent};
use super::effect::{CapabilityGrant, EffectIntent};
use super::events::{ActionEnvelope, MaterialTransitPriority};
use super::governance::{
    GovernanceExecutionPolicy, GovernanceFinalityEpochSnapshot, GovernanceFinalitySignerRegistry,
    GovernanceIdentityPenaltyMonitorStats, GovernanceIdentityPenaltyRecord,
    GovernanceMainTokenControllerRegistry, Proposal,
};
use super::manifest::Manifest;
use super::modules::{ModuleCache, ModuleLimits, ModuleRegistry};
use super::policy::PolicySet;
use super::signer::ReceiptSigner;
use super::snapshot::{Journal, SnapshotCatalog};
use super::state::WorldState;
use super::types::{ActionId, IntentSeq, ProposalId, WorldEventId, WorldTime};
use super::CrisisStatus;

const DEFAULT_MAX_PENDING_ACTIONS: usize = 8_192;
const DEFAULT_MAX_PENDING_EFFECTS: usize = 8_192;
const DEFAULT_MAX_INFLIGHT_EFFECTS: usize = 8_192;
const DEFAULT_MAX_JOURNAL_EVENTS: usize = 65_536;
pub(super) const BUILTIN_MODULE_SIGNER_NODE_ID: &str = "builtin.module.release.signer";
pub(super) const BUILTIN_MODULE_SIGNER_PUBLIC_KEY_HEX: &str =
    "4b97aa20b3abd613401d4f5778eab8b6c019bd2ea912d1ce2234868536389ebb";
#[cfg(any(test, feature = "test_tier_required", feature = "test_tier_full"))]
pub(super) const TEST_MODULE_SIGNER_NODE_ID: &str = "test.module.release.signer";

#[cfg(any(test, feature = "test_tier_required", feature = "test_tier_full"))]
fn test_module_signer_public_key_hex() -> String {
    use ed25519_dalek::SigningKey;

    let seed = crate::runtime::util::sha256_hex(b"oasis7-test-module-artifact-signer-v1");
    let seed_bytes = hex::decode(seed).expect("decode test module signing seed");
    let private_key_bytes: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .expect("test module signing seed is 32 bytes");
    let signing_key = SigningKey::from_bytes(&private_key_bytes);
    hex::encode(signing_key.verifying_key().to_bytes())
}

fn default_next_governance_identity_penalty_id() -> u64 {
    1
}

fn default_tick_consensus_authority_source() -> String {
    BUILTIN_MODULE_SIGNER_NODE_ID.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRuntimeMemoryLimits {
    pub max_pending_actions: usize,
    pub max_pending_effects: usize,
    pub max_inflight_effects: usize,
    pub max_journal_events: usize,
}

impl Default for WorldRuntimeMemoryLimits {
    fn default() -> Self {
        Self {
            max_pending_actions: DEFAULT_MAX_PENDING_ACTIONS,
            max_pending_effects: DEFAULT_MAX_PENDING_EFFECTS,
            max_inflight_effects: DEFAULT_MAX_INFLIGHT_EFFECTS,
            max_journal_events: DEFAULT_MAX_JOURNAL_EVENTS,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRuntimeBackpressureStats {
    pub pending_actions_evicted: u64,
    pub pending_effects_evicted: u64,
    pub inflight_effects_evicted: u64,
    pub inflight_effect_dispatch_blocked: u64,
    pub journal_events_evicted: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogisticsSlaMetrics {
    pub completed_transits: u64,
    pub fulfilled_transits: u64,
    pub breached_transits: u64,
    pub total_delay_ticks: u64,
    pub urgent_completed_transits: u64,
    pub urgent_fulfilled_transits: u64,
    pub urgent_breached_transits: u64,
    pub urgent_total_delay_ticks: u64,
}

impl LogisticsSlaMetrics {
    pub fn breach_rate(&self) -> f64 {
        if self.completed_transits == 0 {
            return 0.0;
        }
        self.breached_transits as f64 / self.completed_transits as f64
    }

    pub fn fulfillment_rate(&self) -> f64 {
        if self.completed_transits == 0 {
            return 1.0;
        }
        self.fulfilled_transits as f64 / self.completed_transits as f64
    }

    pub fn average_delay_ticks(&self) -> f64 {
        if self.completed_transits == 0 {
            return 0.0;
        }
        self.total_delay_ticks as f64 / self.completed_transits as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSecurityPolicy {
    #[serde(default = "default_allow_builtin_manifest_fallback")]
    pub allow_builtin_manifest_fallback: bool,
    #[serde(default = "default_allow_identity_hash_signature")]
    pub allow_identity_hash_signature: bool,
    #[serde(default = "default_allow_local_finality_signing")]
    pub allow_local_finality_signing: bool,
    #[serde(default = "default_allow_runtime_source_compile")]
    pub allow_runtime_source_compile: bool,
}

impl ReleaseSecurityPolicy {
    pub fn production_hardened() -> Self {
        Self {
            allow_builtin_manifest_fallback: false,
            allow_identity_hash_signature: false,
            allow_local_finality_signing: false,
            allow_runtime_source_compile: false,
        }
    }

    pub fn is_production_hardened(&self) -> bool {
        !self.allow_builtin_manifest_fallback
            && !self.allow_identity_hash_signature
            && !self.allow_local_finality_signing
            && !self.allow_runtime_source_compile
    }
}

impl Default for ReleaseSecurityPolicy {
    fn default() -> Self {
        Self {
            allow_builtin_manifest_fallback: default_allow_builtin_manifest_fallback(),
            allow_identity_hash_signature: default_allow_identity_hash_signature(),
            allow_local_finality_signing: default_allow_local_finality_signing(),
            allow_runtime_source_compile: default_allow_runtime_source_compile(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BuiltinReleaseManifestEntry {
    #[serde(default)]
    pub hash_tokens: Vec<String>,
    #[serde(default)]
    pub artifact_identities: BTreeMap<String, crate::runtime::ModuleArtifactIdentity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BuiltinReleaseManifestState {
    #[serde(default)]
    pub module_sets: BTreeMap<String, BTreeMap<String, BuiltinReleaseManifestEntry>>,
}

fn default_allow_builtin_manifest_fallback() -> bool {
    true
}

fn default_allow_identity_hash_signature() -> bool {
    true
}

fn default_allow_local_finality_signing() -> bool {
    true
}

fn default_allow_runtime_source_compile() -> bool {
    true
}

/// The main World runtime that orchestrates the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    manifest: Manifest,
    module_registry: ModuleRegistry,
    module_artifacts: BTreeSet<String>,
    #[serde(skip)]
    module_artifact_bytes: BTreeMap<String, Arc<[u8]>>,
    #[serde(skip)]
    module_cache: ModuleCache,
    #[serde(skip)]
    prepared_subscription_cache: BTreeMap<String, Arc<[PreparedSubscription]>>,
    module_limits_max: ModuleLimits,
    snapshot_catalog: SnapshotCatalog,
    state: WorldState,
    journal: Journal,
    next_event_id: WorldEventId,
    #[serde(default)]
    next_event_id_era: u64,
    next_action_id: ActionId,
    #[serde(default)]
    next_action_id_era: u64,
    next_intent_id: IntentSeq,
    #[serde(default)]
    next_intent_id_era: u64,
    next_proposal_id: ProposalId,
    #[serde(default)]
    next_proposal_id_era: u64,
    pending_actions: VecDeque<ActionEnvelope>,
    pending_effects: VecDeque<EffectIntent>,
    inflight_effects: BTreeMap<String, EffectIntent>,
    #[serde(default)]
    module_tick_schedule: BTreeMap<String, u64>,
    capabilities: BTreeMap<String, CapabilityGrant>,
    policies: PolicySet,
    proposals: BTreeMap<ProposalId, Proposal>,
    scheduler_cursor: Option<String>,
    #[serde(skip)]
    receipt_signer: Option<ReceiptSigner>,
    #[serde(default)]
    runtime_memory_limits: WorldRuntimeMemoryLimits,
    #[serde(default)]
    runtime_backpressure_stats: WorldRuntimeBackpressureStats,
    #[serde(default)]
    logistics_sla_metrics: LogisticsSlaMetrics,
    #[serde(default)]
    threat_heatmap: BTreeMap<String, i64>,
    #[serde(default)]
    tick_consensus_records: Vec<TickConsensusRecord>,
    #[serde(default = "default_tick_consensus_authority_source")]
    tick_consensus_authority_source: String,
    #[serde(default)]
    tick_consensus_rejection_audit_events: Vec<TickConsensusRejectionAuditEvent>,
    #[serde(default)]
    governance_execution_policy: GovernanceExecutionPolicy,
    #[serde(default)]
    governance_finality_epoch_snapshots: BTreeMap<u64, GovernanceFinalityEpochSnapshot>,
    #[serde(default)]
    governance_emergency_brake_until_tick: Option<WorldTime>,
    #[serde(default)]
    governance_identity_penalties: BTreeMap<u64, GovernanceIdentityPenaltyRecord>,
    #[serde(default = "default_next_governance_identity_penalty_id")]
    next_governance_identity_penalty_id: u64,
    #[serde(default)]
    builtin_release_manifest: BuiltinReleaseManifestState,
    #[serde(default)]
    release_security_policy: ReleaseSecurityPolicy,
}

impl World {
    pub fn new() -> Self {
        Self::new_with_state(WorldState::default())
    }

    pub fn new_with_release_security_policy(policy: ReleaseSecurityPolicy) -> Self {
        Self::new().with_release_security_policy(policy)
    }

    pub fn new_production_hardened() -> Self {
        Self::new_with_release_security_policy(ReleaseSecurityPolicy::production_hardened())
    }

    pub fn new_with_state(mut state: WorldState) -> Self {
        state.migrate_compat_material_ledgers();
        state
            .node_identity_bindings
            .entry(BUILTIN_MODULE_SIGNER_NODE_ID.to_string())
            .or_insert_with(|| BUILTIN_MODULE_SIGNER_PUBLIC_KEY_HEX.to_string());
        if let Some(registry) = state.governance_finality_signer_registry.clone() {
            for (node_id, public_key_hex) in registry.signer_bindings {
                state
                    .node_identity_bindings
                    .entry(node_id)
                    .or_insert(public_key_hex);
            }
        }
        #[cfg(any(test, feature = "test_tier_required", feature = "test_tier_full"))]
        state
            .node_identity_bindings
            .entry(TEST_MODULE_SIGNER_NODE_ID.to_string())
            .or_insert_with(test_module_signer_public_key_hex);
        if state.governance_finality_signer_registry.is_none() {
            for (node_id, public_key_hex) in
                governance::local_governance_finality_signer_public_keys()
            {
                state
                    .node_identity_bindings
                    .entry(node_id)
                    .or_insert(public_key_hex);
            }
        }
        Self {
            manifest: Manifest::default(),
            module_registry: ModuleRegistry::default(),
            module_artifacts: BTreeSet::new(),
            module_artifact_bytes: BTreeMap::new(),
            module_cache: ModuleCache::default(),
            prepared_subscription_cache: BTreeMap::new(),
            module_limits_max: ModuleLimits::unbounded(),
            snapshot_catalog: SnapshotCatalog::default(),
            state,
            journal: Journal::new(),
            next_event_id: 1,
            next_event_id_era: 0,
            next_action_id: 1,
            next_action_id_era: 0,
            next_intent_id: 1,
            next_intent_id_era: 0,
            next_proposal_id: 1,
            next_proposal_id_era: 0,
            pending_actions: VecDeque::new(),
            pending_effects: VecDeque::new(),
            inflight_effects: BTreeMap::new(),
            module_tick_schedule: BTreeMap::new(),
            capabilities: BTreeMap::new(),
            policies: PolicySet::default(),
            proposals: BTreeMap::new(),
            scheduler_cursor: None,
            receipt_signer: None,
            runtime_memory_limits: WorldRuntimeMemoryLimits::default(),
            runtime_backpressure_stats: WorldRuntimeBackpressureStats::default(),
            logistics_sla_metrics: LogisticsSlaMetrics::default(),
            threat_heatmap: BTreeMap::new(),
            tick_consensus_records: Vec::new(),
            tick_consensus_authority_source: default_tick_consensus_authority_source(),
            tick_consensus_rejection_audit_events: Vec::new(),
            governance_execution_policy: GovernanceExecutionPolicy::default(),
            governance_finality_epoch_snapshots: BTreeMap::new(),
            governance_emergency_brake_until_tick: None,
            governance_identity_penalties: BTreeMap::new(),
            next_governance_identity_penalty_id: default_next_governance_identity_penalty_id(),
            builtin_release_manifest: BuiltinReleaseManifestState::default(),
            release_security_policy: ReleaseSecurityPolicy::default(),
        }
    }

    pub fn with_release_security_policy(mut self, policy: ReleaseSecurityPolicy) -> Self {
        self.release_security_policy = policy;
        self
    }

    // ---------------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------------

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn module_registry(&self) -> &ModuleRegistry {
        &self.module_registry
    }

    pub fn module_limits_max(&self) -> &ModuleLimits {
        &self.module_limits_max
    }

    pub fn module_cache_len(&self) -> usize {
        self.module_cache.len()
    }

    pub fn snapshot_catalog(&self) -> &SnapshotCatalog {
        &self.snapshot_catalog
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    pub fn policies(&self) -> &PolicySet {
        &self.policies
    }

    pub fn capabilities(&self) -> &BTreeMap<String, CapabilityGrant> {
        &self.capabilities
    }

    pub fn proposals(&self) -> &BTreeMap<ProposalId, Proposal> {
        &self.proposals
    }

    pub fn runtime_backpressure_stats(&self) -> &WorldRuntimeBackpressureStats {
        &self.runtime_backpressure_stats
    }

    pub fn logistics_sla_metrics(&self) -> &LogisticsSlaMetrics {
        &self.logistics_sla_metrics
    }

    pub fn threat_heatmap(&self) -> &BTreeMap<String, i64> {
        &self.threat_heatmap
    }

    pub fn tick_consensus_records(&self) -> &[TickConsensusRecord] {
        self.tick_consensus_records.as_slice()
    }

    pub fn tick_consensus_authority_source(&self) -> &str {
        self.tick_consensus_authority_source.as_str()
    }

    pub fn tick_consensus_rejection_audit_events(&self) -> &[TickConsensusRejectionAuditEvent] {
        self.tick_consensus_rejection_audit_events.as_slice()
    }

    pub fn governance_execution_policy(&self) -> &GovernanceExecutionPolicy {
        &self.governance_execution_policy
    }

    pub fn governance_finality_epoch_snapshots(
        &self,
    ) -> &BTreeMap<u64, GovernanceFinalityEpochSnapshot> {
        &self.governance_finality_epoch_snapshots
    }

    pub fn governance_finality_signer_registry(&self) -> Option<&GovernanceFinalitySignerRegistry> {
        self.state.governance_finality_signer_registry.as_ref()
    }

    pub fn governance_main_token_controller_registry(
        &self,
    ) -> Option<&GovernanceMainTokenControllerRegistry> {
        self.state
            .governance_main_token_controller_registry
            .as_ref()
    }

    pub fn governance_emergency_brake_until_tick(&self) -> Option<WorldTime> {
        self.governance_emergency_brake_until_tick
    }

    pub fn governance_identity_penalties(&self) -> &BTreeMap<u64, GovernanceIdentityPenaltyRecord> {
        &self.governance_identity_penalties
    }

    pub fn governance_identity_penalty_monitor_stats(
        &self,
        high_risk_threshold: i64,
    ) -> GovernanceIdentityPenaltyMonitorStats {
        let mut stats = GovernanceIdentityPenaltyMonitorStats::default();
        for record in self.governance_identity_penalties.values() {
            stats.total_penalties = stats.total_penalties.saturating_add(1);
            if record.status != super::GovernanceIdentityPenaltyStatus::Applied {
                stats.appealed_penalties = stats.appealed_penalties.saturating_add(1);
            }
            if record.status == super::GovernanceIdentityPenaltyStatus::Appealed
                || record.status == super::GovernanceIdentityPenaltyStatus::AppealAccepted
                || record.status == super::GovernanceIdentityPenaltyStatus::AppealRejected
            {
                stats.resolved_appeals = stats.resolved_appeals.saturating_add(u64::from(
                    record.status != super::GovernanceIdentityPenaltyStatus::Appealed,
                ));
                if record.status == super::GovernanceIdentityPenaltyStatus::AppealAccepted {
                    stats.appeal_accepted_penalties =
                        stats.appeal_accepted_penalties.saturating_add(1);
                }
            }
            if record.status == super::GovernanceIdentityPenaltyStatus::Applied
                && record.detection_risk_score >= high_risk_threshold
            {
                stats.high_risk_open_penalties = stats.high_risk_open_penalties.saturating_add(1);
            }
        }
        if stats.resolved_appeals > 0 {
            stats.false_positive_rate_bps =
                ((stats.appeal_accepted_penalties.saturating_mul(10_000)) / stats.resolved_appeals)
                    .min(10_000) as u16;
        }
        stats
    }

    pub fn builtin_release_manifest(&self) -> &BuiltinReleaseManifestState {
        &self.builtin_release_manifest
    }

    pub fn release_security_policy(&self) -> &ReleaseSecurityPolicy {
        &self.release_security_policy
    }

    pub fn set_release_security_policy(&mut self, policy: ReleaseSecurityPolicy) {
        self.release_security_policy = policy;
    }

    pub fn enable_production_release_policy(&mut self) {
        self.release_security_policy = ReleaseSecurityPolicy::production_hardened();
    }

    pub fn with_runtime_memory_limits(mut self, limits: WorldRuntimeMemoryLimits) -> Self {
        self.runtime_memory_limits = limits;
        self.enforce_runtime_memory_limits();
        self
    }

    pub(super) fn allocate_next_event_id(&mut self) -> WorldEventId {
        Self::allocate_rolling_sequence_id(&mut self.next_event_id, &mut self.next_event_id_era)
    }

    pub(super) fn allocate_next_action_id(&mut self) -> ActionId {
        Self::allocate_rolling_sequence_id(&mut self.next_action_id, &mut self.next_action_id_era)
    }

    pub(super) fn allocate_next_intent_seq(&mut self) -> IntentSeq {
        Self::allocate_rolling_sequence_id(&mut self.next_intent_id, &mut self.next_intent_id_era)
    }

    pub(super) fn allocate_next_proposal_id(&mut self) -> ProposalId {
        Self::allocate_rolling_sequence_id(
            &mut self.next_proposal_id,
            &mut self.next_proposal_id_era,
        )
    }

    fn allocate_rolling_sequence_id(next_id: &mut u64, era: &mut u64) -> u64 {
        if *next_id == 0 {
            *next_id = 1;
        }
        let allocated = *next_id;
        if allocated == u64::MAX {
            *next_id = 1;
            *era = era.saturating_add(1);
        } else {
            *next_id = allocated + 1;
        }
        allocated
    }

    pub(super) fn enforce_pending_action_limit(&mut self) {
        let max_len = self.runtime_memory_limits.max_pending_actions.max(1);
        while self.pending_actions.len() > max_len {
            let _ = self.pending_actions.pop_front();
            self.runtime_backpressure_stats.pending_actions_evicted = self
                .runtime_backpressure_stats
                .pending_actions_evicted
                .saturating_add(1);
        }
    }

    pub(super) fn push_pending_effect_bounded(&mut self, intent: EffectIntent) {
        self.pending_effects.push_back(intent);
        self.enforce_pending_effect_limit();
    }

    pub(super) fn record_logistics_sla_completion(
        &mut self,
        expected_ready_at: WorldTime,
        completed_at: WorldTime,
        priority: MaterialTransitPriority,
    ) {
        self.logistics_sla_metrics.completed_transits = self
            .logistics_sla_metrics
            .completed_transits
            .saturating_add(1);
        if priority == MaterialTransitPriority::Urgent {
            self.logistics_sla_metrics.urgent_completed_transits = self
                .logistics_sla_metrics
                .urgent_completed_transits
                .saturating_add(1);
        }
        if completed_at > expected_ready_at {
            let delay = completed_at.saturating_sub(expected_ready_at);
            self.logistics_sla_metrics.breached_transits = self
                .logistics_sla_metrics
                .breached_transits
                .saturating_add(1);
            self.logistics_sla_metrics.total_delay_ticks = self
                .logistics_sla_metrics
                .total_delay_ticks
                .saturating_add(delay);
            if priority == MaterialTransitPriority::Urgent {
                self.logistics_sla_metrics.urgent_breached_transits = self
                    .logistics_sla_metrics
                    .urgent_breached_transits
                    .saturating_add(1);
                self.logistics_sla_metrics.urgent_total_delay_ticks = self
                    .logistics_sla_metrics
                    .urgent_total_delay_ticks
                    .saturating_add(delay);
            }
        } else {
            self.logistics_sla_metrics.fulfilled_transits = self
                .logistics_sla_metrics
                .fulfilled_transits
                .saturating_add(1);
            if priority == MaterialTransitPriority::Urgent {
                self.logistics_sla_metrics.urgent_fulfilled_transits = self
                    .logistics_sla_metrics
                    .urgent_fulfilled_transits
                    .saturating_add(1);
            }
        }
    }

    pub(super) fn refresh_threat_heatmap(&mut self) {
        let mut next = BTreeMap::new();
        for war in self.state.wars.values() {
            if !war.active {
                continue;
            }
            let war_risk = (war.intensity as i64).saturating_mul(10).max(10);
            *next
                .entry(format!("alliance:{}", war.aggressor_alliance_id))
                .or_insert(0) += war_risk;
            *next
                .entry(format!("alliance:{}", war.defender_alliance_id))
                .or_insert(0) += war_risk;
            *next.entry("global:war".to_string()).or_insert(0) += war_risk;
        }
        for crisis in self.state.crises.values() {
            if !matches!(crisis.status, CrisisStatus::Active) {
                continue;
            }
            let crisis_risk = (crisis.severity as i64).saturating_mul(12).max(12);
            *next.entry(format!("crisis:{}", crisis.kind)).or_insert(0) += crisis_risk;
            *next.entry("global:crisis".to_string()).or_insert(0) += crisis_risk;
        }
        self.threat_heatmap = next;
    }

    pub(super) fn enforce_pending_effect_limit(&mut self) {
        let max_len = self.runtime_memory_limits.max_pending_effects.max(1);
        while self.pending_effects.len() > max_len {
            let _ = self.pending_effects.pop_front();
            self.runtime_backpressure_stats.pending_effects_evicted = self
                .runtime_backpressure_stats
                .pending_effects_evicted
                .saturating_add(1);
        }
    }

    pub(super) fn inflight_effect_capacity_reached(&self) -> bool {
        self.inflight_effects.len() >= self.runtime_memory_limits.max_inflight_effects.max(1)
    }

    pub(super) fn record_inflight_effect_dispatch_blocked(&mut self) {
        self.runtime_backpressure_stats
            .inflight_effect_dispatch_blocked = self
            .runtime_backpressure_stats
            .inflight_effect_dispatch_blocked
            .saturating_add(1);
    }

    pub(super) fn enforce_inflight_effect_limit(&mut self) {
        let max_len = self.runtime_memory_limits.max_inflight_effects.max(1);
        while self.inflight_effects.len() > max_len {
            if let Some(first_key) = self.inflight_effects.keys().next().cloned() {
                self.inflight_effects.remove(first_key.as_str());
                self.runtime_backpressure_stats.inflight_effects_evicted = self
                    .runtime_backpressure_stats
                    .inflight_effects_evicted
                    .saturating_add(1);
            } else {
                break;
            }
        }
    }

    pub(super) fn enforce_journal_event_limit(&mut self) {
        let max_len = self.runtime_memory_limits.max_journal_events.max(1);
        let overflow = self.journal.events.len().saturating_sub(max_len);
        if overflow > 0 {
            self.journal.events.drain(0..overflow);
            self.runtime_backpressure_stats.journal_events_evicted = self
                .runtime_backpressure_stats
                .journal_events_evicted
                .saturating_add(overflow as u64);
        }
    }

    pub(super) fn enforce_runtime_memory_limits(&mut self) {
        self.enforce_pending_action_limit();
        self.enforce_pending_effect_limit();
        self.enforce_inflight_effect_limit();
        self.enforce_journal_event_limit();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
