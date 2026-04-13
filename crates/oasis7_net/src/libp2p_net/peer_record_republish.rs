use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use libp2p::identity::Keypair;
use libp2p::kad;
use libp2p::swarm::Swarm;
use libp2p::Multiaddr;

use oasis7_proto::distributed_dht::PeerRecord;

use super::discovery::publish_discovery_provider;
use super::kad_queries::PendingDhtQuery;
use super::peer_record::publish_configured_peer_record;
use super::swarm_behaviour::Behaviour;
use super::utils::now_ms;

pub(super) fn republish_local_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    provider_keys: &mut HashMap<String, i64>,
    keypair: &Keypair,
    peer_record_template: Option<&PeerRecord>,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
    peer_record_last_published_at_ms: &mut Option<i64>,
) {
    if let Some(template) = peer_record_template {
        let _ = publish_configured_peer_record(
            swarm,
            pending_dht,
            keypair,
            template,
            listening_addrs,
            None,
        );
        *peer_record_last_published_at_ms = Some(now_ms());
        publish_discovery_provider(swarm, provider_keys, template.world_id.as_str());
    }
}
