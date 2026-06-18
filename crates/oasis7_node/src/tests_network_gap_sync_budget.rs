use std::fs;

use super::*;

#[derive(Clone)]
struct BudgetCapturingFetchCommitNetwork {
    response: super::replication::FetchCommitResponse,
    budgets: Arc<Mutex<Vec<(u64, u64, Vec<String>)>>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for BudgetCapturingFetchCommitNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        serde_json::to_vec(&super::replication::FetchCommitResponse {
            found: false,
            message: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode generic fetch commit response failed: {err}"),
        })
    }

    fn known_peer_ids(&self) -> Vec<String> {
        vec!["peer-a".to_string()]
    }

    fn request_with_providers_budget(
        &self,
        protocol: &str,
        _payload: &[u8],
        providers: &[String],
        request_timeout_ms: u64,
        retry_budget_ms: u64,
    ) -> Result<Vec<u8>, WorldError> {
        self.budgets
            .lock()
            .expect("lock budget captures")
            .push((request_timeout_ms, retry_budget_ms, providers.to_vec()));
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        let response = if providers.is_empty() {
            super::replication::FetchCommitResponse {
                found: false,
                message: None,
            }
        } else {
            self.response.clone()
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode budgeted fetch commit response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[derive(Clone)]
struct SlowFirstProviderFetchCommitNetwork {
    budgets: Arc<Mutex<Vec<(u64, u64, Vec<String>)>>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for SlowFirstProviderFetchCommitNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        serde_json::to_vec(&super::replication::FetchCommitResponse {
            found: false,
            message: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode generic fetch commit response failed: {err}"),
        })
    }

    fn known_peer_ids(&self) -> Vec<String> {
        vec!["peer-a".to_string(), "peer-b".to_string()]
    }

    fn request_with_providers_budget(
        &self,
        protocol: &str,
        _payload: &[u8],
        providers: &[String],
        request_timeout_ms: u64,
        retry_budget_ms: u64,
    ) -> Result<Vec<u8>, WorldError> {
        self.budgets
            .lock()
            .expect("lock budget captures")
            .push((request_timeout_ms, retry_budget_ms, providers.to_vec()));
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        if providers.is_empty() {
            return serde_json::to_vec(&super::replication::FetchCommitResponse {
                found: false,
                message: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode generic fetch commit response failed: {err}"),
            });
        }
        if providers.first().map(String::as_str) == Some("peer-a") {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrTimeout,
                message: protocol.to_string(),
                retryable: true,
            });
        }
        serde_json::to_vec(&super::replication::FetchCommitResponse {
            found: true,
            message: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode provider fetch commit response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[test]
fn gap_sync_fetch_commit_uses_cold_archive_request_budget() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-cold-budget-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-cold-budget-local");
    let world_id = "world-gap-sync-fetch-commit-cold-budget";
    let budgets = Arc::new(Mutex::new(Vec::<(u64, u64, Vec<String>)>::new()));
    let (_, _, endpoint, _) = network_gap_sync_tests::build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        138,
        139,
        Arc::new(BudgetCapturingFetchCommitNetwork {
            response: super::replication::FetchCommitResponse {
                found: true,
                message: None,
            },
            budgets: Arc::clone(&budgets),
        }),
    );
    let request = signed_fetch_commit_request_for_test(world_id, 1, 139);

    let response = endpoint
        .request_fetch_commit_for_gap_sync(&request)
        .expect("gap-sync fetch commit response");
    assert!(
        response.response.found,
        "expected provider-directed fetch commit to recover after generic not-found"
    );

    let captures = budgets.lock().expect("lock budget captures").clone();
    let (first_timeout_ms, first_budget_ms, _) =
        captures.first().expect("at least one budget capture");
    assert_eq!(
        (*first_timeout_ms, *first_budget_ms),
        (30_000, 30_000),
        "initial gap-sync fetch-commit route must start with cold archive per-route budget, got {captures:?}"
    );
    assert!(
        captures.iter().all(|(timeout_ms, budget_ms, _)| {
            *budget_ms <= 30_000 && *timeout_ms == 30_000_u64.min(*budget_ms)
        }),
        "gap-sync fetch-commit route budgets must cap each route while sharing one sweep budget, got {captures:?}"
    );
    assert!(
        captures.iter().any(|(_, _, providers)| providers.is_empty()),
        "expected generic gap-sync fetch-commit attempt, got {captures:?}"
    );
    assert!(
        captures
            .iter()
            .any(|(_, _, providers)| providers == &vec!["peer-a".to_string()]),
        "expected provider-directed gap-sync fetch-commit attempt, got {captures:?}"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn gap_sync_fetch_commit_provider_sweep_preserves_budget_for_next_provider() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-provider-budget-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-provider-budget-local");
    let world_id = "world-gap-sync-fetch-commit-provider-budget";
    let budgets = Arc::new(Mutex::new(Vec::<(u64, u64, Vec<String>)>::new()));
    let (_, _, endpoint, _) = network_gap_sync_tests::build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        140,
        141,
        Arc::new(SlowFirstProviderFetchCommitNetwork {
            budgets: Arc::clone(&budgets),
        }),
    );
    let request = signed_fetch_commit_request_for_test(world_id, 1, 141);

    let response = endpoint
        .request_fetch_commit_for_gap_sync(&request)
        .expect("provider sweep should continue after first provider timeout");
    assert!(
        response.response.found,
        "second provider should recover the missing commit"
    );

    let captures = budgets.lock().expect("lock budget captures").clone();
    assert!(
        captures
            .iter()
            .any(|(timeout_ms, budget_ms, providers)| *timeout_ms <= 15_000
                && *budget_ms <= 15_000
                && providers == &vec!["peer-a".to_string()]),
        "first provider route should not consume the full sweep budget: {captures:?}"
    );
    assert!(
        captures
            .iter()
            .any(|(_, _, providers)| providers == &vec!["peer-b".to_string()]),
        "provider sweep should try the next provider after timeout: {captures:?}"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
