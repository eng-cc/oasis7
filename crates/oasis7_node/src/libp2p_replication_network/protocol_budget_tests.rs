use super::*;
use std::time::Duration;

#[test]
fn libp2p_replication_network_uses_long_transfer_budget_for_fetch_blob() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        request_timeout: Duration::from_millis(11),
        request_retry_budget: Duration::from_millis(22),
        fetch_blob_request_timeout: Duration::from_millis(333),
        fetch_blob_request_retry_budget: Duration::from_millis(444),
        ..Libp2pReplicationNetworkConfig::default()
    });

    assert_eq!(
        network.request_timeout_for_protocol("/aw/node/replication/fetch-blob/1.0.0"),
        Duration::from_millis(333)
    );
    assert_eq!(
        network.request_retry_budget_for_protocol("/aw/node/replication/fetch-blob/1.0.0"),
        Duration::from_millis(444)
    );
}

#[test]
fn libp2p_replication_network_uses_cold_archive_budget_for_fetch_commit() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        request_timeout: Duration::from_millis(11),
        request_retry_budget: Duration::from_millis(22),
        fetch_blob_request_timeout: Duration::from_millis(333),
        fetch_blob_request_retry_budget: Duration::from_millis(444),
        ..Libp2pReplicationNetworkConfig::default()
    });

    assert_eq!(
        network.request_timeout_for_protocol("/aw/node/replication/fetch-commit/1.0.0"),
        Duration::from_millis(30_000)
    );
    assert_eq!(
        network.request_retry_budget_for_protocol("/aw/node/replication/fetch-commit/1.0.0"),
        Duration::from_millis(120_000)
    );
}

#[test]
fn libp2p_replication_network_inner_timeout_covers_long_transfer_budgets() {
    assert_eq!(
        request_response_timeout_for_config(&Libp2pReplicationNetworkConfig::default()),
        Duration::from_millis(180_000)
    );

    assert_eq!(
        request_response_timeout_for_config(&Libp2pReplicationNetworkConfig {
            request_retry_budget: Duration::from_millis(22),
            fetch_blob_request_retry_budget: Duration::from_millis(444),
            ..Libp2pReplicationNetworkConfig::default()
        }),
        Duration::from_millis(120_000)
    );

    assert_eq!(
        request_response_timeout_for_config(&Libp2pReplicationNetworkConfig {
            request_retry_budget: Duration::from_millis(22),
            fetch_blob_request_retry_budget: Duration::from_millis(240_000),
            ..Libp2pReplicationNetworkConfig::default()
        }),
        Duration::from_millis(240_000)
    );
}

#[test]
fn libp2p_replication_network_keeps_short_budget_for_non_blob_protocols() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        request_timeout: Duration::from_millis(11),
        request_retry_budget: Duration::from_millis(22),
        fetch_blob_request_timeout: Duration::from_millis(333),
        fetch_blob_request_retry_budget: Duration::from_millis(444),
        ..Libp2pReplicationNetworkConfig::default()
    });

    assert_eq!(
        network.request_timeout_for_protocol("/aw/node/replication/ping"),
        Duration::from_millis(11)
    );
    assert_eq!(
        network.request_retry_budget_for_protocol("/aw/node/replication/ping"),
        Duration::from_millis(22)
    );
}
