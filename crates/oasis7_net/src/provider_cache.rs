use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::distributed_dht::{DistributedDht, ProviderRecord};
use super::distributed_index_store::DistributedIndexStore;
use super::error::WorldError;

#[derive(Debug, Clone)]
pub struct ProviderCacheConfig {
    pub provider_ttl_ms: i64,
    pub republish_interval_ms: i64,
    pub max_providers_per_content: usize,
}

impl Default for ProviderCacheConfig {
    fn default() -> Self {
        Self {
            provider_ttl_ms: 10 * 60 * 1000,
            republish_interval_ms: 5 * 60 * 1000,
            max_providers_per_content: 8,
        }
    }
}

#[derive(Clone)]
pub struct ProviderCache {
    dht: Arc<dyn DistributedDht + Send + Sync>,
    store: Arc<dyn DistributedIndexStore + Send + Sync>,
    provider_id: String,
    config: ProviderCacheConfig,
    local_content: Arc<Mutex<BTreeMap<(String, String), i64>>>,
    last_republish_ms: Arc<Mutex<i64>>,
}

impl ProviderCache {
    pub fn new(
        dht: Arc<dyn DistributedDht + Send + Sync>,
        store: Arc<dyn DistributedIndexStore + Send + Sync>,
        provider_id: impl Into<String>,
        config: ProviderCacheConfig,
    ) -> Self {
        Self {
            dht,
            store,
            provider_id: provider_id.into(),
            config,
            local_content: Arc::new(Mutex::new(BTreeMap::new())),
            last_republish_ms: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register_local_content(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<(), WorldError> {
        let now = now_ms();
        self.register_local_content_at(world_id, content_hash, now)
    }

    pub fn register_local_content_at(
        &self,
        world_id: &str,
        content_hash: &str,
        now_ms: i64,
    ) -> Result<(), WorldError> {
        {
            let mut local = self.local_content.lock().expect("lock local content");
            local.insert((world_id.to_string(), content_hash.to_string()), now_ms);
        }
        let record = ProviderRecord {
            provider_id: self.provider_id.clone(),
            last_seen_ms: now_ms,
            storage_total_bytes: None,
            storage_available_bytes: None,
            uptime_ratio_per_mille: None,
            challenge_pass_ratio_per_mille: None,
            load_ratio_per_mille: None,
            p50_read_latency_ms: None,
        };
        self.store
            .put_provider(world_id, content_hash, record.clone())?;
        self.dht
            .publish_provider(world_id, content_hash, &self.provider_id)?;
        Ok(())
    }

    pub fn query_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let now = now_ms();
        self.query_providers_at(world_id, content_hash, now)
    }

    pub fn query_providers_at(
        &self,
        world_id: &str,
        content_hash: &str,
        now_ms: i64,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let cached = self.cached_providers(world_id, content_hash, now_ms)?;
        if !cached.is_empty() {
            return Ok(cached);
        }
        let providers = self.dht.get_providers(world_id, content_hash)?;
        self.cache_providers(world_id, content_hash, &providers)?;
        Ok(self.trim_providers(providers))
    }

    pub fn republish_local(&self) -> Result<usize, WorldError> {
        let now = now_ms();
        self.republish_local_at(now)
    }

    pub fn republish_local_at(&self, now_ms: i64) -> Result<usize, WorldError> {
        if self.config.republish_interval_ms > 0 {
            let last = self.last_republish_ms.lock().expect("lock republish");
            if now_ms.saturating_sub(*last) < self.config.republish_interval_ms {
                return Ok(0);
            }
        }

        let items: Vec<(String, String)> = {
            let local = self.local_content.lock().expect("lock local content");
            local
                .keys()
                .map(|(world_id, content_hash)| (world_id.clone(), content_hash.clone()))
                .collect()
        };

        let mut republished = 0usize;
        for (world_id, content_hash) in items {
            self.dht
                .publish_provider(&world_id, &content_hash, &self.provider_id)?;
            let record = ProviderRecord {
                provider_id: self.provider_id.clone(),
                last_seen_ms: now_ms,
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            };
            self.store.put_provider(&world_id, &content_hash, record)?;
            let mut local = self.local_content.lock().expect("lock local content");
            local.insert((world_id, content_hash), now_ms);
            republished = republished.saturating_add(1);
        }

        if self.config.republish_interval_ms > 0 {
            let mut last = self.last_republish_ms.lock().expect("lock republish");
            *last = now_ms;
        }

        Ok(republished)
    }

    fn cached_providers(
        &self,
        world_id: &str,
        content_hash: &str,
        now_ms: i64,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let mut providers = self.store.get_providers(world_id, content_hash)?;
        if self.config.provider_ttl_ms > 0 {
            providers.retain(|record| {
                now_ms.saturating_sub(record.last_seen_ms) <= self.config.provider_ttl_ms
            });
        }
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

    fn trim_providers(&self, mut providers: Vec<ProviderRecord>) -> Vec<ProviderRecord> {
        if self.config.max_providers_per_content == 0 {
            return providers;
        }
        providers.sort_by_key(|provider| std::cmp::Reverse(provider.last_seen_ms));
        providers.truncate(self.config.max_providers_per_content);
        providers
    }
}

fn now_ms() -> i64 {
    super::util::unix_now_ms_i64()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use oasis7_proto::distributed_dht::DistributedDht as _;

    use super::super::{InMemoryDht, InMemoryIndexStore};
    use super::*;

    fn cache_with_config(config: ProviderCacheConfig) -> ProviderCache {
        ProviderCache::new(
            Arc::new(InMemoryDht::new()),
            Arc::new(InMemoryIndexStore::new()),
            "node-1",
            config,
        )
    }

    #[test]
    fn provider_cache_returns_cached_entries() {
        let config = ProviderCacheConfig {
            provider_ttl_ms: 1000,
            ..ProviderCacheConfig::default()
        };
        let cache = cache_with_config(config);
        cache
            .register_local_content_at("w1", "hash-a", 100)
            .expect("register");

        let providers = cache
            .query_providers_at("w1", "hash-a", 150)
            .expect("query");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_id, "node-1");
    }

    #[test]
    fn provider_cache_refreshes_when_expired() {
        let config = ProviderCacheConfig {
            provider_ttl_ms: 10,
            ..ProviderCacheConfig::default()
        };
        let dht = Arc::new(InMemoryDht::new());
        let store = Arc::new(InMemoryIndexStore::new());
        let cache = ProviderCache::new(dht.clone(), store.clone(), "node-1", config);

        store
            .put_provider(
                "w1",
                "hash-b",
                ProviderRecord {
                    provider_id: "old".to_string(),
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

        dht.publish_provider("w1", "hash-b", "peer-2")
            .expect("publish");

        let providers = cache
            .query_providers_at("w1", "hash-b", 1000)
            .expect("query");
        assert!(!providers.is_empty());
        let cached = store.get_providers("w1", "hash-b").expect("get cached");
        assert!(!cached.is_empty());
    }

    #[test]
    fn provider_cache_republishes_local_content() {
        let config = ProviderCacheConfig {
            republish_interval_ms: 50,
            ..ProviderCacheConfig::default()
        };
        let cache = cache_with_config(config);

        cache
            .register_local_content_at("w1", "hash-c", 10)
            .expect("register");
        let republished = cache.republish_local_at(100).expect("republish");
        assert_eq!(republished, 1);

        let providers = cache
            .query_providers_at("w1", "hash-c", 110)
            .expect("query");
        assert!(!providers.is_empty());
    }
}
