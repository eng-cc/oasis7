use super::*;
use serde_json::json;
pub(crate) fn fixture(scope: &str) -> (ReadWorldViewRequest, ReadWorldViewResponse<Value>) {
    let world = WorldIdentity {
        world_id: "world".into(),
        genesis_digest: "genesis".into(),
    };
    let commit = CommitRef {
        world: world.clone(),
        binding: ExecutionBinding {
            provider_world_id: "world".into(),
            branch_id: "main".into(),
            finality_ref: "finality".into(),
            reorg_generation: 0,
            governing_manifest_ref: "manifest".into(),
            authority_generation: 1,
            permission_generation: 1,
        },
        position: 7,
        execution_block_hash: "block".into(),
        state_root_ref: "root".into(),
    };
    let request = ReadWorldViewRequest {
        observation_nonce: None,
        formal_anchor: None,
        contract_version: 1,
        world,
        scope_id: scope.into(),
        min_commit: Some(commit.clone()),
        fixed_commit: None,
        deadline_unix_ms: None,
    };
    let response = ReadWorldViewResponse {
        formal_profile: None,
        formal_proof: None,
        contract_version: 1,
        execution_evidence_scope: None,
        version: ProjectionVersion {
            commit: commit.clone(),
            projection_revision: "v1".into(),
            visibility_scope: scope.into(),
        },
        logical_tick: 4,
        continuation: EventCursor {
            stream_id: "events".into(),
            scope_id: scope.into(),
            era: 0,
            sequence: 0,
            commit,
        },
        view: json!({ "state": { "time": 4, "industries": { "42": { "count": u64::MAX } } },
            "runtime_binding": null, "agent_context": null, "scheduler_wakes": [], "continuations": [],
            "cognition_leases": [], "continuation_contexts": {}, "events": [] }),
    };
    (request, response)
}
#[test]
fn normalized_projection_validates_before_exposing_read_model_and_retains_all_json_fields() {
    let (request, response) = fixture("public");
    let view = VerifiedTaskWorldView::new(response.clone(), &request).unwrap();
    assert_eq!(view.raw_projection(), &response.view);
    assert_eq!(
        view.projection().state["industries"]["42"]["count"],
        u64::MAX
    );
    for field in [
        "tick",
        "cursor",
        "world",
        "generation",
        "evidence",
        "scope",
        "missing",
        "malformed",
    ] {
        let mut changed = response.clone();
        match field {
            "tick" => changed.view["state"]["time"] = json!(5),
            "cursor" => changed.continuation.commit.position += 1,
            "world" => changed.version.commit.world.genesis_digest = "other".into(),
            "generation" => changed.version.commit.binding.permission_generation += 1,
            "evidence" => changed.execution_evidence_scope = Some("controlled".into()),
            "scope" => changed.view["scheduler_wakes"] = json!([{ "agent_id":"other" }]),
            "missing" => {
                changed.view.as_object_mut().unwrap().remove("events");
            }
            "malformed" => changed.view["cognition_leases"] = json!({}),
            _ => unreachable!(),
        }
        assert!(
            VerifiedTaskWorldView::new(changed, &request).is_err(),
            "{field}"
        );
    }
}
#[test]
fn feedback_and_owner_scopes_fail_closed() {
    for scope in ["public", "agent:a"] {
        let (request, mut response) = fixture(scope);
        response.view["feedback_history"] = json!({ "agent_id":"other", "records": [] });
        assert!(VerifiedTaskWorldView::new(response.clone(), &request).is_err());
        response
            .view
            .as_object_mut()
            .unwrap()
            .remove("feedback_history");
        response.view["canonical_agent_owner"] = json!({ "agent_id":"a", "world_id":"other", "reorg_epoch":0,
            "canonical_authority":{ "branch_id":"main", "agent_identity_generation":1 } });
        assert!(VerifiedTaskWorldView::new(response, &request).is_err());
    }
}
#[test]
fn network_read_uses_distinct_profile_domain_and_rejects_legacy_signature() {
    for legacy in [false, true] {
        let (request, response) = fixture("agent:a");
        let payload = serde_json::to_value(response).unwrap();
        let (endpoint, server) =
            crate::tests::serve_payload(VIEW_JSON_PATH, payload, move |response| {
                if legacy {
                    *response = sign_service_response(
                        VIEW_PATH,
                        response.request_digest.clone(),
                        response.payload.clone(),
                        &hex::encode([9; 32]),
                    )
                    .unwrap();
                }
            });
        let client = crate::WorldServiceTaskClient::new(crate::tests::config(endpoint)).unwrap();
        let result = client.read_view(request);
        if legacy {
            assert!(matches!(result, Err(Error::Assurance(_))));
        } else {
            assert_eq!(result.unwrap().logical_tick(), 4);
        }
        server.join().unwrap();
    }
}
