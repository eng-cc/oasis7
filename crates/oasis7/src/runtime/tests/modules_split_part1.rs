use super::super::*;
use super::pos;
use crate::simulator::{ModuleInstallTarget, ResourceKind};
use oasis7_wasm_abi::{
    ModuleCallErrorCode, ModuleCallFailure, ModuleCallInput, ModuleCallRequest, ModuleEffectIntent,
    ModuleEmit, ModuleOutput, ModuleSandbox, ModuleTickLifecycleDirective,
};
use oasis7_wasm_executor::FixedSandbox;
#[cfg(not(feature = "wasmtime"))]
use oasis7_wasm_executor::{WasmExecutor, WasmExecutorConfig};
use serde_json::json;
use std::collections::VecDeque;

#[test]
fn apply_module_changes_registers_and_activates() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();
    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: vec![ModuleSubscription {
            event_kinds: vec!["WeatherTick".to_string()],
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::PostEvent),
            filters: None,
        }],
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024,
            max_gas: 10_000,
            max_call_rate: 1,
            max_output_bytes: 2048,
            max_effects: 2,
            max_emits: 2,
        },
    };
    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();

    let key = ModuleRegistry::record_key(&module_manifest.module_id, &module_manifest.version);
    let record = world.module_registry().records.get(&key).unwrap();
    assert_eq!(record.manifest, module_manifest);
    assert_eq!(record.registered_by, "alice");
    assert_eq!(
        world
            .module_registry()
            .active
            .get(&module_manifest.module_id),
        Some(&module_manifest.version)
    );

    let module_events: Vec<_> = world
        .journal()
        .events
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::ModuleEvent(module_event) => Some(module_event),
            _ => None,
        })
        .collect();
    assert_eq!(module_events.len(), 2);
    assert!(matches!(
        module_events[0].kind,
        ModuleEventKind::RegisterModule { .. }
    ));
    assert!(matches!(
        module_events[1].kind,
        ModuleEventKind::ActivateModule { .. }
    ));

    if let serde_json::Value::Object(map) = &world.manifest().content {
        assert!(!map.contains_key("module_changes"));
    }
}

#[test]
fn shadow_rejects_missing_module_artifact() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: "missing-hash".to_string(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity("missing-hash")),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    assert!(matches!(err, WorldError::ModuleChangeInvalid { .. }));
}

#[test]
fn shadow_rejects_incomplete_module_artifact_identity() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-identity";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(ModuleArtifactIdentity {
            source_hash: String::new(),
            build_manifest_hash: "build-hash".to_string(),
            signer_node_id: "node-1".to_string(),
            signature_scheme: "ed25519".to_string(),
            artifact_signature: "unsigned:dummy:src:build".to_string(),
        }),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("artifact_identity is incomplete"));
}

#[test]
fn shadow_rejects_module_artifact_identity_signature_mismatch() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-identity-mismatch";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(ModuleArtifactIdentity {
            source_hash: "src-hash".to_string(),
            build_manifest_hash: "build-hash".to_string(),
            signer_node_id: "node-1".to_string(),
            signature_scheme: "ed25519".to_string(),
            artifact_signature: "unsigned:different-wasm-hash:src-hash:build-hash".to_string(),
        }),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("unsigned signature is forbidden"));
}

#[test]
fn shadow_rejects_identity_hash_signature_when_release_policy_disables_fallback() {
    let mut world = World::new();
    world.enable_production_release_policy();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-identity-hash";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let source_hash = util::sha256_hex(b"weather-src");
    let build_manifest_hash = util::sha256_hex(b"weather-build");
    let identity_hash =
        util::sha256_hex(format!("m.weather:{source_hash}:{build_manifest_hash}").as_bytes());
    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(ModuleArtifactIdentity {
            source_hash,
            build_manifest_hash,
            signer_node_id: "builtin.module.release.signer".to_string(),
            signature_scheme: "identity_hash_v1".to_string(),
            artifact_signature: format!("idhash:{identity_hash}"),
        }),
        limits: ModuleLimits::default(),
    };
    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };
    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("signature_scheme identity_hash_v1 is disabled"));
}

#[test]
fn shadow_rejects_unsupported_module_abi_version() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-abi-version";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract {
            abi_version: Some(2),
            input_schema: Some("schema.input@1".to_string()),
            output_schema: Some("schema.output@1".to_string()),
            cap_slots: Default::default(),
            policy_hooks: Vec::new(),
            gameplay: None,
        },
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("abi_version unsupported"));
}

#[test]
fn shadow_rejects_partial_module_schema_contract() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-schema-contract";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract {
            abi_version: Some(1),
            input_schema: Some("schema.input@1".to_string()),
            output_schema: None,
            cap_slots: Default::default(),
            policy_hooks: Vec::new(),
            gameplay: None,
        },
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("input_schema/output_schema pair"));
}

#[test]
fn shadow_rejects_cap_slot_binding_to_unknown_required_cap() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    let wasm_bytes = b"dummy-wasm-weather-cap-slot";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract {
            abi_version: Some(1),
            input_schema: Some("schema.input@1".to_string()),
            output_schema: Some("schema.output@1".to_string()),
            cap_slots: std::collections::BTreeMap::from([(
                "weather_api".to_string(),
                "cap.not-required".to_string(),
            )]),
            policy_hooks: Vec::new(),
            gameplay: None,
        },
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits::default(),
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    let err = world.shadow_proposal(proposal_id).unwrap_err();
    let WorldError::ModuleChangeInvalid { reason } = err else {
        panic!("expected ModuleChangeInvalid");
    };
    assert!(reason.contains("binds unknown cap_ref"));
}

#[test]
fn module_cache_loads_and_evicts() {
    let mut world = World::new();
    let wasm_a = b"module-a";
    let wasm_b = b"module-b";
    let hash_a = util::sha256_hex(wasm_a);
    let hash_b = util::sha256_hex(wasm_b);

    world
        .register_module_artifact(hash_a.clone(), wasm_a)
        .unwrap();
    world
        .register_module_artifact(hash_b.clone(), wasm_b)
        .unwrap();
    world.set_module_cache_max(1);

    let artifact_a = world.load_module(&hash_a).unwrap();
    assert_eq!(artifact_a.wasm_hash, hash_a);
    assert_eq!(artifact_a.bytes, wasm_a.to_vec().into());
    assert_eq!(world.module_cache_len(), 1);

    let artifact_b = world.load_module(&hash_b).unwrap();
    assert_eq!(artifact_b.wasm_hash, hash_b);
    assert_eq!(world.module_cache_len(), 1);

    let artifact_a_again = world.load_module(&hash_a).unwrap();
    assert_eq!(artifact_a_again.wasm_hash, hash_a);
    assert_eq!(world.module_cache_len(), 1);
}

#[test]
fn module_output_limits_reject_excess() {
    let world = World::new();
    let limits = ModuleLimits {
        max_mem_bytes: u64::MAX,
        max_gas: u64::MAX,
        max_call_rate: u32::MAX,
        max_output_bytes: 8,
        max_effects: 1,
        max_emits: 1,
    };

    let err = world
        .validate_module_output_limits("m.test", &limits, 2, 0, 4)
        .unwrap_err();
    assert!(matches!(err, WorldError::ModuleChangeInvalid { .. }));

    let err = world
        .validate_module_output_limits("m.test", &limits, 1, 1, 12)
        .unwrap_err();
    assert!(matches!(err, WorldError::ModuleChangeInvalid { .. }));

    let err = world
        .validate_module_output_limits("m.test", &limits, usize::MAX, 0, 4)
        .unwrap_err();
    assert!(matches!(err, WorldError::ModuleChangeInvalid { .. }));
}

#[test]
fn module_call_queues_effects_and_emits() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    world.set_policy(PolicySet::allow_all());

    let wasm_bytes = b"module-weather";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024,
            max_gas: 10_000,
            max_call_rate: 1,
            max_output_bytes: 1024,
            max_effects: 2,
            max_emits: 2,
        },
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();

    let output = ModuleOutput {
        new_state: None,
        effects: vec![ModuleEffectIntent {
            kind: "http.request".to_string(),
            params: json!({"url": "https://example.com"}),
            cap_ref: "cap.weather".to_string(),
            cap_slot: None,
        }],
        emits: vec![ModuleEmit {
            kind: "WeatherTick".to_string(),
            payload: json!({"ok": true}),
        }],
        tick_lifecycle: None,
        output_bytes: 64,
    };

    let mut sandbox = FixedSandbox::succeed(output);
    world
        .execute_module_call("m.weather", "trace-1", vec![], &mut sandbox)
        .unwrap();

    assert_eq!(world.pending_effects_len(), 1);

    let has_emit = world
        .journal()
        .events
        .iter()
        .any(|event| matches!(event.body, WorldEventBody::ModuleEmitted(_)));
    assert!(has_emit);
}

#[test]
fn module_call_resolves_effect_cap_from_cap_slot() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    world.set_policy(PolicySet::allow_all());

    let wasm_bytes = b"module-weather-cap-slot";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract {
            abi_version: Some(1),
            input_schema: Some("schema.input@1".to_string()),
            output_schema: Some("schema.output@1".to_string()),
            cap_slots: std::collections::BTreeMap::from([(
                "weather_api".to_string(),
                "cap.weather".to_string(),
            )]),
            policy_hooks: Vec::new(),
            gameplay: None,
        },
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024,
            max_gas: 10_000,
            max_call_rate: 1,
            max_output_bytes: 1024,
            max_effects: 2,
            max_emits: 0,
        },
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();

    let output = ModuleOutput {
        new_state: None,
        effects: vec![ModuleEffectIntent {
            kind: "http.request".to_string(),
            params: json!({"url": "https://example.com"}),
            cap_ref: String::new(),
            cap_slot: Some("weather_api".to_string()),
        }],
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 64,
    };

    let mut sandbox = FixedSandbox::succeed(output);
    world
        .execute_module_call("m.weather", "trace-slot", vec![], &mut sandbox)
        .unwrap();

    let queued = world.take_next_effect().expect("queued effect");
    assert_eq!(queued.cap_ref, "cap.weather");
}

#[test]
fn module_call_rejects_effect_with_unbound_cap_slot() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    world.set_policy(PolicySet::allow_all());

    let wasm_bytes = b"module-weather-cap-slot-missing";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract {
            abi_version: Some(1),
            input_schema: Some("schema.input@1".to_string()),
            output_schema: Some("schema.output@1".to_string()),
            cap_slots: std::collections::BTreeMap::new(),
            policy_hooks: Vec::new(),
            gameplay: None,
        },
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024,
            max_gas: 10_000,
            max_call_rate: 1,
            max_output_bytes: 1024,
            max_effects: 2,
            max_emits: 0,
        },
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();

    let output = ModuleOutput {
        new_state: None,
        effects: vec![ModuleEffectIntent {
            kind: "http.request".to_string(),
            params: json!({"url": "https://example.com"}),
            cap_ref: String::new(),
            cap_slot: Some("missing_slot".to_string()),
        }],
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 64,
    };

    let mut sandbox = FixedSandbox::succeed(output);
    let err = world
        .execute_module_call("m.weather", "trace-slot-missing", vec![], &mut sandbox)
        .unwrap_err();
    assert!(matches!(err, WorldError::ModuleCallFailed { .. }));

    let failed = world
        .journal()
        .events
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::ModuleCallFailed(failure) => Some(failure),
            _ => None,
        })
        .last()
        .expect("failure event");
    assert_eq!(failed.code, ModuleCallErrorCode::CapsDenied);
    assert!(failed.detail.contains("cap_slot not bound"));
}

#[test]
fn module_call_policy_denied_records_failure() {
    let mut world = World::new();
    world.add_capability(CapabilityGrant::allow_all("cap.weather"));
    world.set_policy(PolicySet {
        rules: vec![PolicyRule {
            when: PolicyWhen {
                effect_kind: Some("http.request".to_string()),
                origin_kind: None,
                cap_name: None,
            },
            decision: PolicyDecision::Deny {
                reason: "blocked".to_string(),
            },
        }],
    });

    let wasm_bytes = b"module-weather-deny";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .unwrap();

    let module_manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: vec!["cap.weather".to_string()],
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024,
            max_gas: 10_000,
            max_call_rate: 1,
            max_output_bytes: 1024,
            max_effects: 2,
            max_emits: 2,
        },
    };

    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world.propose_manifest_update(manifest, "alice").unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();

    let output = ModuleOutput {
        new_state: None,
        effects: vec![ModuleEffectIntent {
            kind: "http.request".to_string(),
            params: json!({"url": "https://example.com"}),
            cap_ref: "cap.weather".to_string(),
            cap_slot: None,
        }],
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 64,
    };

    let mut sandbox = FixedSandbox::succeed(output);
    let err = world
        .execute_module_call("m.weather", "trace-2", vec![], &mut sandbox)
        .unwrap_err();
    assert!(matches!(err, WorldError::ModuleCallFailed { .. }));
    assert_eq!(world.pending_effects_len(), 0);

    let failed = world
        .journal()
        .events
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::ModuleCallFailed(failure) => Some(failure),
            _ => None,
        })
        .last()
        .unwrap();
    assert_eq!(failed.code, ModuleCallErrorCode::PolicyDenied);
}

struct PurePolicyHookSandbox;

impl ModuleSandbox for PurePolicyHookSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        match request.module_id.as_str() {
            "m.weather" => Ok(ModuleOutput {
                new_state: None,
                effects: vec![ModuleEffectIntent {
                    kind: "http.request".to_string(),
                    params: json!({"url": "https://example.com"}),
                    cap_ref: "cap.weather".to_string(),
                    cap_slot: None,
                }],
                emits: Vec::new(),
                tick_lifecycle: None,
                output_bytes: 64,
            }),
            "m.policy.allow" => Ok(ModuleOutput {
                new_state: None,
                effects: Vec::new(),
                emits: vec![ModuleEmit {
                    kind: "policy.allow".to_string(),
                    payload: json!({}),
                }],
                tick_lifecycle: None,
                output_bytes: 32,
            }),
            "m.policy.deny" => Ok(ModuleOutput {
                new_state: None,
                effects: Vec::new(),
                emits: vec![ModuleEmit {
                    kind: "policy.deny".to_string(),
                    payload: json!({"reason": "blocked_by_pure_policy"}),
                }],
                tick_lifecycle: None,
                output_bytes: 32,
            }),
            other => Err(ModuleCallFailure {
                module_id: request.module_id.clone(),
                trace_id: request.trace_id.clone(),
                code: ModuleCallErrorCode::Trap,
                detail: format!("unexpected module call {other}"),
            }),
        }
    }
}

fn activate_module_manifest(world: &mut World, manifest: ModuleManifest) {
    let changes = ModuleChangeSet {
        register: vec![manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: manifest.module_id.clone(),
            version: manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest_update = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world
        .propose_manifest_update(manifest_update, "alice")
        .unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();
}
