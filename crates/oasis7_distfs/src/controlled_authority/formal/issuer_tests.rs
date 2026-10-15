use super::*;
use crate::controlled_authority::fencing::EndpointInstanceBindingV1;
use crate::controlled_authority::formal::*;
use crate::controlled_authority::replicated_protocol::EndpointRole;

fn h(n: u8) -> String {
    hex::encode([n; 32])
}
fn public(n: u8) -> String {
    hex::encode(SigningKey::from_bytes(&[n; 32]).verifying_key().to_bytes())
}
pub(super) fn policy() -> RecoveryPolicyV1 {
    RecoveryPolicyV1 {
        schema_version: 1,
        recovery_issuer_public_key: public(1),
        fencing_provider_public_key: public(3),
        authorized_fencing_resource_bindings: vec![h(35), h(36)],
        issuer_anchor_public_key: public(2),
        allow_complete_prepared: true,
        allow_replay_frozen: true,
        successor_rule: SuccessorRuleV1::IncrementEpochDistinctBindings,
        terminates_at_profile_transition: true,
    }
}
fn endpoint(role: EndpointRole, n: u8) -> EndpointSignerBindingV1 {
    EndpointSignerBindingV1 {
        instance: EndpointInstanceBindingV1 {
            schema_version: 1,
            world_id: "world".into(),
            chain_id: "chain".into(),
            genesis_digest: h(8),
            authority_epoch: 2,
            endpoint_role: role,
            endpoint_id: format!("endpoint-{n}"),
            endpoint_public_key: public(n),
            instance_id: h(n + 10),
            storage_id: h(n + 20),
            storage_fingerprint: h(n + 30),
        },
        fault_domain_id: format!("domain-{n}"),
    }
}
fn certificate() -> SignedRecoveryCertificateV1 {
    let world = FormalWorldIdentityV1 {
        world_id: "world".into(),
        chain_id: "chain".into(),
        genesis_digest: h(8),
    };
    let body = RecoveryCertificateBodyV1 {
        retained_context: None,
        schema_version: 1,
        world: world.clone(),
        operation_id: h(7),
        predecessor_authority_id: h(10),
        predecessor_epoch: 1,
        sealed_cut: FormalHeadRefV1 {
            position: 4,
            record_id: h(9),
        },
        tail: RecoveryTailV1::Empty,
        fencing_evidence_digest: h(12),
        successor: AuthorityDefinitionV1 {
            schema_version: 1,
            world,
            profile: "controlled_single_authority".into(),
            profile_version: 1,
            epoch: 2,
            writer_public_key: public(4),
            primary: endpoint(EndpointRole::Primary, 5),
            replica: endpoint(EndpointRole::Replica, 6),
            recovery_policy_digest: h(11),
            predecessor_authority_id: Some(h(10)),
            valid_from_position: 5,
        },
        first_normal_position: Some(5),
    };
    let signature_hex = hex::encode(
        SigningKey::from_bytes(&[1; 32])
            .sign(&recovery_signing_bytes(&body).unwrap())
            .to_bytes(),
    );
    SignedRecoveryCertificateV1 {
        body,
        signature_hex,
    }
}
fn receipt(cert: &SignedRecoveryCertificateV1) -> SignedIssuanceWitnessV1 {
    let policy = policy();
    SignedIssuanceWitnessV1::sign(
        IssuanceWitnessBodyV1 {
            schema_version: 1,
            capability: IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1,
            request: IssuanceRequestBodyV1 {
                schema_version: 1,
                issuer_public_key: policy.recovery_issuer_public_key.clone(),
                policy_binding: policy_binding(&policy).unwrap(),
                predecessor_authority_id: cert.body.predecessor_authority_id.clone(),
                certificate_digest: recovery_certificate_digest(cert).unwrap(),
                observation_nonce: h(13),
                minimum_anchor: IssuerAnchorV1::initial(&policy).unwrap(),
            },
            grant_revision: 1,
            current_anchor: IssuerAnchorV1 {
                revision: 1,
                digest: h(14),
            },
        },
        &SigningKey::from_bytes(&[2; 32]),
    )
    .unwrap()
}
fn resign(receipt: &mut SignedIssuanceWitnessV1) {
    *receipt =
        SignedIssuanceWitnessV1::sign(receipt.body.clone(), &SigningKey::from_bytes(&[2; 32]))
            .unwrap();
}
#[test]
fn independently_signed_grant_binds_original_authority_and_certificate() {
    let cert = certificate();
    let witness = receipt(&cert);
    let token = verify_issuance_witness(&witness, &cert, &policy()).unwrap();
    assert_eq!(
        token.certificate_digest(),
        recovery_certificate_digest(&cert).unwrap()
    );
    assert_eq!(token.predecessor_authority_id(), h(10));
    assert_eq!(token.anchor_revision(), 1);
    token.verify_policy(&policy()).unwrap();
    let mut shadow_policy = policy();
    shadow_policy.issuer_anchor_public_key = public(99);
    assert_eq!(
        token.verify_policy(&shadow_policy).unwrap_err(),
        FormalError::IssuerUnproven
    );
    for mutation in 0..5 {
        let mut changed = witness.clone();
        match mutation {
            0 => changed.body.request.certificate_digest = h(99),
            1 => changed.body.request.predecessor_authority_id = h(99),
            2 => changed.body.request.policy_binding = h(99),
            3 => changed.body.grant_revision = 2,
            _ => changed.body.request.minimum_anchor.revision = 2,
        }
        resign(&mut changed);
        assert!(verify_issuance_witness(&changed, &cert, &policy()).is_err());
    }
    let mut changed = cert.clone();
    changed.body.operation_id = h(99);
    assert!(verify_issuance_witness(&witness, &changed, &policy()).is_err());
}
#[test]
fn controlled_receipts_and_rollback_floor_never_qualify() {
    let cert = certificate();
    let mut witness = receipt(&cert);
    witness.body.capability = IssuerWitnessCapabilityV1::ControlledLocalTest;
    resign(&mut witness);
    assert_eq!(
        verify_issuance_witness(&witness, &cert, &policy()).unwrap_err(),
        FormalError::IssuerUnproven
    );
    let mut witness = receipt(&cert);
    witness.body.request.minimum_anchor = IssuerAnchorV1 {
        revision: 1,
        digest: h(99),
    };
    resign(&mut witness);
    assert!(matches!(
        verify_issuance_witness(&witness, &cert, &policy()),
        Err(FormalError::Rollback(_))
    ));
    let mut witness = receipt(&cert);
    witness.signature_hex = h(1).repeat(2);
    assert!(verify_issuance_witness(&witness, &cert, &policy()).is_err());
}
#[test]
fn issuer_request_has_distinct_domain_and_exact_nonce_binding() {
    let cert = certificate();
    let witness = receipt(&cert);
    let body = witness.body.request;
    let signature_hex = hex::encode(
        SigningKey::from_bytes(&[1; 32])
            .sign(&encoded(REQUEST_DOMAIN, &body).unwrap())
            .to_bytes(),
    );
    let mut request = SignedIssuanceRequestV1 {
        body,
        signature_hex,
    };
    request.verify().unwrap();
    request.body.observation_nonce = h(99);
    assert!(request.verify().is_err());
    request.body.observation_nonce = h(13);
    request.signature_hex = cert.signature_hex;
    assert!(request.verify().is_err());
}

// Models a separately retained monotonic witness. Its old grants survive local
// issuer-directory deletion/reopen; it is never a production provider adapter.
struct Witness {
    grants: std::collections::BTreeMap<String, (SignedRecoveryCertificateV1, u64)>,
    anchors: Vec<IssuerAnchorV1>,
    lost_reply: bool,
    policy: RecoveryPolicyV1,
}
impl Witness {
    fn new(policy: &RecoveryPolicyV1) -> Self {
        Self {
            grants: Default::default(),
            anchors: vec![IssuerAnchorV1::initial(policy).unwrap()],
            lost_reply: false,
            policy: policy.clone(),
        }
    }
}
impl IssuanceWitness for Witness {
    fn query_predecessor(
        &mut self,
        query: &SignedIssuerGrantQueryV1,
    ) -> Result<SignedIssuerGrantQueryResultV1, FormalError> {
        query.verify()?;
        if query.body.issuer_public_key != self.policy.recovery_issuer_public_key
            || query.body.policy_binding != policy_binding(&self.policy)?
        {
            return Err(invalid("mock witness issuer/policy authentication"));
        }
        if self
            .anchors
            .get(query.body.minimum_anchor.revision as usize)
            != Some(&query.body.minimum_anchor)
        {
            return Err(FormalError::Rollback(
                "mock query retained floor mismatch".into(),
            ));
        }
        SignedIssuerGrantQueryResultV1::sign(
            IssuerGrantQueryResultBodyV1 {
                schema_version: 1,
                capability: IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1,
                query: query.body.clone(),
                current_anchor: self.anchors.last().unwrap().clone(),
                grant: self.grants.get(&query.body.predecessor_authority_id).map(
                    |(certificate, revision)| ExistingIssuerGrantV1 {
                        certificate: certificate.clone(),
                        grant_revision: *revision,
                    },
                ),
            },
            &SigningKey::from_bytes(&[2; 32]),
        )
    }
    fn authorize_or_query(
        &mut self,
        request: &SignedIssuanceRequestV1,
        certificate: &SignedRecoveryCertificateV1,
    ) -> Result<SignedIssuanceWitnessV1, FormalError> {
        request.verify()?;
        if request.body.issuer_public_key != self.policy.recovery_issuer_public_key
            || request.body.policy_binding != policy_binding(&self.policy)?
            || request.body.certificate_digest != recovery_certificate_digest(certificate)?
        {
            return Err(invalid(
                "mock witness issuer/policy/certificate authentication",
            ));
        }
        if self
            .anchors
            .get(request.body.minimum_anchor.revision as usize)
            != Some(&request.body.minimum_anchor)
        {
            return Err(FormalError::Rollback(
                "mock witness retained floor mismatch".into(),
            ));
        }
        let grant_revision = if let Some((original, revision)) =
            self.grants.get(&request.body.predecessor_authority_id)
        {
            if original != certificate {
                return Err(FormalError::Conflict(
                    "original predecessor grant cannot change".into(),
                ));
            }
            *revision
        } else {
            let revision = self.anchors.len() as u64;
            self.grants.insert(
                request.body.predecessor_authority_id.clone(),
                (certificate.clone(), revision),
            );
            self.anchors.push(IssuerAnchorV1 {
                revision,
                digest: digest("test-witness-anchor", &request.body)?,
            });
            revision
        };
        if self.lost_reply {
            self.lost_reply = false;
            return Err(FormalError::Io(
                "witness persisted grant but reply was lost".into(),
            ));
        }
        SignedIssuanceWitnessV1::sign(
            IssuanceWitnessBodyV1 {
                schema_version: 1,
                capability: IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1,
                request: request.body.clone(),
                grant_revision,
                current_anchor: self.anchors.last().unwrap().clone(),
            },
            &SigningKey::from_bytes(&[2; 32]),
        )
    }
}
fn directory() -> PathBuf {
    let path = fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!(
            "oasis7-issuer-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    // RecoveryIssuer creates only its own final directory beneath existing /tmp.
    assert!(!path.exists());
    path
}
#[test]
#[cfg(unix)]
fn original_intent_survives_lost_witness_reply_and_local_store_reopen() {
    let intent = crate::controlled_authority::formal::tests::recovery_fixture().0;
    let policy = intent.policy().clone();
    let initial = IssuerAnchorV1::initial(&policy).unwrap();
    let path = directory();
    let mut witness = Witness::new(&policy);
    witness.lost_reply = true;
    let mut store = RecoveryIssuer::open(
        &path,
        policy.clone(),
        SigningKey::from_bytes(&[1; 32]),
        initial.clone(),
    )
    .unwrap();
    assert!(matches!(
        store.issue(&intent, &mut witness),
        Err(FormalError::Io(_))
    ));
    assert_eq!(witness.grants.len(), 1);
    assert!(
        path.join(format!(
            "{}.intent.json",
            intent.body().predecessor_authority_id
        ))
        .exists()
    );
    assert!(
        !path
            .join(format!(
                "{}.result.json",
                intent.body().predecessor_authority_id
            ))
            .exists()
    );
    assert!(
        RecoveryIssuer::open(
            &path,
            policy.clone(),
            SigningKey::from_bytes(&[1; 32]),
            initial.clone()
        )
        .is_err()
    );
    drop(store);
    let mut store = RecoveryIssuer::open(
        &path,
        policy.clone(),
        SigningKey::from_bytes(&[1; 32]),
        initial.clone(),
    )
    .unwrap();
    let (original, _, token) = store.issue(&intent, &mut witness).unwrap();
    token.verify_policy(&policy).unwrap();
    let (retry, _, again) = store.issue(&intent, &mut witness).unwrap();
    assert_eq!(original, retry);
    assert_eq!(token.anchor_revision(), again.anchor_revision());
    assert_eq!(witness.grants.len(), 1);
    let retained = store.minimum_anchor().clone();
    drop(store);
    // A stale local snapshot cannot make the external witness forget the grant.
    fs::remove_dir_all(&path).unwrap();
    let mut store =
        RecoveryIssuer::open(&path, policy, SigningKey::from_bytes(&[1; 32]), retained).unwrap();
    assert_eq!(store.issue(&intent, &mut witness).unwrap().0, original);
    drop(store);
    fs::remove_dir_all(path).unwrap();
}
#[test]
#[cfg(unix)]
fn witness_rollback_and_stale_challenge_are_refused_after_issuance() {
    let intent = crate::controlled_authority::formal::tests::recovery_fixture().0;
    let policy = intent.policy().clone();
    let path = directory();
    let mut witness = Witness::new(&policy);
    let mut store = RecoveryIssuer::open(
        &path,
        policy.clone(),
        SigningKey::from_bytes(&[1; 32]),
        IssuerAnchorV1::initial(&policy).unwrap(),
    )
    .unwrap();
    let (_, original, _) = store.issue(&intent, &mut witness).unwrap();
    struct Stale(SignedIssuanceWitnessV1, Witness);
    impl IssuanceWitness for Stale {
        fn query_predecessor(
            &mut self,
            query: &SignedIssuerGrantQueryV1,
        ) -> Result<SignedIssuerGrantQueryResultV1, FormalError> {
            self.1.query_predecessor(query)
        }
        fn authorize_or_query(
            &mut self,
            _: &SignedIssuanceRequestV1,
            _: &SignedRecoveryCertificateV1,
        ) -> Result<SignedIssuanceWitnessV1, FormalError> {
            Ok(self.0.clone())
        }
    }
    assert!(store.issue(&intent, &mut Stale(original, witness)).is_err());
    let mut rolled_back = Witness::new(&policy);
    assert!(matches!(
        store.issue(&intent, &mut rolled_back),
        Err(FormalError::Rollback(_))
    ));
    drop(store);
    fs::remove_dir_all(path).unwrap();
}
#[test]
#[cfg(unix)]
fn restored_old_store_wrong_intent_cannot_block_original_external_grant() {
    let original = crate::controlled_authority::formal::tests::recovery_fixture().0;
    let wrong = crate::controlled_authority::formal::tests::alternate_recovery_fixture();
    assert_eq!(
        original.body().predecessor_authority_id,
        wrong.body().predecessor_authority_id
    );
    assert_ne!(original.body(), wrong.body());
    let policy = original.policy().clone();
    let path = directory();
    let mut witness = Witness::new(&policy);
    let mut store = RecoveryIssuer::open(
        &path,
        policy.clone(),
        SigningKey::from_bytes(&[1; 32]),
        IssuerAnchorV1::initial(&policy).unwrap(),
    )
    .unwrap();
    let (certificate, _, _) = store.issue(&original, &mut witness).unwrap();
    let floor = store.minimum_anchor().clone();
    drop(store);
    fs::remove_dir_all(&path).unwrap();
    let mut restored = RecoveryIssuer::open(
        &path,
        policy.clone(),
        SigningKey::from_bytes(&[1; 32]),
        floor.clone(),
    )
    .unwrap();
    assert!(matches!(
        restored.issue(&wrong, &mut witness),
        Err(FormalError::Conflict(_))
    ));
    assert_eq!(
        restored.issue(&original, &mut witness).unwrap().0,
        certificate
    );
    assert_eq!(witness.grants.len(), 1);
    drop(restored);
    // Also reconcile an old locally persisted, never-granted alternative.
    let wrong_certificate = SignedRecoveryCertificateV1 {
        body: wrong.body().clone(),
        signature_hex: hex::encode(
            SigningKey::from_bytes(&[1; 32])
                .sign(&recovery_signing_bytes(wrong.body()).unwrap())
                .to_bytes(),
        ),
    };
    let path_to_intent = path.join(format!(
        "{}.intent.json",
        original.body().predecessor_authority_id
    ));
    atomic(
        &path,
        &path_to_intent,
        &serde_json::to_vec(&Intent {
            policy: policy.clone(),
            certificate: wrong_certificate.clone(),
        })
        .unwrap(),
    )
    .unwrap();
    let mut restored =
        RecoveryIssuer::open(&path, policy, SigningKey::from_bytes(&[1; 32]), floor).unwrap();
    assert_eq!(
        restored.issue(&original, &mut witness).unwrap().0,
        certificate
    );
    assert!(
        path.join(format!(
            "{}.orphan-intent.json",
            recovery_certificate_digest(&wrong_certificate).unwrap()
        ))
        .exists()
    );
    assert_eq!(witness.grants.len(), 1);
    drop(restored);
    fs::remove_dir_all(path).unwrap();
}
