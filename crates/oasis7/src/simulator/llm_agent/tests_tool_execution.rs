#[test]
fn restored_parser_accepts_provider_neutral_wait_ticks() {
    let turns = completion_turns_from_output(r#"{"decision":"wait_ticks","ticks":3}"#);
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision: AgentDecision::WaitTicks(3),
            parse_error: None,
            ..
        } => {}
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}

#[test]
fn restored_parser_accepts_provider_neutral_industry_actions() {
    let turns = completion_turns_from_output(
        r#"{"decision":"build_factory","location_id":"loc-2","factory_id":"factory-1","factory_kind":"factory.smelter.mk1"}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision:
                AgentDecision::Act(Action::BuildFactory {
                    location_id,
                    factory_id,
                    factory_kind,
                    ..
                }),
            parse_error: None,
            ..
        } => {
            assert_eq!(location_id, "loc-2");
            assert_eq!(factory_id, "factory-1");
            assert_eq!(factory_kind, "factory.smelter.mk1");
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }

    let turns = completion_turns_from_output(
        r#"{"decision":"schedule_recipe","factory_id":"factory-1","recipe_id":"recipe.smelter.iron_ingot","batches":2}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");
    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::Decision {
            decision:
                AgentDecision::Act(Action::ScheduleRecipe {
                    factory_id,
                    recipe_id,
                    batches,
                    ..
                }),
            parse_error: None,
            ..
        } => {
            assert_eq!(factory_id, "factory-1");
            assert_eq!(recipe_id, "recipe.smelter.iron_ingot");
            assert_eq!(*batches, 2);
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}

#[test]
fn restored_parser_keeps_provider_neutral_module_call_turns() {
    let turns = completion_turns_from_output(
        r#"{"type":"module_call","module":"memory.short_term.recent","args":{"limit":2}}"#,
    );
    let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-1");

    match parsed.first().expect("parsed turn") {
        ParsedLlmTurn::ModuleCall { request, .. } => {
            assert_eq!(request.module, "memory.short_term.recent");
            assert_eq!(
                request.args.get("limit").and_then(|value| value.as_u64()),
                Some(2)
            );
        }
        other => panic!("unexpected parsed turn: {other:?}"),
    }
}
