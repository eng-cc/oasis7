use super::*;
use oasis7::runtime::{Action, CognitionLeaseQuoteV1, CognitionLeaseRequestV1, WorldState};
use oasis7::world_service::*;
use oasis7_node::{NodeConsensusAction, NodeExecutionHook};

fn commit(
    driver: &mut NodeRuntimeExecutionDriver,
    height: u64,
    request: SubmitIntentRequest<WorldServicePayloadV1>,
) {
    let payload = correlation::encode_consensus_intent(&request).unwrap();
    let actions = vec![NodeConsensusAction::from_payload(height, "node-a", payload).unwrap()];
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".into(),
            node_id: "node-a".into(),
            proposer_id: "node-a".into(),
            height,
            slot: height,
            epoch: 0,
            node_block_hash: format!("node-h{height}"),
            action_root: compute_consensus_action_root(&actions).unwrap(),
            committed_actions: actions,
            committed_at_unix_ms: height as i64 * 1000,
        })
        .unwrap();
}

fn request(
    identity: &WorldIdentity,
    payload: WorldServicePayloadV1,
) -> SubmitIntentRequest<WorldServicePayloadV1> {
    SubmitIntentRequest {
        contract_version: 1,
        correlation: derive_correlation(identity.clone(), &payload).unwrap(),
        deadline_unix_ms: None,
        signed_payload: payload,
    }
}

#[test]
fn world_service_driver_admin_commit_pin_restart_and_stale_fence() {
    let dir = temp_dir("world-service-admin");
    let owner = hex::encode([7u8; 32]);
    let public = sign_read_request("owner", (), &owner)
        .unwrap()
        .subject_public_key;
    let delegate = sign_read_request("delegate", (), &hex::encode([8u8; 32]))
        .unwrap()
        .subject_public_key;
    let identity = WorldIdentity {
        world_id: "w1".into(),
        genesis_digest: "fixture-genesis-v1".into(),
    };
    let mut world = RuntimeWorld::new_with_state(WorldState::default());
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-a".into(),
        pos: oasis7::GeoPos::new(0, 0, 0),
    });
    world.step().unwrap();
    world.submit_action(Action::ClaimStarterOc {
        agent_id: "agent-a".into(),
        player_id: "owner-a".into(),
        public_key: Some(public),
    });
    world.step().unwrap();
    assert!(world.state().starter_oc_claims.contains_key("agent-a"));
    world
        .install_capability_agent_identity("agent-a", "owner-a", 1)
        .unwrap();
    world
        .bind_cognition_runtime("w1", "main", 0, None, "pending", 0)
        .unwrap();
    world
        .provision_cognition_for_agent("agent-a", "provision-a", "owner-a", 100)
        .unwrap();
    let base = world.current_cognition_base_binding().unwrap();
    let context_hash = super::super::execution_hash::execution_resource_context_hash("w1");
    world
        .save_to_dir_with_chain_resource_context(
            dir.join("world"),
            oasis7::runtime::ChainResourceDerivationContext {
                world_id: "w1",
                chain_id: "w1",
                genesis_ref: Some(identity.genesis_digest.as_str()),
                created_at_height: world.state().time,
                manifest_height: world.state().time,
                commit_block_hash: None,
                tick: world.state().time,
            },
            context_hash.clone(),
            context_hash,
        )
        .unwrap();
    fs::write(
        dir.join("world/world-service-identity.json"),
        serde_json::to_vec(&identity).unwrap(),
    )
    .unwrap();
    let baseline = super::super::local_bootstrap::derive_local_execution_bootstrap(
        &dir.join("world"),
        &dir.join("records"),
        "w1",
        world.state().time,
        "world-service-local-setup-boundary",
        &oasis7::runtime::ReleaseSecurityPolicy::default(),
    )
    .unwrap();
    let mut driver = NodeRuntimeExecutionDriver::new_with_local_bootstrap(
        dir.join("state.json"),
        dir.join("world"),
        dir.join("records"),
        dir.join("store"),
        &oasis7_proto::storage_profile::StorageProfileConfig::default(),
        baseline,
    )
    .unwrap();
    let change = AgentSignerDelegationChangeV1 {
        world_id: "w1".into(),
        branch_id: "main".into(),
        agent_id: "agent-a".into(),
        owner_binding: "owner-a".into(),
        agent_identity_generation: 1,
        generation: 1,
        delegate_public_key: delegate,
        revoked: false,
        nonce: 1,
    };
    let registration = request(
        &identity,
        WorldServicePayloadV1::Delegation(sign_read_request("delegation", change, &owner).unwrap()),
    );
    // Actual node admission rejects the previous unregistered outer frame,
    // accepts the registered inner frame, and preserves controller auth.
    use oasis7::consensus_action_payload::*;
    let node = oasis7_node::NodeRuntime::new(
        oasis7_node::NodeConfig::new("node-a", "w1", NodeRole::Sequencer)
            .expect("valid node admission fixture"),
    );
    let old_frame = encode_consensus_action_payload(&ConsensusActionPayloadEnvelope {
        version: 1,
        auth: None,
        body: ConsensusActionPayloadBody::WorldServiceIntent {
            request: registration.clone(),
        },
    })
    .unwrap();
    assert!(
        format!(
            "{:?}",
            node.submit_consensus_action_payload(100, old_frame)
                .unwrap_err()
        )
        .contains("WorldServiceIntent")
    );
    node.submit_consensus_action_payload(
        101,
        correlation::encode_consensus_intent(&registration).unwrap(),
    )
    .unwrap();
    let mut local_world = driver.execution_world.clone();
    let local_tick = local_world.state().time;
    local_world.submit_action(Action::WorldServiceIntent {
        request: serde_json::to_value(&registration).unwrap(),
    });
    assert!(
        format!("{:?}", local_world.step().unwrap_err()).contains("canonical driver admission")
    );
    assert_eq!(local_world.state().time, local_tick);
    let controller_action: Action =
        serde_json::from_value(serde_json::json!({"type":"TransferMainToken","data":{
            "from_account_id":"a","to_account_id":"b","amount":1,"nonce":1
        }}))
        .unwrap();
    let unsigned_controller = encode_consensus_action_payload(&ConsensusActionPayloadEnvelope {
        version: 1,
        auth: None,
        body: ConsensusActionPayloadBody::RuntimeAction {
            action: controller_action,
        },
    })
    .unwrap();
    assert!(
        format!(
            "{:?}",
            node.submit_consensus_action_payload(102, unsigned_controller)
                .unwrap_err()
        )
        .contains("missing_main_token_auth")
    );
    commit(&mut driver, 3, registration.clone());
    assert_eq!(
        driver
            .execution_world
            .current_cognition_base_binding()
            .unwrap(),
        base
    );
    let authorization_root = driver
        .execution_world
        .capability_authorization_root()
        .to_owned();
    let reserve = SchedulerIntentV1 {
        agent_id: "agent-a".into(),
        request_id: "reserve-1".into(),
        delegation_generation: 0,
        captured_base_binding: base.clone(),
        operation: SchedulerOperationV1::ReserveLease(CognitionLeaseRequestV1::new(
            "lease-1",
            "owner-a",
            "agent-a",
            "session-a",
            "turn-a",
            "decision-a",
            "digest-a",
            CognitionLeaseQuoteV1::new("quote-a", "cognition_units", 10).with_authority(
                "owner-a",
                "v1",
                "cognition",
                "agent-a",
                "v1",
                "owner-a",
                driver
                    .execution_world
                    .current_cognition_runtime_binding()
                    .unwrap()
                    .base_world_hash
                    .to_string(),
            ),
        )),
    };
    let reservation = request(
        &identity,
        WorldServicePayloadV1::Scheduler(sign_read_request("scheduler", reserve, &owner).unwrap()),
    );
    commit(&mut driver, 4, reservation.clone());
    assert_eq!(
        driver
            .execution_world
            .current_cognition_base_binding()
            .unwrap(),
        base
    );
    assert_eq!(
        driver.execution_world.capability_authorization_root(),
        authorization_root
    );
    let result_key = correlation::key_digest(&reservation.correlation.key).unwrap();
    let result: CanonicalIntentResultV1 = serde_json::from_value(
        driver
            .execution_world
            .capability_revocation_state()
            .world_service_results[&result_key]
            .clone(),
    )
    .unwrap();
    assert_eq!(result.rejected, None);
    assert_eq!(result.receipt["reserved_amount"], 10);
    commit(&mut driver, 5, reservation.clone());
    assert_eq!(
        driver
            .execution_world
            .capability_revocation_state()
            .world_service_results
            .len(),
        2
    );
    let pinned = super::super::world_service_read::pin(
        &dir.join("records"),
        &dir.join("store"),
        &identity,
        None,
    )
    .unwrap();
    assert_eq!(pinned.commit.position, 5);
    assert_eq!(pinned.world.state().time, base.base_tick);
    let restarted = NodeRuntimeExecutionDriver::new(
        dir.join("state.json"),
        dir.join("world"),
        dir.join("records"),
        dir.join("store"),
    )
    .unwrap();
    assert_eq!(
        restarted
            .execution_world
            .capability_revocation_state()
            .world_service_results[&result_key],
        driver
            .execution_world
            .capability_revocation_state()
            .world_service_results[&result_key]
    );
    // An ordinary tick must invalidate the captured base, despite admin-only
    // commits preserving it. A fresh request key cannot hide that mismatch.
    driver.execution_world.step().unwrap();
    let mut stale = match reservation.signed_payload {
        WorldServicePayloadV1::Scheduler(signed) => signed.request,
        _ => unreachable!(),
    };
    stale.request_id = "stale-reserve".into();
    let stale = request(
        &identity,
        WorldServicePayloadV1::Scheduler(sign_read_request("scheduler", stale, &owner).unwrap()),
    );
    commit(&mut driver, 6, stale.clone());
    let key = correlation::key_digest(&stale.correlation.key).unwrap();
    let result: CanonicalIntentResultV1 = serde_json::from_value(
        driver
            .execution_world
            .capability_revocation_state()
            .world_service_results[&key]
            .clone(),
    )
    .unwrap();
    assert!(result.rejected.unwrap().contains("base binding changed"));
    let _ = fs::remove_dir_all(dir);
}
