//! Installation contract tests: no OS accounts, sudo, keys, or host config writes.
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use oasis7_local_signer::installation::load_installation_config;
use oasis7_local_signer::job::{resolve_existing_work_dir, resolve_work_dir};
use oasis7_local_signer::types::InstallationConfig;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "oasis7-installation-v3-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn config_json(&self) -> Value {
        let jobs = self.0.join("caller-documents").join("jobs");
        fs::create_dir_all(&jobs).unwrap();
        fs::set_permissions(&jobs, fs::Permissions::from_mode(0o700)).unwrap();
        let metadata = fs::metadata(&jobs).unwrap();
        let uid = metadata.uid();
        assert_ne!(uid, 0, "run caller workspace fixture without root");
        json!({
            "schema_version": "oasis7.local_signer_installation.v3",
            "installation_id": "installation-01", "deployment_id": "deployment-01",
            "store_dir": self.0.join("private-custody"),
            "store_device_id": 1, "store_inode": 2,
            "signer_uid": uid + 10000, "signer_gid": 700,
            "callers": [{"uid": uid, "work_dir": jobs,
                "work_device_id": metadata.dev(), "work_inode": metadata.ino()}],
            "release_id": "release-01", "worker_executable": "/usr/bin/true",
            "worker_sha256": "ab".repeat(32), "control_schema_version": "control-v1"
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn external_caller_jobs_are_independent_of_private_custody_root() {
    let fixture = Fixture::new();
    let value = fixture.config_json();
    let expected = PathBuf::from(value["callers"][0]["work_dir"].as_str().unwrap()).join("job-01");
    let uid = value["callers"][0]["uid"].as_u64().unwrap() as u32;
    let config: InstallationConfig = serde_json::from_value(value).expect("v3 caller binding");
    config.validate().expect("valid external caller workspace");
    assert_eq!(resolve_work_dir(&config, uid, "job-01").unwrap(), expected);
    assert!(!expected.starts_with(&config.store_dir));
}

#[test]
fn legacy_v2_binding_requires_explicit_operator_migration() {
    let fixture = Fixture::new();
    let mut value = fixture.config_json();
    value["schema_version"] = json!("oasis7.local_signer_installation.v2");
    let uid = value["callers"][0]["uid"].clone();
    value["callers"] = json!([{"uid": uid, "work_subdir": "caller-01"}]);
    let accepted = serde_json::from_value::<InstallationConfig>(value)
        .is_ok_and(|config| config.validate().is_ok());
    assert!(
        !accepted,
        "v2 must not silently remain an installed binding"
    );
}

#[test]
fn replacing_external_caller_leaf_denies_old_installation_identity() {
    let fixture = Fixture::new();
    let value = fixture.config_json();
    let jobs = PathBuf::from(value["callers"][0]["work_dir"].as_str().unwrap());
    let uid = value["callers"][0]["uid"].as_u64().unwrap() as u32;
    let config: InstallationConfig = serde_json::from_value(value).expect("v3 caller binding");
    fs::create_dir(jobs.join("job-01")).unwrap();
    fs::set_permissions(jobs.join("job-01"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(resolve_existing_work_dir(&config, uid, "job-01").is_ok());
    fs::rename(&jobs, jobs.with_extension("old")).unwrap();
    fs::create_dir(&jobs).unwrap();
    fs::set_permissions(&jobs, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(jobs.join("job-01")).unwrap();
    fs::set_permissions(jobs.join("job-01"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(resolve_existing_work_dir(&config, uid, "job-01").is_err());
}

#[test]
fn caller_job_symlink_cannot_redirect_into_private_custody() {
    let fixture = Fixture::new();
    let value = fixture.config_json();
    let jobs = PathBuf::from(value["callers"][0]["work_dir"].as_str().unwrap());
    let uid = value["callers"][0]["uid"].as_u64().unwrap() as u32;
    let config: InstallationConfig = serde_json::from_value(value).expect("v3 caller binding");
    let custody = fixture.0.join("private-custody");
    fs::create_dir(&custody).unwrap();
    std::os::unix::fs::symlink(&custody, jobs.join("job-01")).unwrap();
    assert!(resolve_existing_work_dir(&config, uid, "job-01").is_err());
}

#[test]
fn protected_installation_binding_rejects_symlink_ancestor() {
    let fixture = Fixture::new();
    let config = fixture.config_json();
    let actual = fixture.0.join("protected-config");
    fs::create_dir(&actual).unwrap();
    fs::write(
        actual.join("installation.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let alias = fixture.0.join("config-alias");
    std::os::unix::fs::symlink(&actual, &alias).unwrap();
    assert!(load_installation_config(&alias.join("installation.json")).is_err());
}

#[test]
fn v3_caller_root_cannot_overlap_private_custody() {
    let fixture = Fixture::new();
    let mut value = fixture.config_json();
    value["callers"][0]["work_dir"] = value["store_dir"].clone();
    let accepted = serde_json::from_value::<InstallationConfig>(value)
        .is_ok_and(|config| config.validate().is_ok());
    assert!(!accepted);
}

#[test]
fn v3_rejects_relative_traversal_duplicate_and_nested_caller_roots() {
    let fixture = Fixture::new();
    let original = fixture.config_json();
    let work = original["callers"][0]["work_dir"].as_str().unwrap();
    for bad_path in [
        "relative/jobs".to_owned(),
        format!("{work}/../jobs"),
        format!("{work}/./jobs"),
        format!("{}/jobs", original["store_dir"].as_str().unwrap()),
        fixture.0.display().to_string(),
    ] {
        let mut value = original.clone();
        value["callers"][0]["work_dir"] = json!(bad_path);
        assert!(
            !serde_json::from_value::<InstallationConfig>(value)
                .is_ok_and(|config| config.validate().is_ok()),
            "accepted invalid caller root {bad_path}"
        );
    }
    for nested in [false, true] {
        let mut value = original.clone();
        let mut second = value["callers"][0].clone();
        second["uid"] = json!(second["uid"].as_u64().unwrap() + 1);
        if nested {
            second["work_dir"] = json!(format!("{work}/nested"));
        }
        value["callers"].as_array_mut().unwrap().push(second);
        assert!(
            !serde_json::from_value::<InstallationConfig>(value)
                .is_ok_and(|config| config.validate().is_ok()),
            "accepted overlapping caller roots"
        );
    }
}
