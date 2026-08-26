use oasis7_wasm_abi::{
    AgentCommandResponse, CapabilityAudience, CapabilityCatalogEntry, CapabilityGrantV2,
    CapabilityIssuer, CapabilityPresenter, CapabilityScope, CapabilitySubject,
    canonical_sha256_hex, capability_grant_body_hash, capability_request_hash,
    capability_scope_hash,
};

const SCHEMA_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn subject() -> CapabilitySubject {
    CapabilitySubject::Agent {
        agent_id: "agent-7".to_string(),
        owner_binding: "owner-7".to_string(),
        generation: 1,
    }
}

fn presenter() -> CapabilityPresenter {
    CapabilityPresenter {
        presenter_id: "provider-1".to_string(),
        presenter_kind: "provider".to_string(),
        session_id: Some("session-1".to_string()),
        attestation_ref: None,
    }
}

fn audience() -> CapabilityAudience {
    CapabilityAudience {
        world_id: "world.test".to_string(),
        branch_id: "branch-1".to_string(),
        finality_epoch: 4,
        target_kind: "world".to_string(),
        target_id: None,
    }
}

fn scope() -> CapabilityScope {
    CapabilityScope {
        module_id: "module.weather".to_string(),
        module_version: "1.0.0".to_string(),
        namespace: "weather".to_string(),
        object_kind: "command".to_string(),
        object_name: "observe".to_string(),
        operation: "execute".to_string(),
        entity_selector: Some(vec!["station-1".to_string()]),
        resource_selector: Some(vec!["weather.read".to_string()]),
        max_payload_bytes: Some(128),
        policy_class: Some("read-only".to_string()),
    }
}

fn grant() -> CapabilityGrantV2 {
    CapabilityGrantV2 {
        grant_id: "grant-weather-1".to_string(),
        grant_version: 2,
        subject: subject(),
        audience: audience(),
        issuer: CapabilityIssuer {
            issuer_id: "governance-1".to_string(),
            issuer_kind: "governance".to_string(),
            governance_epoch: 9,
            finalized_receipt_id: "finality-9".to_string(),
            key_id: "governance-key-1".to_string(),
            issuer_key_epoch: 3,
            authority_rotation_receipt_id: None,
            signature: "ed25519:opaque".to_string(),
        },
        scope: scope(),
        issued_at_tick: 1,
        expires_at_tick: Some(100),
        grant_nonce: "grant-nonce-1".to_string(),
        parent_grant_id: None,
        delegation_depth: 0,
        revocation_epoch: 2,
        status: "verified".to_string(),
        canonical_body_hash: "body-hash".to_string(),
        issuance_signature: "ed25519:opaque".to_string(),
    }
}

#[test]
fn frozen_fixture_wire_round_trips_without_provider_authority() {
    let value = serde_json::json!({
        "grant_id": "grant-weather-1",
        "grant_version": 2,
        "subject": {"kind":"agent","agent_id":"agent-7","owner_binding":"owner-7","generation":1},
        "audience": {"world_id":"world.test","branch_id":"branch-1","finality_epoch":4,"target_kind":"world","target_id":null},
        "issuer": {"issuer_id":"governance-1","issuer_kind":"governance","governance_epoch":9,"finalized_receipt_id":"finality-9","key_id":"governance-key-1","issuer_key_epoch":3,"authority_rotation_receipt_id":null,"signature":"ed25519:opaque"},
        "scope": {"module_id":"module.weather","module_version":"1.0.0","namespace":"weather","object_kind":"command","object_name":"observe","operation":"execute","entity_selector":["station-1"],"resource_selector":["weather.read"],"max_payload_bytes":128,"policy_class":"read-only"},
        "issued_at_tick":1,"expires_at_tick":100,"grant_nonce":"grant-nonce-1","parent_grant_id":null,"delegation_depth":0,"revocation_epoch":2,"status":"verified","canonical_body_hash":"body-hash","issuance_signature":"ed25519:opaque"
    });
    let decoded: CapabilityGrantV2 = serde_json::from_value(value).expect("v2 fixture wire");
    decoded.validate().expect("structural validation");
    assert_eq!(decoded.issuer.signature, "ed25519:opaque");
    assert_ne!(
        decoded.canonical_body_hash().unwrap(),
        decoded.canonical_body_hash
    );
}

#[test]
fn scope_matching_is_conjunctive_and_subset_only() {
    let parent = scope();
    let mut child = parent.clone();
    child.max_payload_bytes = Some(64);
    assert!(child.is_subset_of(&parent));
    assert!(!parent.matches_exact(&child));
    assert!(parent.allows(&child));

    child.operation = "write".to_string();
    assert!(!child.is_subset_of(&parent));
    child.operation = "execute".to_string();
    child.entity_selector = Some(vec!["*".to_string()]);
    assert!(!child.is_subset_of(&parent));

    let mut omitted = parent.clone();
    omitted.entity_selector = None;
    assert!(!omitted.matches_exact(&parent));
}

#[test]
fn canonical_hashes_are_stable_and_use_sha256() {
    assert_eq!(
        canonical_sha256_hex(b"abc").unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let first = capability_grant_body_hash(&grant()).unwrap();
    assert_eq!(first, capability_grant_body_hash(&grant()).unwrap());
    assert_eq!(first, grant().canonical_body_hash().unwrap());
    assert_eq!(
        capability_scope_hash(&scope()).unwrap(),
        capability_scope_hash(&scope()).unwrap()
    );
}

#[test]
fn response_wire_and_request_hash_bind_exact_selection() {
    let response = AgentCommandResponse {
        response_nonce: "response-1".to_string(),
        subject: subject(),
        presenter: presenter(),
        audience: audience(),
        catalog_snapshot_id: "catalog-weather-1".to_string(),
        selected_entry: CapabilityCatalogEntry {
            module_id: "module.weather".to_string(),
            module_version: "1.0.0".to_string(),
            namespace: "weather".to_string(),
            command: "observe".to_string(),
            schema_version: 1,
            schema_hash: SCHEMA_HASH.to_string(),
            max_payload_bytes: 128,
            eligible_grant_ids: Vec::new(),
        },
        envelope: oasis7_wasm_abi::ModuleCommandEnvelope {
            namespace: "weather".to_string(),
            name: "observe".to_string(),
            schema_version: 1,
            schema_hash: SCHEMA_HASH.to_string(),
            payload: vec![123, 125],
        },
        provider_id: Some("provider-1".to_string()),
        trace_id: Some("trace-1".to_string()),
    };
    response.validate().expect("response structure");
    let first = capability_request_hash(&response).unwrap();
    assert_eq!(first, capability_request_hash(&response).unwrap());
    assert_eq!(
        first,
        oasis7_wasm_abi::canonical_request_hash(&response).unwrap()
    );
    let mut altered = response.clone();
    altered.response_nonce = "response-2".to_string();
    assert_ne!(first, capability_request_hash(&altered).unwrap());
}
