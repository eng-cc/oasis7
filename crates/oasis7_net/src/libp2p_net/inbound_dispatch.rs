use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use libp2p::identity::Keypair;
use libp2p::request_response;
use libp2p::swarm::Swarm;
use libp2p::{Multiaddr, PeerId};
use oasis7_proto::distributed_dht::{PeerRecord, SignedPeerRecord};
use oasis7_proto::distributed_net::{NetworkRequest, NetworkResponse};

use crate::error::WorldError;

use super::discovery::handle_request_response_request;
use super::response_budget::FetchBlobResponseBudget;
use super::response_workers::ResponseWorkers;
use super::runtime_loop::{Handler, HandlerRegistration};
use super::{
    Behaviour, Libp2pReachabilitySnapshot, SharedLibp2pTrafficMetrics, now_ms, push_bounded_clone,
    record_request_inbound,
};

/// Run cheap request admission on the swarm loop before a scarce response-worker reservation.
pub(super) fn admit_inbound_handler(
    registration: &HandlerRegistration,
    payload: &[u8],
) -> Result<Handler, WorldError> {
    if let Some(admission) = registration.admission.as_ref() {
        admission(payload)?;
    }
    Ok(std::sync::Arc::clone(&registration.handler))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_inbound_request(
    request: NetworkRequest,
    channel: request_response::ResponseChannel<NetworkResponse>,
    peer: PeerId,
    handlers: &HashMap<String, HandlerRegistration>,
    response_workers: &ResponseWorkers,
    response_budget: &mut FetchBlobResponseBudget,
    traffic_metrics: &SharedLibp2pTrafficMetrics,
    swarm: &mut Swarm<Behaviour>,
    event_errors: &Arc<Mutex<Vec<String>>>,
    max_error_messages: usize,
    peer_record_template: Option<&PeerRecord>,
    keypair: &Keypair,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    allow_loopback_external_addrs_for_testing: bool,
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
) {
    record_request_inbound(
        traffic_metrics,
        request.protocol.as_str(),
        request.payload.len(),
    );
    if let Some(registration) = handlers.get(&request.protocol) {
        let handler = match admit_inbound_handler(registration, request.payload.as_slice()) {
            Ok(handler) => handler,
            Err(reply) => {
                response_workers.send_immediate(
                    channel,
                    request.protocol.as_str(),
                    peer,
                    Err(reply),
                    response_budget,
                    now_ms(),
                    traffic_metrics,
                    swarm,
                );
                return;
            }
        };
        if let Err(rejected) = response_workers.schedule(
            channel,
            request.protocol,
            peer,
            request.payload,
            now_ms(),
            handler,
        ) {
            let protocol = rejected.protocol;
            let peer = rejected.peer;
            response_workers.send_immediate(
                rejected.channel,
                protocol.as_str(),
                peer,
                Err(rejected.reply),
                response_budget,
                now_ms(),
                traffic_metrics,
                swarm,
            );
            push_bounded_clone(
                event_errors,
                format!("libp2p response worker queue full protocol={protocol} peer={peer}"),
                max_error_messages,
                "lock errors",
            );
        }
        return;
    }
    let reply = handle_request_response_request(
        &request,
        handlers,
        peer_record_template,
        keypair,
        listening_addrs,
        reachability,
        allow_loopback_external_addrs_for_testing,
        discovered_peer_records,
    );
    response_workers.send_immediate(
        channel,
        request.protocol.as_str(),
        peer,
        reply,
        response_budget,
        now_ms(),
        traffic_metrics,
        swarm,
    );
}
