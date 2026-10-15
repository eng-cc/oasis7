use oasis7_agent_api::{world_action::Action, world_quote::*};

#[test]
fn recipe_quote_validator_preserves_selection_and_authority_fences() {
    let plan = builtin_recipe_plan("recipe.smelter.iron_ingot", 1).unwrap();
    let quote = NativeBuiltinRecipeQuote {
        action: Action::ScheduleRecipe {
            requester_agent_id: "agent".into(),
            factory_id: "factory".into(),
            recipe_id: "recipe.smelter.iron_ingot".into(),
            plan,
            logistics_route_ids: vec![],
            logistics_path_ids: vec![],
        },
        quoted_at_tick: 5,
        authorization: AgentDelegationAuthorizationV1 {
            grant: AgentDelegationGrantV1 {
                grant_id: "grant".into(),
                source_id: "source".into(),
                issuer_id: "owner".into(),
                owner_id: "owner".into(),
                organization_id: None,
                agent_id: "agent".into(),
                object_id: "factory".into(),
                action_kinds: vec!["ScheduleRecipe".into()],
                revision: 1,
                period_id: "period".into(),
                valid_from_tick: 0,
                valid_until_tick: 10,
                limit_units: 20,
                resource_kind: "electricity".into(),
                revoked: false,
            },
            cost_units: 8,
            spent_units: 0,
            remaining_units: 20,
        },
        electricity_before: 20,
        electricity_after: None,
        rejection: Some(RejectReason::RuleDenied {
            notes: vec!["denied".into()],
        }),
        preview: None,
    };
    let validate = |q: &NativeBuiltinRecipeQuote| {
        q.validate_selection("agent", "factory", "recipe.smelter.iron_ingot", 1)
    };
    validate(&quote).unwrap();
    let wire = serde_json::to_vec(&quote).unwrap();
    let mut decoded: NativeBuiltinRecipeQuote = serde_json::from_slice(&wire).unwrap();
    validate(&decoded).unwrap();
    decoded.authorization.grant.revoked = true;
    assert!(validate(&decoded).is_err());
    decoded = quote.clone();
    decoded.electricity_after = Some(12);
    assert!(validate(&decoded).is_err());
    assert!(
        quote
            .validate_selection("other", "factory", "recipe.smelter.iron_ingot", 1)
            .is_err()
    );
}
