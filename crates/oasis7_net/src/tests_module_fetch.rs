use super::*;
use crate::util::to_canonical_cbor;
use oasis7_wasm_abi::ModuleAbiContract;

#[test]
fn client_fetch_module_manifest_from_dht_uses_provider_list() {
    let spy = Arc::new(SpyNetwork::default());
    let network: Arc<dyn DistributedNetwork + Send + Sync> = spy.clone();
    let client = DistributedClient::new(network);
    let dht = InMemoryDht::new();
    dht.publish_provider("w1", "manifest-hash", "peer-9")
        .expect("publish provider");

    let manifest = ModuleManifest {
        module_id: "m.weather".to_string(),
        name: "Weather".to_string(),
        version: "0.1.0".to_string(),
        kind: oasis7_wasm_abi::ModuleKind::Pure,
        role: oasis7_wasm_abi::ModuleRole::Domain,
        wasm_hash: "wasm-hash".to_string(),
        interface_version: "aw.abi.module.v1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["call".to_string()],
        subscriptions: Vec::new(),
        required_caps: Vec::new(),
        artifact_identity: None,
        limits: oasis7_wasm_abi::ModuleLimits::unbounded(),
    };
    let bytes = to_canonical_cbor(&manifest).expect("cbor");
    spy.set_blob("manifest-hash", bytes);

    let loaded = client
        .fetch_module_manifest_from_dht("w1", "m.weather", "manifest-hash", &dht)
        .expect("fetch manifest");
    assert_eq!(loaded.module_id, "m.weather");

    let seen = spy.providers();
    assert_eq!(seen, vec!["peer-9".to_string()]);
}

#[test]
fn client_fetch_module_artifact_from_dht_uses_provider_list() {
    let spy = Arc::new(SpyNetwork::default());
    let network: Arc<dyn DistributedNetwork + Send + Sync> = spy.clone();
    let client = DistributedClient::new(network);
    let dht = InMemoryDht::new();
    dht.publish_provider("w1", "wasm-hash", "peer-7")
        .expect("publish provider");

    let artifact = client
        .fetch_module_artifact_from_dht("w1", "wasm-hash", &dht)
        .expect("fetch artifact");
    assert_eq!(artifact.wasm_hash, "wasm-hash");
    assert_eq!(artifact.bytes, b"data".to_vec().into());
    let seen = spy.providers();
    assert_eq!(seen, vec!["peer-7".to_string()]);
}
