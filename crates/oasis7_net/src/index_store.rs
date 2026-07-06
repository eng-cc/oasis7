use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use oasis7_proto::distributed::WorldHeadAnnounce;

use super::distributed_dht::ProviderRecord;
use super::error::WorldError;

type ProviderRecordsByProvider = BTreeMap<String, ProviderRecord>;
type ProviderRecordsByContent = BTreeMap<String, ProviderRecordsByProvider>;
type ProviderRecordsByWorld = BTreeMap<String, ProviderRecordsByContent>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadIndexRecord {
    pub head: WorldHeadAnnounce,
    pub updated_at_ms: i64,
}

pub trait DistributedIndexStore {
    fn put_head(&self, head: WorldHeadAnnounce) -> Result<(), WorldError>;
    fn get_head(&self, world_id: &str) -> Result<Option<HeadIndexRecord>, WorldError>;

    fn put_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        record: ProviderRecord,
    ) -> Result<(), WorldError>;
    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryIndexStore {
    heads: Arc<Mutex<BTreeMap<String, HeadIndexRecord>>>,
    providers: Arc<Mutex<ProviderRecordsByWorld>>,
}

impl InMemoryIndexStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DistributedIndexStore for InMemoryIndexStore {
    fn put_head(&self, head: WorldHeadAnnounce) -> Result<(), WorldError> {
        let mut heads = self.heads.lock().expect("lock heads");
        heads.insert(
            head.world_id.clone(),
            HeadIndexRecord {
                head,
                updated_at_ms: now_ms(),
            },
        );
        Ok(())
    }

    fn get_head(&self, world_id: &str) -> Result<Option<HeadIndexRecord>, WorldError> {
        let heads = self.heads.lock().expect("lock heads");
        Ok(heads.get(world_id).cloned())
    }

    fn put_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        record: ProviderRecord,
    ) -> Result<(), WorldError> {
        let mut providers = self.providers.lock().expect("lock providers");
        providers
            .entry(world_id.to_string())
            .or_default()
            .entry(content_hash.to_string())
            .or_default()
            .insert(record.provider_id.clone(), record);
        Ok(())
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let providers = self.providers.lock().expect("lock providers");
        Ok(providers
            .get(world_id)
            .and_then(|records_by_hash| records_by_hash.get(content_hash))
            .map(|records| records.values().cloned().collect())
            .unwrap_or_default())
    }
}

fn now_ms() -> i64 {
    super::util::unix_now_ms_i64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_store_round_trip_head() {
        let store = InMemoryIndexStore::new();
        let head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 5,
            block_hash: "b1".to_string(),
            state_root: "s1".to_string(),
            timestamp_ms: 1,
            signature: "sig".to_string(),
        };
        store.put_head(head.clone()).expect("put head");
        let loaded = store.get_head("w1").expect("get head");
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("head").head, head);
    }

    #[test]
    fn index_store_round_trip_providers() {
        let store = InMemoryIndexStore::new();
        store
            .put_provider(
                "w1",
                "hash",
                ProviderRecord {
                    provider_id: "p1".to_string(),
                    last_seen_ms: 1,
                    storage_total_bytes: None,
                    storage_available_bytes: None,
                    uptime_ratio_per_mille: None,
                    challenge_pass_ratio_per_mille: None,
                    load_ratio_per_mille: None,
                    p50_read_latency_ms: None,
                },
            )
            .expect("put provider");
        store
            .put_provider(
                "w1",
                "hash",
                ProviderRecord {
                    provider_id: "p2".to_string(),
                    last_seen_ms: 2,
                    storage_total_bytes: None,
                    storage_available_bytes: None,
                    uptime_ratio_per_mille: None,
                    challenge_pass_ratio_per_mille: None,
                    load_ratio_per_mille: None,
                    p50_read_latency_ms: None,
                },
            )
            .expect("put provider");
        let providers = store.get_providers("w1", "hash").expect("get providers");
        assert_eq!(providers.len(), 2);
    }
}
