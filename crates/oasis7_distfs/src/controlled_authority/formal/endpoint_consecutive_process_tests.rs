//! E2 loses its endpoints before first completion; E3 preserves E1's candidate.
//! All external custody and issuer witnesses here are explicitly simulations.
use super::*;
#[expect(
    clippy::too_many_arguments,
    reason = "The process fixture names all independently bound predecessor and successor evidence."
)]
pub(super) fn consecutive(
    fixture: &Fixture,
    binary: &std::ffi::OsStr,
    recovery2: &VerifiedRecovery,
    cut: &VerifiedFormalCommitV1,
    anchor: &VerifiedFormalReadAnchorV1,
    candidate: &FormalCandidateMaterialV1,
    tail: &RecoveryTailWitnessV1,
    p2addr: std::net::SocketAddr,
    r2addr: std::net::SocketAddr,
    configs2: &[std::path::PathBuf],
) {
    use crate::controlled_authority::formal::issuer::*;
    let mut p2 = remote(
        p2addr,
        recovery2.successor(),
        EndpointRole::Primary,
        10,
        cut.head(),
    );
    let mut r2 = remote(
        r2addr,
        recovery2.successor(),
        EndpointRole::Replica,
        10,
        cut.head(),
    );
    let pc = p2.checkpoint().unwrap();
    let rc = r2.checkpoint().unwrap();
    let ps = p2.seal_epoch(h(244), 3, pc).unwrap();
    let rs = r2.seal_epoch(h(244), 3, rc).unwrap();
    let pm = p2.archive_manifest().unwrap();
    let rm = r2.archive_manifest().unwrap();
    let policy = TrustedFencingPolicyV1 {
        provider_public_key: public(3),
        authorized_resource_bindings: fixture
            .authority
            .policy()
            .authorized_fencing_resource_bindings
            .clone(),
    };
    let pv = verify_epoch_seal(&ps, &policy).unwrap();
    let rv = verify_epoch_seal(&rs, &policy).unwrap();
    let context =
        VerifiedActiveRecoveryContextV2::from_previous(recovery2, &fixture.authority, cut, tail)
            .unwrap();
    let pdir = fixture.path.join("epoch3-p");
    let rdir = fixture.path.join("epoch3-r");
    for dir in [&pdir, &rdir] {
        fs::create_dir(dir).unwrap();
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut successor = recovery2.successor().definition().clone();
    successor.epoch = 3;
    successor.writer_public_key = public(13);
    successor.predecessor_authority_id = Some(recovery2.successor().authority_id().unwrap());
    for (binding, dir, n) in [
        (&mut successor.primary, &pdir, 14u8),
        (&mut successor.replica, &rdir, 15u8),
    ] {
        binding.instance.authority_epoch = 3;
        binding.instance.endpoint_id = format!("epoch3-{n}");
        binding.instance.endpoint_public_key = public(n);
        binding.instance.instance_id = h(n + 20);
        binding.instance.storage_id = h(n + 30);
        binding.instance.storage_fingerprint = directory_fingerprint(dir).unwrap();
        binding.fault_domain_id = format!("epoch3-domain-{n}");
    }
    let body = RecoveryCertificateBodyV1 {
        schema_version: 2,
        world: successor.world.clone(),
        operation_id: h(244),
        predecessor_authority_id: recovery2.successor().authority_id().unwrap(),
        predecessor_epoch: 2,
        sealed_cut: cut.head(),
        tail: recovery2.body().tail.clone(),
        fencing_evidence_digest: recovery_fencing_digest([&pv, &rv]).unwrap(),
        successor,
        first_normal_position: Some(3),
        retained_context: Some(context.commitment().clone()),
    };
    let intent =
        VerifiedRecoveryIntent::verify_retained(body.clone(), &context, [&pv, &rv]).unwrap();
    let certificate = SignedRecoveryCertificateV1 {
        signature_hex: signed(recovery_signing_bytes(&body).unwrap(), 1),
        body,
    };
    let config2: serde_json::Value =
        serde_json::from_slice(&fs::read(&configs2[0]).unwrap()).unwrap();
    let prior_witness: SignedIssuanceWitnessV1 =
        serde_json::from_value(config2["recovery_bootstrap"]["issuer_witness"].clone()).unwrap();
    let minimum = prior_witness.body.current_anchor.clone();
    let policy_binding = IssuerAnchorV1::initial(fixture.authority.policy())
        .unwrap()
        .digest;
    let witness = SignedIssuanceWitnessV1::sign(
        IssuanceWitnessBodyV1 {
            schema_version: 1,
            capability: IssuerWitnessCapabilityV1::NonrollbackUniqueSuccessorV1,
            request: IssuanceRequestBodyV1 {
                schema_version: 1,
                issuer_public_key: public(1),
                policy_binding,
                predecessor_authority_id: recovery2.successor().authority_id().unwrap(),
                certificate_digest: recovery_certificate_digest(&certificate).unwrap(),
                observation_nonce: h(245),
                minimum_anchor: minimum.clone(),
            },
            grant_revision: 2,
            current_anchor: IssuerAnchorV1 {
                revision: 2,
                digest: h(246),
            },
        },
        &key(2),
    )
    .unwrap();
    let issuance =
        verify_issuance_witness(&witness, &certificate, fixture.authority.policy()).unwrap();
    let recovery3 =
        VerifiedRecovery::verify(&certificate, &intent, recovery2.successor(), &issuance).unwrap();
    let active_anchor = anchor.with_pending_recovery(recovery2).unwrap();
    let mut prior = config2["recovery_bootstrap"].clone();
    prior.as_object_mut().unwrap().remove("archive_manifest");
    prior.as_object_mut().unwrap().remove("archive_directory");
    // Supply complete hash CAS closures. Copying bytes grants no permission:
    // the E2 protected seal/index roots remain the sole archive authority.
    let old_p = fixture.path.join("epoch2-p");
    let old_r = fixture.path.join("epoch2-r");
    for (source, dest) in [(&fixture.pdir, &old_p), (&fixture.rdir, &old_r)] {
        copy_private_fixture_tree(
            &source.join("formal-objects-v1"),
            &dest.join("formal-objects-v1"),
        );
    }
    let pt = verify_formal_archive_transfer(
        &pm,
        recovery2.successor(),
        Arc::new(FileFormalArchiveSourceV1::new(old_p.clone()).unwrap()),
    )
    .unwrap();
    let rt = verify_formal_archive_transfer(
        &rm,
        recovery2.successor(),
        Arc::new(FileFormalArchiveSourceV1::new(old_r.clone()).unwrap()),
    )
    .unwrap();
    let hot = |role, transfer| {
        super::super::super::super::protected_signer::initial_hot(
            recovery3.successor(),
            &fixture.authority,
            &Some(recovery3.clone()),
            &Some(cut.clone()),
            match role {
                EndpointRole::Primary => &recovery3.successor().definition().primary.instance,
                EndpointRole::Replica => &recovery3.successor().definition().replica.instance,
            },
            Some(transfer),
        )
        .unwrap()
        .0
    };
    let po = Oracle::start_registered(
        recovery3.successor().clone(),
        EndpointRole::Primary,
        key(14),
        policy.authorized_resource_bindings[0].clone(),
        Some(hot(EndpointRole::Primary, &pt)),
    );
    let ro = Oracle::start_registered(
        recovery3.successor().clone(),
        EndpointRole::Replica,
        key(15),
        policy.authorized_resource_bindings[0].clone(),
        Some(hot(EndpointRole::Replica, &rt)),
    );
    let mut configs3 = vec![];
    for (role, dir, oracle, manifest, archive) in [
        (EndpointRole::Primary, &pdir, &po, &pm, &old_p),
        (EndpointRole::Replica, &rdir, &ro, &rm, &old_r),
    ] {
        let config = fixture.path.join(format!("epoch3-{role:?}.json"));
        fs::write(&config,serde_json::to_vec(&serde_json::json!({"schema_version":2,"initial_trust":config2["initial_trust"],"initial_certificate":config2["initial_certificate"],"initial_qualified":null,
            "role":role,"data_dir":dir,"oracle_authentication_key_file":config2["oracle_authentication_key_file"],"oracle_address":oracle.address,"listen":"127.0.0.1:0","timeout_ms":3000,
            "recovery_bootstrap":{"predecessor_checkpoint":active_anchor.checkpoint(),"predecessor_checkpoint_digest":active_anchor.checkpoint().digest().unwrap(),"certificate":certificate,
                "fencing":{"Dual":{"primary":ps,"replica":rs}},"tail":tail,"issuer_witness":witness,"minimum_issuer_anchor":minimum,"archive_manifest":manifest,"archive_directory":archive,"prior_permission":prior}})).unwrap()).unwrap();
        configs3.push(config);
    }
    let (mut pc, paddr) = start(binary, &configs3[0]);
    let (mut rc, raddr) = start(binary, &configs3[1]);
    let mut coordinator = FormalCoordinator::new(
        remote(
            paddr,
            recovery3.successor(),
            EndpointRole::Primary,
            13,
            cut.head(),
        ),
        remote(
            raddr,
            recovery3.successor(),
            EndpointRole::Replica,
            13,
            cut.head(),
        ),
        recovery3.successor().clone(),
        fixture.authority.clone(),
        Some(recovery3.clone()),
        Some(cut.clone()),
    )
    .unwrap();
    let mut retained = candidate.clone();
    retained.branch = FormalCompletionBranchV1::CompletePrepared;
    let result = coordinator.submit(retained.clone()).unwrap();
    assert_eq!(result.decision.record, candidate.candidate.record);
    assert_eq!(
        result.decision.writer_signature_hex,
        candidate.candidate.writer_signature_hex
    );
    let verified =
        verify_formal_qualified(&result, &fixture.authority, Some(cut), Some(&recovery3)).unwrap();
    drop(coordinator);
    pc.kill().unwrap();
    pc.wait().unwrap();
    rc.kill().unwrap();
    rc.wait().unwrap();
    let (mut pc, paddr) = start(binary, &configs3[0]);
    let (mut rc, raddr) = start(binary, &configs3[1]);
    let restored = remote(
        paddr,
        recovery3.successor(),
        EndpointRole::Primary,
        13,
        cut.head(),
    )
    .restore_native_current(&fixture.authority, Some(&recovery3), Some(cut))
    .unwrap();
    assert_eq!(restored.commit().head(), verified.head());
    assert_eq!(restored.proof(), &result);
    let mut coordinator = FormalCoordinator::new(
        remote(
            paddr,
            recovery3.successor(),
            EndpointRole::Primary,
            13,
            cut.head(),
        ),
        remote(
            raddr,
            recovery3.successor(),
            EndpointRole::Replica,
            13,
            cut.head(),
        ),
        recovery3.successor().clone(),
        fixture.authority.clone(),
        Some(recovery3),
        Some(verified),
    )
    .unwrap();
    assert_eq!(coordinator.submit(retained).unwrap(), result);
    drop(coordinator);
    pc.kill().unwrap();
    pc.wait().unwrap();
    rc.kill().unwrap();
    rc.wait().unwrap();
}

// The CAS now contains private pin shards as well as immutable object files.
// Preserve that layout in this sealed-transfer fixture rather than treating
// directories as ordinary objects.
fn copy_private_fixture_tree(source: &std::path::Path, destination: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if !destination.exists() {
        fs::create_dir(destination).unwrap();
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    }
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_private_fixture_tree(&entry.path(), &target);
        } else {
            assert!(kind.is_file(), "fixture CAS must not contain symlinks");
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
