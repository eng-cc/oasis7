#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerManagerPolicy {
    pub min_active_discovery_sources: usize,
    pub min_peer_discovery_sources: usize,
    pub min_active_peers_for_share_limits: usize,
    pub max_ipv4_subnet_active_peers: Option<usize>,
    pub max_ipv4_subnet_share_per_mille: u16,
    pub block_ipv4_subnet_share_per_mille: u16,
    pub max_relay_domain_share_per_mille: u16,
    pub block_relay_domain_share_per_mille: u16,
    pub max_operator_share_per_mille: u16,
    pub block_operator_share_per_mille: u16,
    pub max_asn_share_per_mille: u16,
    pub block_asn_share_per_mille: u16,
    pub max_relayed_active_peer_share_per_mille: u16,
}

impl Default for PeerManagerPolicy {
    fn default() -> Self {
        Self {
            min_active_discovery_sources: 2,
            min_peer_discovery_sources: 2,
            min_active_peers_for_share_limits: 4,
            max_ipv4_subnet_active_peers: None,
            max_ipv4_subnet_share_per_mille: 250,
            block_ipv4_subnet_share_per_mille: 500,
            max_relay_domain_share_per_mille: 250,
            block_relay_domain_share_per_mille: 500,
            max_operator_share_per_mille: 250,
            block_operator_share_per_mille: 500,
            max_asn_share_per_mille: 250,
            block_asn_share_per_mille: 500,
            max_relayed_active_peer_share_per_mille: 500,
        }
    }
}

impl PeerManagerPolicy {
    pub(super) fn applies_share_limits(&self, active_peer_count: usize) -> bool {
        active_peer_count >= self.min_active_peers_for_share_limits
    }
}
