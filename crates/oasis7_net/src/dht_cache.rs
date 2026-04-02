use std::sync::Arc;

use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::DistributedDht as ProtoDistributedDht;

use super::distributed_dht::{
    DistributedDht, MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};
use super::distributed_index_store::DistributedIndexStore;
use super::error::WorldError;

#[derive(Debug, Clone)]
pub struct DhtCacheConfig {
    pub provider_ttl_ms: i64,
    pub head_ttl_ms: i64,
    pub max_providers_per_content: usize,
}

impl Default for DhtCacheConfig {
    fn default() -> Self {
        Self {
            provider_ttl_ms: 10 * 60 * 1000,
            head_ttl_ms: 5 * 60 * 1000,
            max_providers_per_content: 8,
        }
    }
}

#[derive(Clone)]
pub struct CachedDht {
    inner: Arc<dyn DistributedDht + Send + Sync>,
    store: Arc<dyn DistributedIndexStore + Send + Sync>,
    config: DhtCacheConfig,
}

impl CachedDht {
    pub fn new(
        inner: Arc<dyn DistributedDht + Send + Sync>,
        store: Arc<dyn DistributedIndexStore + Send + Sync>,
        config: DhtCacheConfig,
    ) -> Self {
        Self {
            inner,
            store,
            config,
        }
    }

    fn cached_providers(
        &self,
        world_id: &str,
        content_hash: &str,
        now_ms: i64,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        if self.config.provider_ttl_ms <= 0 {
            return Ok(Vec::new());
        }
        let mut providers = self.store.get_providers(world_id, content_hash)?;
        providers.retain(|record| {
            now_ms.saturating_sub(record.last_seen_ms) <= self.config.provider_ttl_ms
        });
        Ok(self.trim_providers(providers))
    }

    fn cache_providers(
        &self,
        world_id: &str,
        content_hash: &str,
        providers: &[ProviderRecord],
    ) -> Result<(), WorldError> {
        for record in providers {
            self.store
                .put_provider(world_id, content_hash, record.clone())?;
        }
        Ok(())
    }

    fn cached_head(
        &self,
        world_id: &str,
        now_ms: i64,
    ) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        if self.config.head_ttl_ms <= 0 {
            return Ok(None);
        }
        let record = self.store.get_head(world_id)?;
        if let Some(record) = record {
            if now_ms.saturating_sub(record.updated_at_ms) <= self.config.head_ttl_ms {
                return Ok(Some(record.head));
            }
        }
        Ok(None)
    }

    fn cache_head(&self, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        self.store.put_head(head.clone())
    }

    fn trim_providers(&self, mut providers: Vec<ProviderRecord>) -> Vec<ProviderRecord> {
        if self.config.max_providers_per_content == 0 {
            return providers;
        }
        providers.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));
        providers.truncate(self.config.max_providers_per_content);
        providers
    }
}

impl ProtoDistributedDht<WorldError> for CachedDht {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        provider_id: &str,
    ) -> Result<(), WorldError> {
        self.inner
            .publish_provider(world_id, content_hash, provider_id)?;
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
        self.store.put_provider(world_id, content_hash, record)
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let now = now_ms();
        let cached = self.cached_providers(world_id, content_hash, now)?;
        if !cached.is_empty() {
            return Ok(cached);
        }
        let providers = self.inner.get_providers(world_id, content_hash)?;
        self.cache_providers(world_id, content_hash, &providers)?;
        Ok(self.trim_providers(providers))
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        self.inner.put_world_head(world_id, head)?;
        self.cache_head(head)
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        let now = now_ms();
        if let Some(head) = self.cached_head(world_id, now)? {
            return Ok(Some(head));
        }
        let head = self.inner.get_world_head(world_id)?;
        if let Some(ref head) = head {
            self.cache_head(head)?;
        }
        Ok(head)
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        self.inner.put_membership_directory(world_id, snapshot)
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        self.inner.get_membership_directory(world_id)
    }

    fn put_peer_record(&self, world_id: &str, record: &SignedPeerRecord) -> Result<(), WorldError> {
        self.inner.put_peer_record(world_id, record)
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        self.inner.get_peer_record(world_id, peer_id)
    }
}

fn now_ms() -> i64 {
    super::util::unix_now_ms_i64()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use oasis7_proto::distributed_dht::DistributedDht as _;

    use super::super::{HeadIndexRecord, InMemoryDht};
    use super::*;

    #[derive(Default, Clone)]
    struct TestIndexStore {
        heads: Arc<Mutex<BTreeMap<String, HeadIndexRecord>>>,
        providers: Arc<Mutex<BTreeMap<(String, String), BTreeMap<String, ProviderRecord>>>>,
    }

    impl TestIndexStore {
        fn insert_provider(&self, world_id: &str, content_hash: &str, record: ProviderRecord) {
            let mut providers = self.providers.lock().expect("lock providers");
            providers
                .entry((world_id.to_string(), content_hash.to_string()))
                .or_default()
                .insert(record.provider_id.clone(), record);
        }

        fn insert_head(&self, head: WorldHeadAnnounce, updated_at_ms: i64) {
            let mut heads = self.heads.lock().expect("lock heads");
            heads.insert(
                head.world_id.clone(),
                HeadIndexRecord {
                    head,
                    updated_at_ms,
                },
            );
        }
    }

    impl DistributedIndexStore for TestIndexStore {
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
            self.insert_provider(world_id, content_hash, record);
            Ok(())
        }

        fn get_providers(
            &self,
            world_id: &str,
            content_hash: &str,
        ) -> Result<Vec<ProviderRecord>, WorldError> {
            let providers = self.providers.lock().expect("lock providers");
            Ok(providers
                .get(&(world_id.to_string(), content_hash.to_string()))
                .map(|records| records.values().cloned().collect())
                .unwrap_or_default())
        }
    }

    #[test]
    fn cached_dht_prefers_cached_providers() {
        let inner = Arc::new(InMemoryDht::new());
        inner
            .publish_provider("w1", "hash", "peer-2")
            .expect("publish provider");

        let store = Arc::new(TestIndexStore::default());
        store.insert_provider(
            "w1",
            "hash",
            ProviderRecord {
                provider_id: "peer-1".to_string(),
                last_seen_ms: now_ms(),
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            },
        );

        let cache = CachedDht::new(
            inner,
            store,
            DhtCacheConfig {
                provider_ttl_ms: 60_000,
                head_ttl_ms: 60_000,
                max_providers_per_content: 8,
            },
        );

        let providers = cache.get_providers("w1", "hash").expect("providers");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_id, "peer-1");
    }

    #[test]
    fn cached_dht_refreshes_providers_when_expired() {
        let inner = Arc::new(InMemoryDht::new());
        inner
            .publish_provider("w1", "hash", "peer-2")
            .expect("publish provider");

        let store = Arc::new(TestIndexStore::default());
        store.insert_provider(
            "w1",
            "hash",
            ProviderRecord {
                provider_id: "peer-1".to_string(),
                last_seen_ms: 0,
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            },
        );

        let cache = CachedDht::new(
            inner,
            store.clone(),
            DhtCacheConfig {
                provider_ttl_ms: 1,
                head_ttl_ms: 60_000,
                max_providers_per_content: 8,
            },
        );

        let providers = cache.get_providers("w1", "hash").expect("providers");
        assert!(!providers.is_empty());
        assert_eq!(providers[0].provider_id, "peer-2");

        let cached = store.get_providers("w1", "hash").expect("cached");
        assert!(cached.iter().any(|record| record.provider_id == "peer-2"));
    }

    #[test]
    fn cached_dht_prefers_cached_head() {
        let inner = Arc::new(InMemoryDht::new());
        let store = Arc::new(TestIndexStore::default());

        let cached_head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 1,
            block_hash: "b1".to_string(),
            state_root: "s1".to_string(),
            timestamp_ms: 1,
            signature: "sig".to_string(),
        };
        store.insert_head(cached_head.clone(), now_ms());

        let fresh_head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 2,
            block_hash: "b2".to_string(),
            state_root: "s2".to_string(),
            timestamp_ms: 2,
            signature: "sig".to_string(),
        };
        inner.put_world_head("w1", &fresh_head).expect("put head");

        let cache = CachedDht::new(
            inner,
            store,
            DhtCacheConfig {
                provider_ttl_ms: 60_000,
                head_ttl_ms: 60_000,
                max_providers_per_content: 8,
            },
        );

        let head = cache.get_world_head("w1").expect("head");
        assert_eq!(head, Some(cached_head));
    }

    #[test]
    fn cached_dht_refreshes_head_when_expired() {
        let inner = Arc::new(InMemoryDht::new());
        let store = Arc::new(TestIndexStore::default());

        let cached_head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 1,
            block_hash: "b1".to_string(),
            state_root: "s1".to_string(),
            timestamp_ms: 1,
            signature: "sig".to_string(),
        };
        store.insert_head(cached_head, 0);

        let fresh_head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 3,
            block_hash: "b3".to_string(),
            state_root: "s3".to_string(),
            timestamp_ms: 3,
            signature: "sig".to_string(),
        };
        inner.put_world_head("w1", &fresh_head).expect("put head");

        let cache = CachedDht::new(
            inner,
            store.clone(),
            DhtCacheConfig {
                provider_ttl_ms: 60_000,
                head_ttl_ms: 1,
                max_providers_per_content: 8,
            },
        );

        let head = cache.get_world_head("w1").expect("head");
        assert_eq!(head, Some(fresh_head.clone()));

        let cached = store.get_head("w1").expect("cached head");
        assert!(cached.is_some());
        assert_eq!(cached.expect("head").head, fresh_head);
    }
}
