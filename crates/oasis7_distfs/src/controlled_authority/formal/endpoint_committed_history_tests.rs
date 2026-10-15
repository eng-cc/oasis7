use super::*;
use crate::controlled_authority::formal::business_index::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

struct ContentCache<'a, R> {
    source: &'a R,
    nodes: RefCell<BTreeMap<String, Vec<u8>>>,
    source_reads: Cell<usize>,
    installs: Cell<usize>,
}

impl<'a, R> ContentCache<'a, R> {
    fn new(source: &'a R) -> Self {
        Self {
            source,
            nodes: RefCell::new(BTreeMap::new()),
            source_reads: Cell::new(0),
            installs: Cell::new(0),
        }
    }
}

impl<R: BusinessIndexNodeReader> BusinessIndexNodeReader for ContentCache<'_, R> {
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        if let Some(bytes) = self.nodes.borrow().get(digest) {
            return Ok(Some(bytes.clone()));
        }
        self.source_reads.set(self.source_reads.get() + 1);
        self.source.read_node(digest)
    }
    fn cache_content_node(&self, digest: &str, bytes: &[u8]) -> Result<(), String> {
        if blake3::hash(bytes).to_hex().as_str() != digest {
            return Err("test content hash".into());
        }
        self.nodes
            .borrow_mut()
            .insert(digest.into(), bytes.to_vec());
        self.installs.set(self.installs.get() + 1);
        Ok(())
    }
}

struct MissingNodes;
impl BusinessIndexNodeReader for MissingNodes {
    fn read_node(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
}
#[test]
fn committed_historical_cut_uses_current_canonical_inclusion_and_keeps_main_anchor() {
    let (fixture, initial_envelope) = super::business_tests::fixture();
    let mut coordinator = fixture.coordinator();
    let initial = coordinator.submit(fixture.initial.clone()).unwrap();
    let body = fixture.authority.initial.clone().unwrap();
    let certificate = SignedInitialActivationV1 {
        signature_hex: signed(initial_activation_signing_bytes(&body).unwrap(), 9),
        body,
    };
    let trust = InitialActivationTrustV1 {
        issuer_public_key: public(9),
        world: fixture.authority.definition().world.clone(),
        initial_state_root: certificate.body.initial_state_root.clone(),
        governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
        recovery_policy_digest: fixture.authority.policy().digest().unwrap(),
    };
    let origin = VerifiedFormalReadAnchorV1::from_initial(&certificate, &trust, &initial).unwrap();
    let mut anchor = origin.clone();
    let mut previous_proof = initial.clone();
    let mut tree = initial_envelope.delta.new;
    let mut proofs = vec![initial];
    for nonce in [121, 122] {
        let (input, material) = fixture.frozen(anchor.head(), nonce);
        let frozen = coordinator.freeze(input, material).unwrap();
        let mut candidate = fixture.candidate(&frozen);
        let mut updates = vec![
            BusinessIndexLeafV1 {
                key_digest: metadata_key_digest(),
                kind: BusinessIndexLeafKindV1::CommitMetadata,
                canonical_value: vec![nonce],
            },
            FormalHistoricalCutV1::from_proof(&previous_proof)
                .unwrap()
                .leaf()
                .unwrap(),
        ];
        updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
        let delta =
            BusinessIndexDeltaV1::build(tree, updates, &coordinator.endpoints_mut().0.store)
                .unwrap();
        tree = delta.new.clone();
        let envelope = super::business_tests::envelope(delta);
        super::business_tests::replace_result(&mut candidate.material, envelope.encode().unwrap());
        candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
        candidate.candidate.record.result_root = envelope.delta.new_root;
        candidate.candidate.writer_signature_hex = signed(
            canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
            4,
        );
        let proof = coordinator.submit(candidate).unwrap();
        let commit =
            verify_formal_qualified(&proof, &fixture.authority, Some(anchor.head()), None).unwrap();
        anchor = anchor
            .after_verified_commit(fixture.authority.clone(), commit)
            .unwrap();
        previous_proof = proof.clone();
        proofs.push(proof);
    }
    let current_head = anchor.head().head();
    for position in [1, 2] {
        let key =
            formal_historical_cut_key(&fixture.authority.definition().world, position).unwrap();
        let inclusion = tree
            .prove(&key, &coordinator.endpoints_mut().0.store)
            .unwrap();
        let cut =
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &inclusion, position).unwrap();
        assert_eq!(
            cut.commit().record(),
            &proofs[position as usize - 1].decision.record
        );
        assert_eq!(cut.proof(), &proofs[position as usize - 1]);
        assert_eq!(cut.delegating_head(), &current_head);
        let FormalQueryOutcomeV1::Recorded(state) = coordinator
            .endpoints_mut()
            .0
            .query(&cut.commit().record().request_digest)
            .unwrap()
        else {
            panic!("published exact old query");
        };
        let restored = verify_committed_historical_material(
            &cut,
            (*state).clone(),
            &coordinator.endpoints_mut().0.store,
        )
        .unwrap();
        assert_eq!(restored.commit().record(), cut.commit().record());
        let source = &coordinator.endpoints_mut().0.store;
        let baseline = ContentCache::new(source);
        let original =
            verify_committed_historical_material(&cut, (*state).clone(), &baseline).unwrap();
        super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
            &original, &baseline, None,
        )
        .unwrap();
        let cached = ContentCache::new(source);
        let from_cache =
            verify_committed_historical_material(&cut, (*state).clone(), &cached).unwrap();
        super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
            &from_cache,
            &cached,
            Some(&cached),
        )
        .unwrap();
        assert_eq!(from_cache.proof(), restored.proof());
        assert_eq!(from_cache.commit().record(), restored.commit().record());
        assert!(cached.source_reads.get() < baseline.source_reads.get());
        assert!(cached.installs.get() > 0);
        let envelope = FormalBusinessResultEnvelopeV1::decode(
            &from_cache.material().objects.iter().find(|object| object.content_hash == from_cache.material().roots[&crate::controlled_authority::replicated_protocol::ArtifactRole::Result]).unwrap().bytes,
        ).unwrap();
        let reads = cached.source_reads.get();
        for write in envelope.delta.node_writes.iter().chain(
            envelope
                .retained_history_delta
                .iter()
                .flat_map(|delta| &delta.node_writes),
        ) {
            assert_eq!(
                cached.read_node(&write.digest).unwrap().as_deref(),
                Some(write.bytes.as_slice())
            );
        }
        assert_eq!(
            cached.source_reads.get(),
            reads,
            "transport's existence loop must use the installed exact content"
        );
        assert!(
            super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
                &original,
                &MissingNodes,
                None
            )
            .is_err(),
            "no-cache API still rejects unavailable raw CAS"
        );
        if position == 1 {
            let independent_cache = ContentCache::new(&MissingNodes);
            super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
                &original,
                &independent_cache,
                Some(&independent_cache),
            )
            .unwrap();
            assert_eq!(independent_cache.source_reads.get(), 0);
        }
        let mut corrupt = (*state).clone();
        if let FormalRequestStateV1::Qualified { proof, .. } = &mut corrupt {
            proof.primary_complete.signature_hex = h(1);
        }
        assert!(verify_committed_historical_material(&cut, corrupt.clone(), source).is_err());
        let rejected = ContentCache::new(source);
        assert!(
            verify_committed_historical_material(&cut, corrupt, &rejected)
                .and_then(|material| {
                    super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
                        &material,
                        &rejected,
                        Some(&rejected),
                    )
                })
                .is_err()
        );
        assert_eq!(
            rejected.installs.get(),
            0,
            "invalid proof must not install any Result writes"
        );
        let mut corrupt = *state;
        if let FormalRequestStateV1::Qualified { candidate, .. } = &mut corrupt {
            candidate.material.objects[0].bytes.push(0);
        }
        assert!(verify_committed_historical_material(&cut, corrupt.clone(), source).is_err());
        let rejected = ContentCache::new(&MissingNodes);
        assert!(
            verify_committed_historical_material(&cut, corrupt, &rejected)
                .and_then(|material| {
                    super::super::transport::RemoteFormalEndpoint::check_committed_material_content(
                        &material,
                        &rejected,
                        Some(&rejected),
                    )
                })
                .is_err()
        );
        assert_eq!(
            rejected.installs.get(),
            0,
            "invalid material must not install any Result writes"
        );

        assert_eq!(anchor.head().head(), current_head);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&origin, &tree, &inclusion, position).is_err()
        );
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &inclusion, position + 1)
                .is_err()
        );
        let mut corrupt = inclusion.clone();
        corrupt.siblings[0] = h(203);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &corrupt, position).is_err()
        );
        let mut corrupt = inclusion.clone();
        corrupt.leaf.canonical_value.push(0);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &corrupt, position).is_err()
        );
        let mut count = tree.clone();
        count.leaf_count += 1;
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &count, &inclusion, position)
                .is_err()
        );
    }
}

#[path = "business_history_transition_tests.rs"]
mod transition_tests;
