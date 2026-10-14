use oasis7_distfs::controlled_authority::formal::business_index::BusinessIndexNodeReader;
use std::sync::Arc;

#[cfg(test)]
use crate::runtime::modules::ModuleSubscription;
#[cfg(test)]
use oasis7_wasm_router::PreparedSubscription;

#[derive(Clone)]
pub(crate) struct SharedFormalHistoryReader {
    reader: Arc<dyn BusinessIndexNodeReader + Send + Sync>,
}

impl SharedFormalHistoryReader {
    pub(crate) fn new(reader: Arc<dyn BusinessIndexNodeReader + Send + Sync>) -> Self {
        Self { reader }
    }
}

impl std::fmt::Debug for SharedFormalHistoryReader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SharedFormalHistoryReader(attached)")
    }
}

impl BusinessIndexNodeReader for SharedFormalHistoryReader {
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        self.reader.read_node(digest)
    }
}

#[derive(Debug, Clone)]
pub(super) struct PreparedSubscriptionCacheEntry {
    #[cfg(test)]
    pub(super) subscriptions: Vec<ModuleSubscription>,
    pub(super) _subscription_fingerprint: String,
    #[cfg(test)]
    pub(super) prepared: Arc<[PreparedSubscription]>,
}
