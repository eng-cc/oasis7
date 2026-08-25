//! Serialization and compatibility tests for host-injected module provenance.

use oasis7_wasm_abi::{
    ModuleCallCaller, ModuleCallOrigin, ModuleCommandEnvelope, ModuleContext,
    ModuleInvocationProvenance, ModuleLimits,
};

fn context_json_without_caller() -> serde_json::Value {
    serde_json::json!({
        "v": "wasm-1",
        "module_id": "bank.acme",
        "trace_id": "trace-legacy",
        "time": 7,
        "origin": {"kind": "event", "id": "event-7"},
        "limits": {},
    })
}

#[test]
fn caller_variants_roundtrip_through_json_and_cbor() {
    let callers = [
        ModuleCallCaller::LegacyUnspecified,
        ModuleCallCaller::System {
            system_id: "runtime".to_string(),
        },
        ModuleCallCaller::Agent {
            agent_id: "agent-7".to_string(),
        },
        ModuleCallCaller::Module {
            module_id: "bank.acme".to_string(),
        },
    ];

    for caller in callers {
        let json = serde_json::to_vec(&caller).expect("serialize caller as JSON");
        let json_roundtrip: ModuleCallCaller =
            serde_json::from_slice(&json).expect("deserialize caller from JSON");
        assert_eq!(json_roundtrip, caller);

        let cbor = serde_cbor::to_vec(&caller).expect("serialize caller as CBOR");
        let cbor_roundtrip: ModuleCallCaller =
            serde_cbor::from_slice(&cbor).expect("deserialize caller from CBOR");
        assert_eq!(cbor_roundtrip, caller);
    }
}

#[test]
fn missing_context_caller_defaults_to_legacy_unspecified() {
    let context: ModuleContext =
        serde_json::from_value(context_json_without_caller()).expect("legacy context");
    assert_eq!(context.caller, ModuleCallCaller::LegacyUnspecified);
}

#[test]
fn provenance_roundtrips_separately_from_command_envelope() {
    let provenance = ModuleInvocationProvenance {
        caller: ModuleCallCaller::Agent {
            agent_id: "agent-7".to_string(),
        },
        origin: ModuleCallOrigin {
            kind: "agent_decision".to_string(),
            id: "decision-7".to_string(),
        },
    };
    let encoded = serde_cbor::to_vec(&provenance).expect("serialize provenance");
    let decoded: ModuleInvocationProvenance =
        serde_cbor::from_slice(&encoded).expect("deserialize provenance");
    assert_eq!(decoded, provenance);

    let envelope = ModuleCommandEnvelope {
        namespace: "bank.acme".to_string(),
        name: "open_account".to_string(),
        schema_version: 1,
        schema_hash: "00".repeat(32),
        payload: vec![1],
    };
    let envelope_json = serde_json::to_value(envelope).expect("serialize envelope");
    assert!(envelope_json.get("caller").is_none());
    assert!(envelope_json.get("origin").is_none());
}

#[test]
fn context_serializes_injected_caller_without_changing_origin() {
    let context = ModuleContext {
        v: "wasm-1".to_string(),
        module_id: "bank.acme".to_string(),
        trace_id: "trace-7".to_string(),
        time: 7,
        origin: ModuleCallOrigin {
            kind: "agent_decision".to_string(),
            id: "decision-7".to_string(),
        },
        caller: ModuleCallCaller::Agent {
            agent_id: "agent-7".to_string(),
        },
        limits: ModuleLimits::default(),
        stage: None,
        world_config_hash: None,
        manifest_hash: None,
        journal_height: None,
        module_version: None,
        module_kind: None,
        module_role: None,
    };

    let value = serde_json::to_value(&context).expect("serialize context");
    assert_eq!(value["caller"]["kind"], "agent");
    assert_eq!(value["caller"]["agent_id"], "agent-7");
    assert_eq!(value["origin"]["kind"], "agent_decision");
    assert_eq!(value["origin"]["id"], "decision-7");
}
