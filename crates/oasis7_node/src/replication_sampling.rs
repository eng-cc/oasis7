use std::collections::BTreeSet;

use oasis7_distfs::{StorageChallengeProbeConfig, StorageChallengeProbeReport};

use crate::NodeError;

use super::support::distfs_error_to_node_error;
use super::ReplicationRuntime;

impl ReplicationRuntime {
    pub(crate) fn probe_storage_challenges(
        &self,
        world_id: &str,
        node_id: &str,
        observed_at_unix_ms: i64,
    ) -> Result<StorageChallengeProbeReport, NodeError> {
        let config = StorageChallengeProbeConfig::default();
        self.store
            .probe_storage_challenges(world_id, node_id, observed_at_unix_ms, &config)
            .map_err(distfs_error_to_node_error)
    }

    pub(crate) fn recent_replicated_content_hashes(
        &self,
        world_id: &str,
        max_samples: usize,
    ) -> Result<Vec<String>, NodeError> {
        Ok(self
            .recent_replicated_content_refs(world_id, max_samples)?
            .into_iter()
            .map(|(_, content_hash)| content_hash)
            .collect())
    }

    pub(crate) fn recent_replicated_content_refs(
        &self,
        world_id: &str,
        max_samples: usize,
    ) -> Result<Vec<(u64, String)>, NodeError> {
        if max_samples == 0 || self.writer_state.last_replicated_height == 0 {
            return Ok(Vec::new());
        }

        let mut samples = Vec::with_capacity(max_samples);
        let mut seen = BTreeSet::new();
        let mut height = self.writer_state.last_replicated_height;
        while height > 0 && samples.len() < max_samples {
            if let Some(message) = self.load_commit_message_by_height(world_id, height)? {
                let content_hash = message.record.content_hash.trim();
                if !content_hash.is_empty() && seen.insert(content_hash.to_string()) {
                    samples.push((height, content_hash.to_string()));
                }
            }
            height -= 1;
        }
        Ok(samples)
    }

    pub(crate) fn replicated_content_refs_from_height(
        &self,
        world_id: &str,
        start_height: u64,
        max_samples: usize,
    ) -> Result<Vec<(u64, String)>, NodeError> {
        if max_samples == 0 || self.writer_state.last_replicated_height == 0 {
            return Ok(Vec::new());
        }

        let mut height = start_height.max(1);
        let latest_height = self.writer_state.last_replicated_height;
        if height > latest_height {
            return Ok(Vec::new());
        }

        let mut samples = Vec::with_capacity(max_samples);
        let mut seen = BTreeSet::new();
        while height <= latest_height && samples.len() < max_samples {
            if let Some(message) = self.load_commit_message_by_height(world_id, height)? {
                let content_hash = message.record.content_hash.trim();
                if !content_hash.is_empty() && seen.insert(content_hash.to_string()) {
                    samples.push((height, content_hash.to_string()));
                }
            }
            height = match height.checked_add(1) {
                Some(next_height) => next_height,
                None => break,
            };
        }
        Ok(samples)
    }
}
