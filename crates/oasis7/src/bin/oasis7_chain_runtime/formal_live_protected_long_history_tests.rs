//! Long-running actual Node/World/P/R administrative publications, beyond
//! native-only C10. Wasmtime execution is covered by gameplay crash scenarios.
use super::*;
use oasis7::world_service::AgentSignerDelegationChangeV1;
use oasis7_distfs::controlled_authority::formal::endpoint::FormalCandidateMaterialV1;

#[test]
#[ignore = "long-running actual protected endpoints; requires OASIS7_FORMAL_ENDPOINT_TEST_BIN"]
fn protected_endpoint_children_world_long_history_prunes_reopens_and_retains_original() {
    thread::Builder::new()
        .name("formal-long-world".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| run_protected_runtime_case(false, None, true))
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn exercise_long_history(
    mut runtime: FormalLiveRuntime,
    config: &Path,
    writer_directory: &Path,
    public_config: WorldServiceClientConfig,
    owner_config: WorldServiceClientConfig,
    identity: &WorldIdentity,
    owner_key: &SigningKey,
    original: &SubmitIntentRequest<WorldServicePayloadV1>,
    original_commit: &oasis7_client_api::world_service::CommitRef,
) -> Result<(), String> {
    let owner = RemoteWorldServiceClient::new(owner_config.clone()).map_err(|e| e.to_string())?;
    let original_tick = runtime
        .published
        .current()?
        .world
        .read_tick_consensus_record(1)
        .map_err(|e| format!("original Tick fixture: {e:?}"))?
        .ok_or("original Tick certificate missing")?;
    for ordinal in 0..70_u64 {
        let current = runtime.published.current()?;
        let binding = current
            .world
            .current_cognition_runtime_binding()
            .map_err(|e| format!("long history runtime binding: {e:?}"))?;
        let agent_identity = current
            .world
            .capability_revocation_state()
            .agent_identities
            .get("agent")
            .ok_or("long history canonical Agent identity missing")?;
        let previous = current
            .world
            .capability_revocation_state()
            .agent_signer_delegations
            .get("agent");
        let change = AgentSignerDelegationChangeV1 {
            world_id: binding.world_id,
            branch_id: binding.branch_id,
            agent_id: "agent".into(),
            owner_binding: agent_identity.owner_binding.clone(),
            agent_identity_generation: agent_identity.generation,
            generation: previous.map_or(Ok(1), |value| {
                value
                    .generation
                    .checked_add(1)
                    .ok_or("long history generation overflow")
            })?,
            delegate_public_key: hex::encode(fixtures::key(72).verifying_key().to_bytes()),
            revoked: ordinal % 2 == 1,
            nonce: previous.map_or(Ok(1), |value| {
                value
                    .nonce
                    .checked_add(1)
                    .ok_or("long history nonce overflow")
            })?,
        };
        let payload = WorldServicePayloadV1::Delegation(
            sign_read_request(
                "delegation",
                change.clone(),
                &hex::encode(owner_key.to_bytes()),
            )
            .map_err(|e| e.to_string())?,
        );
        let request = SubmitIntentRequest {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            correlation: correlation::derive_correlation(identity.clone(), &payload)
                .map_err(|e| e.to_string())?,
            deadline_unix_ms: None,
            signed_payload: payload,
        };
        let admission = owner.submit(request.clone()).map_err(|e| e.to_string())?;
        assert_volatile_admission(admission, &request.correlation)?;
        wait_for_new_long_history_result(&runtime, &request)
            .map_err(|error| format!("long history change {ordinal}: {error}"))?;
        if runtime
            .published
            .current()?
            .world
            .capability_revocation_state()
            .agent_signer_delegations
            .get("agent")
            != Some(&change)
        {
            return Err(
                "long history accepted delegation did not update the canonical ledger".into(),
            );
        }
        if ordinal % 16 == 15 {
            eprintln!(
                "long history reopening after {} completed changes",
                ordinal + 1
            );
            runtime.stop(Duration::from_secs(10))?;
            let before = runtime.published.current()?;
            drop(runtime);
            runtime = FormalLiveRuntime::open_with_native_history(config).map_err(|error| {
                format!("long history reopen after {} changes: {error}", ordinal + 1)
            })?;
            if runtime.published.current()?.world.snapshot() != before.world.snapshot() {
                return Err("long-running World changed after qualified reopen".into());
            }
            runtime.start()?;
        }
    }
    let before = runtime.published.current()?;
    if before.anchor.head().record().position <= 64
        || before.world.tick_consensus_records().len() > 32
        || before.world.tick_consensus_record_count() <= 64
    {
        return Err("actual long-running World did not retain bounded hot Tick history".into());
    }
    if before
        .world
        .read_tick_consensus_record(1)
        .map_err(|e| format!("cold original Tick: {e:?}"))?
        != Some(original_tick.clone())
        || before
            .world
            .read_tick_consensus_at_tick(original_tick.block.header.tick)
            .map_err(|e| format!("cold original Tick locator: {e:?}"))?
            != Some(original_tick.clone())
    {
        return Err("actual World archive changed the original Tick certificate".into());
    }
    let mut bytes = 0_u64;
    for entry in std::fs::read_dir(writer_directory.join("published")).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let publication: serde_cbor::Value =
            serde_cbor::from_slice(&std::fs::read(entry.path()).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let serde_cbor::Value::Map(fields) = publication else {
            return Err("stored qualified publication is not a map".into());
        };
        let candidate: FormalCandidateMaterialV1 = serde_cbor::value::from_value(
            fields
                .get(&serde_cbor::Value::Text("candidate".into()))
                .ok_or("stored qualified candidate missing")?
                .clone(),
        )
        .map_err(|e| e.to_string())?;
        bytes = bytes
            .checked_add(
                serde_cbor::to_vec(&candidate.material)
                    .map_err(|e| e.to_string())?
                    .len() as u64,
            )
            .ok_or("record byte count overflow")?;
    }
    if bytes <= 16 * 1024 * 1024 {
        return Err(format!(
            "actual World closed-record bytes only {bytes}; C10 threshold unmet"
        ));
    }
    runtime.stop(Duration::from_secs(10))?;
    drop(runtime);
    eprintln!("long history final reopen after 70 completed changes");
    let mut reopened = FormalLiveRuntime::open_with_native_history(config)
        .map_err(|error| format!("long history final reopen: {error}"))?;
    reopened.start()?;
    if reopened
        .published
        .current()?
        .world
        .read_tick_consensus_record(1)
        .map_err(|e| format!("reopened cold original Tick: {e:?}"))?
        != Some(original_tick)
    {
        return Err("reopened archive changed original Tick bytes".into());
    }
    let owner = RemoteWorldServiceClient::new(owner_config).map_err(|e| e.to_string())?;
    let public = RemoteWorldServiceClient::new(public_config).map_err(|e| e.to_string())?;
    let view = read_until_position(&public, identity, before.anchor.head().record().position)?;
    assert_live_descends_from_view(&reopened, &reopened.published.current()?, &view)?;
    if lookup_original_commit(&owner, original)? != *original_commit {
        return Err("pruned World reopen lost original qualified outcome".into());
    }
    let original_key =
        correlation::key_digest(&original.correlation.key).map_err(|e| e.to_string())?;
    let original_result = retained_result_from_hot_or_cold(&before.world, &original_key)?;
    let control = game_task::current_control(&before.world, "agent").map_err(|e| e.to_string())?;
    let next = before.anchor.head().record().position + 1;
    assert_volatile_admission(
        owner.submit(original.clone()).map_err(|e| e.to_string())?,
        &original.correlation,
    )?;
    wait_for_published_position(&reopened, next)?;
    let after = reopened.published.current()?;
    if retained_result_from_hot_or_cold(&after.world, &original_key)? != original_result
        || game_task::current_control(&after.world, "agent").map_err(|e| e.to_string())? != control
        || lookup_original_commit(&owner, original)? != *original_commit
    {
        return Err(
            "old request produced a second effect or changed its original CommitRef".into(),
        );
    }
    eprintln!(
        "actual World completed-owner-changes=70, qualified={}, closed-record-bytes={bytes}, hot-ticks={}, total-ticks={}",
        before.anchor.head().record().position,
        before.world.tick_consensus_records().len(),
        before.world.tick_consensus_record_count()
    );
    reopened.stop(Duration::from_secs(10))
}

/// A background empty Tick may advance position before the submitted task runs.
/// Require this exact request's published outcome, so 70 loop iterations prove
/// 70 completed owner-signed delegation changes rather than merely 70 head changes.
fn wait_for_new_long_history_result(
    runtime: &FormalLiveRuntime,
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
) -> Result<(), String> {
    let key = correlation::key_digest(&request.correlation.key).map_err(|e| e.to_string())?;
    // Volatile admission has no execution SLO. Keep this fixture bounded while
    // allowing a request to wait for an already-running Tick and its own Tick.
    let started = Instant::now();
    let deadline = started + Duration::from_secs(120);
    let mut next_diagnostic = Duration::from_secs(20);
    loop {
        let current = runtime.published.current()?;
        // Each generation/nonce here is new and the next request is not
        // submitted until this one is published. Its result must therefore be
        // in this qualified publication's recent hot results. Probing older
        // cold history for an unexecuted request competes with publication.
        if fresh_long_history_result(&current.world, &key, request)?.is_some() {
            return Ok(());
        }
        let snapshot = runtime
            .runtime
            .lock()
            .map_err(|_| "long history runtime lock poisoned")?
            .snapshot();
        if let Some(error) = snapshot.last_error {
            return Err(format!(
                "long history task failed before publication: {error}"
            ));
        }
        let now = Instant::now();
        if now >= deadline {
            // Publication may have advanced while the Node snapshot was read.
            let current = runtime.published.current()?;
            if fresh_long_history_result(&current.world, &key, request)?.is_some() {
                return Ok(());
            }
            return Err(format!(
                "long history task {key} missing after 120s at position {}; running={} ticks={} slot={} height={} committed={} executed={} pending={:?} proposal={:?} status={:?}",
                current.anchor.head().record().position,
                snapshot.running,
                snapshot.tick_count,
                snapshot.consensus.slot,
                snapshot.consensus.latest_height,
                snapshot.consensus.committed_height,
                snapshot.consensus.last_execution_height,
                snapshot.consensus.pending_consensus_actions,
                snapshot.consensus.pending_proposal,
                snapshot.consensus.last_status,
            ));
        }
        if now.duration_since(started) >= next_diagnostic {
            eprintln!(
                "long history task {key} still awaiting exact publication after {}s (diagnostic threshold {}s); position={} running={} ticks={} slot={} height={} committed={} executed={} pending={:?} proposal={:?} status={:?}",
                now.duration_since(started).as_secs(),
                next_diagnostic.as_secs(),
                current.anchor.head().record().position,
                snapshot.running,
                snapshot.tick_count,
                snapshot.consensus.slot,
                snapshot.consensus.latest_height,
                snapshot.consensus.committed_height,
                snapshot.consensus.last_execution_height,
                snapshot.consensus.pending_consensus_actions,
                snapshot.consensus.pending_proposal,
                snapshot.consensus.last_status,
            );
            next_diagnostic += Duration::from_secs(20);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn fresh_long_history_result(
    world: &oasis7::runtime::World,
    key: &str,
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
) -> Result<Option<oasis7::world_service::CanonicalIntentResultV1>, String> {
    let Some(value) = world
        .capability_revocation_state()
        .world_service_results
        .get(key)
    else {
        return Ok(None);
    };
    correlation::validate_result(key, value)?;
    let result: oasis7::world_service::CanonicalIntentResultV1 =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if result.request != *request || result.rejected.is_some() {
        return Err(format!(
            "long history exact task outcome mismatch or rejection: {:?}",
            result.rejected
        ));
    }
    Ok(Some(result))
}

#[test]
fn fresh_long_history_wait_avoids_cold_absence_and_requires_exact_success() {
    use oasis7::{runtime::World, world_service::CanonicalIntentResultV1};
    let mut staged = World::new();
    staged.initialize_empty_formal_history().unwrap();
    let mut world = World::from_snapshot(staged.snapshot(), staged.journal().clone()).unwrap();
    let identity = WorldIdentity {
        world_id: "formal-fixture-world".into(),
        genesis_digest: "08".repeat(32),
    };
    let payload = WorldServicePayloadV1::Delegation(
        sign_read_request(
            "delegation",
            AgentSignerDelegationChangeV1 {
                world_id: identity.world_id.clone(),
                branch_id: "main".into(),
                agent_id: "agent".into(),
                owner_binding: "a".repeat(64),
                agent_identity_generation: 1,
                generation: 1,
                delegate_public_key: hex::encode(fixtures::key(72).verifying_key().to_bytes()),
                revoked: false,
                nonce: 1,
            },
            &hex::encode(fixtures::key(91).to_bytes()),
        )
        .unwrap(),
    );
    let request = SubmitIntentRequest {
        contract_version: WORLD_SERVICE_CONTRACT_VERSION,
        correlation: correlation::derive_correlation(identity, &payload).unwrap(),
        deadline_unix_ms: None,
        signed_payload: payload,
    };
    let key = correlation::key_digest(&request.correlation.key).unwrap();
    assert!(world.read_retained_world_service_result(&key).is_err());
    assert!(
        fresh_long_history_result(&world, &key, &request)
            .unwrap()
            .is_none()
    );
    let result = CanonicalIntentResultV1 {
        request: request.clone(),
        action_id: 1,
        committed_height: 1,
        receipt: serde_json::json!({}),
        rejected: None,
    };
    world.record_world_service_result(result.clone()).unwrap();
    assert_eq!(
        fresh_long_history_result(&world, &key, &request).unwrap(),
        Some(result.clone())
    );
    let mut different = request.clone();
    different.deadline_unix_ms = Some(1);
    assert!(fresh_long_history_result(&world, &key, &different).is_err());
    let mut rejected = result;
    rejected.rejected = Some("canonical rejection".into());
    let mut rejected_world = World::new();
    rejected_world
        .record_world_service_result(rejected)
        .unwrap();
    assert!(fresh_long_history_result(&rejected_world, &key, &request).is_err());
}
