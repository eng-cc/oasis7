use super::super::*;
use super::pos;
use crate::simulator::{ModuleInstallTarget, ResourceKind};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const SOURCE_COMPILER_ENV: &str = "OASIS7_MODULE_SOURCE_COMPILER";
const SOURCE_MAX_FILES_ENV: &str = "OASIS7_MODULE_SOURCE_MAX_FILES";
const SOURCE_COMPILE_TIMEOUT_MS_ENV: &str = "OASIS7_MODULE_SOURCE_COMPILE_TIMEOUT_MS";
const SOURCE_SANDBOX_SECRET_ENV: &str = "OASIS7_SOURCE_SANDBOX_TEST_SECRET";
static SOURCE_COMPILER_ENV_LOCK: Mutex<()> = Mutex::new(());
const TEST_FINALITY_SIGNER_NODE_1: &str = "governance.local.finality.signer.1";
const TEST_FINALITY_SIGNER_SEED_1: &str = "oasis7-governance-local-finality-signer-1-v1";
const TEST_FINALITY_SIGNER_NODE_2: &str = "governance.local.finality.signer.2";
const TEST_FINALITY_SIGNER_SEED_2: &str = "oasis7-governance-local-finality-signer-2-v1";

#[path = "module_action_loop_market_tests.rs"]
mod market_tests;

fn removed_old_brand_module_source_env(suffix: &str) -> String {
    ["AGENT", "WORLD", "MODULE", "SOURCE", suffix].join("_")
}

fn register_agent(world: &mut World, agent_id: &str) {
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: pos(0.0, 0.0),
    });
    world.step().expect("register agent");
    world
        .set_agent_resource_balance(agent_id, ResourceKind::Electricity, 128)
        .expect("seed electricity");
    world
        .set_agent_resource_balance(agent_id, ResourceKind::Data, 64)
        .expect("seed data");
}

fn set_agent_resource(world: &mut World, agent_id: &str, kind: ResourceKind, amount: i64) {
    world
        .set_agent_resource_balance(agent_id, kind, amount)
        .expect("set agent resource balance");
}

fn base_manifest(module_id: &str, version: &str, wasm_hash: &str) -> ModuleManifest {
    ModuleManifest {
        module_id: module_id.to_string(),
        name: format!("module-{module_id}"),
        version: version.to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.to_string(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: Vec::new(),
        required_caps: Vec::new(),
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash)),
        limits: ModuleLimits::default(),
    }
}

fn set_test_governance_finality_epoch_snapshot(
    world: &mut World,
    threshold: u16,
    signer_node_ids: &[&str],
) {
    let epoch_len = world
        .governance_execution_policy()
        .epoch_length_ticks
        .max(1);
    let epoch_id = world.state().time / epoch_len;
    world
        .set_governance_finality_epoch_snapshot(GovernanceFinalityEpochSnapshot {
            epoch_id,
            threshold,
            signer_node_ids: signer_node_ids
                .iter()
                .map(|signer| signer.to_string())
                .collect(),
            ..GovernanceFinalityEpochSnapshot::default()
        })
        .expect("set test finality epoch snapshot");
}

fn test_finality_signing_key(seed_label: &str) -> SigningKey {
    let seed = util::sha256_hex(seed_label.as_bytes());
    let seed_bytes = hex::decode(seed).expect("decode test finality seed");
    let private_key_bytes: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .expect("test finality seed is 32 bytes");
    SigningKey::from_bytes(&private_key_bytes)
}

fn test_finality_seed_label(node_id: &str) -> &'static str {
    match node_id {
        TEST_FINALITY_SIGNER_NODE_1 => TEST_FINALITY_SIGNER_SEED_1,
        TEST_FINALITY_SIGNER_NODE_2 => TEST_FINALITY_SIGNER_SEED_2,
        _ => panic!("missing test finality seed for signer {node_id}"),
    }
}

fn extract_latest_module_apply_proposal_id(world: &World) -> ProposalId {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ModuleInstalled { proposal_id, .. })
            | WorldEventBody::Domain(DomainEvent::ModuleUpgraded { proposal_id, .. })
            | WorldEventBody::Domain(DomainEvent::ModuleRollbackApplied { proposal_id, .. }) => {
                Some(*proposal_id)
            }
            _ => None,
        })
        .expect("module apply proposal id")
}

fn extract_queued_manifest_hash(world: &World, proposal_id: ProposalId) -> String {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Governance(GovernanceEvent::Queued {
                proposal_id: event_proposal_id,
                manifest_hash,
                ..
            }) if *event_proposal_id == proposal_id => Some(manifest_hash.clone()),
            _ => None,
        })
        .expect("governance queued manifest hash")
}

fn extract_governance_applied_consensus_height(world: &World, proposal_id: ProposalId) -> u64 {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Governance(GovernanceEvent::Applied {
                proposal_id: event_proposal_id,
                consensus_height: Some(consensus_height),
                ..
            }) if *event_proposal_id == proposal_id => Some(*consensus_height),
            _ => None,
        })
        .expect("governance applied consensus height")
}

fn build_external_finality_certificate(
    world: &World,
    proposal_id: ProposalId,
    manifest_hash: &str,
    consensus_height: u64,
) -> GovernanceFinalityCertificate {
    let epoch_len = world
        .governance_execution_policy()
        .epoch_length_ticks
        .max(1);
    let epoch_id = world.state().time / epoch_len;
    let snapshot = world
        .governance_finality_epoch_snapshots()
        .get(&epoch_id)
        .cloned()
        .expect("governance finality snapshot for current epoch");
    let min_unique_signers = snapshot.effective_min_unique_signers();
    let mut signatures = BTreeMap::new();
    for node_id in &snapshot.signer_node_ids {
        let payload = GovernanceFinalityCertificate::signing_payload_v1(
            proposal_id,
            manifest_hash,
            consensus_height,
            epoch_id,
            snapshot.validator_set_hash.as_str(),
            snapshot.stake_root.as_str(),
            snapshot.threshold_bps,
            min_unique_signers,
            node_id.as_str(),
        );
        let signing_key = test_finality_signing_key(test_finality_seed_label(node_id.as_str()));
        let signature = signing_key.sign(payload.as_slice());
        signatures.insert(
            node_id.clone(),
            format!(
                "{}{}",
                GovernanceFinalityCertificate::SIGNATURE_PREFIX_ED25519_V1,
                hex::encode(signature.to_bytes())
            ),
        );
    }

    GovernanceFinalityCertificate {
        proposal_id,
        manifest_hash: manifest_hash.to_string(),
        consensus_height,
        epoch_id,
        validator_set_hash: snapshot.validator_set_hash,
        stake_root: snapshot.stake_root,
        threshold_bps: snapshot.threshold_bps,
        min_unique_signers,
        threshold: min_unique_signers,
        signatures,
    }
}

fn derive_module_action_finality_certificate(
    world: &World,
    submit_action: impl FnOnce(&mut World),
) -> GovernanceFinalityCertificate {
    let mut simulated = world.clone();
    if !simulated
        .release_security_policy()
        .allow_local_finality_signing
    {
        let mut policy = simulated.release_security_policy().clone();
        policy.allow_local_finality_signing = true;
        simulated.set_release_security_policy(policy);
    }
    submit_action(&mut simulated);
    simulated.step().expect("simulate module action finality");
    let proposal_id = extract_latest_module_apply_proposal_id(&simulated);
    let manifest_hash = extract_queued_manifest_hash(&simulated, proposal_id);
    let consensus_height = extract_governance_applied_consensus_height(&simulated, proposal_id);
    build_external_finality_certificate(world, proposal_id, &manifest_hash, consensus_height)
}

fn assert_last_rejection_note(world: &World, action_id: ActionId, expected: &str) {
    let event = world.journal().events.last().expect("last event");
    let WorldEventBody::Domain(DomainEvent::ActionRejected {
        action_id: rejected_action_id,
        reason: RejectReason::RuleDenied { notes },
    }) = &event.body
    else {
        panic!(
            "expected action rejected rule denied event: {:?}",
            event.body
        );
    };
    assert_eq!(*rejected_action_id, action_id);
    assert!(
        notes.iter().any(|note| note.contains(expected)),
        "missing expected note `{expected}` in {notes:?}"
    );
}

fn sample_module_source_package() -> ModuleSourcePackage {
    ModuleSourcePackage {
        manifest_path: "Cargo.toml".to_string(),
        files: BTreeMap::from([
            (
                "Cargo.toml".to_string(),
                br#"[package]
name = "sample_module"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#
                .to_vec(),
            ),
            (
                "src/lib.rs".to_string(),
                b"#[no_mangle] pub extern \"C\" fn reduce() {}".to_vec(),
            ),
        ]),
    }
}

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration since epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oasis7-runtime-tests-{prefix}-{}-{unique}",
        std::process::id()
    ))
}

fn write_fake_source_compiler(script_path: &Path, produced_wasm_bytes: &str) {
    let script = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nout_path=\"$4\"\nprintf '%s' '{produced_wasm_bytes}' > \"$out_path\"\n"
    );
    write_executable_script(script_path, script.as_str());
}

fn write_executable_script(script_path: &Path, script: &str) {
    fs::write(script_path, script).expect("write fake source compiler");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(script_path, fs::Permissions::from_mode(0o755))
            .expect("chmod fake source compiler");
    }
}

struct EnvVarGuard {
    key: String,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn capture(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            previous: std::env::var(key.as_str()).ok(),
            key,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key.as_str(), value),
            None => std::env::remove_var(self.key.as_str()),
        }
    }
}

#[test]
fn deploy_module_artifact_action_registers_artifact_bytes() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    let wasm_bytes = b"module-action-loop-deploy".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes: wasm_bytes.clone(),
    });
    world.step().expect("deploy artifact");

    let event = world.journal().events.last().expect("last event");
    let WorldEventBody::Domain(DomainEvent::ModuleArtifactDeployed {
        publisher_agent_id,
        wasm_hash: event_hash,
        bytes_len,
        fee_kind,
        fee_amount,
    }) = &event.body
    else {
        panic!("expected module artifact deployed event: {:?}", event.body);
    };
    assert_eq!(publisher_agent_id, "publisher-1");
    assert_eq!(event_hash, &wasm_hash);
    assert_eq!(*bytes_len, wasm_bytes.len() as u64);
    assert_eq!(*fee_kind, ResourceKind::Electricity);
    assert!(*fee_amount > 0);

    let loaded = world.load_module(&wasm_hash).expect("load deployed module");
    assert_eq!(loaded.wasm_hash, wasm_hash);
    assert_eq!(loaded.bytes, wasm_bytes);
}

#[test]
fn deploy_module_artifact_action_rejects_hash_mismatch() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    let action_id = world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: "sha256-mismatch".to_string(),
        wasm_bytes: b"module-action-loop-deploy-mismatch".to_vec(),
    });
    world.step().expect("deploy mismatch action");

    assert_last_rejection_note(&world, action_id, "artifact hash mismatch");
}

#[test]
fn compile_module_artifact_from_source_registers_compiled_artifact() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _env_guard = EnvVarGuard::capture(SOURCE_COMPILER_ENV);
    let removed_old_brand_compiler = removed_old_brand_module_source_env("COMPILER");
    let _removed_old_brand_env_guard =
        EnvVarGuard::capture(removed_old_brand_compiler.as_str());
    let temp_root = temp_dir("compile-module-artifact");
    fs::create_dir_all(&temp_root).expect("create temp dir");
    let compiler_script = temp_root.join("compiler.sh");
    let produced_wasm_bytes = "compiled-from-source-runtime";
    write_fake_source_compiler(compiler_script.as_path(), produced_wasm_bytes);
    std::env::set_var(SOURCE_COMPILER_ENV, compiler_script.as_os_str());
    std::env::remove_var(removed_old_brand_compiler.as_str());

    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.compile".to_string(),
        source_package: sample_module_source_package(),
    });
    world.step().expect("compile from source action");

    let wasm_bytes = produced_wasm_bytes.as_bytes().to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ModuleArtifactDeployed {
            publisher_agent_id,
            wasm_hash: event_hash,
            bytes_len,
            ..
        })) if publisher_agent_id == "publisher-1" && event_hash == &wasm_hash && *bytes_len == wasm_bytes.len() as u64
    ));
    assert_eq!(
        world.state().module_artifact_owners.get(&wasm_hash),
        Some(&"publisher-1".to_string())
    );
    assert_eq!(
        world
            .load_module(&wasm_hash)
            .expect("load compiled module")
            .bytes,
        wasm_bytes
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_module_artifact_from_source_rejects_removed_old_brand_compiler_env() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _env_guard = EnvVarGuard::capture(SOURCE_COMPILER_ENV);
    let removed_old_brand_compiler = removed_old_brand_module_source_env("COMPILER");
    let _removed_old_brand_env_guard =
        EnvVarGuard::capture(removed_old_brand_compiler.as_str());

    let temp_root = temp_dir("compile-module-artifact-prefers-oasis7-env");
    fs::create_dir_all(&temp_root).expect("create temp dir");
    let primary_script = temp_root.join("compiler-primary.sh");
    let removed_old_brand_script = temp_root.join("compiler-removed-old-brand.sh");
    write_fake_source_compiler(primary_script.as_path(), "compiled-from-oasis7-env");
    write_fake_source_compiler(
        removed_old_brand_script.as_path(),
        "compiled-from-removed-old-brand-env",
    );
    std::env::set_var(SOURCE_COMPILER_ENV, primary_script.as_os_str());
    std::env::set_var(removed_old_brand_compiler.as_str(), removed_old_brand_script.as_os_str());

    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.compile.prefers-oasis7".to_string(),
        source_package: sample_module_source_package(),
    });
    world.step().expect("compile from source action");

    let wasm_bytes = b"compiled-from-oasis7-env".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ModuleArtifactDeployed {
            publisher_agent_id,
            wasm_hash: event_hash,
            bytes_len,
            ..
        })) if publisher_agent_id == "publisher-1" && event_hash == &wasm_hash && *bytes_len == wasm_bytes.len() as u64
    ));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_module_artifact_from_source_rejects_in_production_release_policy() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _env_guard = EnvVarGuard::capture(SOURCE_COMPILER_ENV);
    let removed_old_brand_compiler = removed_old_brand_module_source_env("COMPILER");
    let _removed_old_brand_env_guard =
        EnvVarGuard::capture(removed_old_brand_compiler.as_str());

    let temp_root = temp_dir("compile-module-artifact-production-disabled");
    fs::create_dir_all(&temp_root).expect("create temp dir");
    let compiler_script = temp_root.join("compiler.sh");
    write_fake_source_compiler(compiler_script.as_path(), "compiled-in-production");
    std::env::set_var(SOURCE_COMPILER_ENV, compiler_script.as_os_str());
    std::env::remove_var(removed_old_brand_compiler.as_str());

    let mut world = World::new();
    world.enable_production_release_policy();
    register_agent(&mut world, "publisher-1");

    let action_id = world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.production-disabled".to_string(),
        source_package: sample_module_source_package(),
    });
    world
        .step()
        .expect("compile source rejected by production policy");

    assert_last_rejection_note(
        &world,
        action_id,
        "runtime source compile is disabled by production release policy",
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_module_artifact_from_source_rejects_when_manifest_path_missing_in_files() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    let action_id = world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.invalid".to_string(),
        source_package: ModuleSourcePackage {
            manifest_path: "Cargo.toml".to_string(),
            files: BTreeMap::from([(
                "src/lib.rs".to_string(),
                b"#[no_mangle] pub extern \"C\" fn reduce() {}".to_vec(),
            )]),
        },
    });
    world.step().expect("compile invalid source action");

    assert_last_rejection_note(&world, action_id, "manifest path missing");
}

#[test]
fn compile_module_artifact_from_source_rejects_when_file_count_exceeds_limit() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _env_guard = EnvVarGuard::capture(SOURCE_MAX_FILES_ENV);
    let removed_old_brand_max_files = removed_old_brand_module_source_env("MAX_FILES");
    let _removed_old_brand_env_guard =
        EnvVarGuard::capture(removed_old_brand_max_files.as_str());
    std::env::set_var(SOURCE_MAX_FILES_ENV, "1");
    std::env::remove_var(removed_old_brand_max_files.as_str());

    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    let action_id = world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.too-many-files".to_string(),
        source_package: sample_module_source_package(),
    });
    world.step().expect("compile source with too many files");

    assert_last_rejection_note(&world, action_id, "source file count exceeds limit");
}

#[test]
fn compile_module_artifact_from_source_rejects_when_compiler_times_out() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _compiler_guard = EnvVarGuard::capture(SOURCE_COMPILER_ENV);
    let removed_old_brand_compiler = removed_old_brand_module_source_env("COMPILER");
    let _removed_old_brand_compiler_guard =
        EnvVarGuard::capture(removed_old_brand_compiler.as_str());
    let _timeout_guard = EnvVarGuard::capture(SOURCE_COMPILE_TIMEOUT_MS_ENV);

    let temp_root = temp_dir("compile-module-artifact-timeout");
    fs::create_dir_all(&temp_root).expect("create temp dir");
    let compiler_script = temp_root.join("compiler-timeout.sh");
    write_executable_script(
        compiler_script.as_path(),
        "#!/usr/bin/env bash\nset -euo pipefail\nsleep 1\nprintf '%s' 'late-module' > \"$4\"\n",
    );
    std::env::set_var(SOURCE_COMPILER_ENV, compiler_script.as_os_str());
    std::env::remove_var(removed_old_brand_compiler.as_str());
    std::env::set_var(SOURCE_COMPILE_TIMEOUT_MS_ENV, "20");

    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    let action_id = world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.timeout".to_string(),
        source_package: sample_module_source_package(),
    });
    world.step().expect("compile timeout action");

    assert_last_rejection_note(&world, action_id, "compiler timed out");
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_module_artifact_from_source_sanitizes_env_and_isolates_tmpdir() {
    let _env_lock = SOURCE_COMPILER_ENV_LOCK.lock().expect("lock compile env");
    let _compiler_guard = EnvVarGuard::capture(SOURCE_COMPILER_ENV);
    let removed_old_brand_compiler = removed_old_brand_module_source_env("COMPILER");
    let _removed_old_brand_compiler_guard =
        EnvVarGuard::capture(removed_old_brand_compiler.as_str());
    let _secret_guard = EnvVarGuard::capture(SOURCE_SANDBOX_SECRET_ENV);

    let temp_root = temp_dir("compile-module-artifact-sandbox-env");
    fs::create_dir_all(&temp_root).expect("create temp dir");
    let compiler_script = temp_root.join("compiler-sandbox-env.sh");
    write_executable_script(
        compiler_script.as_path(),
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nworkspace=\"$2\"\nif [[ -n \"${{{SOURCE_SANDBOX_SECRET_ENV}:-}}\" ]]; then\n  exit 17\nfi\nif [[ \"${{TMPDIR:-}}\" != \"$workspace/tmp\" ]]; then\n  exit 19\nfi\nprintf '%s' 'compiled-sandbox-env' > \"$4\"\n"
        )
        .as_str(),
    );
    std::env::set_var(SOURCE_COMPILER_ENV, compiler_script.as_os_str());
    std::env::remove_var(removed_old_brand_compiler.as_str());
    std::env::set_var(SOURCE_SANDBOX_SECRET_ENV, "must-not-leak");

    let mut world = World::new();
    register_agent(&mut world, "publisher-1");

    world.submit_action(Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "publisher-1".to_string(),
        module_id: "m.loop.source.sandbox-env".to_string(),
        source_package: sample_module_source_package(),
    });
    world.step().expect("compile source with sandbox env");

    let wasm_bytes = b"compiled-sandbox-env".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ModuleArtifactDeployed {
            publisher_agent_id,
            wasm_hash: event_hash,
            ..
        })) if publisher_agent_id == "publisher-1" && event_hash == &wasm_hash
    ));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn install_module_from_artifact_action_runs_governance_closure() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let wasm_bytes = b"module-action-loop-install".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy artifact");

    let manifest = base_manifest("m.loop.active", "0.1.0", &wasm_hash);
    world.submit_action(Action::InstallModuleFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: manifest.clone(),
        activate: true,
    });
    world.step().expect("install module action");

    let event = world.journal().events.last().expect("last event");
    let WorldEventBody::Domain(DomainEvent::ModuleInstalled {
        installer_agent_id,
        instance_id,
        module_id,
        module_version,
        wasm_hash: event_wasm_hash,
        active,
        install_target,
        proposal_id,
        manifest_hash,
        fee_kind,
        fee_amount,
    }) = &event.body
    else {
        panic!("expected module installed event: {:?}", event.body);
    };
    assert_eq!(installer_agent_id, "installer-1");
    assert!(!instance_id.is_empty());
    assert_eq!(module_id, "m.loop.active");
    assert_eq!(module_version, "0.1.0");
    assert_eq!(event_wasm_hash, &wasm_hash);
    assert!(*active);
    assert_eq!(*install_target, ModuleInstallTarget::SelfAgent);
    assert!(!manifest_hash.is_empty());
    assert_eq!(*fee_kind, ResourceKind::Electricity);
    assert!(*fee_amount > 0);

    let key = ModuleRegistry::record_key(&manifest.module_id, &manifest.version);
    assert!(world.module_registry().records.contains_key(&key));
    assert_eq!(
        world.module_registry().active.get(&manifest.module_id),
        Some(&manifest.version)
    );
    assert!(matches!(
        world
            .proposals()
            .get(proposal_id)
            .map(|proposal| &proposal.status),
        Some(ProposalStatus::Applied { .. })
    ));
}

#[test]
fn install_module_to_target_from_artifact_action_emits_target() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let wasm_bytes = b"module-action-loop-install-target".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy artifact");

    let manifest = base_manifest("m.loop.targeted", "0.1.0", &wasm_hash);
    world.submit_action(Action::InstallModuleToTargetFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: manifest.clone(),
        activate: true,
        install_target: ModuleInstallTarget::LocationInfrastructure {
            location_id: "loc-edge".to_string(),
        },
    });
    world.step().expect("install module to target action");

    let event = world.journal().events.last().expect("last event");
    let WorldEventBody::Domain(DomainEvent::ModuleInstalled {
        module_id,
        install_target,
        ..
    }) = &event.body
    else {
        panic!("expected module installed event: {:?}", event.body);
    };
    assert_eq!(module_id, "m.loop.targeted");
    assert_eq!(
        *install_target,
        ModuleInstallTarget::LocationInfrastructure {
            location_id: "loc-edge".to_string(),
        }
    );
}

#[test]
fn install_module_from_artifact_action_without_activate_keeps_module_inactive() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let wasm_bytes = b"module-action-loop-install-inactive".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy artifact");

    let manifest = base_manifest("m.loop.inactive", "0.1.0", &wasm_hash);
    world.submit_action(Action::InstallModuleFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: manifest.clone(),
        activate: false,
    });
    world.step().expect("install module inactive action");

    let event = world.journal().events.last().expect("last event");
    let WorldEventBody::Domain(DomainEvent::ModuleInstalled {
        active, module_id, ..
    }) = &event.body
    else {
        panic!("expected module installed event: {:?}", event.body);
    };
    assert!(!*active);
    assert_eq!(module_id, "m.loop.inactive");

    let key = ModuleRegistry::record_key(&manifest.module_id, &manifest.version);
    assert!(world.module_registry().records.contains_key(&key));
    assert!(!world
        .module_registry()
        .active
        .contains_key(&manifest.module_id));
}

#[test]
fn install_module_from_artifact_action_rejects_missing_artifact() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let action_id = world.submit_action(Action::InstallModuleFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: base_manifest("m.loop.missing", "0.1.0", "sha256-missing"),
        activate: true,
    });
    world.step().expect("install missing artifact action");

    assert_last_rejection_note(&world, action_id, "module artifact missing");
    assert!(world.module_registry().records.is_empty());
    assert!(world.module_registry().active.is_empty());
}

#[test]
fn upgrade_module_from_artifact_action_updates_instance_and_emits_audit_event() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let wasm_v1_bytes = b"module-action-loop-upgrade-v1".to_vec();
    let wasm_v1_hash = util::sha256_hex(&wasm_v1_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_v1_hash.clone(),
        wasm_bytes: wasm_v1_bytes,
    });
    world.step().expect("deploy v1 artifact");

    let manifest_v1 = base_manifest("m.loop.upgrade", "0.1.0", &wasm_v1_hash);
    world.submit_action(Action::InstallModuleFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: manifest_v1.clone(),
        activate: true,
    });
    world.step().expect("install v1 module");

    let (instance_id, install_target) = {
        let event = world.journal().events.last().expect("install event");
        let WorldEventBody::Domain(DomainEvent::ModuleInstalled {
            instance_id,
            install_target,
            ..
        }) = &event.body
        else {
            panic!("expected module installed event: {:?}", event.body);
        };
        (instance_id.clone(), install_target.clone())
    };

    let wasm_v2_bytes = b"module-action-loop-upgrade-v2".to_vec();
    let wasm_v2_hash = util::sha256_hex(&wasm_v2_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_v2_hash.clone(),
        wasm_bytes: wasm_v2_bytes,
    });
    world.step().expect("deploy v2 artifact");

    let manifest_v2 = base_manifest("m.loop.upgrade", "0.2.0", &wasm_v2_hash);
    world.submit_action(Action::UpgradeModuleFromArtifact {
        upgrader_agent_id: "installer-1".to_string(),
        instance_id: instance_id.clone(),
        from_module_version: "0.1.0".to_string(),
        manifest: manifest_v2.clone(),
        activate: true,
    });
    world.step().expect("upgrade module action");

    let proposal_id = {
        let event = world.journal().events.last().expect("upgrade event");
        let WorldEventBody::Domain(DomainEvent::ModuleUpgraded {
            upgrader_agent_id,
            instance_id: event_instance_id,
            module_id,
            from_module_version,
            to_module_version,
            wasm_hash,
            install_target: event_target,
            active,
            proposal_id,
            manifest_hash,
            fee_kind,
            fee_amount,
        }) = &event.body
        else {
            panic!("expected module upgraded event: {:?}", event.body);
        };
        assert_eq!(upgrader_agent_id, "installer-1");
        assert_eq!(event_instance_id, &instance_id);
        assert_eq!(module_id, "m.loop.upgrade");
        assert_eq!(from_module_version, "0.1.0");
        assert_eq!(to_module_version, "0.2.0");
        assert_eq!(wasm_hash, &wasm_v2_hash);
        assert_eq!(event_target, &install_target);
        assert!(*active);
        assert!(*proposal_id > 0);
        assert!(!manifest_hash.is_empty());
        assert_eq!(*fee_kind, ResourceKind::Electricity);
        assert!(*fee_amount > 0);
        *proposal_id
    };

    let upgraded = world
        .state()
        .module_instances
        .get(&instance_id)
        .expect("upgraded instance state");
    assert_eq!(upgraded.instance_id, instance_id);
    assert_eq!(upgraded.owner_agent_id, "installer-1");
    assert_eq!(upgraded.module_id, "m.loop.upgrade");
    assert_eq!(upgraded.module_version, "0.2.0");
    assert_eq!(upgraded.wasm_hash, wasm_v2_hash);
    assert_eq!(upgraded.install_target, install_target);
    assert!(upgraded.active);

    let v2_key = ModuleRegistry::record_key("m.loop.upgrade", "0.2.0");
    assert!(world.module_registry().records.contains_key(&v2_key));
    assert_eq!(
        world.module_registry().active.get("m.loop.upgrade"),
        Some(&"0.2.0".to_string())
    );
    assert!(matches!(
        world
            .proposals()
            .get(&proposal_id)
            .map(|proposal| &proposal.status),
        Some(ProposalStatus::Applied { .. })
    ));
}

#[test]
fn upgrade_module_from_artifact_rejects_incompatible_interface() {
    let mut world = World::new();
    register_agent(&mut world, "installer-1");

    let wasm_v1_bytes = b"module-action-loop-upgrade-guard-v1".to_vec();
    let wasm_v1_hash = util::sha256_hex(&wasm_v1_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_v1_hash.clone(),
        wasm_bytes: wasm_v1_bytes,
    });
    world.step().expect("deploy v1 artifact");

    let manifest_v1 = base_manifest("m.loop.upgrade.guard", "0.1.0", &wasm_v1_hash);
    world.submit_action(Action::InstallModuleFromArtifact {
        installer_agent_id: "installer-1".to_string(),
        manifest: manifest_v1.clone(),
        activate: true,
    });
    world.step().expect("install v1 module");
    let instance_id = {
        let event = world.journal().events.last().expect("install event");
        let WorldEventBody::Domain(DomainEvent::ModuleInstalled { instance_id, .. }) = &event.body
        else {
            panic!("expected module installed event: {:?}", event.body);
        };
        instance_id.clone()
    };

    let wasm_v2_bytes = b"module-action-loop-upgrade-guard-v2".to_vec();
    let wasm_v2_hash = util::sha256_hex(&wasm_v2_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "installer-1".to_string(),
        wasm_hash: wasm_v2_hash.clone(),
        wasm_bytes: wasm_v2_bytes,
    });
    world.step().expect("deploy v2 artifact");

    let mut incompatible_manifest = base_manifest("m.loop.upgrade.guard", "0.2.0", &wasm_v2_hash);
    incompatible_manifest.interface_version = "wasm-2".to_string();
    let action_id = world.submit_action(Action::UpgradeModuleFromArtifact {
        upgrader_agent_id: "installer-1".to_string(),
        instance_id: instance_id.clone(),
        from_module_version: "0.1.0".to_string(),
        manifest: incompatible_manifest,
        activate: true,
    });
    world.step().expect("upgrade incompatible module");

    assert_last_rejection_note(&world, action_id, "interface_version mismatch");
    let instance = world
        .state()
        .module_instances
        .get(&instance_id)
        .expect("instance state after rejected upgrade");
    assert_eq!(instance.module_version, "0.1.0");
    let v2_key = ModuleRegistry::record_key("m.loop.upgrade.guard", "0.2.0");
    assert!(!world.module_registry().records.contains_key(&v2_key));
}

#[test]
fn module_installed_domain_event_legacy_payload_defaults_install_target() {
    let current = DomainEvent::ModuleInstalled {
        installer_agent_id: "agent-legacy".to_string(),
        instance_id: "m.legacy#1".to_string(),
        module_id: "m.legacy".to_string(),
        module_version: "0.1.0".to_string(),
        wasm_hash: "legacy-hash".to_string(),
        install_target: ModuleInstallTarget::SelfAgent,
        active: true,
        proposal_id: 7,
        manifest_hash: "manifest-hash".to_string(),
        fee_kind: ResourceKind::Electricity,
        fee_amount: 2,
    };
    let mut value = serde_json::to_value(current).expect("serialize domain event");
    value
        .get_mut("data")
        .and_then(|data| data.as_object_mut())
        .expect("domain event data")
        .remove("install_target");
    let decoded: DomainEvent = serde_json::from_value(value).expect("deserialize legacy payload");
    let DomainEvent::ModuleInstalled { install_target, .. } = decoded else {
        panic!("expected module installed event");
    };
    assert_eq!(install_target, ModuleInstallTarget::SelfAgent);
}
