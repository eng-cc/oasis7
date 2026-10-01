use std::io::Cursor;
use std::path::Path;

use oasis7_local_signer::admin::AdminStore;
use oasis7_local_signer::authorization::authorize_sign;
use oasis7_local_signer::error::SignerError;
use oasis7_local_signer::installation::{current_egid, current_euid, trusted_sudo_caller};
use oasis7_local_signer::protocol::{
    ExplicitNull, IPC_SCHEMA, IpcRequest, RollbackProtocolContext, SignContext, decode_payload,
    encode_request_frame, read_request_frame,
};
use oasis7_local_signer::types::{
    BatchGrant, CallerBinding, GRANT_SCHEMA, GrantItem, INSTALLATION_SCHEMA, InstallationConfig,
    KeyBinding, POLICY_SCHEMA, Policy, PolicyLimits,
};

fn frame_json(json: &[u8]) -> Vec<u8> {
    let mut frame = (json.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(json);
    frame
}

fn rollback_sign_request(purpose: &str, provider_id: Option<&str>) -> IpcRequest {
    IpcRequest::Sign {
        schema_version: "oasis7.local_signer_ipc.v1".to_owned(),
        installation_id: "install-01".to_owned(),
        request_id: "request-01".to_owned(),
        purpose: purpose.to_owned(),
        provider_id: provider_id.map(str::to_owned).into(),
        signer_id: "rollback-audit-r1".to_owned(),
        grant_id: Some("grant-01".to_owned()).into(),
        context: SignContext {
            deployment_id: "testnet-a".to_owned(),
            network_id: "network-01".to_owned(),
            task_uid: "task_0123456789abcdef0123456789abcdef".to_owned(),
            source_head_oid: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            protocol_context: RollbackProtocolContext {
                authority_id: "authority-01".to_owned(),
                rollback_ticket: "ticket-01".to_owned(),
                receipt_id: "receipt-01".to_owned(),
                nonce: "nonce-01".to_owned(),
            },
        },
        payload_base64: "eA==".to_owned(),
    }
}

fn auth_fixture(store_dir: &Path) -> (InstallationConfig, Policy, BatchGrant, IpcRequest) {
    let context = SignContext {
        deployment_id: "testnet-a".to_owned(),
        network_id: "network-01".to_owned(),
        task_uid: "task_0123456789abcdef0123456789abcdef".to_owned(),
        source_head_oid: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        protocol_context: RollbackProtocolContext {
            authority_id: "authority-01".to_owned(),
            rollback_ticket: "ticket-01".to_owned(),
            receipt_id: "receipt-01".to_owned(),
            nonce: "nonce-01".to_owned(),
        },
    };
    let installation = InstallationConfig {
        schema_version: INSTALLATION_SCHEMA.to_owned(),
        installation_id: "install-01".to_owned(),
        deployment_id: "testnet-a".to_owned(),
        store_dir: store_dir.display().to_string(),
        store_device_id: 0,
        store_inode: 0,
        signer_uid: 1,
        signer_gid: 2,
        callers: vec![
            CallerBinding {
                uid: 100,
                work_dir: "/caller-jobs-100".to_owned(),
                work_device_id: 1,
                work_inode: 2,
            },
            CallerBinding {
                uid: 200,
                work_dir: "/caller-jobs-200".to_owned(),
                work_device_id: 1,
                work_inode: 2,
            },
        ],
        release_id: "release-01".to_owned(),
        worker_executable: "/usr/bin/true".to_owned(),
        worker_sha256: "44".repeat(32),
        control_schema_version: "control-v1".to_owned(),
    };
    let policy = Policy {
        schema_version: POLICY_SCHEMA.to_owned(),
        installation_id: "install-01".to_owned(),
        deployment_id: "testnet-a".to_owned(),
        policy_revision: "revision-01".to_owned(),
        enabled_purposes: vec!["rollback_strict_audit_v1".to_owned()],
        key_bindings: vec![KeyBinding {
            purpose: "rollback_strict_audit_v1".to_owned(),
            signer_id: "rollback-audit-r1".to_owned(),
            public_key_sha256: "11".repeat(32),
            protocol_authorities: vec!["authority-01".to_owned()],
            provider_ids: Vec::new(),
            deployment_ids: vec!["testnet-a".to_owned()],
            network_ids: vec!["network-01".to_owned()],
        }],
        limits: PolicyLimits {
            max_batch_items: 1,
            max_distinct_requests_per_item: 1,
            max_payload_bytes: 1024,
        },
    };
    let grant = BatchGrant {
        schema_version: GRANT_SCHEMA.to_owned(),
        grant_id: "grant-01".to_owned(),
        installation_id: "install-01".to_owned(),
        deployment_id: "testnet-a".to_owned(),
        caller_uid: 100,
        network_id: "network-01".to_owned(),
        task_uid: context.task_uid.clone(),
        source_head_oid: context.source_head_oid.clone(),
        policy_revision: "revision-01".to_owned(),
        not_before: 1_000,
        expires_at: 2_000,
        approval_record_ref: "approval-01".to_owned(),
        items: vec![GrantItem {
            purpose: "rollback_strict_audit_v1".to_owned(),
            provider_id: ExplicitNull(None),
            signer_id: "rollback-audit-r1".to_owned(),
            public_key_sha256: "11".repeat(32),
            operation_key: "22".repeat(32),
            payload_sha256: "33".repeat(32),
            protocol_bindings: context.protocol_context.clone(),
            max_distinct_requests: 1,
        }],
    };
    let request = IpcRequest::Sign {
        schema_version: IPC_SCHEMA.to_owned(),
        installation_id: "install-01".to_owned(),
        request_id: "request-01".to_owned(),
        purpose: "rollback_strict_audit_v1".to_owned(),
        provider_id: ExplicitNull(None),
        signer_id: "rollback-audit-r1".to_owned(),
        grant_id: ExplicitNull(Some("grant-01".to_owned())),
        context,
        payload_base64: "eA==".to_owned(),
    };
    (installation, policy, grant, request)
}

#[test]
fn invalid_utf8_and_empty_frames_fail_closed() {
    let invalid_utf8 = frame_json(&[0xff]);
    assert!(read_request_frame(&mut Cursor::new(invalid_utf8)).is_err());

    assert!(read_request_frame(&mut Cursor::new([0_u8; 4])).is_err());
}

#[test]
fn unsupported_schema_and_world_apply_command_are_rejected() {
    let wrong_schema = br#"{"schema_version":"oasis7.local_signer_ipc.v2","command":"doctor","installation_id":"install-01"}"#;
    assert!(read_request_frame(&mut Cursor::new(frame_json(wrong_schema))).is_err());

    let world_apply = br#"{"schema_version":"oasis7.local_signer_ipc.v1","command":"apply","installation_id":"install-01"}"#;
    assert!(read_request_frame(&mut Cursor::new(frame_json(world_apply))).is_err());
}

#[test]
fn role_and_task_claims_do_not_authenticate_the_os_caller() {
    let role_claim = br#"{"schema_version":"oasis7.local_signer_ipc.v1","command":"doctor","installation_id":"install-01","role":"admin"}"#;
    assert!(read_request_frame(&mut Cursor::new(frame_json(role_claim))).is_err());

    let (installation, policy, grant, request) = auth_fixture(Path::new("/tmp/qa-signer-store"));
    installation
        .validate()
        .expect("valid disposable installation fixture");
    policy
        .validate(&installation)
        .expect("valid policy fixture");
    grant
        .validate(&installation, &policy)
        .expect("valid grant fixture");

    // The request carries the exact task/source claims authorized by the grant,
    // but those claims must not allow another bound UID to impersonate caller 100.
    let error = authorize_sign(&installation, &policy, &grant, 200, &request, 1_500)
        .expect_err("task claims must not substitute for the trusted caller UID");
    assert!(matches!(error, SignerError::AuthorizationDenied));
}

#[test]
fn installation_v1_and_missing_signer_gid_are_rejected() {
    let (mut installation, _policy, _grant, _request) =
        auth_fixture(Path::new("/tmp/qa-signer-store"));
    installation.schema_version = "oasis7.local_signer_installation.v1".to_owned();
    assert!(installation.validate().is_err());

    installation.schema_version = "oasis7.local_signer_installation.v2".to_owned();
    let mut serialized = serde_json::to_value(&installation).expect("serialize v2 fixture");
    serialized
        .as_object_mut()
        .expect("installation JSON object")
        .remove("signer_gid");
    assert!(
        serde_json::from_value::<oasis7_local_signer::types::InstallationConfig>(serialized)
            .is_err()
    );
}

#[test]
fn signer_effective_gid_must_match_before_sudo_caller_lookup() {
    let (mut installation, _policy, _grant, _request) =
        auth_fixture(Path::new("/tmp/qa-signer-store"));
    installation.signer_uid = current_euid();
    installation.signer_gid = if current_egid() == u32::MAX {
        current_egid() - 1
    } else {
        current_egid().saturating_add(1).max(1)
    };

    let error = trusted_sudo_caller(&installation)
        .expect_err("mismatched effective signer GID must be denied");
    assert!(matches!(error, SignerError::AuthorizationDenied));
}

#[test]
fn admin_store_rejects_non_root_before_loading_host_installation() {
    if current_euid() == 0 {
        println!("SKIP: root eUID would reach fixed host installation; no host config was opened");
        return;
    }

    let error = AdminStore::open_fixed_root()
        .expect_err("non-root caller cannot open the root-only admin store");
    assert!(matches!(error, SignerError::AuthorizationDenied));
}

#[cfg(unix)]
mod job_path_tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use oasis7_local_signer::job::{resolve_existing_work_dir, resolve_work_dir};
    use oasis7_local_signer::types::{CallerBinding, INSTALLATION_SCHEMA, InstallationConfig};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let base = std::env::temp_dir()
                .canonicalize()
                .expect("canonical temporary directory");
            loop {
                let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
                let path = base.join(format!(
                    "oasis7-signer-qa-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => {
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                            .expect("private temp root");
                        return Self(path);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create temp root: {error}"),
                }
            }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn private_dir(path: &std::path::Path) {
        fs::create_dir_all(path).expect("create fixture directory");
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .expect("set private fixture mode");
    }

    fn installation(store_dir: &std::path::Path, other_uid: u32) -> InstallationConfig {
        let caller_uid = oasis7_local_signer::installation::current_uid().max(100);
        let current_root = store_dir.parent().unwrap().join("caller-current");
        let other_root = store_dir.parent().unwrap().join("caller-other");
        private_dir(&current_root);
        private_dir(&other_root);
        use std::os::unix::fs::MetadataExt;
        let current = fs::metadata(&current_root).unwrap();
        let other = fs::metadata(&other_root).unwrap();
        InstallationConfig {
            schema_version: INSTALLATION_SCHEMA.to_owned(),
            installation_id: "install-01".to_owned(),
            deployment_id: "testnet-a".to_owned(),
            store_dir: store_dir.display().to_string(),
            store_device_id: 0,
            store_inode: 0,
            signer_uid: 1,
            signer_gid: 2,
            callers: vec![
                CallerBinding {
                    uid: caller_uid,
                    work_dir: current_root.display().to_string(),
                    work_device_id: current.dev(),
                    work_inode: current.ino(),
                },
                CallerBinding {
                    uid: other_uid,
                    work_dir: other_root.display().to_string(),
                    work_device_id: other.dev(),
                    work_inode: other.ino(),
                },
            ],
            release_id: "release-01".to_owned(),
            worker_executable: "/usr/bin/true".to_owned(),
            worker_sha256: "44".repeat(32),
            control_schema_version: "control-v1".to_owned(),
        }
    }

    #[test]
    fn traversal_job_ids_are_rejected_without_creating_paths() {
        let temp = TempRoot::new();
        let store_dir = temp.0.join("store");
        let caller_uid = oasis7_local_signer::installation::current_uid().max(100);
        let installation = installation(&store_dir, caller_uid.saturating_add(1));

        for job_id in ["..", "../escape", "nested/job", "/tmp/outside"] {
            let error = resolve_work_dir(&installation, caller_uid, job_id)
                .expect_err("unsafe job ID must be rejected");
            assert_eq!(error.code(), "INVALID_INPUT", "job ID {job_id:?}");
        }
        assert!(!store_dir.exists(), "resolution must not create the store");
        assert!(!temp.0.join("escape").exists());
    }

    #[test]
    fn job_symlink_and_cross_caller_owned_root_are_rejected() {
        let temp = TempRoot::new();
        let store_dir = temp.0.join("store");
        let caller_uid = oasis7_local_signer::installation::current_uid().max(100);
        let other_uid = caller_uid.saturating_add(1);
        let installation = installation(&store_dir, other_uid);
        let current_root = temp.0.join("caller-current");
        let other_root = temp.0.join("caller-other");
        let target_job = current_root.join("owned-job");
        private_dir(&target_job);

        let linked_job = current_root.join("linked-job");
        symlink(&target_job, &linked_job).expect("create job symlink fixture");
        let error = resolve_existing_work_dir(&installation, caller_uid, "linked-job")
            .expect_err("symlinked job directory must be rejected");
        assert_eq!(error.code(), "INSTALLATION_DRIFT");

        // This root is intentionally created by this test process, not by the
        // other bound caller. The resolver must refuse to treat it as theirs.
        private_dir(&other_root.join("foreign-job"));
        let error = resolve_existing_work_dir(&installation, other_uid, "foreign-job")
            .expect_err("other caller must not use a root owned by this caller");
        assert_eq!(error.code(), "INSTALLATION_DRIFT");
        assert!(target_job.is_dir());
    }
}

#[test]
fn m0_signing_rejects_identity_purpose_and_provider_bound_rollback() {
    let identity = rollback_sign_request("public_testnet_identity_v2", Some("provider-01"));
    assert!(encode_request_frame(&identity).is_err());

    let provider_bound_rollback =
        rollback_sign_request("rollback_strict_audit_v1", Some("provider-01"));
    assert!(encode_request_frame(&provider_bound_rollback).is_err());
}

#[test]
fn empty_malformed_and_noncanonical_payload_encodings_are_rejected() {
    assert!(decode_payload("").is_err());
    assert!(decode_payload("YQ").is_err());
    assert!(decode_payload("YR==").is_err());
}
