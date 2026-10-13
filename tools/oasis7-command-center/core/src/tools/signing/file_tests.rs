use super::*;
#[test]
fn replacement_after_admission_cannot_become_the_prepared_source() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("selected");
    fs::write(&path, b"original").unwrap();
    let admitted = selected_file_identity(&path, 1024).unwrap();
    // Deterministic pause after admission: same bytes, different inode still differs.
    let replacement = temp.path().join("replacement");
    fs::write(&replacement, b"original").unwrap();
    fs::rename(&replacement, &path).unwrap();
    let actual = selected_file_identity(&path, 1024).unwrap();
    assert_ne!(admitted, actual);
    let resource = "signer:installation:job";
    let fingerprint = |source: &(String, u64, u64, u64)| {
        identity::sha256_hex(&identity::canonical_json(source).unwrap())
    };
    assert!(
        require_prepare_identity(
            resource,
            &fingerprint(&actual),
            resource,
            &fingerprint(&admitted)
        )
        .is_err()
    );
    fs::write(&path, b"replacement payload").unwrap();
    let actual = selected_file_identity(&path, 1024).unwrap();
    assert!(
        require_prepare_identity(
            resource,
            &fingerprint(&actual),
            resource,
            &fingerprint(&admitted)
        )
        .is_err()
    );
    assert!(
        require_prepare_identity(
            resource,
            &fingerprint(&admitted),
            "signer:replacement:job",
            &fingerprint(&admitted)
        )
        .is_err()
    );
}
#[test]
fn prepare_input_validation_rejects_correctable_inputs_before_job_creation() {
    let base = FileSignInput {
        job_id: "job-one".into(),
        file_path: "/selected/file".into(),
        signer_id: "signer-one".into(),
        grant_id: "grant-one".into(),
        context: serde_json::json!({
            "deployment_id":"deployment-one","network_id":"network-one",
            "task_uid":"task-one","source_head_oid":"a".repeat(40),
            "protocol_context":{"authority_id":"authority-one","rollback_ticket":"ticket-one","receipt_id":"receipt-one","nonce":"nonce-one"}
        }),
    };
    assert!(validate_prepare_input(&base, "deployment-one").is_ok());
    assert!(validate_prepare_input(&base, "different-deployment").is_err());
    let mut invalid = base.clone();
    invalid.context = serde_json::Value::Null;
    assert!(validate_prepare_input(&invalid, "deployment-one").is_err());
    invalid = base.clone();
    invalid.context["source_head_oid"] = "invalid-oid".into();
    assert!(validate_prepare_input(&invalid, "deployment-one").is_err());
    invalid = base.clone();
    invalid.grant_id.clear();
    assert!(validate_prepare_input(&invalid, "deployment-one").is_err());
    invalid = base.clone();
    invalid.job_id = "../../another-job".into();
    assert!(validate_prepare_input(&invalid, "deployment-one").is_err());
    invalid = base;
    invalid.file_path = "relative-file".into();
    assert!(validate_prepare_input(&invalid, "deployment-one").is_err());
}
fn rejected_input() -> FileSignInput {
    FileSignInput {
        job_id: "job-one".into(),
        file_path: "/missing/file".into(),
        signer_id: "signer-one".into(),
        grant_id: "grant-one".into(),
        context: serde_json::Value::Null,
    }
}
#[test]
fn no_effect_failure_claim_prevents_same_id_from_starting_prepare() {
    let jobs = jobs::ToolJobs::open(":memory:").unwrap();
    let error = record_prepare_validation_failure(
        &jobs,
        "operation-one",
        &rejected_input(),
        Some("signer:installation:job-one"),
        "invalid context",
    );
    assert!(matches!(error.effect, FilePrepareEffect::None));
    assert_eq!(jobs.list().unwrap()[0].state, "failed");
    assert!(
        jobs.admit_once(
            "operation-one",
            "prepared-fingerprint",
            "signer:installation:job-one",
            "file_prepare"
        )
        .is_err()
    );
    assert!(
        jobs.admit_once(
            "corrected-operation",
            "prepared-fingerprint",
            "signer:installation:job-one",
            "file_prepare"
        )
        .unwrap()
        .1
    );
}
#[test]
fn rejected_preflight_cannot_label_original_unknown_operation_no_effect() {
    let jobs = jobs::ToolJobs::open(":memory:").unwrap();
    jobs.admit_once(
        "operation-one",
        "prepared-fingerprint",
        "signer:installation:job-one",
        "file_prepare",
    )
    .unwrap();
    jobs.finish("operation-one", "unknown", None).unwrap();
    let error = record_prepare_validation_failure(
        &jobs,
        "operation-one",
        &rejected_input(),
        Some("signer:installation:job-one"),
        "file disappeared",
    );
    assert!(matches!(error.effect, FilePrepareEffect::Unknown));
    assert_eq!(jobs.list().unwrap()[0].state, "unknown");
}
#[test]
fn concurrent_preflight_failure_and_prepare_cannot_both_claim_same_id() {
    use std::sync::{Arc, Barrier};
    let jobs = Arc::new(jobs::ToolJobs::open(":memory:").unwrap());
    let barrier = Arc::new(Barrier::new(2));
    let prepare_jobs = jobs.clone();
    let prepare_barrier = barrier.clone();
    let prepare = std::thread::spawn(move || {
        prepare_barrier.wait();
        prepare_jobs.admit_once(
            "operation-one",
            "prepared-fingerprint",
            "signer:installation:job-one",
            "file_prepare",
        )
    });
    barrier.wait();
    let failure = record_prepare_validation_failure(
        &jobs,
        "operation-one",
        &rejected_input(),
        Some("signer:installation:job-one"),
        "invalid context",
    );
    let preparation = prepare.join().unwrap();
    match failure.effect {
        FilePrepareEffect::None => assert!(preparation.is_err()),
        FilePrepareEffect::Unknown => assert!(preparation.unwrap().1),
    }
}
#[test]
fn different_operation_cannot_unlock_running_or_unknown_job() {
    for state in ["running", "unknown"] {
        let jobs = jobs::ToolJobs::open(":memory:").unwrap();
        jobs.admit_once(
            "old-operation",
            "original-input",
            "signer:installation:job-one",
            "file_prepare",
        )
        .unwrap();
        if state == "unknown" {
            jobs.finish("old-operation", state, None).unwrap();
        }
        let failure = record_prepare_validation_failure(
            &jobs,
            "new-operation",
            &rejected_input(),
            Some("signer:installation:job-one"),
            "invalid context",
        );
        assert!(matches!(failure.effect, FilePrepareEffect::Unknown));
        let observed = jobs.list().unwrap();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].id, "old-operation");
        assert_eq!(observed[0].state, state);
    }
}
#[test]
fn valid_job_with_unresolved_installation_is_never_declared_no_effect() {
    let jobs = jobs::ToolJobs::open(":memory:").unwrap();
    let failure = record_prepare_validation_failure(
        &jobs,
        "new-operation",
        &rejected_input(),
        None,
        "binding unavailable",
    );
    assert!(matches!(failure.effect, FilePrepareEffect::Unknown));
    assert!(jobs.list().unwrap().is_empty());
    let mut invalid_id = rejected_input();
    invalid_id.job_id = "../../another-job".into();
    assert!(file_prepare_resource(&invalid_id.job_id).unwrap().is_none());
    let failure = record_prepare_validation_failure(
        &jobs,
        "invalid-job-operation",
        &invalid_id,
        None,
        "invalid job",
    );
    assert!(matches!(failure.effect, FilePrepareEffect::None));
}
#[test]
fn concurrent_valid_prepare_observes_failure_reservation_before_starting() {
    use std::sync::{Arc, Barrier};
    let jobs = Arc::new(jobs::ToolJobs::open(":memory:").unwrap());
    let barrier = Arc::new(Barrier::new(2));
    let worker_jobs = jobs.clone();
    let worker_barrier = barrier.clone();
    let prepare = std::thread::spawn(move || {
        worker_barrier.wait();
        let admission = worker_jobs.admit_once(
            "valid-operation",
            "valid-input",
            "signer:installation:job-one",
            "file_prepare",
        );
        // An accepted valid prepare can only follow a completed no-effect
        // failure, or win first and force that failure to remain unknown.
        let observed = worker_jobs.list().unwrap();
        (admission, observed)
    });
    barrier.wait();
    let failure = record_prepare_validation_failure(
        &jobs,
        "invalid-operation",
        &rejected_input(),
        Some("signer:installation:job-one"),
        "invalid context",
    );
    let (admission, observed) = prepare.join().unwrap();
    match failure.effect {
        FilePrepareEffect::Unknown => assert!(admission.unwrap().1),
        FilePrepareEffect::None => {
            if admission.is_ok() {
                assert_eq!(
                    observed
                        .iter()
                        .find(|job| job.id == "invalid-operation")
                        .unwrap()
                        .state,
                    "failed"
                );
            }
        }
    }
    assert!(
        jobs.list()
            .unwrap()
            .iter()
            .filter(|job| matches!(job.state.as_str(), "running" | "unknown"))
            .count()
            <= 1
    );
}
#[test]
fn changed_original_file_cannot_reuse_prepared_digest() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    fs::write(&path, b"original").unwrap();
    let (digest, size, file_device_id, file_inode) =
        selected_file_identity(&path, 1024 * 1024 * 1024).unwrap();
    let evidence = FileSourceEvidence {
        schema_version: 1,
        file_path: path.display().to_string(),
        file_sha256: digest,
        file_size_bytes: size,
        file_device_id,
        file_inode,
    };
    assert!(source_is_current(&evidence).unwrap());
    fs::write(&path, b"modified").unwrap();
    assert!(!source_is_current(&evidence).unwrap());
    fs::write(&path, b"original").unwrap();
    assert!(source_is_current(&evidence).unwrap());
    let replacement = temp.path().join("replacement");
    fs::write(&replacement, b"original").unwrap();
    fs::rename(&replacement, &path).unwrap();
    assert!(!source_is_current(&evidence).unwrap());
}
#[test]
fn rejects_fifo_and_symlink_without_waiting() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let fifo = root.path().join("fifo");
    let name = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(file_digest(&fifo).is_err());
    let file = root.path().join("file");
    fs::write(&file, b"actual").unwrap();
    let alias = root.path().join("alias");
    symlink(&file, &alias).unwrap();
    assert!(file_digest(&alias).is_err());
    assert_eq!(
        file_digest(&file).unwrap(),
        (identity::sha256_hex(b"actual"), 6)
    );
}
