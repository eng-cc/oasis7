use std::os::unix::fs::{MetadataExt, PermissionsExt};

use base64::Engine;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

use super::*;
use crate::identity::{operation_key, request_key};
use crate::installation::{current_egid, current_uid};
use crate::local_fs::{create_private_dir, write_private_new};
use crate::protocol::{IPC_SCHEMA, RollbackProtocolContext, SignContext};
use crate::rollback::{UnsignedRollbackPayload, canonical_signing_payload};
use crate::types::{
    CallerBinding, GRANT_SCHEMA, GrantItem, INSTALLATION_SCHEMA, KeyBinding, POLICY_SCHEMA,
    PolicyLimits,
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).expect("random scratch suffix");
        let root = std::env::current_dir()
            .expect("working directory")
            .join(format!(
                ".oasis7-local-signer-test-{}-{}",
                std::process::id(),
                hex::encode(random)
            ));
        fs::create_dir(&root).expect("create scratch root");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (
    Scratch,
    SignerStore,
    IpcRequest,
    Vec<u8>,
    String,
    UnsignedRollbackPayload,
) {
    let scratch = Scratch::new();
    let root = scratch.path().join("store");
    create_private_dir(&root).expect("store root");
    for relative in [
        "control",
        "control/grants",
        "control/revoked-grants",
        "keys",
        "keys/signer-01",
        "state",
        "state/records",
        "work",
        "backup-staging",
    ] {
        create_private_dir(&root.join(relative)).expect("private fixture directory");
    }

    let uid = current_uid();
    let caller_uid = uid.checked_add(1).expect("distinct fixture caller UID");
    let gid = current_egid();
    let signing_key = SigningKey::from_bytes(&[0x5a; 32]);
    let public_key = signing_key.verifying_key().to_bytes();
    let public_key_sha256 = sha256_hex(&public_key);
    write_private_new(&root.join("keys/signer-01/seed.bin"), &[0x5a; 32]).expect("seed file");
    write_private_new(&root.join("keys/signer-01/public.bin"), &public_key).expect("public file");

    let installation_metadata = fs::metadata(&root).expect("store metadata");
    let installation = InstallationConfig {
        schema_version: INSTALLATION_SCHEMA.to_owned(),
        installation_id: "install-01".to_owned(),
        deployment_id: "deployment-01".to_owned(),
        store_dir: root.to_string_lossy().into_owned(),
        store_device_id: installation_metadata.dev(),
        store_inode: installation_metadata.ino(),
        signer_uid: uid,
        signer_gid: gid,
        callers: vec![CallerBinding {
            uid: caller_uid,
            work_dir: format!("/caller-jobs-{caller_uid}"),
            work_device_id: 1,
            work_inode: 2,
        }],
        release_id: "release-01".to_owned(),
        worker_executable: "/usr/local/libexec/oasis7/test-worker".to_owned(),
        worker_sha256: "00".repeat(32),
        control_schema_version: "v1".to_owned(),
    };

    let context = SignContext {
        deployment_id: "deployment-01".to_owned(),
        network_id: "network-01".to_owned(),
        task_uid: "task-01".to_owned(),
        source_head_oid: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        protocol_context: RollbackProtocolContext {
            authority_id: "authority-01".to_owned(),
            rollback_ticket: "ticket-01".to_owned(),
            receipt_id: "receipt-01".to_owned(),
            nonce: "nonce-01".to_owned(),
        },
    };
    let mut evidence_hash = Sha256::new();
    let report = b"strict audit report";
    let manifest = b"strict manifest";
    evidence_hash.update(b"oasis7:rollback-strict-audit-artifacts:v1\0");
    evidence_hash.update((report.len() as u64).to_be_bytes());
    evidence_hash.update(report);
    evidence_hash.update((manifest.len() as u64).to_be_bytes());
    evidence_hash.update(manifest);
    let evidence = UnsignedRollbackPayload {
        schema_version: 1,
        authority_id: context.protocol_context.authority_id.clone(),
        rollback_ticket: context.protocol_context.rollback_ticket.clone(),
        receipt_id: context.protocol_context.receipt_id.clone(),
        canonical_intent_digest: "11".repeat(32),
        recovery_snapshot_hash: "22".repeat(32),
        reorg_epoch: 4,
        candidate_state_root: "33".repeat(32),
        strict_registry_audit_passed: true,
        strict_manifest_audit_passed: true,
        audit_report_bytes: report.to_vec(),
        manifest_bytes: manifest.to_vec(),
        evidence_digest: hex::encode(evidence_hash.finalize()),
        issued_at_ms: unix_time_ms().expect("clock") - 1_000,
        expires_at_ms: unix_time_ms().expect("clock") + 600_000,
        nonce: context.protocol_context.nonce.clone(),
        signature_scheme: "ed25519".to_owned(),
    };
    let payload = canonical_signing_payload(&evidence).expect("canonical payload");
    let payload_sha256 = sha256_hex(&payload);
    let operation_key = operation_key(
        &installation.installation_id,
        &installation.deployment_id,
        &context.network_id,
        "rollback_strict_audit_v1",
        "signer-01",
        &crate::identity::sha256(&payload),
    )
    .expect("operation key");
    let policy = Policy {
        schema_version: POLICY_SCHEMA.to_owned(),
        installation_id: installation.installation_id.clone(),
        deployment_id: installation.deployment_id.clone(),
        policy_revision: "policy-01".to_owned(),
        enabled_purposes: vec!["rollback_strict_audit_v1".to_owned()],
        key_bindings: vec![KeyBinding {
            purpose: "rollback_strict_audit_v1".to_owned(),
            signer_id: "signer-01".to_owned(),
            public_key_sha256: public_key_sha256.clone(),
            protocol_authorities: vec!["authority-01".to_owned()],
            provider_ids: Vec::new(),
            deployment_ids: vec![installation.deployment_id.clone()],
            network_ids: vec![context.network_id.clone()],
        }],
        limits: PolicyLimits {
            max_batch_items: 64,
            max_distinct_requests_per_item: 16,
            max_payload_bytes: MAX_RESULT_FILE_BYTES as u64,
        },
    };
    let grant = BatchGrant {
        schema_version: GRANT_SCHEMA.to_owned(),
        grant_id: "grant-01".to_owned(),
        installation_id: installation.installation_id.clone(),
        deployment_id: installation.deployment_id.clone(),
        caller_uid,
        network_id: context.network_id.clone(),
        task_uid: context.task_uid.clone(),
        source_head_oid: context.source_head_oid.clone(),
        policy_revision: policy.policy_revision.clone(),
        not_before: unix_time_ms().expect("clock") - 1_000,
        expires_at: unix_time_ms().expect("clock") + 600_000,
        approval_record_ref: "approval-01".to_owned(),
        items: vec![GrantItem {
            purpose: "rollback_strict_audit_v1".to_owned(),
            provider_id: ExplicitNull(None),
            signer_id: "signer-01".to_owned(),
            public_key_sha256,
            operation_key: operation_key.clone(),
            payload_sha256,
            protocol_bindings: context.protocol_context.clone(),
            max_distinct_requests: 1,
        }],
    };
    let policy_bytes = canonical_json(&policy).expect("policy canonical JSON");
    write_control_fixture(&root.join(POLICY_FILE), &policy_bytes);
    let grant_bytes = canonical_json(&grant).expect("grant canonical JSON");
    write_control_fixture(&root.join(GRANTS_DIR).join("grant-01.json"), &grant_bytes);

    let store = SignerStore {
        installation,
        root,
        caller_uid,
        signer_uid: uid,
        control_owner_uid: uid,
        control_group_gid: gid,
        test_fault: Cell::new(None),
    };
    let request = IpcRequest::Sign {
        schema_version: IPC_SCHEMA.to_owned(),
        installation_id: "install-01".to_owned(),
        request_id: "request-01".to_owned(),
        purpose: "rollback_strict_audit_v1".to_owned(),
        provider_id: ExplicitNull(None),
        signer_id: "signer-01".to_owned(),
        grant_id: ExplicitNull(Some("grant-01".to_owned())),
        context,
        payload_base64: base64::engine::general_purpose::STANDARD.encode(&payload),
    };
    let request_key = request_key(
        "install-01",
        "rollback_strict_audit_v1",
        caller_uid,
        None,
        "request-01",
    )
    .expect("request key");
    (scratch, store, request, payload, request_key, evidence)
}

fn write_control_fixture(path: &Path, bytes: &[u8]) {
    write_private_new(path, bytes).expect("write fixture control file");
    fs::set_permissions(path, fs::Permissions::from_mode(0o640))
        .expect("set fixture control permissions");
}

/// Importer-only installer boundary evidence. Production has no fixture selector.
/// Ownership is deliberately projected to this nonprivileged test UID/GID;
/// the inventory checks below verify the intended privileged identities.
#[test]
#[ignore = "requires emitted installer-boundary.json fixture via OASIS7_INSTALLER_BOUNDARY_FIXTURE"]
fn emitted_installer_binding_is_v3_and_policyless_doctor_is_unready() {
    let path = std::env::var_os("OASIS7_INSTALLER_BOUNDARY_FIXTURE")
        .expect("explicit installer fixture path required");
    let report: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let installation: InstallationConfig =
        serde_json::from_value(report["installation_config"].clone()).unwrap();
    installation.validate().expect("actual emitted v3 binding");
    assert_eq!(installation.schema_version, INSTALLATION_SCHEMA);
    assert!(report["initial_files"].as_array().unwrap().is_empty());
    let inventory = report["inventory"].as_array().unwrap();
    let expected = [
        (".", 0, 0, 0o711),
        ("control", 0, installation.signer_gid, 0o750),
        ("control/grants", 0, installation.signer_gid, 0o750),
        ("control/revoked-grants", 0, installation.signer_gid, 0o750),
        (
            "keys",
            installation.signer_uid,
            installation.signer_gid,
            0o700,
        ),
        (
            "state",
            installation.signer_uid,
            installation.signer_gid,
            0o700,
        ),
        (
            "state/records",
            installation.signer_uid,
            installation.signer_gid,
            0o700,
        ),
        ("work", 0, 0, 0o711),
        (
            "backup-staging",
            installation.signer_uid,
            installation.signer_gid,
            0o700,
        ),
    ];
    assert_eq!(inventory.len(), expected.len());
    let scratch = Scratch::new();
    for (relative, uid, gid, mode) in expected {
        let entry = inventory
            .iter()
            .find(|entry| entry["relative"] == relative)
            .expect("mandatory directory inventory");
        assert_eq!(entry["owner_uid"].as_u64(), Some(u64::from(uid)));
        assert_eq!(entry["owner_gid"].as_u64(), Some(u64::from(gid)));
        assert_eq!(entry["mode"].as_u64(), Some(mode));
        let path = scratch.path().join(relative);
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode as u32)).unwrap();
        let metadata = fs::metadata(path).unwrap();
        assert_eq!(
            metadata.uid(),
            current_uid(),
            "honest synthetic owner projection"
        );
        assert_eq!(metadata.mode() & 0o7777, mode as u32);
    }
    assert!(!scratch.path().join(POLICY_FILE).exists());
    assert_eq!(
        fs::read_dir(scratch.path().join("keys")).unwrap().count(),
        0
    );
    assert_eq!(
        fs::read_dir(scratch.path().join(GRANTS_DIR))
            .unwrap()
            .count(),
        0
    );
    let caller_uid = installation.callers[0].uid;
    let store = SignerStore {
        installation,
        root: scratch.path().to_owned(),
        caller_uid,
        signer_uid: current_uid(),
        control_owner_uid: current_uid(),
        control_group_gid: current_egid(),
        test_fault: Cell::new(None),
    };
    let IpcResponse::Doctor(response) = store.doctor().unwrap() else {
        panic!("doctor response");
    };
    assert!(!response.ready);
    assert_eq!(response.status_code, "AUTHORIZATION_DENIED");
}

#[test]
fn signing_persists_full_context_and_replays_committed_response() {
    let (_scratch, store, request, payload, request_key, evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    let first = store
        .handle_request_at(request.clone(), now)
        .expect("first sign");
    let IpcResponse::Sign(first_result) = first else {
        panic!("expected sign result");
    };
    assert_eq!(first_result.status, "committed");
    assert!(first_result.error_code.0.is_none());
    let expected_public_key = SigningKey::from_bytes(&[0x5a; 32])
        .verifying_key()
        .to_bytes();
    let public_bytes = base64::engine::general_purpose::STANDARD
        .decode(
            first_result
                .public_key_base64
                .0
                .as_deref()
                .expect("public key"),
        )
        .expect("public key base64");
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(
            first_result
                .signature_base64
                .0
                .as_deref()
                .expect("signature"),
        )
        .expect("signature base64");
    assert_eq!(public_bytes, expected_public_key);
    assert_eq!(signature_bytes.len(), 64);
    verify_response_signature(&first_result, &sha256_hex(&expected_public_key), &payload)
        .expect("signature verifies");
    let assembled_payload =
        canonical_signing_payload(&evidence).expect("reassemble legacy strict-audit evidence");
    assert_eq!(assembled_payload, payload);
    let signature = Signature::from_slice(&signature_bytes).expect("64-byte signature");
    VerifyingKey::from_bytes(&expected_public_key)
        .expect("public key")
        .verify(&assembled_payload, &signature)
        .expect("legacy strict-audit verifier accepts assembled evidence");

    let record_dir = store.root.join(RECORDS_DIR).join(&request_key);
    let reservation: Reservation =
        read_json(&record_dir.join("reservation.json")).expect("reservation");
    if let IpcRequest::Sign { context, .. } = &request {
        assert_eq!(reservation.context, *context);
    } else {
        panic!("expected sign request");
    }

    let replay = store
        .handle_request_at(request, now + 1)
        .expect("idempotent replay");
    assert_eq!(replay, IpcResponse::Sign(first_result.clone()));
    let inspect = store
        .handle_request_at(
            IpcRequest::Inspect {
                schema_version: IPC_SCHEMA.to_owned(),
                installation_id: "install-01".to_owned(),
                request_id: "request-01".to_owned(),
                purpose: "rollback_strict_audit_v1".to_owned(),
                provider_id: ExplicitNull(None),
            },
            now + 1,
        )
        .expect("inspect committed result");
    let IpcResponse::Inspect(inspect) = inspect else {
        panic!("expected inspect result");
    };
    assert_eq!(inspect.status, "committed");
    assert_eq!(inspect.audit_digest.0, first_result.audit_digest.0);
}

#[test]
fn committed_reservation_consumes_exact_batch_request_budget() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    store
        .handle_request_at(request.clone(), now)
        .expect("first sign");
    let IpcRequest::Sign {
        schema_version,
        installation_id,
        purpose,
        provider_id,
        signer_id,
        grant_id,
        context,
        payload_base64,
        ..
    } = request
    else {
        panic!("expected sign request");
    };
    let second = IpcRequest::Sign {
        schema_version,
        installation_id,
        request_id: "request-02".to_owned(),
        purpose,
        provider_id,
        signer_id,
        grant_id,
        context,
        payload_base64,
    };
    assert!(matches!(
        store.handle_request_at(second, now + 1),
        Err(SignerError::BudgetExhausted)
    ));
}

#[test]
fn committed_response_replay_rechecks_live_grant_revocation() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    store
        .handle_request_at(request.clone(), now)
        .expect("first sign");
    write_private_new(
        &store.root.join(REVOKED_GRANTS_DIR).join("grant-01.json"),
        b"revoked\n",
    )
    .expect("revoke marker");

    assert!(matches!(
        store.handle_request_at(request, now + 1),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn signing_rejects_unbound_caller_purpose_payload_context_and_expiry() {
    let (_scratch, mut store, request, payload, _request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");

    store.caller_uid = store.caller_uid.saturating_add(1);
    assert!(matches!(
        store.handle_request_at(request.clone(), now),
        Err(SignerError::AuthorizationDenied)
    ));
    store.caller_uid = store.installation.callers[0].uid;

    let mut wrong_purpose = request.clone();
    if let IpcRequest::Sign { purpose, .. } = &mut wrong_purpose {
        *purpose = "identity_v2".to_owned();
    }
    assert!(matches!(
        store.handle_request_at(wrong_purpose, now),
        Err(SignerError::Protocol(_))
    ));

    let mut wrong_payload = request.clone();
    if let IpcRequest::Sign { payload_base64, .. } = &mut wrong_payload {
        *payload_base64 = base64::engine::general_purpose::STANDARD.encode(b"different");
    }
    assert!(matches!(
        store.handle_request_at(wrong_payload, now),
        Err(SignerError::CryptoOrBindingInvalid)
    ));

    let mut wrong_context = request.clone();
    if let IpcRequest::Sign { context, .. } = &mut wrong_context {
        context.protocol_context.nonce = "nonce-other".to_owned();
    }
    assert!(matches!(
        store.handle_request_at(wrong_context, now),
        Err(SignerError::CryptoOrBindingInvalid)
    ));

    let mut wrong_grant = request.clone();
    if let IpcRequest::Sign { grant_id, .. } = &mut wrong_grant {
        *grant_id = ExplicitNull(Some("grant-missing".to_owned()));
    }
    assert!(matches!(
        store.handle_request_at(wrong_grant, now),
        Err(SignerError::AuthorizationDenied)
    ));

    assert!(matches!(
        store.handle_request_at(request, now + 700_000),
        Err(SignerError::AuthorizationDenied)
    ));
    assert!(!payload.is_empty());
}

#[test]
fn same_request_id_cannot_change_signer_even_with_another_exact_grant_item() {
    let (_scratch, store, mut request, payload, request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    store
        .handle_request_at(request.clone(), now)
        .expect("initial sign");
    let mut policy: Policy = read_control(
        &store.root.join(POLICY_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("policy");
    let mut second_key = policy.key_bindings[0].clone();
    second_key.signer_id = "signer-02".to_owned();
    policy.key_bindings.push(second_key);
    fs::write(
        store.root.join(POLICY_FILE),
        canonical_json(&policy).expect("policy bytes"),
    )
    .expect("update policy fixture");

    let mut grant: BatchGrant = read_control(
        &store.root.join(GRANTS_DIR).join("grant-01.json"),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("grant");
    let mut second_item = grant.items[0].clone();
    second_item.signer_id = "signer-02".to_owned();
    second_item.operation_key = operation_key(
        &store.installation.installation_id,
        &store.installation.deployment_id,
        "network-01",
        "rollback_strict_audit_v1",
        "signer-02",
        &crate::identity::sha256(&payload),
    )
    .expect("second operation key");
    grant.items.push(second_item);
    fs::write(
        store.root.join(GRANTS_DIR).join("grant-01.json"),
        canonical_json(&grant).expect("grant bytes"),
    )
    .expect("update grant fixture");

    if let IpcRequest::Sign { signer_id, .. } = &mut request {
        *signer_id = "signer-02".to_owned();
    }
    let Err(error) = store.handle_request_at(request, now) else {
        panic!("changed signer reused a request ID");
    };
    assert_eq!(error.code(), "ID_CONFLICT", "unexpected error: {error:?}");
    assert!(store.root.join(RECORDS_DIR).join(request_key).is_dir());
}

#[test]
fn reservation_without_result_recovers_same_request_but_staging_requires_recovery() {
    let (_scratch, store, request, _payload, request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    let authorized = store.authorize(&request, now).expect("authorization");
    let record_dir = store.root.join(RECORDS_DIR).join(&request_key);
    create_private_dir(&record_dir).expect("reservation directory");
    sync_dir(&store.root.join(RECORDS_DIR)).expect("records sync");
    let reservation = store.make_reservation(&authorized);
    write_private_new(
        &record_dir.join("reservation.json"),
        &canonical_json(&reservation).expect("reservation bytes"),
    )
    .expect("persist reservation");
    sync_dir(&record_dir).expect("reservation sync");

    let recovered = store
        .handle_request_at(request.clone(), now + 1)
        .expect("recover reservation");
    assert!(
        matches!(recovered, IpcResponse::Sign(SignResult { status, .. }) if status == "committed")
    );

    let second = fixture();
    let (_scratch, interrupted_store, interrupted_request, _payload, interrupted_key, _evidence) =
        second;
    let interrupted_dir = interrupted_store
        .root
        .join(RECORDS_DIR)
        .join(interrupted_key);
    create_private_dir(&interrupted_dir).expect("interrupted reservation directory");
    let authorized = interrupted_store
        .authorize(&interrupted_request, now)
        .expect("interrupted authorization");
    write_private_new(
        &interrupted_dir.join("reservation.json"),
        &canonical_json(&interrupted_store.make_reservation(&authorized))
            .expect("interrupted reservation bytes"),
    )
    .expect("interrupted reservation");
    create_private_dir(&interrupted_dir.join(".result-staging-crash")).expect("staging residue");
    assert!(matches!(
        interrupted_store.handle_request_at(interrupted_request, now + 1),
        Err(SignerError::RecoveryRequired)
    ));
}

#[test]
fn reservation_sync_fault_keeps_exact_request_recoverable() {
    let (_scratch, store, request, _payload, request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    store
        .test_fault
        .set(Some(StoreFaultPoint::AfterReservationSync));

    assert!(matches!(
        store.handle_request_at(request.clone(), now),
        Err(SignerError::PersistenceFailed(_))
    ));
    let record_dir = store.root.join(RECORDS_DIR).join(request_key);
    assert!(record_dir.join("reservation.json").is_file());
    assert!(!record_dir.join("result").exists());

    let recovered = store
        .handle_request_at(request, now + 1)
        .expect("same durable reservation can resume");
    assert!(matches!(
        recovered,
        IpcResponse::Sign(SignResult { status, .. }) if status == "committed"
    ));
}

#[test]
fn staging_sync_and_pre_rename_faults_fail_closed_for_recovery() {
    for point in [
        StoreFaultPoint::AfterResultStagingSync,
        StoreFaultPoint::BeforeResultRename,
    ] {
        let (_scratch, store, request, _payload, request_key, _evidence) = fixture();
        let now = unix_time_ms().expect("clock");
        store.test_fault.set(Some(point));

        assert!(matches!(
            store.handle_request_at(request.clone(), now),
            Err(SignerError::PersistenceFailed(_))
        ));
        let record_dir = store.root.join(RECORDS_DIR).join(request_key);
        assert!(has_result_staging(&record_dir).expect("inspect staging"));
        assert!(!record_dir.join("result").exists());
        assert!(matches!(
            store.handle_request_at(request, now + 1),
            Err(SignerError::RecoveryRequired)
        ));
    }
}

#[test]
fn post_rename_sync_fault_requires_durable_parent_sync_before_replay() {
    let (_scratch, store, request, _payload, request_key, _evidence) = fixture();
    let now = unix_time_ms().expect("clock");
    store
        .test_fault
        .set(Some(StoreFaultPoint::AfterResultRename));

    assert!(matches!(
        store.handle_request_at(request.clone(), now),
        Err(SignerError::PersistenceFailed(_))
    ));
    let record_dir = store.root.join(RECORDS_DIR).join(request_key);
    assert!(record_dir.join("result").is_dir());

    store
        .test_fault
        .set(Some(StoreFaultPoint::BeforeReplayParentSync));
    assert!(matches!(
        store.handle_request_at(request.clone(), now + 1),
        Err(SignerError::PersistenceFailed(_))
    ));

    let replay = store
        .handle_request_at(request, now + 1)
        .expect("visible committed result replays");
    assert!(matches!(
        replay,
        IpcResponse::Sign(SignResult { status, .. }) if status == "committed"
    ));
}

#[test]
fn admin_key_create_returns_public_key_only_and_persists_signer_owned_files() {
    let (_scratch, store, _request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let public_key = admin
        .create_key("signer-02", "rollback_strict_audit_v1")
        .expect("create fixture key");
    assert_eq!(
        admin.key_public("signer-02").expect("read public"),
        public_key
    );

    let key_dir = store.root.join("keys/signer-02");
    let seed_path = key_dir.join("seed.bin");
    let seed_metadata = fs::metadata(&seed_path).expect("seed metadata");
    assert_eq!(seed_metadata.uid(), store.signer_uid);
    assert_eq!(seed_metadata.gid(), store.installation.signer_gid);
    assert_eq!(seed_metadata.permissions().mode() & 0o7777, 0o600);
    let seed: [u8; 32] = fs::read(seed_path)
        .expect("test-only seed inspection")
        .try_into()
        .expect("32-byte seed");
    assert_eq!(
        SigningKey::from_bytes(&seed).verifying_key().to_bytes(),
        public_key
    );
}

#[test]
fn admin_policy_install_requires_exact_digest_disabled_state_and_immutable_revision() {
    let (_scratch, store, _request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let mut policy: Policy = read_control(
        &store.root.join(POLICY_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("fixture policy");
    policy.enabled_purposes.clear();
    policy.policy_revision = "policy-02".to_owned();
    let bytes = canonical_json(&policy).expect("candidate bytes");
    let digest = sha256_hex(&bytes);

    admin
        .install_policy_candidate(&bytes, &digest)
        .expect("install disabled candidate");
    admin
        .install_policy_candidate(&bytes, &digest)
        .expect("same exact candidate is idempotent");

    let mut changed = policy;
    changed.limits.max_payload_bytes -= 1;
    let changed_bytes = canonical_json(&changed).expect("changed candidate");
    assert!(matches!(
        admin.install_policy_candidate(&changed_bytes, &sha256_hex(&changed_bytes)),
        Err(SignerError::IdConflict)
    ));
    assert!(matches!(
        admin.install_policy_candidate(&bytes, &"00".repeat(32)),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn admin_intent_before_target_publication_blocks_signing_until_exact_retry() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let mut policy: Policy = read_control(
        &store.root.join(POLICY_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("fixture policy");
    policy.enabled_purposes.clear();
    policy.policy_revision = "policy-maintenance-prepublish".to_owned();
    let bytes = canonical_json(&policy).expect("candidate bytes");
    let digest = sha256_hex(&bytes);

    admin
        .test_fault
        .set(Some(crate::admin::AdminFaultPoint::AfterIntentSync));
    assert!(matches!(
        admin.install_policy_candidate(&bytes, &digest),
        Err(SignerError::PersistenceFailed(_))
    ));
    let maintenance: MaintenanceRecord = read_control(
        &store.root.join(MAINTENANCE_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("durable prepared intent");
    assert_eq!(maintenance.phase, "prepared");
    assert!(matches!(
        store.handle_request_at(request.clone(), unix_time_ms().expect("clock")),
        Err(SignerError::RecoveryRequired)
    ));

    admin
        .install_policy_candidate(&bytes, &digest)
        .expect("same exact operation resumes");
    let maintenance: MaintenanceRecord = read_control(
        &store.root.join(MAINTENANCE_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("committed intent");
    assert_eq!(maintenance.phase, "committed");
    let result = store.handle_request_at(request, unix_time_ms().expect("clock"));
    assert!(
        matches!(&result, Err(SignerError::InvalidInput(_))),
        "unexpected result after exact retry: {result:?}"
    );
}

#[test]
fn admin_published_target_sync_ambiguity_stays_blocked_until_retry_syncs_target() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let mut policy: Policy = read_control(
        &store.root.join(POLICY_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .expect("fixture policy");
    policy.enabled_purposes.clear();
    policy.policy_revision = "policy-maintenance-postpublish".to_owned();
    let bytes = canonical_json(&policy).expect("candidate bytes");
    let digest = sha256_hex(&bytes);

    admin.test_fault.set(Some(
        crate::admin::AdminFaultPoint::AfterTargetPublishBeforeParentSync,
    ));
    assert!(matches!(
        admin.install_policy_candidate(&bytes, &digest),
        Err(SignerError::PersistenceFailed(_))
    ));
    assert_eq!(
        fs::read(store.root.join(POLICY_FILE)).expect("published target"),
        bytes
    );
    assert!(matches!(
        store.handle_request_at(request.clone(), unix_time_ms().expect("clock")),
        Err(SignerError::RecoveryRequired)
    ));

    admin
        .install_policy_candidate(&bytes, &digest)
        .expect("exact retry syncs target and commits intent");
    let result = store.handle_request_at(request, unix_time_ms().expect("clock"));
    assert!(
        matches!(&result, Err(SignerError::InvalidInput(_))),
        "unexpected result after exact retry: {result:?}"
    );
}

#[test]
fn interrupted_key_create_without_complete_staging_remains_manual_recovery() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin
        .test_fault
        .set(Some(crate::admin::AdminFaultPoint::AfterIntentSync));

    assert!(matches!(
        admin.create_key("signer-02", "rollback_strict_audit_v1"),
        Err(SignerError::PersistenceFailed(_))
    ));
    assert!(matches!(
        store.handle_request_at(request, unix_time_ms().expect("clock")),
        Err(SignerError::RecoveryRequired)
    ));
    assert!(matches!(
        admin.create_key("signer-02", "rollback_strict_audit_v1"),
        Err(SignerError::RecoveryRequired)
    ));
    assert!(!store.root.join("keys/signer-02").exists());
}

#[test]
fn key_create_post_rename_retry_returns_same_public_key_and_commits_intent() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin.test_fault.set(Some(
        crate::admin::AdminFaultPoint::AfterTargetPublishBeforeParentSync,
    ));
    assert!(matches!(
        admin.create_key("signer-02", "rollback_strict_audit_v1"),
        Err(SignerError::PersistenceFailed(_))
    ));
    let public_path = store.root.join("keys/signer-02/public.bin");
    let first_public = fs::read(&public_path).expect("published public key");
    assert!(matches!(
        store.handle_request_at(request.clone(), unix_time_ms().expect("clock")),
        Err(SignerError::RecoveryRequired)
    ));

    let retried_public = admin
        .create_key("signer-02", "rollback_strict_audit_v1")
        .expect("retry validates and syncs published key");
    assert_eq!(first_public, retried_public);
    assert_eq!(
        first_public,
        fs::read(public_path).expect("same public key")
    );
    assert!(matches!(
        store.handle_request_at(request, unix_time_ms().expect("clock")),
        Ok(IpcResponse::Sign(SignResult { status, .. })) if status == "committed"
    ));
}

#[test]
fn create_only_admin_link_residue_fails_closed_and_exact_revoke_retry_recovers() {
    use std::os::unix::fs::MetadataExt;

    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin.test_fault.set(Some(
        crate::admin::AdminFaultPoint::AfterCreateOnlyLinkBeforeUnlink,
    ));

    assert!(matches!(
        admin.revoke_grant("grant-01"),
        Err(SignerError::PersistenceFailed(_))
    ));
    let marker_path = store.root.join(REVOKED_GRANTS_DIR).join("grant-01.json");
    assert_eq!(
        fs::symlink_metadata(&marker_path)
            .expect("linked marker")
            .nlink(),
        2
    );
    assert!(matches!(
        store.handle_request_at(request.clone(), unix_time_ms().expect("clock")),
        Err(SignerError::RecoveryRequired)
    ));

    admin
        .revoke_grant("grant-01")
        .expect("same revoke removes staging link and commits intent");
    assert_eq!(
        fs::symlink_metadata(marker_path)
            .expect("single-link marker")
            .nlink(),
        1
    );
    assert!(matches!(
        store.handle_request_at(request, unix_time_ms().expect("clock")),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn admin_grant_install_is_immutable_and_revocation_survives_replay() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let grant_path = store.root.join(GRANTS_DIR).join("grant-01.json");
    let bytes = fs::read(&grant_path).expect("grant bytes");
    let digest = sha256_hex(&bytes);
    admin
        .install_grant_candidate(&bytes, &digest)
        .expect("same grant install is idempotent");

    let mut changed: BatchGrant = serde_json::from_slice(&bytes).expect("grant");
    changed.approval_record_ref = "approval-changed".to_owned();
    let changed_bytes = canonical_json(&changed).expect("changed grant");
    assert!(matches!(
        admin.install_grant_candidate(&changed_bytes, &sha256_hex(&changed_bytes)),
        Err(SignerError::IdConflict)
    ));

    admin.revoke_grant("grant-01").expect("revoke grant");
    admin
        .revoke_grant("grant-01")
        .expect("revoke is idempotent");
    assert!(matches!(
        store.handle_request_at(request, unix_time_ms().expect("clock")),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn admin_enable_requires_valid_bound_key_and_disable_is_immediate() {
    let (_scratch, store, request, _payload, _request_key, _evidence) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin
        .set_purpose_enabled("rollback_strict_audit_v1", false)
        .expect("disable purpose");
    assert!(matches!(
        store.handle_request_at(request.clone(), unix_time_ms().expect("clock")),
        Err(SignerError::AuthorizationDenied)
    ));

    admin
        .set_purpose_enabled("rollback_strict_audit_v1", true)
        .expect("re-enable bound key");
    assert!(matches!(
        store.handle_request_at(request, unix_time_ms().expect("clock")),
        Ok(IpcResponse::Sign(SignResult { status, .. })) if status == "committed"
    ));
}

fn backup_ready(admin: &crate::admin::AdminStore) {
    admin
        .set_purpose_enabled("rollback_strict_audit_v1", false)
        .unwrap();
    for d in ["control", "control/grants", "control/revoked-grants"] {
        fs::set_permissions(admin.root.join(d), fs::Permissions::from_mode(0o750)).unwrap();
    }
}
fn empty_restore_fixture(store: &SignerStore) {
    for d in ["keys", "control", "state"] {
        fs::remove_dir_all(store.root.join(d)).unwrap();
        create_private_dir(&store.root.join(d)).unwrap();
    }
    for d in ["control/grants", "control/revoked-grants", "state/records"] {
        create_private_dir(&store.root.join(d)).unwrap();
    }
    for d in ["control", "control/grants", "control/revoked-grants"] {
        fs::set_permissions(store.root.join(d), fs::Permissions::from_mode(0o750)).unwrap();
    }
}
#[test]
fn backup_restores_actual_committed_record_without_reenabling_authority() {
    let (_scratch, store, request, payload, request_key, _) = fixture();
    store
        .handle_request_at(request.clone(), unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    assert!(matches!(
        admin.backup_encrypted(b"a sufficiently long passphrase", false),
        Err(SignerError::AuthorizationDenied)
    ));
    let encrypted = admin
        .backup_encrypted(b"a sufficiently long passphrase", true)
        .unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    let plan = restore
        .restore_plan(&encrypted, b"a sufficiently long passphrase")
        .unwrap();
    restore
        .restore_commit(&encrypted, b"a sufficiently long passphrase", &plan)
        .unwrap();
    restore
        .restore_commit(&encrypted, b"a sufficiently long passphrase", &plan)
        .unwrap();
    let dir = target.root.join(RECORDS_DIR).join(&request_key);
    let reservation: Reservation = read_json(&dir.join("reservation.json")).unwrap();
    let (response, _, _) = read_committed_result(&dir, &reservation).unwrap();
    verify_response_signature(&response, &reservation.public_key_sha256, &payload).unwrap();
    let policy: Policy = read_control(
        &target.root.join(POLICY_FILE),
        target.control_owner_uid,
        target.control_group_gid,
    )
    .unwrap();
    assert!(policy.enabled_purposes.is_empty());
    assert!(
        target
            .root
            .join(REVOKED_GRANTS_DIR)
            .join("grant-01.json")
            .exists()
    );
    assert!(matches!(
        target.handle_request_at(request, unix_time_ms().unwrap()),
        Err(SignerError::AuthorizationDenied)
    ));
}
#[test]
fn backup_rejects_corrupt_committed_record_and_incomplete_key_before_restore_write() {
    let (_scratch, store, request, _, request_key, _) = fixture();
    store
        .handle_request_at(request, unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let password = b"a sufficiently long passphrase";
    let encrypted = admin.backup_encrypted(password, true).unwrap();
    let plain = crate::key_envelope::open("backup", &encrypted, password).unwrap();
    let snapshot: serde_json::Value = serde_json::from_slice(&plain).unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    for path in [
        format!("state/records/{request_key}/result/manifest.json"),
        "keys/signer-01/seed.bin".into(),
    ] {
        let mut changed = snapshot.clone();
        changed["files"].as_object_mut().unwrap().remove(&path);
        let envelope =
            crate::key_envelope::seal("backup", &canonical_json(&changed).unwrap(), password)
                .unwrap();
        assert!(restore.restore_plan(&envelope, password).is_err());
        assert!(!target.root.join("control/restore.json").exists());
        assert_eq!(fs::read_dir(target.root.join("keys")).unwrap().count(), 0);
    }
    let mut changed = snapshot;
    let policy_path = "control/policy.json";
    let raw = base64::engine::general_purpose::STANDARD
        .decode(changed["files"][policy_path].as_str().unwrap())
        .unwrap();
    let mut policy: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    policy["unknown"] = true.into();
    changed["files"][policy_path] = base64::engine::general_purpose::STANDARD
        .encode(canonical_json(&policy).unwrap())
        .into();
    let envelope =
        crate::key_envelope::seal("backup", &canonical_json(&changed).unwrap(), password).unwrap();
    assert!(restore.restore_plan(&envelope, password).is_err());
}

#[test]
fn backup_restore_retry_syncs_existing_tree_after_directory_sync_failure() {
    let (_scratch, store, request, _, _, _) = fixture();
    store
        .handle_request_at(request, unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let password = b"a sufficiently long passphrase";
    let encrypted = admin.backup_encrypted(password, true).unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    let plan = restore.restore_plan(&encrypted, password).unwrap();
    crate::backup::FAIL_RESTORE_DIR_SYNC.with(|f| f.set(true));
    assert!(matches!(
        restore.restore_commit(&encrypted, password, &plan),
        Err(SignerError::PersistenceFailed(_))
    ));
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(target.root.join("control/restore.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["phase"], "prepared");
    assert_eq!(
        restore.read_maintenance().unwrap().unwrap().phase,
        "prepared"
    );
    restore.restore_commit(&encrypted, password, &plan).unwrap();
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(target.root.join("control/restore.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["phase"], "committed");
    assert_eq!(
        restore.read_maintenance().unwrap().unwrap().phase,
        "committed"
    );
}

#[test]
fn backup_nonexportable_check_covers_deleted_seed_residue_under_lock() {
    let (_scratch, store, _, _, _, _) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let mut catalog = admin.catalog().unwrap();
    catalog.keys = admin.key_list().unwrap();
    assert_eq!(catalog.keys.len(), 1);
    catalog.keys[0].exportable = true;
    catalog.keys[0].state = "deleted".into();
    write_control_fixture(
        &store.root.join(crate::key_management::CATALOG),
        &canonical_json(&catalog).unwrap(),
    );
    assert!(store.root.join("keys/signer-01/seed.bin").exists());
    assert!(matches!(
        admin.backup_encrypted(b"a sufficiently long passphrase", false),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn inactive_catalog_key_blocks_worker_sign_and_committed_inspect_replay() {
    let (_scratch, store, request, _, _, _) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin.key_set_state("signer-01", "active").unwrap();
    let now = unix_time_ms().unwrap();
    assert!(
        matches!(store.handle_request_at(request.clone(),now),Ok(IpcResponse::Sign(SignResult{status,..})) if status=="committed")
    );
    let inspect = IpcRequest::Inspect {
        schema_version: IPC_SCHEMA.into(),
        installation_id: "install-01".into(),
        request_id: "request-01".into(),
        purpose: "rollback_strict_audit_v1".into(),
        provider_id: ExplicitNull(None),
    };
    assert!(matches!(
        store.handle_request_at(inspect.clone(), now + 1),
        Ok(IpcResponse::Inspect(_))
    ));
    admin.key_set_state("signer-01", "inactive").unwrap();
    assert!(matches!(
        store.handle_request_at(request, now + 2),
        Err(SignerError::AuthorizationDenied)
    ));
    assert!(matches!(
        store.handle_request_at(inspect, now + 2),
        Err(SignerError::AuthorizationDenied)
    ));
}
