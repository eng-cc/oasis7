//! Periodic service reads never occupy a connection's receive loop.
//! One worker and one queued job belong to one Viewer server. Each connection
//! owns at most one result mailbox; disconnect drops it without joining I/O.
use super::chain_link::PreparedChainLinkedRuntimeUpdate;
use super::world_service_link::prepare_world_service_update_until;
use super::*;
use crate::world_service::WorldServicePayloadV1;
use crate::world_service::client::{WorldServiceClientConfig, WorldServiceQueryState};
use crate::world_service::verified_view::VerifiedWorldView;
use oasis7_client_api::world_service::RequestCorrelation;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};

type Pending = Vec<(RequestCorrelation, WorldServicePayloadV1)>;

pub(super) struct Executor {
    sender: SyncSender<Job>,
    outstanding: Arc<AtomicUsize>,
    #[cfg(any(test, feature = "test_tier_required"))]
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

struct Permit(Arc<AtomicUsize>);
impl Permit {
    fn acquire(outstanding: &Arc<AtomicUsize>) -> Option<Self> {
        outstanding
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .ok()?;
        Some(Self(outstanding.clone()))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

struct Job {
    token: Token,
    pending: Pending,
    query_state: WorldServiceQueryState,
    destination: SyncSender<Completion>,
    permit: Permit,
}

struct Token {
    config: WorldServiceClientConfig,
    baseline: VerifiedWorldView,
    player: Option<String>,
    streams: HashSet<ViewerStream>,
    event_filters: Option<HashSet<ViewerEventKind>>,
    protocol: crate::viewer::protocol::NegotiatedViewerProtocol,
}

struct Completion {
    token: Token,
    originals: Pending,
    outcome: Result<PreparedChainLinkedRuntimeUpdate, ViewerRuntimeLiveServerError>,
    _permit: Permit,
}

impl Executor {
    fn new() -> std::io::Result<Arc<Self>> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(1);
        let worker = std::thread::Builder::new()
            .name("viewer-periodic-service".into())
            .spawn(move || {
                // The worker owns no server/Sidecar Arc. Closing the server's
                // sender drains at most one queued job, then terminates it.
                while let Ok(job) = receiver.recv() {
                    let outcome = prepare_world_service_update_until(
                        job.token.config.clone(),
                        Some(job.token.baseline.clone()),
                        job.pending.clone(),
                        job.query_state,
                        Some(Instant::now() + Duration::from_secs(2)),
                    );
                    let _ = job.destination.try_send(Completion {
                        token: job.token,
                        originals: job.pending,
                        outcome,
                        _permit: job.permit,
                    });
                }
            })?;
        #[cfg(not(any(test, feature = "test_tier_required")))]
        drop(worker);
        Ok(Arc::new(Self {
            sender,
            outstanding: Arc::new(AtomicUsize::new(0)),
            #[cfg(any(test, feature = "test_tier_required"))]
            worker: Mutex::new(Some(worker)),
        }))
    }
}

#[derive(Default)]
pub(super) struct SessionPoll {
    result: Option<Receiver<Completion>>,
    executor: Option<Arc<Executor>>,
    retry_at: Option<Instant>,
}

fn same_config(left: &WorldServiceClientConfig, right: &WorldServiceClientConfig) -> bool {
    left.endpoint == right.endpoint
        && left.trusted_service_public_key == right.trusted_service_public_key
        && left.expected_world == right.expected_world
        && left.scope_id == right.scope_id
        && left.read_private_key_hex == right.read_private_key_hex
        && left.timeout == right.timeout
        && left.max_response_bytes == right.max_response_bytes
}

impl Token {
    fn matches(&self, server: &ViewerRuntimeLiveServer, session: &RuntimeLiveSession) -> bool {
        server.authoritative_recovery_write_fence.is_none()
            && server
                .config
                .world_service
                .as_ref()
                .is_some_and(|current| same_config(current, &self.config))
            && server.verified_world_view.as_ref().is_some_and(|current| {
                current.version() == self.baseline.version()
                    && current.continuation() == self.baseline.continuation()
            })
            && session.current_player_id == self.player
            && session.subscribed == self.streams
            && session.event_filters == self.event_filters
            && session.negotiated_protocol == self.protocol
    }
}

impl SessionPoll {
    fn defer(&mut self, interval: Duration) {
        self.retry_at = Some(
            Instant::now() + interval.clamp(Duration::from_millis(250), Duration::from_secs(2)),
        );
    }

    pub(super) fn poll(
        &mut self,
        shared: &Arc<Mutex<ViewerRuntimeLiveServer>>,
        session: &mut RuntimeLiveSession,
        writer: &mut dyn Write,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        if let Some(receiver) = &self.result {
            match receiver.try_recv() {
                Ok(completion) => {
                    self.result = None;
                    let mut server = lock_shared_server(shared)?;
                    let interval = server.config.chain_poll_interval;
                    self.defer(interval);
                    if !completion.token.matches(&server, session) {
                        // Another verified read/configuration/session advanced
                        // while I/O ran. Re-read its cursor; never publish gaps.
                        return Ok(());
                    }
                    let mut prepared = match completion.outcome {
                        Ok(prepared) => prepared,
                        Err(_) => {
                            server.set_latest_player_gameplay_feedback(SelfFeedback::failure());
                            return Err(ViewerRuntimeLiveServerError::Init(
                                "periodic service read deferred; original intents retained".into(),
                            ));
                        }
                    };
                    let next = prepared.verified_view.as_ref().ok_or_else(|| {
                        ViewerRuntimeLiveServerError::Init(
                            "periodic service read lacks verified view".into(),
                        )
                    })?;
                    let baseline = completion.token.baseline.continuation();
                    let cursor = next.continuation();
                    if cursor.stream_id != baseline.stream_id
                        || cursor.scope_id != baseline.scope_id
                        || cursor.era != baseline.era
                        || cursor.sequence < baseline.sequence
                        || !next
                            .version()
                            .commit
                            .satisfies_minimum(&completion.token.baseline.version().commit)
                            .map_err(|_| {
                                ViewerRuntimeLiveServerError::Init(
                                    "periodic service binding changed; re-read required".into(),
                                )
                            })?
                    {
                        return Ok(());
                    }
                    // Only originals still present may be reconciled. New
                    // admissions and changed payloads remain pending intact.
                    prepared.intent_results.retain(|result| {
                        completion.originals.iter().any(|original| {
                            original.0 == result.correlation
                                && server.pending_world_service_gameplay.iter().any(|current| {
                                    current.0 == original.0 && current.1 == original.1
                                })
                        })
                    });
                    if server
                        .latest_player_gameplay_feedback
                        .as_ref()
                        .is_some_and(|feedback| feedback.action == "world_service_read")
                    {
                        server.latest_player_gameplay_feedback = None;
                    }
                    let dispatch = server.apply_chain_linked_runtime_update(prepared, session)?;
                    for response in dispatch.responses {
                        send_response(writer, &response)?;
                    }
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    self.result = None;
                    let mut server = lock_shared_server(shared)?;
                    if let (Some(current), Some(owned)) =
                        (&server.periodic_service_executor, &self.executor)
                        && Arc::ptr_eq(current, owned)
                    {
                        server.periodic_service_executor = None;
                    }
                    self.executor = None;
                    self.defer(server.config.chain_poll_interval);
                    server.set_latest_player_gameplay_feedback(SelfFeedback::failure());
                    return Err(ViewerRuntimeLiveServerError::Init(
                        "periodic service worker unavailable; original intents retained".into(),
                    ));
                }
            }
        }
        if self
            .retry_at
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return Ok(());
        }
        let mut server = lock_shared_server(shared)?;
        if server.authoritative_recovery_write_fence.is_some() || !session.initial_snapshot_sent {
            return Ok(());
        }
        let Some(config) = server.config.world_service.clone() else {
            return Ok(());
        };
        let Some(baseline) = server.verified_world_view.clone() else {
            return Ok(());
        };
        if !session.should_poll_chain(server.config.chain_poll_interval) {
            return Ok(());
        }
        let executor = match &server.periodic_service_executor {
            Some(executor) => executor.clone(),
            None => {
                let executor = Executor::new()?;
                server.periodic_service_executor = Some(executor.clone());
                executor
            }
        };
        let Some(permit) = Permit::acquire(&executor.outstanding) else {
            self.defer(server.config.chain_poll_interval);
            return Ok(());
        };
        let (destination, result) = mpsc::sync_channel(1);
        let job = Job {
            token: Token {
                config,
                baseline,
                player: session.current_player_id.clone(),
                streams: session.subscribed.clone(),
                event_filters: session.event_filters.clone(),
                protocol: session.negotiated_protocol.clone(),
            },
            pending: server
                .pending_world_service_gameplay
                .iter()
                .take(32)
                .cloned()
                .collect(),
            query_state: server.world_service_query_state.clone(),
            destination,
            permit,
        };
        match executor.sender.try_send(job) {
            Ok(()) => {
                self.result = Some(result);
                self.executor = Some(executor);
            }
            Err(TrySendError::Full(_)) => self.defer(server.config.chain_poll_interval),
            Err(TrySendError::Disconnected(_)) => {
                server.periodic_service_executor = None;
                self.defer(server.config.chain_poll_interval);
                return Err(ViewerRuntimeLiveServerError::Init(
                    "periodic service worker unavailable; original intents retained".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_mailboxes_hold_admission_until_consumed_or_disconnected() {
        let outstanding = Arc::new(AtomicUsize::new(0));
        let first = Permit::acquire(&outstanding).unwrap();
        let second = Permit::acquire(&outstanding).unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .try_send(first)
            .unwrap_or_else(|_| panic!("mailbox accepts its sole result"));
        assert!(
            Permit::acquire(&outstanding).is_none(),
            "completed but unread results consume capacity"
        );
        drop(receiver);
        let recovered =
            Permit::acquire(&outstanding).expect("disconnect releases mailbox capacity");
        assert!(Permit::acquire(&outstanding).is_none());
        drop(second);
        drop(recovered);
        assert_eq!(outstanding.load(Ordering::Acquire), 0);
    }

    #[test]
    fn configuration_fence_rejects_trust_scope_world_and_private_identity_drift() {
        let config = WorldServiceClientConfig {
            endpoint: "http://127.0.0.1:1".into(),
            trusted_service_public_key: "service".into(),
            expected_world: oasis7_client_api::world_service::WorldIdentity {
                world_id: "world".into(),
                genesis_digest: "genesis".into(),
            },
            scope_id: "public".into(),
            read_private_key_hex: "private".into(),
            timeout: Duration::from_secs(1),
            max_response_bytes: 1024,
        };
        assert!(same_config(&config, &config.clone()));
        for index in 0..5 {
            let mut changed = config.clone();
            match index {
                0 => changed.endpoint.push('2'),
                1 => changed.trusted_service_public_key.push('2'),
                2 => changed.expected_world.genesis_digest.push('2'),
                3 => changed.scope_id.push('2'),
                _ => changed.read_private_key_hex.push('2'),
            }
            assert!(!same_config(&config, &changed));
        }
    }
}

struct SelfFeedback;
impl SelfFeedback {
    fn failure() -> PlayerGameplayRecentFeedback {
        ViewerRuntimeLiveServer::make_player_gameplay_feedback(
            "world_service_read",
            "blocked",
            "periodic service read deferred; original intents retained",
            None,
            None,
            None,
            Some("wait for authenticated read/Lookup recovery".into()),
            0,
            0,
        )
    }
}

#[cfg(any(test, feature = "test_tier_required"))]
#[path = "periodic_service_io_test_drive.rs"]
mod test_drive;
