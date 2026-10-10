use crate::{
    admin::AdminStore,
    installation::{current_egid, current_uid},
    key_management::require_active_key,
    types::{CallerBinding, INSTALLATION_SCHEMA, InstallationConfig},
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};
struct Fixture {
    root: PathBuf,
    admin: AdminStore,
}
impl Fixture {
    fn new() -> Self {
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).unwrap();
        let root = std::env::current_dir()
            .unwrap()
            .join(format!(".key-manager-test-{}", hex::encode(random)));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for relative in [
            "control",
            "control/grants",
            "control/revoked-grants",
            "keys",
            "state",
            "state/records",
            "backup-staging",
        ] {
            let path = root.join(relative);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let metadata = fs::metadata(&root).unwrap();
        let uid = current_uid();
        let gid = current_egid();
        let installation = InstallationConfig {
            schema_version: INSTALLATION_SCHEMA.into(),
            installation_id: "install-01".into(),
            deployment_id: "deployment-01".into(),
            store_dir: root.to_str().unwrap().into(),
            store_device_id: metadata.dev(),
            store_inode: metadata.ino(),
            signer_uid: uid,
            signer_gid: gid,
            callers: vec![CallerBinding {
                uid: uid + 1,
                work_dir: "/caller-jobs".into(),
                work_device_id: 1,
                work_inode: 1,
            }],
            release_id: "release-01".into(),
            worker_executable: "/usr/local/libexec/test-worker".into(),
            worker_sha256: "aa".repeat(32),
            control_schema_version: "control-v1".into(),
        };
        Self {
            root,
            admin: AdminStore::from_config_fixture(installation),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn managed_metadata_state_and_tombstone_enforce_lifecycle() {
    let f = Fixture::new();
    let key = f
        .admin
        .key_create_managed("file-key", "file_ed25519_v1", "Files", false)
        .unwrap();
    assert_eq!(key.state, "active");
    assert!(!key.exportable);
    let updated = f
        .admin
        .key_update(
            "file-key",
            "Release files",
            "Audit key",
            vec!["release".into()],
        )
        .unwrap();
    assert_eq!(updated.description, "Audit key");
    assert_eq!(updated.tags, vec!["release"]);
    assert!(
        f.admin
            .key_export("file-key", b"long-test-passphrase")
            .is_err()
    );
    f.admin.key_set_state("file-key", "inactive").unwrap();
    assert!(
        require_active_key(
            &f.root,
            current_uid(),
            current_egid(),
            "install-01",
            "file-key",
            "file_ed25519_v1"
        )
        .is_err()
    );
    assert!(
        require_active_key(
            &f.root,
            current_uid(),
            current_egid(),
            "install-01",
            "file-key",
            "rollback_strict_audit_v1"
        )
        .is_err()
    );
    f.admin.key_set_state("file-key", "deleted").unwrap();
    assert!(!f.root.join("keys/file-key/seed.bin").exists());
    assert_eq!(
        f.admin.key_get("file-key").unwrap().public_key_hex,
        key.public_key_hex
    );
    assert!(f.admin.key_set_state("file-key", "active").is_err());
    assert!(
        f.admin
            .key_create_managed("file-key", "file_ed25519_v1", "Again", false)
            .is_err()
    );
    assert!(
        f.admin
            .key_events()
            .unwrap()
            .iter()
            .any(|event| event.action == "key-state")
    );
}
#[test]
fn encrypted_import_is_inactive_nonexportable_and_rejects_duplicates() {
    let source = Fixture::new();
    let target = Fixture::new();
    let key = source
        .admin
        .key_create_managed("source-key", "file_ed25519_v1", "Portable", true)
        .unwrap();
    let encrypted = source
        .admin
        .key_export("source-key", b"long-test-passphrase")
        .unwrap();
    assert!(
        target
            .admin
            .key_import("bad-pass", &encrypted, b"wrong-test-passphrase", false)
            .is_err()
    );
    assert!(target.admin.key_list().unwrap().is_empty());
    let imported = target
        .admin
        .key_import("import-key", &encrypted, b"long-test-passphrase", false)
        .unwrap();
    assert_eq!(imported.public_key_hex, key.public_key_hex);
    assert_eq!(imported.state, "inactive");
    assert!(!imported.exportable);
    assert!(
        target
            .admin
            .key_export("import-key", b"long-test-passphrase")
            .is_err()
    );
    assert!(
        target
            .admin
            .key_import("duplicate-key", &encrypted, b"long-test-passphrase", false)
            .is_err()
    );
}
#[test]
fn rotation_keeps_old_public_history_and_does_not_rebind_policy() {
    let f = Fixture::new();
    let old = f
        .admin
        .key_create_managed("old-key", "file_ed25519_v1", "Files", false)
        .unwrap();
    let new = f.admin.key_rotate("old-key", "new-key").unwrap();
    assert_eq!(new.rotated_from.as_deref(), Some("old-key"));
    assert_ne!(old.public_key_hex, new.public_key_hex);
    assert_eq!(f.admin.key_get("old-key").unwrap().state, "inactive");
    assert!(!f.root.join("control/policy.json").exists());
    assert_eq!(f.admin.key_list().unwrap().len(), 2);
}

#[test]
fn legacy_creation_cannot_create_an_uncatalogued_generic_key() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .admin
            .create_key("generic-key", "file_ed25519_v1")
            .is_err()
    );
    assert!(!fixture.root.join("keys/generic-key").exists());
    assert!(!fixture.root.join("control/key-catalog.json").exists());
    assert!(!fixture.root.join("state/maintenance.json").exists());
    assert!(fixture.admin.key_list().unwrap().is_empty());
    let managed = fixture
        .admin
        .key_create_managed("generic-key", "file_ed25519_v1", "Files", false)
        .unwrap();
    assert_eq!(managed.purpose, "file_ed25519_v1");
    assert!(fixture.root.join("control/key-catalog.json").exists());
}
