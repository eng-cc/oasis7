use super::*;
use crate::runtime::{Action, AgentDelegationGrantV1};

#[test]
fn native_grant_projection_is_agent_scoped_and_absent_from_generic_reads() {
    let mut world = World::new();
    let public = super::super::authority::sign_read_request("fixture", (), &"71".repeat(32))
        .unwrap()
        .subject_public_key;
    for agent in ["agent-a", "agent-b"] {
        world.submit_action(Action::RegisterAgent {
            agent_id: agent.into(),
            pos: crate::GeoPos::new(0, 0, 0),
        });
    }
    world.step().unwrap();
    // Independent initial fixture claims provide the existing native grant source.
    let mut state = world.state().clone();
    for agent in ["agent-a", "agent-b"] {
        state.starter_oc_claims.insert(
            agent.into(),
            serde_json::from_value(serde_json::json!({
                "agent_id": agent, "player_id": "owner", "public_key": public,
                "amount": 1, "claimed_at": 0, "source_treasury_bucket_id": null,
            }))
            .unwrap(),
        );
    }
    let mut world = World::new_with_state(state);
    for agent in ["agent-a", "agent-b"] {
        world
            .install_capability_agent_identity(agent, "owner", 1)
            .unwrap();
        world
            .install_agent_delegation_grant(AgentDelegationGrantV1 {
                grant_id: format!("grant-{agent}"),
                source_id: format!("starter_claim:{agent}:0"),
                issuer_id: "owner".into(),
                owner_id: "owner".into(),
                organization_id: None,
                agent_id: agent.into(),
                object_id: agent.into(),
                action_kinds: vec!["MoveAgent".into()],
                revision: 1,
                period_id: "period".into(),
                valid_from_tick: 0,
                valid_until_tick: 100,
                limit_units: 7,
                resource_kind: "electricity".into(),
                revoked: false,
            })
            .unwrap();
    }
    world
        .bind_cognition_runtime("world", "main", 0, None, "pending", 0)
        .unwrap();
    let before = serde_json::to_value(world.snapshot()).unwrap();
    for scope in [None, Some("agent-a")] {
        let generic = WorldServiceProjection::from_world(&world, scope).unwrap();
        assert!(generic.native_delegation_authorizations.is_empty());
        assert!(
            serde_json::to_value(generic)
                .unwrap()
                .get("native_delegation_authorizations")
                .is_none()
        );
    }
    let scoped =
        WorldServiceProjection::from_authenticated_world(&world, Some("agent-a"), &public).unwrap();
    assert_eq!(scoped.native_delegation_authorizations.len(), 1);
    let authorization = &scoped.native_delegation_authorizations[0];
    assert_eq!(authorization.grant.agent_id, "agent-a");
    assert_eq!(authorization.remaining_units, 7);
    assert_eq!(authorization.spent_units, 0);
    assert_eq!(serde_json::to_value(world.snapshot()).unwrap(), before);
    let mut legacy = serde_json::to_value(scoped).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("native_delegation_authorizations");
    assert!(
        serde_json::from_value::<WorldServiceProjection>(legacy)
            .unwrap()
            .native_delegation_authorizations
            .is_empty()
    );
}
