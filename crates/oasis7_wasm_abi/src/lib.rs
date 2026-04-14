use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

mod economy;

pub use economy::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Reducer,
    Pure,
}

impl ModuleKind {
    pub fn entrypoint(&self) -> &'static str {
        match self {
            ModuleKind::Reducer => "reduce",
            ModuleKind::Pure => "call",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleLimits {
    pub max_mem_bytes: u64,
    pub max_gas: u64,
    pub max_call_rate: u32,
    pub max_output_bytes: u64,
    pub max_effects: u32,
    pub max_emits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleArtifact {
    pub wasm_hash: String,
    pub bytes: Arc<[u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleArtifactIdentity {
    pub source_hash: String,
    pub build_manifest_hash: String,
    pub signer_node_id: String,
    pub signature_scheme: String,
    pub artifact_signature: String,
}

impl ModuleArtifactIdentity {
    pub const SIGNATURE_SCHEME_ED25519: &'static str = "ed25519";
    pub const SIGNATURE_PREFIX_ED25519_V1: &'static str = "modsig:ed25519:v1:";

    pub fn is_complete(&self) -> bool {
        !self.source_hash.trim().is_empty()
            && !self.build_manifest_hash.trim().is_empty()
            && !self.signer_node_id.trim().is_empty()
            && !self.signature_scheme.trim().is_empty()
            && !self.artifact_signature.trim().is_empty()
    }

    pub fn signing_payload_v1(
        wasm_hash: &str,
        source_hash: &str,
        build_manifest_hash: &str,
        signer_node_id: &str,
    ) -> Vec<u8> {
        format!(
            "modsig:ed25519:v1|{wasm_hash}|{source_hash}|{build_manifest_hash}|{signer_node_id}"
        )
        .into_bytes()
    }

    pub fn expected_signature_prefix(&self) -> Option<&'static str> {
        match self.signature_scheme.as_str() {
            Self::SIGNATURE_SCHEME_ED25519 => Some(Self::SIGNATURE_PREFIX_ED25519_V1),
            _ => None,
        }
    }

    pub fn has_unsigned_prefix(&self) -> bool {
        self.artifact_signature.starts_with("unsigned:")
    }
}

impl Default for ModuleArtifactIdentity {
    fn default() -> Self {
        Self {
            source_hash: String::new(),
            build_manifest_hash: String::new(),
            signer_node_id: String::new(),
            signature_scheme: String::new(),
            artifact_signature: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleCache {
    max_cached_modules: usize,
    cache: BTreeMap<String, ModuleArtifact>,
    lru: VecDeque<String>,
}

impl ModuleCache {
    pub fn new(max_cached_modules: usize) -> Self {
        Self {
            max_cached_modules,
            cache: BTreeMap::new(),
            lru: VecDeque::new(),
        }
    }

    pub fn max_cached_modules(&self) -> usize {
        self.max_cached_modules
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn set_max_cached_modules(&mut self, max_cached_modules: usize) {
        self.max_cached_modules = max_cached_modules;
        self.prune();
    }

    pub fn get(&mut self, wasm_hash: &str) -> Option<ModuleArtifact> {
        let artifact = self.cache.get(wasm_hash)?.clone();
        self.touch(wasm_hash);
        Some(artifact)
    }

    pub fn insert(&mut self, artifact: ModuleArtifact) {
        let key = artifact.wasm_hash.clone();
        self.cache.insert(key.clone(), artifact);
        self.touch(&key);
        self.prune();
    }

    fn touch(&mut self, wasm_hash: &str) {
        self.lru.retain(|entry| entry != wasm_hash);
        self.lru.push_back(wasm_hash.to_string());
    }

    fn prune(&mut self) {
        if self.max_cached_modules == 0 {
            self.cache.clear();
            self.lru.clear();
            return;
        }
        while self.cache.len() > self.max_cached_modules {
            if let Some(evicted) = self.lru.pop_front() {
                self.cache.remove(&evicted);
            } else {
                break;
            }
        }
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new(8)
    }
}

impl Default for ModuleLimits {
    fn default() -> Self {
        Self {
            max_mem_bytes: 0,
            max_gas: 0,
            max_call_rate: 0,
            max_output_bytes: 0,
            max_effects: 0,
            max_emits: 0,
        }
    }
}

impl ModuleLimits {
    pub fn unbounded() -> Self {
        Self {
            max_mem_bytes: u64::MAX,
            max_gas: u64::MAX,
            max_call_rate: u32::MAX,
            max_output_bytes: u64::MAX,
            max_effects: u32::MAX,
            max_emits: u32::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleSubscription {
    #[serde(default)]
    pub event_kinds: Vec<String>,
    #[serde(default)]
    pub action_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<ModuleSubscriptionStage>,
    #[serde(default)]
    pub filters: Option<JsonValue>,
}

impl ModuleSubscription {
    pub fn resolved_stage(&self) -> ModuleSubscriptionStage {
        self.stage.unwrap_or_else(|| {
            if !self.event_kinds.is_empty() {
                ModuleSubscriptionStage::PostEvent
            } else {
                ModuleSubscriptionStage::PreAction
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleSubscriptionStage {
    PreAction,
    PostAction,
    PostEvent,
    Tick,
}

impl Default for ModuleSubscriptionStage {
    fn default() -> Self {
        ModuleSubscriptionStage::PostEvent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleRole {
    Rule,
    Domain,
    Gameplay,
    Body,
    AgentInternal,
}

impl Default for ModuleRole {
    fn default() -> Self {
        ModuleRole::Domain
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum GameplayModuleKind {
    #[default]
    War,
    Governance,
    Crisis,
    Economic,
    Meta,
}

impl GameplayModuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GameplayModuleKind::War => "war",
            GameplayModuleKind::Governance => "governance",
            GameplayModuleKind::Crisis => "crisis",
            GameplayModuleKind::Economic => "economic",
            GameplayModuleKind::Meta => "meta",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayContract {
    pub kind: GameplayModuleKind,
    #[serde(default)]
    pub game_modes: Vec<String>,
    #[serde(default = "default_gameplay_min_players")]
    pub min_players: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_players: Option<u16>,
}

fn default_gameplay_min_players() -> u16 {
    1
}

impl Default for GameplayContract {
    fn default() -> Self {
        Self {
            kind: GameplayModuleKind::War,
            game_modes: Vec::new(),
            min_players: default_gameplay_min_players(),
            max_players: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub module_id: String,
    pub name: String,
    pub version: String,
    pub kind: ModuleKind,
    #[serde(default)]
    pub role: ModuleRole,
    pub wasm_hash: String,
    pub interface_version: String,
    #[serde(default)]
    pub exports: Vec<String>,
    #[serde(default)]
    pub subscriptions: Vec<ModuleSubscription>,
    #[serde(default)]
    pub required_caps: Vec<String>,
    #[serde(default)]
    pub abi_contract: ModuleAbiContract,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_identity: Option<ModuleArtifactIdentity>,
    #[serde(default)]
    pub limits: ModuleLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModuleAbiContract {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<String>,
    #[serde(default)]
    pub cap_slots: BTreeMap<String, String>,
    #[serde(default)]
    pub policy_hooks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gameplay: Option<GameplayContract>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModuleChangeSet {
    #[serde(default)]
    pub register: Vec<ModuleManifest>,
    #[serde(default)]
    pub activate: Vec<ModuleActivation>,
    #[serde(default)]
    pub deactivate: Vec<ModuleDeactivation>,
    #[serde(default)]
    pub upgrade: Vec<ModuleUpgrade>,
}

impl ModuleChangeSet {
    pub fn is_empty(&self) -> bool {
        self.register.is_empty()
            && self.activate.is_empty()
            && self.deactivate.is_empty()
            && self.upgrade.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleActivation {
    pub module_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDeactivation {
    pub module_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleUpgrade {
    pub module_id: String,
    pub from_version: String,
    pub to_version: String,
    pub wasm_hash: String,
    pub manifest: ModuleManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModuleRegistry {
    pub records: BTreeMap<String, ModuleRecord>,
    pub active: BTreeMap<String, String>,
}

impl ModuleRegistry {
    pub fn record_key(module_id: &str, version: &str) -> String {
        format!("{module_id}@{version}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub manifest: ModuleManifest,
    pub registered_at: u64,
    pub registered_by: String,
    pub audit_event_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEvent {
    pub proposal_id: u64,
    #[serde(flatten)]
    pub kind: ModuleEventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ModuleEventKind {
    RegisterModule {
        module: ModuleManifest,
        registered_by: String,
    },
    ActivateModule {
        module_id: String,
        version: String,
        activated_by: String,
    },
    DeactivateModule {
        module_id: String,
        reason: String,
        deactivated_by: String,
    },
    UpgradeModule {
        module_id: String,
        from_version: String,
        to_version: String,
        wasm_hash: String,
        manifest: ModuleManifest,
        upgraded_by: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEffectIntent {
    pub kind: String,
    pub params: JsonValue,
    pub cap_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_slot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEmit {
    pub kind: String,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleOutput {
    pub new_state: Option<Vec<u8>>,
    #[serde(default)]
    pub effects: Vec<ModuleEffectIntent>,
    #[serde(default)]
    pub emits: Vec<ModuleEmit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_lifecycle: Option<ModuleTickLifecycleDirective>,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ModuleTickLifecycleDirective {
    WakeAfterTicks { ticks: u64 },
    Suspend,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleCallRequest {
    pub module_id: String,
    pub wasm_hash: String,
    pub trace_id: String,
    pub entrypoint: String,
    pub input: Vec<u8>,
    pub limits: ModuleLimits,
    #[serde(default)]
    pub wasm_bytes: Arc<[u8]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleCallOrigin {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleContext {
    pub v: String,
    pub module_id: String,
    pub trace_id: String,
    pub time: u64,
    pub origin: ModuleCallOrigin,
    pub limits: ModuleLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_config_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_height: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleCallInput {
    pub ctx: ModuleContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCallErrorCode {
    Trap,
    Timeout,
    OutOfFuel,
    Interrupted,
    OutputTooLarge,
    EffectLimitExceeded,
    EmitLimitExceeded,
    CapsDenied,
    PolicyDenied,
    SandboxUnavailable,
    InvalidOutput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleCallFailure {
    pub module_id: String,
    pub trace_id: String,
    pub code: ModuleCallErrorCode,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEmitEvent {
    pub module_id: String,
    pub trace_id: String,
    pub kind: String,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleStateUpdate {
    pub module_id: String,
    pub trace_id: String,
    pub state: Vec<u8>,
}

pub trait ModuleSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn artifact(hash: &str, byte: u8) -> ModuleArtifact {
        ModuleArtifact {
            wasm_hash: hash.to_string(),
            bytes: Arc::<[u8]>::from(vec![byte]),
        }
    }

    #[test]
    fn module_cache_evicts_lru_entry() {
        let mut cache = ModuleCache::new(2);
        cache.insert(artifact("a", 1));
        cache.insert(artifact("b", 2));
        let _ = cache.get("a");
        cache.insert(artifact("c", 3));

        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn module_cache_zero_capacity_stays_empty() {
        let mut cache = ModuleCache::new(0);
        cache.insert(artifact("a", 1));
        assert_eq!(cache.len(), 0);
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn module_cache_set_max_prunes_existing_entries() {
        let mut cache = ModuleCache::new(3);
        cache.insert(artifact("a", 1));
        cache.insert(artifact("b", 2));
        cache.insert(artifact("c", 3));
        cache.set_max_cached_modules(1);

        assert_eq!(cache.len(), 1);
        assert!(cache.get("c").is_some());
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_none());
    }

    #[test]
    fn module_change_set_is_empty_checks_all_sections() {
        let mut changes = ModuleChangeSet::default();
        assert!(changes.is_empty());

        changes.activate.push(ModuleActivation {
            module_id: "m.test".to_string(),
            version: "v1".to_string(),
        });
        assert!(!changes.is_empty());
    }

    #[test]
    fn module_registry_record_key_uses_module_and_version() {
        assert_eq!(
            ModuleRegistry::record_key("m.rule", "1.2.3"),
            "m.rule@1.2.3"
        );
    }

    #[test]
    fn module_event_kind_serialization_keeps_tag_format() {
        let kind = ModuleEventKind::DeactivateModule {
            module_id: "m.rule".to_string(),
            reason: "manual".to_string(),
            deactivated_by: "tester".to_string(),
        };

        let json = serde_json::to_value(&kind).expect("serialize module event kind");
        assert_eq!(json["type"], "DeactivateModule");
        assert_eq!(json["data"]["module_id"], "m.rule");
        assert_eq!(json["data"]["reason"], "manual");
        assert_eq!(json["data"]["deactivated_by"], "tester");
    }

    #[test]
    fn module_artifact_identity_payload_and_prefix() {
        let identity = ModuleArtifactIdentity {
            source_hash: "src-1".to_string(),
            build_manifest_hash: "build-1".to_string(),
            signer_node_id: "node-1".to_string(),
            signature_scheme: ModuleArtifactIdentity::SIGNATURE_SCHEME_ED25519.to_string(),
            artifact_signature: format!(
                "{}{}",
                ModuleArtifactIdentity::SIGNATURE_PREFIX_ED25519_V1,
                "abcd"
            ),
        };
        assert!(identity.is_complete());
        assert_eq!(
            identity.expected_signature_prefix(),
            Some(ModuleArtifactIdentity::SIGNATURE_PREFIX_ED25519_V1)
        );
        assert_eq!(
            ModuleArtifactIdentity::signing_payload_v1("hash-1", "src-1", "build-1", "node-1"),
            b"modsig:ed25519:v1|hash-1|src-1|build-1|node-1".to_vec()
        );
        assert!(!identity.has_unsigned_prefix());
    }

    #[test]
    #[ignore = "local perf probe"]
    fn perf_probe_module_cache_clone_cost_scales_with_wasm_size() {
        let sizes = [4 * 1024usize, 256 * 1024usize, 4 * 1024 * 1024usize];

        for size in sizes {
            let mut cache = ModuleCache::new(1);
            let key = format!("hash-{size}");
            cache.insert(ModuleArtifact {
                wasm_hash: key.clone(),
                bytes: vec![7_u8; size].into(),
            });
            let iterations = match size {
                0..=16_384 => 200_000u32,
                16_385..=1_048_576 => 20_000u32,
                _ => 2_000u32,
            };

            let started = Instant::now();
            let mut bytes_observed = 0usize;
            for _ in 0..iterations {
                let artifact = cache.get(&key).expect("cache hit");
                bytes_observed = bytes_observed.saturating_add(artifact.bytes.len());
            }
            let elapsed = started.elapsed();
            let avg_us = elapsed.as_secs_f64() * 1_000_000.0 / f64::from(iterations);
            let throughput_mib_s = if elapsed.as_secs_f64() == 0.0 {
                0.0
            } else {
                (bytes_observed as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
            };
            eprintln!(
                "perf_probe_module_cache_clone_cost_scales_with_wasm_size: size_bytes={size} iterations={iterations} total_ms={:.3} avg_us_per_get={:.3} throughput_mib_s={:.2}",
                elapsed.as_secs_f64() * 1_000.0,
                avg_us,
                throughput_mib_s
            );
        }
    }
}
