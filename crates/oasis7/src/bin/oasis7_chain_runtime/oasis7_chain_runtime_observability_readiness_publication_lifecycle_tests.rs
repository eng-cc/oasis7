use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Barrier};
use std::thread;

const STATE_FILE: &str = "sequencer-publication-lag-state.json";
const STATE_TEMP_FILE: &str = "sequencer-publication-lag-state.json.tmp";

#[derive(Clone, Copy, Debug)]
enum StatusMethod {
    Get,
    Head,
}

fn lifecycle_snapshot(
    local_height: u64,
    local_committed_at_ms: i64,
    peer_height: u64,
    peer_committed_at_ms: i64,
    peer_observed_at_ms: i64,
) -> NodeSnapshot {
    let mut snapshot = publication_test_snapshot(NodeRole::Sequencer, 0);
    snapshot.consensus.latest_height = local_height;
    snapshot.consensus.committed_height = local_height;
    snapshot.consensus.network_committed_height = local_height;
    snapshot.consensus.replication_persisted_height = local_height;
    snapshot.consensus.last_execution_height = local_height;
    snapshot.consensus.last_committed_at_ms = Some(local_committed_at_ms);
    snapshot.consensus.last_block_hash = Some(format!("block-h{local_height}"));
    snapshot.consensus.last_execution_block_hash = Some(format!("execution-h{local_height}"));
    snapshot.consensus.last_execution_state_root = Some(format!("state-h{local_height}"));
    snapshot.consensus.peer_heads[0].height = peer_height;
    snapshot.consensus.peer_heads[0].block_hash = format!("block-h{peer_height}");
    snapshot.consensus.peer_heads[0].committed_at_ms = peer_committed_at_ms;
    snapshot.consensus.peer_heads[0].observed_at_ms = peer_observed_at_ms;
    snapshot.consensus.peer_heads[0].execution_block_hash =
        Some(format!("execution-h{peer_height}"));
    snapshot.consensus.peer_heads[0].execution_state_root = Some(format!("state-h{peer_height}"));
    snapshot
}

fn write_record(records_dir: &std::path::Path, height: u64, timestamp_ms: i64) {
    publication_test_write_execution_record(
        records_dir,
        &publication_test_execution_record_at(
            height,
            format!("block-h{height}").as_str(),
            format!("block-h{}", height - 1).as_str(),
            format!("execution-h{height}").as_str(),
            format!("state-h{height}").as_str(),
            timestamp_ms,
        ),
    );
}

fn prepare_world(
    label: &str,
    chronology: &[(u64, i64)],
) -> (PathBuf, PathBuf, LoadedNetworkTierManifest) {
    let root = publication_test_temp_dir(label);
    let records_dir = root.join("records");
    fs::create_dir_all(root.join("execution-world")).expect("create execution world dir");
    fs::create_dir_all(&records_dir).expect("create execution records dir");
    for &(height, timestamp_ms) in chronology {
        write_record(&records_dir, height, timestamp_ms);
    }
    (
        root,
        records_dir,
        publication_test_manifest("public_testnet", 2),
    )
}

fn install_lifecycle_state(root: &std::path::Path, kind: &str, height: u64, timestamp_ms: i64) {
    let binding = serde_json::json!({
        "height": height,
        "node_block_hash": format!("block-h{height}"),
        "execution_block_hash": format!("execution-h{height}"),
        "execution_state_root": format!("state-h{height}"),
        "timestamp_ms": timestamp_ms,
    });
    let state = match kind {
        "episode" => serde_json::json!({
            "schema_version": 1,
            "world_id": "world-public-testnet",
            "episode": binding,
            "catch_up": null,
        }),
        "catch_up" => serde_json::json!({
            "schema_version": 1,
            "world_id": "world-public-testnet",
            "episode": null,
            "catch_up": binding,
        }),
        other => panic!("unsupported lifecycle state kind: {other}"),
    };
    fs::write(
        root.join("execution-world").join(STATE_FILE),
        serde_json::to_vec_pretty(&state).expect("serialize lifecycle state"),
    )
    .expect("install lifecycle-owned publication state");
}

fn construct_status(
    method: StatusMethod,
    snapshot: NodeSnapshot,
    manifest: &LoadedNetworkTierManifest,
    root: &std::path::Path,
    records_dir: &std::path::Path,
) -> super::super::super::status_payload::ChainStatusResponse {
    // GET and HEAD share this exact constructor after status-server method dispatch. The method is
    // kept explicit here so both public endpoint paths remain represented in the regression.
    assert!(matches!(method, StatusMethod::Get | StatusMethod::Head));
    publication_test_full_payload_from_records(snapshot, manifest, root, records_dir)
}

fn tree_snapshot(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(base: &std::path::Path, path: &std::path::Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| panic!("collect {}: {error}", path.display()));
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let entry_path = entry.path();
            if entry
                .file_type()
                .unwrap_or_else(|error| panic!("type {}: {error}", entry_path.display()))
                .is_dir()
            {
                visit(base, &entry_path, out);
            } else {
                out.push((
                    entry_path
                        .strip_prefix(base)
                        .expect("tree entry below root")
                        .to_path_buf(),
                    fs::read(&entry_path)
                        .unwrap_or_else(|error| panic!("read {}: {error}", entry_path.display())),
                ));
            }
        }
    }

    let mut snapshot = Vec::new();
    visit(root, root, &mut snapshot);
    snapshot
}

fn changed_tree_paths(before: &[(PathBuf, Vec<u8>)], after: &[(PathBuf, Vec<u8>)]) -> Vec<PathBuf> {
    let before = before.iter().cloned().collect::<BTreeMap<_, _>>();
    let after = after.iter().cloned().collect::<BTreeMap<_, _>>();
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}

fn outcome_signature(
    payload: &super::super::super::status_payload::ChainStatusResponse,
) -> (String, bool, String, bool, Vec<(String, String)>) {
    (
        payload.observability.status.clone(),
        payload.observability.ready,
        payload.readiness.status.clone(),
        payload.readiness.ready,
        payload
            .observability
            .alerts
            .iter()
            .map(|alert| (alert.severity.clone(), alert.code.clone()))
            .collect(),
    )
}

fn outcome_diagnostic(
    payload: &super::super::super::status_payload::ChainStatusResponse,
) -> String {
    format!(
        "observability={{status:{},ready:{},alerts:{:?}}} readiness={{status:{},ready:{},failed_gates:{:?}}}",
        payload.observability.status,
        payload.observability.ready,
        payload
            .observability
            .alerts
            .iter()
            .map(|alert| format!("{}:{}:{}", alert.severity, alert.code, alert.summary))
            .collect::<Vec<_>>(),
        payload.readiness.status,
        payload.readiness.ready,
        payload.readiness.failed_gates,
    )
}

fn is_warning_ready(payload: &super::super::super::status_payload::ChainStatusResponse) -> bool {
    payload.observability.status == "warn"
        && payload.observability.ready
        && payload.readiness.status == "ready"
        && payload.readiness.ready
        && payload.readiness.failed_gates.is_empty()
        && payload.observability.alerts.iter().any(|alert| {
            alert.severity == "warn" && alert.code == "sequencer_head_publication_pending"
        })
        && !payload
            .observability
            .alerts
            .iter()
            .any(|alert| alert.code == "local_chain_ahead_of_network_head")
}

fn is_critical_not_ready(
    payload: &super::super::super::status_payload::ChainStatusResponse,
) -> bool {
    payload.observability.status == "critical"
        && !payload.observability.ready
        && payload.readiness.status == "not_ready"
        && !payload.readiness.ready
        && payload
            .observability
            .alerts
            .iter()
            .any(|alert| alert.severity == "critical")
}

fn remove_stake_authority(snapshot: &mut NodeSnapshot) {
    snapshot.consensus.required_stake = 0;
    snapshot.consensus.total_stake = 0;
    snapshot.consensus.validator_stakes.clear();
    snapshot.consensus.validator_set_hash.clear();
    snapshot.consensus.validator_stake_root.clear();
    snapshot.consensus.validator_stake_proofs.clear();
}

fn non_unix_replace_body() -> &'static str {
    let source = include_str!("publication_lifecycle.rs");
    source
        .split_once("#[cfg(not(unix))]\nfn replace_file")
        .expect("non-Unix publication replacement implementation exists")
        .1
        .split_once("#[cfg(unix)]\nfn sync_parent_dir")
        .expect("non-Unix publication replacement implementation is bounded")
        .0
}

fn destination_after_injected_replace_failure(replace_body: &str) -> Option<&'static [u8]> {
    const LAST_GOOD: &[u8] = b"last-good-durable-publication-state";
    let remove_at = replace_body.find("remove_file(target)");
    let replace_at = replace_body
        .find("rename(temp, target)")
        .expect("replacement operation remains explicit");
    if remove_at.is_some_and(|position| position < replace_at) {
        None
    } else {
        Some(LAST_GOOD)
    }
}

#[test]
fn publication_lifecycle_rejects_zero_or_missing_stake_authority_before_persisting_state() {
    let manifest = publication_test_manifest("public_testnet", 2);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64;
    let mut failures = Vec::new();

    for authority in ["zero-required-stake", "missing-stake-authority"] {
        for (phase, local_height, peer_height) in [
            ("equal-head-catch-up", 1_200, 1_200),
            ("one-block-lag-episode", 1_201, 1_200),
        ] {
            let label = format!("{authority}-{phase}");
            let (root, records_dir, _) = prepare_world(
                label.as_str(),
                &[
                    (1_199, now_ms - 3_000),
                    (1_200, now_ms - 2_000),
                    (1_201, now_ms - 1_000),
                ],
            );
            let mut snapshot = lifecycle_snapshot(
                local_height,
                if local_height == 1_200 {
                    now_ms - 2_000
                } else {
                    now_ms - 1_000
                },
                peer_height,
                now_ms - 2_000,
                now_ms - 100,
            );
            snapshot.consensus.required_stake = 0;
            if authority == "missing-stake-authority" {
                remove_stake_authority(&mut snapshot);
            }

            let network_head = super::super::super::status_payload::build_network_head_status(
                &snapshot,
                now_ms,
                Some(&manifest),
            );
            assert_eq!(network_head.quorum_mode, "count", "{label}");
            assert_eq!(network_head.source, "peer_quorum", "{label}");
            assert_eq!(network_head.decision, "ready", "{label}");
            assert!(
                network_head.fresh_peer_count >= network_head.required_peer_count,
                "{label}: count quorum fixture is not ready"
            );
            assert!(
                network_head.stake_quorum_met,
                "{label}: count-mode fixture must expose the vacuous stake-quorum signal"
            );

            let execution_world_dir = root.join("execution-world");
            if phase == "one-block-lag-episode" {
                install_lifecycle_state(&root, "catch_up", 1_200, now_ms - 2_000);
            }
            let state_path = execution_world_dir.join(STATE_FILE);
            let state_before = fs::read(&state_path).ok();
            let result = super::super::super::publication_lifecycle::reconcile(
                &snapshot,
                Some(&manifest),
                execution_world_dir.as_path(),
                records_dir.as_path(),
                now_ms,
            );
            let state_after = fs::read(&state_path).ok();
            if state_after != state_before {
                failures.push(format!(
                    "{label}: lifecycle mutated unauthoritative state after result={:?}: before={} after={}",
                    result
                        .as_ref()
                        .err()
                        .map(|error| (error.reason, error.detail)),
                    state_before
                        .as_deref()
                        .map(String::from_utf8_lossy)
                        .unwrap_or_else(|| "<missing>".into()),
                    state_after
                        .as_deref()
                        .map(String::from_utf8_lossy)
                        .unwrap_or_else(|| "<missing>".into()),
                ));
            }
            fs::remove_dir_all(root).expect("remove stake-authority lifecycle fixture");
        }
    }

    assert!(
        failures.is_empty(),
        "publication lifecycle accepted count quorum without authoritative stake:\n{}",
        failures.join("\n")
    );
}

#[test]
fn publication_state_replacement_never_deletes_last_good_destination_before_atomic_success() {
    const LAST_GOOD: &[u8] = b"last-good-durable-publication-state";
    let root = publication_test_temp_dir("atomic-replace-contract");
    fs::create_dir_all(&root).expect("create replacement contract fixture");
    let destination = root.join("destination.json");
    let replacement = root.join("replacement.tmp");
    fs::write(&destination, LAST_GOOD).expect("write last good destination");
    fs::write(&replacement, b"next-durable-publication-state").expect("write replacement");

    fs::rename(&replacement, &destination).expect("platform atomic replacement success control");
    assert_eq!(
        fs::read(&destination).expect("read successful replacement"),
        b"next-durable-publication-state"
    );

    let destination_after_failure =
        destination_after_injected_replace_failure(non_unix_replace_body());
    assert_eq!(
        destination_after_failure,
        Some(LAST_GOOD),
        "non-Unix replacement deletes the last good destination before the injected atomic replace failure"
    );
    fs::remove_dir_all(root).expect("remove replacement contract fixture");
}

#[test]
fn publication_status_get_and_head_construction_never_mutate_durable_state() {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64;
    let mut failures = Vec::new();
    for method in [StatusMethod::Get, StatusMethod::Head] {
        let (root, records_dir, manifest) = prepare_world(
            format!("status-read-only-{method:?}").as_str(),
            &[(800, now_ms - 2_000), (801, now_ms - 1_000)],
        );
        install_lifecycle_state(&root, "catch_up", 800, now_ms - 2_000);
        fs::create_dir(root.join("execution-world").join(STATE_TEMP_FILE))
            .expect("reserve lifecycle owner temp path");
        let before = tree_snapshot(&root);
        let payload = construct_status(
            method,
            lifecycle_snapshot(801, now_ms - 1_000, 800, now_ms - 2_000, now_ms - 100),
            &manifest,
            &root,
            &records_dir,
        );
        let after = tree_snapshot(&root);

        if !is_warning_ready(&payload) {
            failures.push(format!(
                "{method:?} required a status-side write: {}",
                outcome_diagnostic(&payload)
            ));
        }
        let changed = changed_tree_paths(&before, &after);
        if !changed.is_empty() {
            failures.push(format!("{method:?} mutated paths: {changed:?}"));
        }
        fs::remove_dir_all(root).expect("remove read-only status fixture");
    }
    assert!(
        failures.is_empty(),
        "GET/HEAD status purity violations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn publication_readiness_transitions_are_invariant_to_polling_schedule_and_get_head_mix() {
    #[derive(Clone, Copy)]
    enum PollStep {
        OldLag(StatusMethod),
        CatchUp(StatusMethod),
        FreshLag(StatusMethod),
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64;
    let chronology = [
        (900, now_ms - 31_100),
        (901, now_ms - 31_000),
        (902, now_ms - 1_000),
        (903, now_ms),
    ];
    let schedules = [
        (
            "no-intermediate-polling",
            vec![PollStep::FreshLag(StatusMethod::Get)],
        ),
        (
            "sparse-head-polling",
            vec![
                PollStep::CatchUp(StatusMethod::Head),
                PollStep::FreshLag(StatusMethod::Get),
            ],
        ),
        (
            "frequent-mixed-polling",
            vec![
                PollStep::OldLag(StatusMethod::Get),
                PollStep::OldLag(StatusMethod::Head),
                PollStep::CatchUp(StatusMethod::Get),
                PollStep::CatchUp(StatusMethod::Head),
                PollStep::FreshLag(StatusMethod::Head),
                PollStep::FreshLag(StatusMethod::Get),
            ],
        ),
    ];
    let mut final_outcomes = Vec::new();
    let mut polling_mutations = Vec::new();
    let mut schedule_failures = Vec::new();

    for (label, schedule) in schedules {
        let (root, records_dir, manifest) = prepare_world(label, &chronology);
        // This fixture write models the serialized lifecycle owner observing equality at H=902.
        // Status reads must consume this immutable snapshot, never create or renew it.
        install_lifecycle_state(&root, "catch_up", 902, now_ms - 1_000);
        let lifecycle_state = tree_snapshot(&root);
        let mut final_payload = None;
        for step in schedule {
            let (method, snapshot) = match step {
                PollStep::OldLag(method) => (
                    method,
                    lifecycle_snapshot(902, now_ms - 1_000, 901, now_ms - 31_000, now_ms - 500),
                ),
                PollStep::CatchUp(method) => (
                    method,
                    lifecycle_snapshot(902, now_ms - 1_000, 902, now_ms - 1_000, now_ms - 400),
                ),
                PollStep::FreshLag(method) => (
                    method,
                    lifecycle_snapshot(903, now_ms, 902, now_ms - 1_000, now_ms + 1),
                ),
            };
            let payload = construct_status(method, snapshot, &manifest, &root, &records_dir);
            if matches!(step, PollStep::FreshLag(_)) {
                final_payload = Some(payload);
            }
        }
        let final_payload = final_payload.expect("schedule includes final lag read");
        if !is_warning_ready(&final_payload) {
            schedule_failures.push(format!(
                "{label} final transition: {}",
                outcome_diagnostic(&final_payload)
            ));
        }
        let changed = changed_tree_paths(&lifecycle_state, &tree_snapshot(&root));
        if !changed.is_empty() {
            polling_mutations.push(format!("{label}: {changed:?}"));
        }
        final_outcomes.push((label, outcome_signature(&final_payload)));
        fs::remove_dir_all(root).expect("remove polling schedule fixture");
    }

    for pair in final_outcomes.windows(2) {
        if pair[0].1 != pair[1].1 {
            schedule_failures.push(format!(
                "{} != {}: {:?} != {:?}",
                pair[0].0, pair[1].0, pair[0].1, pair[1].1
            ));
        }
    }
    schedule_failures.extend(
        polling_mutations
            .into_iter()
            .map(|mutation| format!("filesystem mutation: {mutation}")),
    );
    assert!(
        schedule_failures.is_empty(),
        "polling invariance violations:\n{}",
        schedule_failures.join("\n")
    );
}

#[test]
fn concurrent_publication_status_reads_are_side_effect_free_and_race_free() {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64;
    let (root, records_dir, manifest) = prepare_world(
        "concurrent-status-reads",
        &[(1_000, now_ms - 2_000), (1_001, now_ms - 1_000)],
    );
    install_lifecycle_state(&root, "catch_up", 1_000, now_ms - 2_000);
    let before = tree_snapshot(&root);
    let root = Arc::new(root);
    let records_dir = Arc::new(records_dir);
    let manifest = Arc::new(manifest);
    let barrier = Arc::new(Barrier::new(16));
    let handles = (0..16)
        .map(|index| {
            let root = Arc::clone(&root);
            let records_dir = Arc::clone(&records_dir);
            let manifest = Arc::clone(&manifest);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                construct_status(
                    if index % 2 == 0 {
                        StatusMethod::Get
                    } else {
                        StatusMethod::Head
                    },
                    lifecycle_snapshot(1_001, now_ms - 1_000, 1_000, now_ms - 2_000, now_ms - 100),
                    &manifest,
                    &root,
                    &records_dir,
                )
            })
        })
        .collect::<Vec<_>>();
    let payloads = handles
        .into_iter()
        .map(|handle| handle.join().expect("concurrent status reader panicked"))
        .collect::<Vec<_>>();

    let expected = outcome_signature(&payloads[0]);
    assert!(
        payloads.iter().all(is_warning_ready),
        "concurrent status reads raced on persistence: {:?}",
        payloads
            .iter()
            .map(outcome_diagnostic)
            .collect::<BTreeSet<_>>()
    );
    assert!(
        payloads
            .iter()
            .all(|payload| outcome_signature(payload) == expected),
        "concurrent GET/HEAD reads produced race-dependent readiness"
    );
    let changed = changed_tree_paths(&before, &tree_snapshot(&root));
    assert!(
        changed.is_empty(),
        "concurrent status reads changed filesystem paths: {changed:?}"
    );
    assert!(
        !root.join("execution-world").join(STATE_TEMP_FILE).exists(),
        "concurrent status reads left the shared temporary path behind"
    );
    fs::remove_dir_all(root.as_ref()).expect("remove concurrent status fixture");
}

#[test]
fn lifecycle_owned_episode_survives_restart_replay_and_reorgs_fail_closed_without_read_writes() {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64;
    let chronology = [
        (1_100, now_ms - 10_000),
        (1_101, now_ms - 9_000),
        (1_102, now_ms - 1_000),
    ];
    let (root, records_dir, manifest) = prepare_world("lifecycle-restart-replay", &chronology);
    install_lifecycle_state(&root, "episode", 1_101, now_ms - 9_000);
    // A lifecycle owner may use its own atomic protocol. A status reader must never touch this
    // reserved path; making it a directory turns any attempted status-side write into a stable RED.
    fs::create_dir(root.join("execution-world").join(STATE_TEMP_FILE))
        .expect("reserve lifecycle owner temp path");
    let durable_before = tree_snapshot(&root);
    let valid = lifecycle_snapshot(1_102, now_ms - 1_000, 1_101, now_ms - 9_000, now_ms - 100);

    let first_process = construct_status(
        StatusMethod::Get,
        valid.clone(),
        &manifest,
        &root,
        &records_dir,
    );
    let restarted_process = construct_status(
        StatusMethod::Head,
        valid.clone(),
        &manifest,
        &root,
        &records_dir,
    );
    let replayed_process = construct_status(
        StatusMethod::Get,
        valid.clone(),
        &manifest,
        &root,
        &records_dir,
    );
    assert!(
        is_warning_ready(&first_process),
        "initial lifecycle snapshot required a status-side write: {}",
        outcome_diagnostic(&first_process)
    );
    assert_eq!(
        outcome_signature(&first_process),
        outcome_signature(&restarted_process),
        "restart did not restore the serialized lifecycle episode"
    );
    assert_eq!(
        outcome_signature(&first_process),
        outcome_signature(&replayed_process),
        "replay changed the serialized lifecycle episode"
    );

    let mut rollback = valid.clone();
    rollback.consensus.last_block_hash = Some("rollback-conflict-h1102".to_string());
    let rollback_payload =
        construct_status(StatusMethod::Head, rollback, &manifest, &root, &records_dir);
    assert!(
        is_critical_not_ready(&rollback_payload),
        "rollback/reorg binding mismatch must fail closed"
    );

    let mut two_block_lead = valid.clone();
    two_block_lead.consensus.peer_heads[0].height = 1_100;
    two_block_lead.consensus.peer_heads[0].block_hash = "block-h1100".to_string();
    two_block_lead.consensus.peer_heads[0].execution_block_hash =
        Some("execution-h1100".to_string());
    two_block_lead.consensus.peer_heads[0].execution_state_root = Some("state-h1100".to_string());
    assert!(
        is_critical_not_ready(&construct_status(
            StatusMethod::Get,
            two_block_lead,
            &manifest,
            &root,
            &records_dir,
        )),
        "publication grace broadened beyond exact one-block lag"
    );

    let mut incomplete_local_proof = valid.clone();
    incomplete_local_proof.consensus.last_execution_state_root = None;
    assert!(
        is_critical_not_ready(&construct_status(
            StatusMethod::Head,
            incomplete_local_proof,
            &manifest,
            &root,
            &records_dir,
        )),
        "publication grace accepted incomplete local execution proof"
    );

    let mut conflicting_quorum = valid;
    let mut conflict = conflicting_quorum.consensus.peer_heads[0].clone();
    conflict.node_id = "validator-conflict".to_string();
    conflict.validator_id = Some("validator-conflict".to_string());
    conflict.block_hash = "competing-parent-h1101".to_string();
    conflicting_quorum.consensus.peer_heads.push(conflict);
    conflicting_quorum
        .consensus
        .validator_stakes
        .insert("validator-conflict".to_string(), 67);
    assert!(
        is_critical_not_ready(&construct_status(
            StatusMethod::Get,
            conflicting_quorum,
            &manifest,
            &root,
            &records_dir,
        )),
        "publication grace accepted a conflicting quorum"
    );

    let changed = changed_tree_paths(&durable_before, &tree_snapshot(&root));
    assert!(
        changed.is_empty(),
        "restart/replay/rollback status reads changed lifecycle paths: {changed:?}"
    );
    fs::remove_dir_all(root).expect("remove lifecycle restart fixture");
}
