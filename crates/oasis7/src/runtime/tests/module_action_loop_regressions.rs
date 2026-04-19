fn product_only_profile_changes(product_id: &str) -> ModuleProfileChanges {
    ModuleProfileChanges {
        product_profiles: vec![ProductProfileV1 {
            product_id: product_id.to_string(),
            role_tag: "scale".to_string(),
            maintenance_sink: vec![MaterialStack::new("hardware_part", 1)],
            tradable: true,
            unlock_stage: "scale_out".to_string(),
        }],
        recipe_profiles: Vec::new(),
        factory_profiles: Vec::new(),
    }
}

fn recipe_only_profile_changes(recipe_id: &str) -> ModuleProfileChanges {
    ModuleProfileChanges {
        product_profiles: Vec::new(),
        recipe_profiles: vec![RecipeProfileV1 {
            recipe_id: recipe_id.to_string(),
            bottleneck_tags: vec!["control_chip".to_string()],
            stage_gate: "scale_out".to_string(),
            preferred_factory_tags: vec!["assembler".to_string()],
        }],
        factory_profiles: Vec::new(),
    }
}

fn factory_only_profile_changes(factory_id: &str) -> ModuleProfileChanges {
    ModuleProfileChanges {
        product_profiles: Vec::new(),
        recipe_profiles: Vec::new(),
        factory_profiles: vec![FactoryProfileV1 {
            factory_id: factory_id.to_string(),
            tier: 2,
            recipe_slots: 4,
            tags: vec!["assembler".to_string()],
        }],
    }
}

fn submit_release_request(
    world: &mut World,
    requester_agent_id: &str,
    manifest: ModuleManifest,
    profile_changes: ModuleProfileChanges,
) -> u64 {
    world.submit_action(Action::ModuleReleaseSubmit {
        requester_agent_id: requester_agent_id.to_string(),
        manifest,
        activate: true,
        install_target: ModuleInstallTarget::SelfAgent,
        required_roles: vec!["security".to_string()],
        profile_changes,
    });
    world.step().expect("submit module release request");
    match &world.journal().events.last().expect("submit event").body {
        WorldEventBody::Domain(DomainEvent::ModuleReleaseRequested { request_id, .. }) => {
            *request_id
        }
        other => panic!("expected module release requested event: {other:?}"),
    }
}

fn shadow_approve_and_apply_request(world: &mut World, operator_agent_id: &str, request_id: u64) {
    world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: operator_agent_id.to_string(),
        request_id,
    });
    world.step().expect("shadow module release request");
    bind_release_roles(world, operator_agent_id, operator_agent_id, &["security"]);
    world.submit_action(Action::ModuleReleaseApproveRole {
        approver_agent_id: operator_agent_id.to_string(),
        request_id,
        role: "security".to_string(),
    });
    world.step().expect("approve module release role");
    set_module_release_attestation_epoch_snapshot(
        world,
        2,
        &[LOCAL_FINALITY_SIGNER_1, LOCAL_FINALITY_SIGNER_2],
    );
    for (index, signer_node_id) in [LOCAL_FINALITY_SIGNER_1, LOCAL_FINALITY_SIGNER_2]
        .iter()
        .enumerate()
    {
        submit_test_module_release_attestation(
            world,
            operator_agent_id,
            request_id,
            signer_node_id,
            "linux-x86_64",
            format!("bafyshadowapply{request_id}{index:02}").as_str(),
        );
        world
            .step()
            .expect("submit module release attestation before apply");
    }
    world.submit_action(Action::ModuleReleaseApply {
        operator_agent_id: operator_agent_id.to_string(),
        request_id,
    });
    world.step().expect("apply module release request");
}

#[test]
fn module_release_shadow_rejects_existing_product_profile_even_when_payload_matches() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");
    register_agent(&mut world, "operator-1");

    let wasm_bytes = b"module-release-existing-product-profile".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy module artifact");

    let module_id = "m.loop.release.product.existing";
    let first_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        product_only_profile_changes("product.non_overwrite"),
    );
    shadow_approve_and_apply_request(&mut world, "operator-1", first_request_id);

    let second_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        product_only_profile_changes("product.non_overwrite"),
    );
    let action_id = world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: "operator-1".to_string(),
        request_id: second_request_id,
    });
    world
        .step()
        .expect("shadow module release request with existing product profile");
    assert_rule_denied_note_for_action(
        &world,
        action_id,
        "product profile_id already exists in state product.non_overwrite",
    );
}

#[test]
fn module_release_shadow_rejects_existing_recipe_profile_even_when_payload_matches() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");
    register_agent(&mut world, "operator-1");

    let wasm_bytes = b"module-release-existing-recipe-profile".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy module artifact");

    let module_id = "m.loop.release.recipe.existing";
    let first_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        recipe_only_profile_changes("recipe.non_overwrite"),
    );
    shadow_approve_and_apply_request(&mut world, "operator-1", first_request_id);

    let second_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        recipe_only_profile_changes("recipe.non_overwrite"),
    );
    let action_id = world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: "operator-1".to_string(),
        request_id: second_request_id,
    });
    world
        .step()
        .expect("shadow module release request with existing recipe profile");
    assert_rule_denied_note_for_action(
        &world,
        action_id,
        "recipe profile_id already exists in state recipe.non_overwrite",
    );
}

#[test]
fn module_release_shadow_rejects_existing_factory_profile_even_when_payload_matches() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");
    register_agent(&mut world, "operator-1");

    let wasm_bytes = b"module-release-existing-factory-profile".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy module artifact");

    let module_id = "m.loop.release.factory.existing";
    let first_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        factory_only_profile_changes("factory.non_overwrite"),
    );
    shadow_approve_and_apply_request(&mut world, "operator-1", first_request_id);

    let second_request_id = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        factory_only_profile_changes("factory.non_overwrite"),
    );
    let action_id = world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: "operator-1".to_string(),
        request_id: second_request_id,
    });
    world
        .step()
        .expect("shadow module release request with existing factory profile");
    assert_rule_denied_note_for_action(
        &world,
        action_id,
        "factory profile_id already exists in state factory.non_overwrite",
    );
}

#[test]
fn module_release_apply_rechecks_and_rejects_existing_profile_overwrite() {
    let mut world = World::new();
    register_agent(&mut world, "publisher-1");
    register_agent(&mut world, "operator-1");

    let wasm_bytes = b"module-release-apply-recheck-overwrite".to_vec();
    let wasm_hash = util::sha256_hex(&wasm_bytes);
    world.submit_action(Action::DeployModuleArtifact {
        publisher_agent_id: "publisher-1".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes,
    });
    world.step().expect("deploy module artifact");

    let module_id = "m.loop.release.apply.recheck";
    let changes = product_only_profile_changes("product.apply.non_overwrite");
    let request_a = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        changes.clone(),
    );
    let request_b = submit_release_request(
        &mut world,
        "publisher-1",
        base_manifest(module_id, "0.1.0", &wasm_hash),
        changes,
    );

    world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: "operator-1".to_string(),
        request_id: request_a,
    });
    world.step().expect("shadow request a");
    world.submit_action(Action::ModuleReleaseShadow {
        operator_agent_id: "operator-1".to_string(),
        request_id: request_b,
    });
    world.step().expect("shadow request b");

    bind_release_roles(&mut world, "operator-1", "operator-1", &["security"]);
    world.submit_action(Action::ModuleReleaseApproveRole {
        approver_agent_id: "operator-1".to_string(),
        request_id: request_a,
        role: "security".to_string(),
    });
    world.step().expect("approve request a");
    world.submit_action(Action::ModuleReleaseApproveRole {
        approver_agent_id: "operator-1".to_string(),
        request_id: request_b,
        role: "security".to_string(),
    });
    world.step().expect("approve request b");
    set_module_release_attestation_epoch_snapshot(
        &mut world,
        2,
        &[LOCAL_FINALITY_SIGNER_1, LOCAL_FINALITY_SIGNER_2],
    );
    for request_id in [request_a, request_b] {
        for (index, signer_node_id) in [LOCAL_FINALITY_SIGNER_1, LOCAL_FINALITY_SIGNER_2]
            .iter()
            .enumerate()
        {
            submit_test_module_release_attestation(
                &mut world,
                "operator-1",
                request_id,
                signer_node_id,
                "linux-x86_64",
                format!("bafyapplyrecheck{request_id}{index:02}").as_str(),
            );
            world
                .step()
                .expect("submit module release attestation for apply recheck");
        }
    }

    world.submit_action(Action::ModuleReleaseApply {
        operator_agent_id: "operator-1".to_string(),
        request_id: request_a,
    });
    world.step().expect("apply request a");

    let action_id = world.submit_action(Action::ModuleReleaseApply {
        operator_agent_id: "operator-1".to_string(),
        request_id: request_b,
    });
    world.step().expect("apply request b should reject");
    assert_rule_denied_note_for_action(
        &world,
        action_id,
        "product profile_id already exists in state product.apply.non_overwrite",
    );
    assert!(matches!(
        world
            .state()
            .module_release_requests
            .get(&request_b)
            .map(|item| item.status),
        Some(ModuleReleaseRequestStatus::Approved)
    ));
}
