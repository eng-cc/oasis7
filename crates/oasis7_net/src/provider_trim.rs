use super::distributed_dht::ProviderRecord;

const BOUNDED_RECENCY_TRIM_MAX: usize = 64;

pub(crate) fn trim_providers_by_recency(
    providers: Vec<ProviderRecord>,
    max_providers: usize,
) -> Vec<ProviderRecord> {
    if max_providers == 0 {
        return providers;
    }

    if providers.len() <= max_providers {
        return sort_recent_first(providers);
    }

    if max_providers > BOUNDED_RECENCY_TRIM_MAX {
        return sort_recent_first_limited(providers, max_providers);
    }

    let mut top = Vec::with_capacity(max_providers);
    for (index, provider) in providers.into_iter().enumerate() {
        if top.len() < max_providers {
            top.push((index, provider));
            continue;
        }

        let Some((worst_index, _)) =
            top.iter()
                .enumerate()
                .min_by(|(_, (left_index, left)), (_, (right_index, right))| {
                    left.last_seen_ms
                        .cmp(&right.last_seen_ms)
                        .then_with(|| right_index.cmp(left_index))
                })
        else {
            continue;
        };

        if provider.last_seen_ms > top[worst_index].1.last_seen_ms {
            top[worst_index] = (index, provider);
        }
    }

    top.sort_by(|(left_index, left), (right_index, right)| {
        right
            .last_seen_ms
            .cmp(&left.last_seen_ms)
            .then_with(|| left_index.cmp(right_index))
    });
    top.into_iter().map(|(_, provider)| provider).collect()
}

fn sort_recent_first(mut providers: Vec<ProviderRecord>) -> Vec<ProviderRecord> {
    providers.sort_by_key(|provider| std::cmp::Reverse(provider.last_seen_ms));
    providers
}

fn sort_recent_first_limited(
    providers: Vec<ProviderRecord>,
    max_providers: usize,
) -> Vec<ProviderRecord> {
    let mut providers = sort_recent_first(providers);
    providers.truncate(max_providers);
    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(provider_id: &str, last_seen_ms: i64) -> ProviderRecord {
        ProviderRecord {
            provider_id: provider_id.to_string(),
            last_seen_ms,
            storage_total_bytes: None,
            storage_available_bytes: None,
            uptime_ratio_per_mille: None,
            challenge_pass_ratio_per_mille: None,
            load_ratio_per_mille: None,
            p50_read_latency_ms: None,
        }
    }

    fn provider_ids(providers: Vec<ProviderRecord>) -> Vec<String> {
        providers
            .into_iter()
            .map(|provider| provider.provider_id)
            .collect()
    }

    #[test]
    fn trim_providers_by_recency_keeps_unlimited_input_order() {
        let providers = vec![provider("older", 10), provider("newer", 20)];

        assert_eq!(
            provider_ids(trim_providers_by_recency(providers, 0)),
            vec!["older", "newer"]
        );
    }

    #[test]
    fn trim_providers_by_recency_sorts_when_under_limit() {
        let providers = vec![provider("older", 10), provider("newer", 20)];

        assert_eq!(
            provider_ids(trim_providers_by_recency(providers, 8)),
            vec!["newer", "older"]
        );
    }

    #[test]
    fn trim_providers_by_recency_keeps_only_recent_limit() {
        let providers = vec![
            provider("stale", 1),
            provider("freshest", 5),
            provider("fresh", 4),
            provider("old", 2),
        ];

        assert_eq!(
            provider_ids(trim_providers_by_recency(providers, 2)),
            vec!["freshest", "fresh"]
        );
    }

    #[test]
    fn trim_providers_by_recency_preserves_equal_timestamp_order() {
        let providers = vec![
            provider("first", 10),
            provider("second", 10),
            provider("third", 10),
        ];

        assert_eq!(
            provider_ids(trim_providers_by_recency(providers, 2)),
            vec!["first", "second"]
        );
    }

    #[test]
    fn trim_providers_by_recency_uses_sort_fallback_for_large_limits() {
        let providers = (0..80)
            .map(|index| provider(&format!("peer-{index}"), index))
            .collect();

        let trimmed = trim_providers_by_recency(providers, 65);

        assert_eq!(trimmed.len(), 65);
        assert_eq!(trimmed[0].provider_id, "peer-79");
        assert_eq!(trimmed[64].provider_id, "peer-15");
    }
}
