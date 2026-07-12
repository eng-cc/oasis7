use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use futures::channel::mpsc as futures_mpsc;
use libp2p::request_response;

use crate::error::WorldError;
use oasis7_proto::distributed_net::NetworkResponse;

use super::Handler;
use super::response_budget::{FetchBlobResponseBudget, budgeted_response_bytes};

const CONTROL_QUEUE_CAPACITY: usize = 32;
const BLOB_QUEUE_CAPACITY: usize = 8;
const CONTROL_WORKERS: usize = 2;
const BLOB_WORKERS: usize = 2;

pub(super) struct CompletedResponse {
    pub channel: request_response::ResponseChannel<NetworkResponse>,
    pub protocol: String,
    pub peer: libp2p::PeerId,
    pub queued_at_ms: i64,
    pub reply: Result<Vec<u8>, WorldError>,
}

pub(super) struct Job {
    channel: request_response::ResponseChannel<NetworkResponse>,
    protocol: String,
    peer: libp2p::PeerId,
    payload: Vec<u8>,
    queued_at_ms: i64,
    handler: Handler,
}

pub(super) struct ResponseWorkers {
    control: mpsc::SyncSender<Job>,
    blob: mpsc::SyncSender<Job>,
}

impl ResponseWorkers {
    pub(super) fn new() -> (Self, futures_mpsc::UnboundedReceiver<CompletedResponse>) {
        let (completed, receiver) = futures_mpsc::unbounded();
        let (control, control_rx) = mpsc::sync_channel(CONTROL_QUEUE_CAPACITY);
        let (blob, blob_rx) = mpsc::sync_channel(BLOB_QUEUE_CAPACITY);
        spawn_workers(control_rx, CONTROL_WORKERS, completed.clone());
        spawn_workers(blob_rx, BLOB_WORKERS, completed);
        (Self { control, blob }, receiver)
    }

    pub(super) fn schedule(
        &self,
        channel: request_response::ResponseChannel<NetworkResponse>,
        protocol: String,
        peer: libp2p::PeerId,
        payload: Vec<u8>,
        queued_at_ms: i64,
        handler: Handler,
        event_errors: &Arc<Mutex<Vec<String>>>,
        max_error_messages: usize,
    ) {
        if self
            .submit(Job {
                channel,
                protocol,
                peer,
                payload,
                queued_at_ms,
                handler,
            })
            .is_err()
        {
            super::push_bounded_clone(
                event_errors,
                "libp2p response worker queue full".to_string(),
                max_error_messages,
                "lock errors",
            );
        }
    }

    pub(super) fn complete(
        &self,
        completed: CompletedResponse,
        budget: &mut FetchBlobResponseBudget,
        now_ms: i64,
        traffic_metrics: &super::SharedLibp2pTrafficMetrics,
        send: impl FnOnce(request_response::ResponseChannel<NetworkResponse>, NetworkResponse) -> bool,
        event_errors: &Arc<Mutex<Vec<String>>>,
        max_error_messages: usize,
    ) {
        let response = response(
            budget,
            completed.peer,
            completed.protocol.as_str(),
            completed.reply,
            now_ms,
            traffic_metrics,
        );
        if !send(completed.channel, response) {
            super::push_bounded_clone(
                event_errors,
                format!(
                    "libp2p response cancelled before send protocol={} peer={} queue_wait_ms={}",
                    completed.protocol,
                    completed.peer,
                    now_ms.saturating_sub(completed.queued_at_ms)
                ),
                max_error_messages,
                "lock errors",
            );
        }
    }

    pub(super) fn send_immediate(
        &self,
        channel: request_response::ResponseChannel<NetworkResponse>,
        protocol: &str,
        peer: libp2p::PeerId,
        reply: Result<Vec<u8>, WorldError>,
        budget: &mut FetchBlobResponseBudget,
        now_ms: i64,
        traffic_metrics: &super::SharedLibp2pTrafficMetrics,
        send: impl FnOnce(request_response::ResponseChannel<NetworkResponse>, NetworkResponse),
    ) {
        send(
            channel,
            response(budget, peer, protocol, reply, now_ms, traffic_metrics),
        );
    }

    fn submit(&self, job: Job) -> Result<(), Job> {
        let sender = if job.protocol.contains("fetch-blob") {
            &self.blob
        } else {
            &self.control
        };
        sender.try_send(job).map_err(|error| match error {
            mpsc::TrySendError::Full(job) | mpsc::TrySendError::Disconnected(job) => job,
        })
    }
}

fn response(
    budget: &mut FetchBlobResponseBudget,
    peer: libp2p::PeerId,
    protocol: &str,
    reply: Result<Vec<u8>, WorldError>,
    now_ms: i64,
    traffic_metrics: &super::SharedLibp2pTrafficMetrics,
) -> NetworkResponse {
    let payload = budgeted_response_bytes(budget, peer, protocol, reply, now_ms);
    super::record_response_outbound(traffic_metrics, protocol, payload.len());
    NetworkResponse { payload }
}

fn spawn_workers(
    rx: mpsc::Receiver<Job>,
    count: usize,
    completed: futures_mpsc::UnboundedSender<CompletedResponse>,
) {
    let rx = Arc::new(std::sync::Mutex::new(rx));
    for _ in 0..count {
        let rx = Arc::clone(&rx);
        let completed = completed.clone();
        thread::spawn(move || {
            loop {
                let Ok(job) = rx.lock().expect("response worker receiver").recv() else {
                    break;
                };
                let reply = (job.handler)(&job.payload);
                let _ = completed.unbounded_send(CompletedResponse {
                    channel: job.channel,
                    protocol: job.protocol,
                    peer: job.peer,
                    queued_at_ms: job.queued_at_ms,
                    reply,
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fetch_blob_is_routed_to_the_bounded_blob_lane() {
        assert!("/aw/node/replication/fetch-blob/1.0.0".contains("fetch-blob"));
    }
}
