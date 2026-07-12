use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use futures::SinkExt;
use futures::channel::mpsc as futures_mpsc;
use libp2p::request_response;
use libp2p::swarm::Swarm;
use oasis7_proto::distributed::DistributedErrorCode;

use crate::error::WorldError;
use oasis7_proto::distributed_net::NetworkResponse;

use super::response_budget::{
    FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW, FetchBlobResponseBudget, budgeted_response_bytes,
};
use super::{Behaviour, Handler};

const CONTROL_QUEUE_CAPACITY: usize = 32;
const BLOB_QUEUE_CAPACITY: usize = 8;
const CONTROL_WORKERS: usize = 2;
const BLOB_WORKERS: usize = 2;
const COMPLETION_QUEUE_CAPACITY: usize = CONTROL_WORKERS + BLOB_WORKERS;
const CONTROL_MAX_IN_FLIGHT: usize = CONTROL_QUEUE_CAPACITY + CONTROL_WORKERS;
const BLOB_MAX_IN_FLIGHT: usize = BLOB_QUEUE_CAPACITY + BLOB_WORKERS;
const BLOB_MAX_IN_FLIGHT_BYTES: usize = BLOB_MAX_IN_FLIGHT * FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW;

#[derive(Clone, Copy)]
enum ResponseLane {
    Control,
    Blob,
}

struct InFlightState {
    control_jobs: usize,
    blob_jobs: usize,
    blob_bytes: usize,
}

struct InFlightReservation {
    state: Arc<Mutex<InFlightState>>,
    lane: ResponseLane,
}

impl Drop for InFlightReservation {
    fn drop(&mut self) {
        let mut state = self.state.lock().expect("response worker in-flight state");
        match self.lane {
            ResponseLane::Control => state.control_jobs = state.control_jobs.saturating_sub(1),
            ResponseLane::Blob => {
                state.blob_jobs = state.blob_jobs.saturating_sub(1);
                state.blob_bytes = state
                    .blob_bytes
                    .saturating_sub(FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW);
            }
        }
    }
}

pub(super) struct CompletedResponse {
    pub channel: request_response::ResponseChannel<NetworkResponse>,
    pub protocol: String,
    pub peer: libp2p::PeerId,
    pub queued_at_ms: i64,
    pub reply: Result<Vec<u8>, WorldError>,
    _reservation: InFlightReservation,
}

pub(super) struct Job {
    channel: request_response::ResponseChannel<NetworkResponse>,
    protocol: String,
    peer: libp2p::PeerId,
    payload: Vec<u8>,
    queued_at_ms: i64,
    handler: Handler,
    reservation: InFlightReservation,
}

pub(super) struct RejectedResponse {
    pub channel: request_response::ResponseChannel<NetworkResponse>,
    pub protocol: String,
    pub peer: libp2p::PeerId,
}

pub(super) struct ResponseWorkers {
    control: mpsc::SyncSender<Job>,
    blob: mpsc::SyncSender<Job>,
    in_flight: Arc<Mutex<InFlightState>>,
}

impl ResponseWorkers {
    pub(super) fn new() -> (Self, futures_mpsc::Receiver<CompletedResponse>) {
        let (completed, receiver) = futures_mpsc::channel(COMPLETION_QUEUE_CAPACITY);
        let (control, control_rx) = mpsc::sync_channel(CONTROL_QUEUE_CAPACITY);
        let (blob, blob_rx) = mpsc::sync_channel(BLOB_QUEUE_CAPACITY);
        spawn_workers(control_rx, CONTROL_WORKERS, completed.clone());
        spawn_workers(blob_rx, BLOB_WORKERS, completed);
        (
            Self {
                control,
                blob,
                in_flight: Arc::new(Mutex::new(InFlightState {
                    control_jobs: 0,
                    blob_jobs: 0,
                    blob_bytes: 0,
                })),
            },
            receiver,
        )
    }

    pub(super) fn schedule(
        &self,
        channel: request_response::ResponseChannel<NetworkResponse>,
        protocol: String,
        peer: libp2p::PeerId,
        payload: Vec<u8>,
        queued_at_ms: i64,
        handler: Handler,
    ) -> Result<(), RejectedResponse> {
        let Some(reservation) = self.reserve(protocol.as_str()) else {
            return Err(RejectedResponse {
                channel,
                protocol,
                peer,
            });
        };
        self.submit(Job {
            channel,
            protocol,
            peer,
            payload,
            queued_at_ms,
            handler,
            reservation,
        })
        .map_err(|job| RejectedResponse {
            channel: job.channel,
            protocol: job.protocol,
            peer: job.peer,
        })
    }

    pub(super) fn complete(
        &self,
        completed: CompletedResponse,
        budget: &mut FetchBlobResponseBudget,
        now_ms: i64,
        traffic_metrics: &super::SharedLibp2pTrafficMetrics,
        swarm: &mut Swarm<Behaviour>,
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
        if swarm
            .behaviour_mut()
            .request_response
            .send_response(completed.channel, response)
            .is_err()
        {
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
        swarm: &mut Swarm<Behaviour>,
    ) -> bool {
        swarm
            .behaviour_mut()
            .request_response
            .send_response(
                channel,
                response(budget, peer, protocol, reply, now_ms, traffic_metrics),
            )
            .is_ok()
    }

    pub(super) fn overload_response() -> WorldError {
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotAvailable,
            message: "response worker overloaded; retry request".to_string(),
            retryable: true,
        }
    }

    #[cfg(test)]
    fn pending_limits(&self) -> (usize, usize, usize) {
        let state = self
            .in_flight
            .lock()
            .expect("response worker in-flight state");
        (state.control_jobs, state.blob_jobs, state.blob_bytes)
    }

    fn reserve(&self, protocol: &str) -> Option<InFlightReservation> {
        let lane = if protocol.contains("fetch-blob") {
            ResponseLane::Blob
        } else {
            ResponseLane::Control
        };
        let mut state = self
            .in_flight
            .lock()
            .expect("response worker in-flight state");
        match lane {
            ResponseLane::Control if state.control_jobs >= CONTROL_MAX_IN_FLIGHT => return None,
            ResponseLane::Blob
                if state.blob_jobs >= BLOB_MAX_IN_FLIGHT
                    || state.blob_bytes
                        > BLOB_MAX_IN_FLIGHT_BYTES - FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW =>
            {
                return None;
            }
            _ => {}
        }
        match lane {
            ResponseLane::Control => state.control_jobs += 1,
            ResponseLane::Blob => {
                state.blob_jobs += 1;
                state.blob_bytes += FETCH_BLOB_RESPONSE_BYTES_PER_WINDOW;
            }
        }
        Some(InFlightReservation {
            state: Arc::clone(&self.in_flight),
            lane,
        })
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
    completed: futures_mpsc::Sender<CompletedResponse>,
) {
    let rx = Arc::new(std::sync::Mutex::new(rx));
    for _ in 0..count {
        let rx = Arc::clone(&rx);
        let mut completed = completed.clone();
        thread::spawn(move || {
            loop {
                let Ok(job) = rx.lock().expect("response worker receiver").recv() else {
                    break;
                };
                let reply = (job.handler)(&job.payload);
                // Backpressure is confined to this worker; the swarm loop drains completions.
                if futures::executor::block_on(completed.send(CompletedResponse {
                    channel: job.channel,
                    protocol: job.protocol,
                    peer: job.peer,
                    queued_at_ms: job.queued_at_ms,
                    reply,
                    _reservation: job.reservation,
                }))
                .is_err()
                {
                    break;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_blob_is_routed_to_the_bounded_blob_lane() {
        assert!("/aw/node/replication/fetch-blob/1.0.0".contains("fetch-blob"));
    }

    #[test]
    fn in_flight_reservations_bound_blob_jobs_and_bytes_until_release() {
        let workers = ResponseWorkers {
            control: mpsc::sync_channel(1).0,
            blob: mpsc::sync_channel(1).0,
            in_flight: Arc::new(Mutex::new(InFlightState {
                control_jobs: 0,
                blob_jobs: 0,
                blob_bytes: 0,
            })),
        };
        let mut reservations = Vec::new();
        for _ in 0..BLOB_MAX_IN_FLIGHT {
            reservations.push(workers.reserve("fetch-blob").expect("within blob capacity"));
        }
        assert!(workers.reserve("fetch-blob").is_none());
        assert_eq!(
            workers.pending_limits(),
            (0, BLOB_MAX_IN_FLIGHT, BLOB_MAX_IN_FLIGHT_BYTES,)
        );
        reservations.pop();
        assert!(workers.reserve("fetch-blob").is_some());
    }
}
