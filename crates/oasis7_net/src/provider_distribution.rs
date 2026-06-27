use std::collections::{BTreeMap, BTreeSet};

use super::distributed_dht::DistributedDht;
use super::error::WorldError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDistributionPolicy {
    pub min_replicas_per_blob: usize,
    pub forbid_single_provider_full_coverage: bool,
}

impl Default for ProviderDistributionPolicy {
    fn default() -> Self {
        Self {
            min_replicas_per_blob: 2,
            forbid_single_provider_full_coverage: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDistributionAudit {
    pub required_blob_count: usize,
    pub distinct_provider_count: usize,
}

pub fn audit_provider_distribution(
    dht: &impl DistributedDht,
    world_id: &str,
    content_hashes: &[String],
    policy: ProviderDistributionPolicy,
) -> Result<ProviderDistributionAudit, WorldError> {
    if policy.min_replicas_per_blob == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "provider distribution policy requires min_replicas_per_blob > 0".to_string(),
        });
    }

    let required_hashes = normalize_required_hashes(content_hashes);
    if required_hashes.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "provider distribution audit requires at least one content hash".to_string(),
        });
    }

    let mut provider_coverage_count: BTreeMap<String, usize> = BTreeMap::new();
    for content_hash in &required_hashes {
        let providers = dht.get_providers(world_id, content_hash)?;
        let provider_ids: BTreeSet<&str> = providers
            .iter()
            .map(|provider| provider.provider_id.as_str())
            .collect();
        let replica_count = provider_ids.len();
        if replica_count < policy.min_replicas_per_blob {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "provider replicas insufficient for content_hash={content_hash}: required={}, actual={replica_count}",
                    policy.min_replicas_per_blob
                ),
            });
        }
        for provider_id in provider_ids {
            if let Some(coverage) = provider_coverage_count.get_mut(provider_id) {
                *coverage = coverage.saturating_add(1);
            } else {
                provider_coverage_count.insert(provider_id.to_string(), 1);
            }
        }
    }

    if policy.forbid_single_provider_full_coverage && required_hashes.len() > 1 {
        for (provider_id, covered_hashes) in &provider_coverage_count {
            if *covered_hashes == required_hashes.len() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "provider full coverage forbidden: provider_id={provider_id} covers all {} required blobs",
                        required_hashes.len()
                    ),
                });
            }
        }
    }

    Ok(ProviderDistributionAudit {
        required_blob_count: required_hashes.len(),
        distinct_provider_count: provider_coverage_count.len(),
    })
}

fn normalize_required_hashes(content_hashes: &[String]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for content_hash in content_hashes {
        unique.insert(content_hash.clone());
    }
    unique.into_iter().collect()
}
