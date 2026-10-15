use super::*;
use std::sync::Mutex;

use crate::formal_read_fixtures as fixtures;

#[test]
fn native_resolver_preserves_content_cache_hooks_without_endpoint_io() {
    use crate::formal_live_runtime::RemoteBusinessIndexReader;
    let (_, _, anchor, _) = fixtures::actual_world_initial_fixture();
    let address = "127.0.0.1:1".parse().unwrap();
    let endpoint = |role| {
        RemoteFormalEndpoint::new(
            address,
            std::time::Duration::from_secs(1),
            anchor.authority().clone(),
            role,
            fixtures::key(4),
            anchor.head().head(),
        )
        .unwrap()
    };
    let reader = Arc::new(RemoteBusinessIndexReader::new(
        endpoint(EndpointRole::Primary),
        endpoint(EndpointRole::Replica),
        1024 * 1024,
        16,
    ));
    let resolver = NativeHistoryResolver {
        addresses: [address; 2],
        timeout: std::time::Duration::from_secs(1),
        authentication: fixtures::key(4),
        external_origin: anchor.clone(),
        trusted: anchor.clone(),
        permission: None,
        active: anchor.authority().clone(),
        permissions: Vec::new(),
        release: ReleaseSecurityPolicy::default(),
        business_reader: reader,
        writer_directory: PathBuf::new(),
        origin_history_directory: None,
        activation_markers:
            super::super::durable_set_activation::DurableSetActivationMarkerStoreV1::new(
                PathBuf::new(),
            ),
    };
    let bytes = BusinessIndexNodeV1::branch(0, "0".repeat(64), "1".repeat(64))
        .unwrap()
        .encode()
        .unwrap();
    let digest = hex::encode(blake3::hash(&bytes).as_bytes());
    assert_eq!(resolver.read_cached_node(&digest).unwrap(), None);
    resolver.cache_content_node(&digest, &bytes).unwrap();
    assert_eq!(
        resolver.read_cached_node(&digest).unwrap(),
        Some(bytes.clone())
    );
    assert_eq!(resolver.read_node(&digest).unwrap(), Some(bytes.clone()));
    assert!(resolver.cache_content_node(&digest, b"corrupt").is_err());
    assert_eq!(resolver.read_cached_node(&digest).unwrap(), Some(bytes));
}

#[derive(Default)]
struct CacheRecorder {
    writes: Mutex<Vec<BusinessIndexNodeWriteV1>>,
}

impl BusinessIndexNodeReader for CacheRecorder {
    fn read_node(&self, _digest: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }

    fn cache_content_node(&self, digest: &str, bytes: &[u8]) -> Result<(), String> {
        self.writes
            .lock()
            .map_err(|_| "cache recorder poisoned")?
            .push(BusinessIndexNodeWriteV1 {
                digest: digest.to_owned(),
                bytes: bytes.to_vec(),
            });
        Ok(())
    }
}

fn staged_node() -> StartupBusinessNodeStage {
    let bytes = BusinessIndexNodeV1::branch(0, "0".repeat(64), "1".repeat(64))
        .unwrap()
        .encode()
        .unwrap();
    let digest = hex::encode(blake3::hash(&bytes).as_bytes());
    let mut staged = StartupBusinessNodeStage::default();
    staged
        .insert(BusinessIndexNodeWriteV1 { digest, bytes })
        .unwrap();
    staged
}

fn finish_hint(
    replayed: Result<super::super::ReplayedHistoryCut, super::super::ReplayFailure>,
    reader: &CacheRecorder,
) -> Result<(), String> {
    finish_prewarm_result(
        replayed,
        staged_node(),
        None,
        true,
        |_| Ok(true),
        std::time::Instant::now() + std::time::Duration::from_secs(1),
        reader,
    )
}

#[test]
fn missing_partial_prefix_discards_already_staged_business_bytes() {
    let reader = CacheRecorder::default();
    finish_hint(
        Err(super::super::ReplayFailure::LocalPublicationMissing),
        &reader,
    )
    .unwrap();
    assert!(reader.writes.lock().unwrap().is_empty());
}

#[test]
fn mismatching_protected_head_discards_staged_bytes_as_optional_warmup_miss() {
    let reader = CacheRecorder::default();
    finish_hint(Err(super::super::ReplayFailure::TargetMismatch), &reader).unwrap();
    assert!(reader.writes.lock().unwrap().is_empty());
}

#[test]
fn corrupt_proof_error_is_not_converted_to_an_optional_warmup_miss() {
    let reader = CacheRecorder::default();
    let result = finish_hint(
        Err(super::super::ReplayFailure::Invalid(
            "HistoryUnavailable: invalid protected proof".into(),
        )),
        &reader,
    );
    assert!(result.is_err());
    assert!(reader.writes.lock().unwrap().is_empty());
}

#[test]
fn exact_head_with_different_verified_context_does_not_seed() {
    let (_world, _trust, anchor, _) = fixtures::actual_world_initial_fixture();
    let replayed = super::super::ReplayedHistoryCut {
        anchor,
        target_proof: None,
        target_recovery: None,
        activation: None,
        activation_predecessor: None,
        activation_step: None,
    };
    let reader = CacheRecorder::default();
    finish_prewarm_result(
        Ok(replayed),
        staged_node(),
        None,
        true,
        |_| Ok(false),
        std::time::Instant::now() + std::time::Duration::from_secs(1),
        &reader,
    )
    .unwrap();
    assert!(reader.writes.lock().unwrap().is_empty());
}
