use super::distributed_dht::ProviderRecord;

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
}
