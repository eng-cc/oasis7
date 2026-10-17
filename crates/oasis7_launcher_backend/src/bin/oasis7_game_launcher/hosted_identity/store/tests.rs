use super::*;
fn config() -> (IdentityNamespace, RpConfig) {
    (
        IdentityNamespace {
            identity_service_id: "test".into(),
            environment_id: "isolated".into(),
        },
        RpConfig {
            rp_id: "localhost".into(),
            allowed_origins: vec!["http://localhost:4173".into()],
            version: 1,
            identity_instance_epoch: 1,
        },
    )
}
fn path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oasis7-identity-{}.sqlite", uuid::Uuid::new_v4()))
}
fn open(path: &Path) -> IdentityStore {
    let (ns, rp) = config();
    IdentityStore::open(path, ns, rp).unwrap()
}
#[test]
fn rollback_and_conflict_do_not_leave_partial_security_effects() {
    let p = path();
    let mut store = open(&p);
    let result = store.transaction(|tx| {
        tx.create("account:a", "account", Some("a"), None, &"created")?;
        tx.create("account:a", "account", Some("a"), None, &"duplicate")
    });
    assert!(matches!(result, Err(StoreError::Conflict)));
    assert!(
        store
            .transaction(|tx| tx.get::<String>("account:a"))
            .unwrap()
            .is_none()
    );
    store
        .transaction(|tx| tx.create("account:a", "account", Some("a"), None, &"created"))
        .unwrap();
    let result = store.transaction(|tx| {
        tx.create("receipt:r", "receipt", Some("a"), None, &"should roll back")?;
        tx.replace("account:a", 2, "account", Some("a"), None, &"stale")
    });
    assert!(matches!(result, Err(StoreError::Conflict)));
    assert!(
        store
            .transaction(|tx| tx.get::<String>("receipt:r"))
            .unwrap()
            .is_none()
    );
}
#[test]
fn namespace_and_rp_mismatch_never_initialize_empty_authority() {
    let p = path();
    drop(open(&p));
    let (ns, mut rp) = config();
    rp.rp_id = "another.invalid".into();
    assert!(matches!(
        IdentityStore::open(&p, ns, rp),
        Err(StoreError::Rejected(_))
    ));
}
#[test]
fn independent_connections_serialize_single_challenge_consumption() {
    let p = path();
    let mut initial = open(&p);
    initial
        .transaction(|tx| tx.create("ceremony:c", "ceremony", None, None, &0u64))
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut s = open(&p);
                barrier.wait();
                s.transaction(|tx| {
                    let row = tx.get::<u64>("ceremony:c")?.unwrap();
                    if row.value != 0 {
                        return Err(StoreError::Rejected("consumed".into()));
                    }
                    tx.replace("ceremony:c", row.row_version, "ceremony", None, None, &1u64)?;
                    tx.create("receipt:once", "receipt", None, None, &true)
                })
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        initial
            .transaction(|tx| tx.get::<u64>("ceremony:c"))
            .unwrap()
            .unwrap()
            .value,
        1
    );
}
#[test]
fn legacy_import_preserves_ids_is_idempotent_and_source_is_read_only() {
    let p = path();
    let source = p.with_extension("json");
    let raw = r#"{"next_account_sequence":7,"next_player_sequence":9,"accounts_by_id":{"oasis-account-00000007":{"hosted_account_id":"oasis-account-00000007","player_id":"hosted-player-account-00000009","login_channel":"email","normalized_login_hint":"a+tag@example.test","status":"active","created_at_unix_ms":10,"last_verified_at_unix_ms":20}},"account_id_by_factor":{"email:a+tag@example.test":"oasis-account-00000007"}}"#;
    std::fs::write(&source, raw).unwrap();
    let mut s = open(&p);
    let protector = |_: &str| {
        Ok(super::super::model::CredentialMaterial::Email {
            encrypted_address: "ciphertext".into(),
            lookup_hmac: "secret-hmac".into(),
            normalization_version: 1,
            verified_at: 20,
            bound_at: 10,
            recovery_enabled: true,
            recovery_eligible_since: 10,
        })
    };
    assert!(
        s.migrate_legacy_json(&source, "world:a", 100, false, protector)
            .is_err()
    );
    assert_eq!(
        s.migrate_legacy_json(&source, "world:a", 100, true, protector)
            .unwrap(),
        1
    );
    assert_eq!(
        s.migrate_legacy_json(&source, "world:a", 200, true, protector)
            .unwrap(),
        1
    );
    assert_eq!(std::fs::read_to_string(source).unwrap(), raw);
    let a = s
        .transaction(|tx| {
            tx.get::<super::super::model::AccountRecord>("account:oasis-account-00000007")
        })
        .unwrap()
        .unwrap()
        .value;
    assert!(!a.has_ever_public_key);
    assert_eq!(a.user_handle.len(), 16);
    let binding = s
        .transaction(|tx| {
            tx.get::<super::super::model::PlayerBinding>("binding:oasis-account-00000007:world:a")
        })
        .unwrap()
        .unwrap()
        .value;
    assert_eq!(binding.player_id, "hosted-player-account-00000009");
}
#[test]
fn consistent_backup_includes_wal_and_epoch_advance_requires_verified_maintenance() {
    let p = path();
    let mut s = open(&p);
    s.transaction(|tx| tx.create("account:a", "account", Some("a"), None, &"from-wal"))
        .unwrap();
    let backup = path();
    s.backup(&backup).unwrap();
    let mut restored = open(&backup);
    assert_eq!(
        restored
            .transaction(|tx| tx.get::<String>("account:a"))
            .unwrap()
            .unwrap()
            .value,
        "from-wal"
    );
    drop(restored);
    let (ns, mut rp) = config();
    rp.identity_instance_epoch = 2;
    assert!(IdentityStore::advance_instance_epoch(&backup, &ns, &rp, true, false).is_err());
    assert!(IdentityStore::advance_instance_epoch(&backup, &ns, &rp, false, true).is_err());
    IdentityStore::advance_instance_epoch(&backup, &ns, &rp, true, true).unwrap();
    let (old_ns, old_rp) = config();
    assert!(IdentityStore::open(&backup, old_ns, old_rp).is_err());
    assert!(IdentityStore::open(&backup, ns, rp).is_ok());
}
#[test]
fn expiry_cleanup_is_bounded_and_retains_accounts_and_credential_tombstones() {
    let p = path();
    let mut s = open(&p);
    s.transaction(|tx| {
        tx.create("account:a", "account", Some("a"), None, &"persistent")?;
        tx.create(
            "credential:revoked",
            "credential",
            Some("a"),
            None,
            &"tombstone",
        )?;
        for i in 0..120 {
            tx.create(
                &format!("receipt:{i:04}"),
                "receipt",
                Some("a"),
                Some(100),
                &true,
            )?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(s.cleanup_expired(99).unwrap(), 0);
    assert_eq!(s.cleanup_expired(100).unwrap(), 100);
    assert_eq!(s.cleanup_expired(100).unwrap(), 20);
    assert!(
        s.transaction(|tx| tx.get::<String>("account:a"))
            .unwrap()
            .is_some()
    );
    assert!(
        s.transaction(|tx| tx.get::<String>("credential:revoked"))
            .unwrap()
            .is_some()
    );
}
