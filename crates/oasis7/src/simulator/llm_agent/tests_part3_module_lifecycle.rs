use crate::simulator::ModuleInstallTarget;

#[test]
fn restored_llm_parse_compile_module_artifact_from_source_action() {
    let turns = completion_turns_from_output(
        r#"{"decision":"compile_module_artifact_from_source","publisher":"self","module_id":"m.llm.compile","manifest_path":"Cargo.toml","source_files":{"Cargo.toml":"cargo-content","src/lib.rs":"lib-content"}}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision:
                AgentDecision::Act(Action::CompileModuleArtifactFromSource {
                    publisher_agent_id,
                    module_id,
                    manifest_path,
                    source_files,
                }),
            ..
        } => {
            assert_eq!(publisher_agent_id, "agent-1");
            assert_eq!(module_id, "m.llm.compile");
            assert_eq!(manifest_path, "Cargo.toml");
            assert!(source_files.contains_key("Cargo.toml"));
            assert!(source_files.contains_key("src/lib.rs"));
            assert_eq!(
                source_files
                    .get("src/lib.rs")
                    .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                    .as_deref(),
                Some("lib-content")
            );
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}

#[test]
fn restored_llm_parse_deploy_module_artifact_rejects_invalid_hex_bytes() {
    let turns = completion_turns_from_output(
        r#"{"decision":"deploy_module_artifact","publisher":"self","wasm_hash":"abc","wasm_bytes_hex":"not-hex"}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Invalid(message) => {
            assert!(
                message.contains("wasm_bytes_hex") && message.contains("valid hex"),
                "unexpected parse error: {message}"
            );
        }
        other => panic!("expected invalid decision, got {other:?}"),
    }
}

#[test]
fn restored_llm_parse_install_module_from_artifact_defaults_version_and_activate() {
    let turns = completion_turns_from_output(
        r#"{"decision":"install_module_from_artifact","installer":"self","module_id":"m.llm.install","wasm_hash":"abcd"}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision:
                AgentDecision::Act(Action::InstallModuleFromArtifact {
                    installer_agent_id,
                    module_id,
                    module_version,
                    wasm_hash,
                    activate,
                }),
            ..
        } => {
            assert_eq!(installer_agent_id, "agent-1");
            assert_eq!(module_id, "m.llm.install");
            assert_eq!(module_version, "0.1.0");
            assert_eq!(wasm_hash, "abcd");
            assert!(*activate);
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}

#[test]
fn restored_llm_parse_install_module_to_target_from_artifact_action() {
    let turns = completion_turns_from_output(
        r#"{"decision":"install_module_to_target_from_artifact","installer":"self","module_id":"m.llm.install.target","module_version":"0.2.0","wasm_hash":"hash-target","activate":false,"install_target_type":"location_infrastructure","install_target_location_id":"loc-hub"}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision:
                AgentDecision::Act(Action::InstallModuleToTargetFromArtifact {
                    installer_agent_id,
                    module_id,
                    module_version,
                    wasm_hash,
                    activate,
                    install_target,
                }),
            ..
        } => {
            assert_eq!(installer_agent_id, "agent-1");
            assert_eq!(module_id, "m.llm.install.target");
            assert_eq!(module_version, "0.2.0");
            assert_eq!(wasm_hash, "hash-target");
            assert!(!*activate);
            assert_eq!(
                install_target,
                &ModuleInstallTarget::LocationInfrastructure {
                    location_id: "loc-hub".to_string(),
                }
            );
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}
