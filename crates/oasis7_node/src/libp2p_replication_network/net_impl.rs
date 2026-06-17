use std::sync::Arc;
use std::time::Duration;

use oasis7_proto::distributed_net::{
    DistributedNetwork as ProtoDistributedNetwork, NetworkSubscription,
};
use oasis7_proto::world_error::WorldError;

use super::{Handler, Libp2pReplicationNetwork};

impl Libp2pReplicationNetwork {
    pub fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        ProtoDistributedNetwork::request(self, protocol, payload)
    }

    pub fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        ProtoDistributedNetwork::request_with_providers(self, protocol, payload, providers)
    }

    pub fn known_peer_ids(&self) -> Vec<String> {
        ProtoDistributedNetwork::known_peer_ids(self)
    }

    pub fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        ProtoDistributedNetwork::register_handler(self, protocol, handler)
    }
}

impl ProtoDistributedNetwork<WorldError> for Libp2pReplicationNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn publish_best_effort(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish_best_effort(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let peers = self.wait_for_connected_peers();
        if peers.is_empty() {
            if self.allow_local_handler_fallback_when_no_peers {
                return self.call_local_handler(protocol, payload);
            }
            let detail = if self.inner.connected_peers().is_empty() {
                "no connected peers"
            } else {
                "no admissible connected peers"
            };
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication {detail} for protocol {protocol}"),
            });
        }
        self.request_over_refreshed_peers(
            protocol,
            payload,
            peers,
            || self.connected_peers_sorted(),
            || WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            },
        )
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        let mut peers = self
            .connected_peers()
            .into_iter()
            .map(|peer_id| peer_id.to_string())
            .collect::<Vec<_>>();
        peers.sort();
        peers.dedup();
        peers
    }

    fn known_peer_ids(&self) -> Vec<String> {
        let mut peers = self.connected_peer_ids();
        peers.extend(
            self.inner
                .debug_peer_healths()
                .into_iter()
                .map(|health| health.peer_id),
        );
        peers.extend(
            self.bootstrap_addrs_by_peer_id
                .keys()
                .map(ToString::to_string),
        );
        peers.sort();
        peers.dedup();
        peers
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if providers.is_empty() {
            return self.request(protocol, payload);
        }
        let peers = self.wait_for_connected_provider_peers(providers);
        if peers.is_empty() {
            return Err(no_connected_providers(protocol));
        }
        self.request_over_refreshed_peers(
            protocol,
            payload,
            peers,
            || self.collect_connected_provider_peers(providers),
            || no_connected_providers(protocol),
        )
    }

    fn request_with_providers_budget(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
        request_timeout_ms: u64,
        retry_budget_ms: u64,
    ) -> Result<Vec<u8>, WorldError> {
        let request_timeout = Duration::from_millis(request_timeout_ms);
        let retry_budget = Duration::from_millis(retry_budget_ms);
        if providers.is_empty() {
            let peers = self.wait_for_connected_peers();
            if peers.is_empty() {
                return self.request(protocol, payload);
            }
            return self.request_over_refreshed_peers_with_budget(
                protocol,
                payload,
                peers,
                || self.connected_peers_sorted(),
                || WorldError::NetworkProtocolUnavailable {
                    protocol: format!(
                        "libp2p-replication no connected peers for protocol {protocol}"
                    ),
                },
                request_timeout,
                retry_budget,
            );
        }
        let started_at = std::time::Instant::now();
        let peers = self.wait_for_connected_provider_peers_within(providers, retry_budget);
        if peers.is_empty() {
            return Err(no_connected_providers(protocol));
        }
        let retry_budget = retry_budget.saturating_sub(started_at.elapsed());
        self.request_over_refreshed_peers_with_budget(
            protocol,
            payload,
            peers,
            || self.collect_connected_provider_peers(providers),
            || no_connected_providers(protocol),
            request_timeout,
            retry_budget,
        )
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        let handler: Handler = Arc::from(handler);
        self.inner.register_handler(
            protocol,
            Box::new({
                let handler = Arc::clone(&handler);
                move |payload| handler(payload)
            }),
        )?;
        self.handlers
            .lock()
            .expect("lock libp2p replication handlers")
            .insert(protocol.to_string(), handler);
        Ok(())
    }
}

fn no_connected_providers(protocol: &str) -> WorldError {
    WorldError::NetworkProtocolUnavailable {
        protocol: format!("libp2p-replication no connected providers for protocol {protocol}"),
    }
}
