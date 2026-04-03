use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::error::WorldError;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht as proto_dht;

pub use proto_dht::{MembershipDirectorySnapshot, ProviderRecord};
pub use proto_dht::{PeerDiscoverySource, PeerReachabilityClass, PeerRecord, SignedPeerRecord};

pub trait DistributedDht: proto_dht::DistributedDht<WorldError> {}

impl<T> DistributedDht for T where T: proto_dht::DistributedDht<WorldError> {}

#[derive(Debug, Clone, Default)]
pub struct InMemoryDht {
    providers: Arc<Mutex<BTreeMap<(String, String), BTreeMap<String, ProviderRecord>>>>,
    heads: Arc<Mutex<BTreeMap<String, WorldHeadAnnounce>>>,
    memberships: Arc<Mutex<BTreeMap<String, MembershipDirectorySnapshot>>>,
    peer_records: Arc<Mutex<BTreeMap<(String, String), SignedPeerRecord>>>,
}

impl InMemoryDht {
    pub fn new() -> Self {
        Self::default()
    }
}

impl proto_dht::DistributedDht<WorldError> for InMemoryDht {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        provider_id: &str,
    ) -> Result<(), WorldError> {
        let mut providers = self.providers.lock().expect("lock providers");
        let key = (world_id.to_string(), content_hash.to_string());
        let record = ProviderRecord {
            provider_id: provider_id.to_string(),
            last_seen_ms: now_ms(),
            storage_total_bytes: None,
            storage_available_bytes: None,
            uptime_ratio_per_mille: None,
            challenge_pass_ratio_per_mille: None,
            load_ratio_per_mille: None,
            p50_read_latency_ms: None,
        };
        providers
            .entry(key)
            .or_default()
            .insert(provider_id.to_string(), record);
        Ok(())
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let providers = self.providers.lock().expect("lock providers");
        let key = (world_id.to_string(), content_hash.to_string());
        Ok(providers
            .get(&key)
            .map(|records| records.values().cloned().collect())
            .unwrap_or_default())
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        let mut heads = self.heads.lock().expect("lock heads");
        heads.insert(world_id.to_string(), head.clone());
        Ok(())
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        let heads = self.heads.lock().expect("lock heads");
        Ok(heads.get(world_id).cloned())
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        let mut memberships = self.memberships.lock().expect("lock memberships");
        memberships.insert(world_id.to_string(), snapshot.clone());
        Ok(())
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        let memberships = self.memberships.lock().expect("lock memberships");
        Ok(memberships.get(world_id).cloned())
    }

    fn put_peer_record(&self, world_id: &str, record: &SignedPeerRecord) -> Result<(), WorldError> {
        let mut peer_records = self.peer_records.lock().expect("lock peer records");
        peer_records.insert(
            (world_id.to_string(), record.record.peer_id.clone()),
            record.clone(),
        );
        Ok(())
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        let peer_records = self.peer_records.lock().expect("lock peer records");
        Ok(peer_records
            .get(&(world_id.to_string(), peer_id.to_string()))
            .cloned())
    }
}

fn now_ms() -> i64 {
    super::util::unix_now_ms_i64()
}
