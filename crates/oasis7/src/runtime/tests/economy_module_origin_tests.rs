use super::*;

#[test]
fn committed_recipe_origin_module_due_completion_preserves_pending_and_legacy() {
    for has_origin in [false, true] {
        let mut world = logistics_drone_module_recipe_world("factory.test");
        let mut origin = crate::runtime::events::recipe_origin_tests::origin();
        origin.submission.recipe_id = "recipe.test".into();
        let action = Action::ScheduleRecipe {
            requester_agent_id: "builder-a".into(),
            factory_id: "factory.test".into(),
            recipe_id: "recipe.test".into(),
            plan: RecipeExecutionPlan::accepted(
                1,
                vec![],
                vec![MaterialStack::new("iron_ingot", 3)],
                vec![],
                1,
                1,
            ),
            logistics_route_ids: vec![],
            logistics_path_ids: vec![],
        };
        let id = if has_origin {
            world
                .submit_recipe_action_with_origin(action, origin.clone())
                .unwrap()
        } else {
            world.submit_action(action)
        };
        let expected = has_origin.then_some(origin);
        let mut sandbox = CaptureContextSandbox::with_outputs(vec![]);
        world
            .step_with_modules(&mut sandbox)
            .expect("start with modules");
        assert_eq!(
            world.state().pending_recipe_jobs[&id].committed_recipe_origin,
            expected
        );
        world
            .step_with_modules(&mut sandbox)
            .expect("module due completion");
        assert_eq!(
            world.state().recipe_completion_receipts[&id].committed_recipe_origin,
            expected
        );
        let completed = world
            .journal()
            .events
            .iter()
            .find_map(|event| match &event.body {
                WorldEventBody::Domain(DomainEvent::RecipeCompleted {
                    job_id,
                    committed_recipe_origin,
                    ..
                }) if *job_id == id => Some(committed_recipe_origin.clone()),
                _ => None,
            })
            .expect("actual module completion event");
        assert_eq!(completed, expected);
        let settled = world.state().material_ledgers.clone();
        world
            .step_with_modules(&mut sandbox)
            .expect("later module tick");
        assert_eq!(
            world.state().material_ledgers,
            settled,
            "completion is not repeated"
        );
        assert_eq!(world.state().industry_progress.completed_recipe_jobs, 1);
    }
}
