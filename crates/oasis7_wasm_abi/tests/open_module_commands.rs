//! RED contract for the governed open-module command ABI.
//!
//! These tests deliberately describe the next versioned command surface.  The
//! current `wasm-1` ABI has no command declarations or command envelope yet;
//! the implementation slice must make these tests pass without weakening the
//! legacy manifest decoding assertions.

use oasis7_wasm_abi::{
    validate_module_command_declarations, validate_module_command_envelope, ModuleAbiContract,
    ModuleCommandDeclaration, ModuleCommandEnvelope, ModuleKind, ModuleLimits, ModuleManifest,
    ModuleRole, ModuleSchemaDeclarations,
};

const VALID_SCHEMA_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn declaration(namespace: &str, name: &str) -> ModuleCommandDeclaration {
    ModuleCommandDeclaration {
        namespace: namespace.to_string(),
        name: name.to_string(),
        schema_version: 1,
        schema_hash: VALID_SCHEMA_HASH.to_string(),
        max_payload_bytes: 4096,
    }
}

fn manifest_with_commands(commands: Vec<ModuleCommandDeclaration>) -> ModuleManifest {
    ModuleManifest {
        module_id: "module.bank".to_string(),
        name: "Bank commands".to_string(),
        version: "1.0.0".to_string(),
        kind: ModuleKind::Pure,
        role: ModuleRole::Domain,
        wasm_hash: VALID_SCHEMA_HASH.to_string(),
        interface_version: "wasm-1".to_string(),
        exports: vec!["call".to_string()],
        subscriptions: Vec::new(),
        required_caps: Vec::new(),
        abi_contract: ModuleAbiContract {
            declarations: ModuleSchemaDeclarations {
                commands,
                ..ModuleSchemaDeclarations::default()
            },
            ..ModuleAbiContract::default()
        },
        artifact_identity: None,
        limits: ModuleLimits::default(),
    }
}

#[test]
fn legacy_manifest_deserializes_with_empty_command_declarations() {
    let legacy = serde_json::json!({
        "module_id": "legacy.module",
        "name": "Legacy",
        "version": "0.1.0",
        "kind": "pure",
        "wasm_hash": VALID_SCHEMA_HASH,
        "interface_version": "wasm-1",
        "abi_contract": {},
        "limits": {}
    });

    let manifest: ModuleManifest = serde_json::from_value(legacy).expect("legacy manifest");
    assert!(manifest.abi_contract.declarations.commands.is_empty());
}

#[test]
fn command_declaration_validation_accepts_versioned_schema_and_bound() {
    let declarations = ModuleSchemaDeclarations {
        commands: vec![declaration("bank.acme", "open_account")],
        ..ModuleSchemaDeclarations::default()
    };

    assert!(validate_module_command_declarations(&declarations).is_ok());
}

#[test]
fn command_declaration_validation_rejects_reserved_duplicate_zero_and_malformed_entries() {
    for invalid in [
        {
            let value = declaration("core", "open_account");
            value
        },
        {
            let value = declaration("kernel", "open_account");
            value
        },
        {
            let mut value = declaration("bank.acme", "open_account");
            value.schema_version = 0;
            value
        },
        {
            let mut value = declaration("bank.acme", "open_account");
            value.schema_hash = "not-a-sha256".to_string();
            value
        },
        {
            let mut value = declaration("bank.acme", "open_account");
            value.max_payload_bytes = 0;
            value
        },
        {
            let mut value = declaration("bank.acme", "open_account");
            value.max_payload_bytes = u64::MAX;
            value
        },
    ] {
        assert!(
            validate_module_command_declarations(&ModuleSchemaDeclarations {
                commands: vec![invalid],
                ..ModuleSchemaDeclarations::default()
            })
            .is_err()
        );
    }

    let duplicate = ModuleSchemaDeclarations {
        commands: vec![
            declaration("bank.acme", "open_account"),
            declaration("bank.acme", "open_account"),
        ],
        ..ModuleSchemaDeclarations::default()
    };
    assert!(validate_module_command_declarations(&duplicate).is_err());
}

#[test]
fn command_envelope_roundtrips_canonically_and_rejects_before_execution() {
    let manifest = manifest_with_commands(vec![declaration("bank.acme", "open_account")]);
    let envelope = ModuleCommandEnvelope {
        namespace: "bank.acme".to_string(),
        name: "open_account".to_string(),
        schema_version: 1,
        schema_hash: VALID_SCHEMA_HASH.to_string(),
        payload: vec![0xa1, 0x61, 0x69, 0x01],
    };

    let encoded = envelope.encode_canonical().expect("canonical envelope");
    assert_eq!(
        encoded,
        envelope.encode_canonical().expect("deterministic encoding")
    );
    assert_eq!(
        ModuleCommandEnvelope::decode_canonical(&encoded).unwrap(),
        envelope
    );
    assert!(
        validate_module_command_envelope(&envelope, &manifest.abi_contract.declarations).is_ok()
    );

    for invalid in [
        ModuleCommandEnvelope {
            namespace: "unknown.acme".to_string(),
            ..envelope.clone()
        },
        ModuleCommandEnvelope {
            name: "close_account".to_string(),
            ..envelope.clone()
        },
        ModuleCommandEnvelope {
            schema_version: 2,
            ..envelope.clone()
        },
        ModuleCommandEnvelope {
            schema_hash: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_string(),
            ..envelope.clone()
        },
        ModuleCommandEnvelope {
            namespace: "core".to_string(),
            ..envelope.clone()
        },
        ModuleCommandEnvelope {
            payload: vec![0; 4097],
            ..envelope.clone()
        },
    ] {
        assert!(
            validate_module_command_envelope(&invalid, &manifest.abi_contract.declarations).is_err()
        );
    }
}
