use std::collections::BTreeMap;
use std::sync::Arc;

use oasis7_proto::distributed as proto_distributed;
use oasis7_proto::distributed::WorldHeadAnnounce;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::distributed_dht::DistributedDht;
use super::distributed_net::DistributedNetwork;
use super::error::WorldError;
use super::modules::{ModuleArtifact, ModuleManifest};
use super::provider_distribution::{audit_provider_distribution, ProviderDistributionPolicy};
use super::provider_selection::ProviderSelectionPolicy;
use super::util::{to_canonical_cbor, unix_now_ms_i64};

#[derive(Clone)]
pub struct DistributedClient {
    network: Arc<dyn DistributedNetwork + Send + Sync>,
    provider_selection_policy: ProviderSelectionPolicy,
}

impl DistributedClient {
    pub fn new(network: Arc<dyn DistributedNetwork + Send + Sync>) -> Self {
        Self {
            network,
            provider_selection_policy: ProviderSelectionPolicy::default(),
        }
    }

    pub fn new_with_provider_selection_policy(
        network: Arc<dyn DistributedNetwork + Send + Sync>,
        provider_selection_policy: ProviderSelectionPolicy,
    ) -> Self {
        Self {
            network,
            provider_selection_policy,
        }
    }

    pub fn get_world_head(&self, world_id: &str) -> Result<WorldHeadAnnounce, WorldError> {
        let request = proto_distributed::GetWorldHeadRequest {
            world_id: world_id.to_string(),
        };
        let response: proto_distributed::GetWorldHeadResponse =
            self.request(proto_distributed::RR_GET_WORLD_HEAD, &request)?;
        Ok(response.head)
    }

    pub fn get_block(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<proto_distributed::WorldBlock, WorldError> {
        Ok(self.get_block_response(world_id, height)?.block)
    }

    pub fn get_block_response(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<proto_distributed::GetBlockResponse, WorldError> {
        let request = proto_distributed::GetBlockRequest {
            world_id: world_id.to_string(),
            height,
        };
        self.request(proto_distributed::RR_GET_BLOCK, &request)
    }

    pub fn get_snapshot_manifest(
        &self,
        world_id: &str,
        epoch: u64,
    ) -> Result<proto_distributed::SnapshotManifest, WorldError> {
        let request = proto_distributed::GetSnapshotRequest {
            world_id: world_id.to_string(),
            epoch,
        };
        let response: proto_distributed::GetSnapshotResponse =
            self.request(proto_distributed::RR_GET_SNAPSHOT, &request)?;
        Ok(response.manifest)
    }

    pub fn fetch_blob(&self, content_hash: &str) -> Result<Vec<u8>, WorldError> {
        let request = proto_distributed::FetchBlobRequest {
            content_hash: content_hash.to_string(),
        };
        let response: proto_distributed::FetchBlobResponse =
            self.request(proto_distributed::RR_FETCH_BLOB, &request)?;
        Ok(response.blob)
    }

    pub fn fetch_blob_with_providers(
        &self,
        content_hash: &str,
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        let request = proto_distributed::FetchBlobRequest {
            content_hash: content_hash.to_string(),
        };
        let response: proto_distributed::FetchBlobResponse =
            self.request_with_providers(proto_distributed::RR_FETCH_BLOB, &request, providers)?;
        Ok(response.blob)
    }

    pub fn fetch_blob_from_dht(
        &self,
        world_id: &str,
        content_hash: &str,
        dht: &impl DistributedDht,
    ) -> Result<Vec<u8>, WorldError> {
        let providers = dht.get_providers(world_id, content_hash)?;
        if providers.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "dht providers missing for world={world_id}, content_hash={content_hash}"
                ),
            });
        }

        let now_ms = unix_now_ms_i64();
        let ranked = self
            .provider_selection_policy
            .rank_providers(&providers, now_ms);

        let mut last_error: Option<WorldError> = None;
        for record in ranked {
            let provider = [record.provider_id];
            match self.fetch_blob_with_providers(content_hash, &provider) {
                Ok(bytes) => return Ok(bytes),
                Err(error) => last_error = Some(error),
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "dht providers exhausted for world={world_id}, content_hash={content_hash}"
                ),
            }),
        }
    }

    pub fn fetch_blobs_from_dht_with_distribution(
        &self,
        world_id: &str,
        content_hashes: &[String],
        dht: &impl DistributedDht,
        policy: ProviderDistributionPolicy,
    ) -> Result<BTreeMap<String, Vec<u8>>, WorldError> {
        audit_provider_distribution(dht, world_id, content_hashes, policy)?;
        let mut blobs = BTreeMap::new();
        for content_hash in content_hashes {
            let bytes = self.fetch_blob_from_dht(world_id, content_hash, dht)?;
            blobs.insert(content_hash.clone(), bytes);
        }
        Ok(blobs)
    }

    pub fn get_journal_segment(
        &self,
        world_id: &str,
        from_event_id: u64,
    ) -> Result<proto_distributed::BlobRef, WorldError> {
        let request = proto_distributed::GetJournalSegmentRequest {
            world_id: world_id.to_string(),
            from_event_id,
        };
        let response: proto_distributed::GetJournalSegmentResponse =
            self.request(proto_distributed::RR_GET_JOURNAL_SEGMENT, &request)?;
        Ok(response.segment)
    }

    pub fn get_receipt_segment(
        &self,
        world_id: &str,
        from_event_id: u64,
    ) -> Result<proto_distributed::BlobRef, WorldError> {
        let request = proto_distributed::GetReceiptSegmentRequest {
            world_id: world_id.to_string(),
            from_event_id,
        };
        let response: proto_distributed::GetReceiptSegmentResponse =
            self.request(proto_distributed::RR_GET_RECEIPT_SEGMENT, &request)?;
        Ok(response.segment)
    }

    pub fn get_module_manifest(
        &self,
        module_id: &str,
        manifest_hash: &str,
    ) -> Result<proto_distributed::BlobRef, WorldError> {
        let request = proto_distributed::GetModuleManifestRequest {
            module_id: module_id.to_string(),
            manifest_hash: manifest_hash.to_string(),
        };
        let response: proto_distributed::GetModuleManifestResponse =
            self.request(proto_distributed::RR_GET_MODULE_MANIFEST, &request)?;
        Ok(response.manifest_ref)
    }

    pub fn get_module_artifact(
        &self,
        wasm_hash: &str,
    ) -> Result<proto_distributed::BlobRef, WorldError> {
        let request = proto_distributed::GetModuleArtifactRequest {
            wasm_hash: wasm_hash.to_string(),
        };
        let response: proto_distributed::GetModuleArtifactResponse =
            self.request(proto_distributed::RR_GET_MODULE_ARTIFACT, &request)?;
        Ok(response.artifact_ref)
    }

    pub fn fetch_module_manifest_from_dht(
        &self,
        world_id: &str,
        module_id: &str,
        manifest_hash: &str,
        dht: &impl DistributedDht,
    ) -> Result<ModuleManifest, WorldError> {
        let manifest_ref = self.get_module_manifest(module_id, manifest_hash)?;
        let bytes = self.fetch_blob_from_dht(world_id, &manifest_ref.content_hash, dht)?;
        Ok(serde_cbor::from_slice(&bytes)?)
    }

    pub fn fetch_module_artifact_from_dht(
        &self,
        world_id: &str,
        wasm_hash: &str,
        dht: &impl DistributedDht,
    ) -> Result<ModuleArtifact, WorldError> {
        let artifact_ref = self.get_module_artifact(wasm_hash)?;
        let bytes = self.fetch_blob_from_dht(world_id, &artifact_ref.content_hash, dht)?;
        Ok(ModuleArtifact {
            wasm_hash: wasm_hash.to_string(),
            bytes: bytes.into(),
        })
    }

    fn request<T: Serialize, R: DeserializeOwned>(
        &self,
        protocol: &str,
        request: &T,
    ) -> Result<R, WorldError> {
        let payload = to_canonical_cbor(request)?;
        let response_bytes = self.network.request(protocol, &payload)?;
        decode_response(&response_bytes)
    }

    fn request_with_providers<T: Serialize, R: DeserializeOwned>(
        &self,
        protocol: &str,
        request: &T,
        providers: &[String],
    ) -> Result<R, WorldError> {
        let payload = to_canonical_cbor(request)?;
        let response_bytes = self
            .network
            .request_with_providers(protocol, &payload, providers)?;
        decode_response(&response_bytes)
    }
}

fn decode_response<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, WorldError> {
    if let Ok(error) = serde_cbor::from_slice::<proto_distributed::ErrorResponse>(bytes) {
        return Err(WorldError::NetworkRequestFailed {
            code: error.code,
            message: error.message,
            retryable: error.retryable,
        });
    }
    Ok(serde_cbor::from_slice(bytes)?)
}
