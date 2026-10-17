use super::{model::*, random_token, service::valid_session, store::IdentityStore};
fn fixture() -> IdentityStore {
    let path = std::env::temp_dir().join(format!("identity-session-{}.sqlite", random_token()));
    IdentityStore::open(
        &path,
        IdentityNamespace {
            identity_service_id: "session-contract".into(),
            environment_id: "test".into(),
        },
        RpConfig {
            rp_id: "localhost".into(),
            allowed_origins: vec!["https://localhost".into()],
            version: 1,
            identity_instance_epoch: 4,
        },
    )
    .unwrap()
}
fn populate(store: &mut IdentityStore) {
    store
        .transaction(|tx| {
            tx.create(
                "account:a",
                "account",
                Some("a"),
                None,
                &AccountRecord {
                    account_id: "a".into(),
                    user_handle: vec![0; 16],
                    display_name: "玩家".into(),
                    status: "active".into(),
                    policy_version: 1,
                    security_epoch: 9,
                    has_ever_public_key: true,
                    created_at: 1,
                    last_authenticated_at: None,
                    last_recovery_id: None,
                },
            )?;
            for id in ["p", "q"] {
                tx.create(
                    &format!("credential:{id}"),
                    "credential",
                    Some("a"),
                    None,
                    &CredentialRecord {
                        credential_record_id: id.into(),
                        account_id: "a".into(),
                        state: "active".into(),
                        version: 1,
                        label: "key".into(),
                        created_at: 1,
                        last_used_at: None,
                        public_material: CredentialMaterial::WebAuthn {
                            rp_id: "localhost".into(),
                            credential_id: vec![id.as_bytes()[0]],
                            user_handle: vec![0; 16],
                            library_record: serde_json::json!({}),
                            library_record_version: 1,
                        },
                        registered_via: "new_account".into(),
                    },
                )?;
            }
            for (sid, cid) in [("s1", "p"), ("s2", "p"), ("s3", "q")] {
                tx.create(
                    sid,
                    "session",
                    Some("a"),
                    Some(10000),
                    &AccountSession {
                        session_id: sid.into(),
                        token_digest: sid.into(),
                        account_id: "a".into(),
                        origin_credential_id: cid.into(),
                        origin_credential_version: 1,
                        issued_security_epoch: 9,
                        identity_instance_epoch: 4,
                        auth_method: "webauthn".into(),
                        auth_time: 1,
                        expires_at: 10000,
                        idle_expires_at: 10000,
                        revoked_at: None,
                        recovery_context: None,
                        csrf_digest: "test".into(),
                    },
                )?;
            }
            Ok(())
        })
        .unwrap();
}
#[test]
fn one_device_logout_does_not_revoke_shared_passkey_other_device() {
    let mut store = fixture();
    populate(&mut store);
    store
        .transaction(|tx| {
            let mut s = tx.get::<AccountSession>("s1")?.unwrap();
            s.value.revoked_at = Some(5);
            tx.replace(
                "s1",
                s.row_version,
                "session",
                Some("a"),
                Some(10000),
                &s.value,
            )?;
            assert!(valid_session(tx, "s1", 4, 6).is_err());
            assert!(valid_session(tx, "s2", 4, 6).is_ok());
            assert!(valid_session(tx, "s3", 4, 6).is_ok());
            Ok(())
        })
        .unwrap();
}
#[test]
fn credential_revocation_invalidates_every_derived_session_only() {
    let mut store = fixture();
    populate(&mut store);
    store
        .transaction(|tx| {
            let mut c = tx.get::<CredentialRecord>("credential:p")?.unwrap();
            c.value.state = "revoked".into();
            c.value.version += 1;
            tx.replace(
                "credential:p",
                c.row_version,
                "credential",
                Some("a"),
                None,
                &c.value,
            )?;
            assert!(valid_session(tx, "s1", 4, 6).is_err());
            assert!(valid_session(tx, "s2", 4, 6).is_err());
            assert!(valid_session(tx, "s3", 4, 6).is_ok());
            Ok(())
        })
        .unwrap();
}
#[test]
fn recovery_epoch_freeze_and_instance_fence_fail_closed() {
    let mut store = fixture();
    populate(&mut store);
    store
        .transaction(|tx| {
            assert!(valid_session(tx, "s1", 5, 6).is_err());
            let mut a = tx.get::<AccountRecord>("account:a")?.unwrap();
            a.value.security_epoch += 1;
            tx.replace(
                "account:a",
                a.row_version,
                "account",
                Some("a"),
                None,
                &a.value,
            )?;
            for key in ["s1", "s2", "s3"] {
                assert!(valid_session(tx, key, 4, 6).is_err());
            }
            let mut a = tx.get::<AccountRecord>("account:a")?.unwrap();
            a.value.security_epoch = 9;
            a.value.status = "frozen".into();
            tx.replace(
                "account:a",
                a.row_version,
                "account",
                Some("a"),
                None,
                &a.value,
            )?;
            assert!(valid_session(tx, "s1", 4, 6).is_err());
            Ok(())
        })
        .unwrap();
}
