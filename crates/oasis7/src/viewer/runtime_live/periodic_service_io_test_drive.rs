//! Required-tier test drive of the real periodic executor and its actual HTTP completion.
//! Session and pending-map changes below are internal test seams, not public admissions.
use super::*;

fn witness(server: &ViewerRuntimeLiveServer) -> serde_json::Value {
    serde_json::json!({
        "world": server.world.snapshot(),
        "cognition": server.world.cognition(),
        "pending": server.pending_world_service_gameplay,
        "feedback": server.latest_player_gameplay_feedback,
        "version": server.verified_world_view.as_ref().map(VerifiedWorldView::version),
        "cursor": server.verified_world_view.as_ref().map(VerifiedWorldView::continuation),
    })
}

impl ViewerRuntimeLiveServer {
    /// The callback holds/releases a parsed, genuinely signed fixture ReadView.
    /// No Completion or canonical result is constructed by this test drive.
    pub fn test_drive_periodic_late_completion(
        shared: &Arc<Mutex<Self>>,
        baseline: VerifiedWorldView,
        original: Pending,
        replacement: Pending,
        change: &str,
        mut gate: impl FnMut(&str) -> Option<VerifiedWorldView>,
    ) -> serde_json::Value {
        let original_config = shared.lock().unwrap().config.world_service.clone();
        let mut session = RuntimeLiveSession::new_with_playing(false);
        session.initial_snapshot_sent = true;
        session.subscribed.insert(ViewerStream::Snapshot);
        {
            let mut server = shared.lock().unwrap();
            server.verified_world_view = Some(baseline);
            server.pending_world_service_gameplay = original;
        }
        let mut poll = SessionPoll::default();
        let mut emitted = Vec::new();
        poll.poll(shared, &mut session, &mut emitted).unwrap();
        assert!(
            poll.result.is_some(),
            "ordinary production poll must dispatch actual I/O"
        );
        let advanced = gate("held");
        {
            let mut server = shared.lock().unwrap();
            match change {
                "config" => server
                    .config
                    .world_service
                    .as_mut()
                    .unwrap()
                    .endpoint
                    .push_str("/changed"),
                "cursor" => {
                    let advanced = advanced.expect("cursor change needs an actual verified read");
                    assert_ne!(
                        advanced.version(),
                        server.verified_world_view.as_ref().unwrap().version()
                    );
                    server.verified_world_view = Some(advanced);
                }
                "player" => session.current_player_id = Some("late-periodic-other-player".into()),
                "subscription" => {
                    session.subscribed.insert(ViewerStream::Metrics);
                }
                "event_filter" => {
                    session.event_filters = Some(HashSet::from([ViewerEventKind::AgentMoved]))
                }
                "protocol" => {
                    session.negotiated_protocol =
                        crate::viewer::protocol::NegotiatedViewerProtocol {
                            version: 2,
                            capabilities: Vec::new(),
                        }
                }
                "pending" => server.pending_world_service_gameplay = replacement.clone(),
                "matching" => {}
                other => panic!("unknown late completion test seam {other}"),
            }
        }
        let before = witness(&shared.lock().unwrap());
        gate("release");
        // Receive the genuine worker capsule, then transfer it unchanged into
        // production poll's mailbox. This bounds the test without inventing an outcome.
        let completion = poll
            .result
            .take()
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        let prepared = completion
            .outcome
            .as_ref()
            .expect("actual signed View/Changes must validate");
        assert!(prepared.verified_view.is_some());
        assert_eq!(prepared.intent_results.len(), completion.originals.len());
        assert!(
            prepared.intent_results.iter().all(|result| matches!(
                result.outcome,
                oasis7_client_api::world_service::IntentOutcome::Committed { .. }
            )),
            "original Lookup must have a genuine committed canonical result"
        );
        let actual_lookup_count = prepared.intent_results.len();
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(completion)
            .unwrap_or_else(|_| panic!("transfer actual completion"));
        poll.result = Some(receiver);
        poll.poll(shared, &mut session, &mut emitted).unwrap();
        assert!(
            poll.result.is_none(),
            "completion must be consumed without fresh dispatch"
        );
        let after = witness(&shared.lock().unwrap());
        if change == "matching" {
            assert!(
                shared
                    .lock()
                    .unwrap()
                    .pending_world_service_gameplay
                    .is_empty()
            );
            assert_eq!(after["feedback"]["action"], "world_service_intent");
            assert_eq!(after["feedback"]["stage"], "committed");
        } else if change == "pending" {
            assert_eq!(
                shared.lock().unwrap().pending_world_service_gameplay,
                replacement
            );
            assert_ne!(after["feedback"]["action"], "world_service_intent");
        } else {
            assert_eq!(
                after, before,
                "stale completion changed world/cognition/pending/cursor/feedback"
            );
            assert!(
                emitted.is_empty(),
                "stale completion emitted a Viewer response"
            );
        }
        let mut fresh_poll_verified = false;
        if !matches!(change, "pending" | "matching") {
            // Resume from the current verified cursor and session token, after
            // restoring only intentionally invalid endpoint/player selections.
            // Cadence is driven by this internal test seam, not a fabricated result.
            if change == "config" {
                shared.lock().unwrap().config.world_service = original_config;
            }
            if change == "player" {
                session.current_player_id = None;
            }
            let current_commit = shared
                .lock()
                .unwrap()
                .verified_world_view
                .as_ref()
                .unwrap()
                .version()
                .commit
                .clone();
            poll.retry_at = None;
            session.next_chain_poll_at = None;
            poll.poll(shared, &mut session, &mut emitted).unwrap();
            let completion = poll
                .result
                .take()
                .expect("current token must dispatch a fresh real read")
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
            let prepared = completion
                .outcome
                .as_ref()
                .expect("fresh signed read validates");
            assert!(
                prepared
                    .verified_view
                    .as_ref()
                    .unwrap()
                    .version()
                    .commit
                    .satisfies_minimum(&current_commit)
                    .unwrap()
            );
            assert_eq!(prepared.intent_results.len(), 1);
            assert!(matches!(
                prepared.intent_results[0].outcome,
                oasis7_client_api::world_service::IntentOutcome::Committed { .. }
            ));
            let (sender, receiver) = mpsc::sync_channel(1);
            sender
                .send(completion)
                .unwrap_or_else(|_| panic!("transfer fresh actual completion"));
            poll.result = Some(receiver);
            poll.poll(shared, &mut session, &mut emitted).unwrap();
            assert!(
                shared
                    .lock()
                    .unwrap()
                    .pending_world_service_gameplay
                    .is_empty(),
                "fresh current-token completion must retire the exact original operation"
            );
            fresh_poll_verified = true;
        }
        drop(poll);
        let executor = shared
            .lock()
            .unwrap()
            .periodic_service_executor
            .take()
            .unwrap();
        let executor = Arc::try_unwrap(executor)
            .unwrap_or_else(|_| panic!("test owns sole remaining executor"));
        let Executor {
            sender,
            outstanding,
            worker,
        } = executor;
        drop(sender);
        worker.into_inner().unwrap().take().unwrap().join().unwrap();
        assert_eq!(
            outstanding.load(Ordering::Acquire),
            0,
            "all actual completion permits released"
        );
        serde_json::json!({"change":change,"actual_lookup_count":actual_lookup_count,
            "actual_signed_view_changes_validated":true,"before":before,"after":after,
            "emitted_bytes":emitted.len(),"fresh_current_token_poll_verified":fresh_poll_verified,"internal_test_drive":true})
    }
}
