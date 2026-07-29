use super::*;

fn kernel_for_fragment_refill_preview(
    interval_ticks: i64,
    current_tick: u64,
    fragment_count: usize,
    max_fragments_per_chunk: i64,
) -> (WorldKernel, ChunkCoord) {
    let chunk = ChunkCoord { x: 0, y: 0, z: 0 };
    let mut config = WorldConfig::default();
    config.space = SpaceConfig {
        width_cm: 200_000,
        depth_cm: 200_000,
        height_cm: 200_000,
    };
    config.asteroid_fragment.max_fragments_per_chunk = max_fragments_per_chunk;
    config.asteroid_fragment.replenish_interval_ticks = interval_ticks;
    config.asteroid_fragment.replenish_percent_ppm = 10_000;

    let mut model = WorldModel::default();
    model.chunks.insert(chunk, ChunkState::Generated);
    let mut budget = ChunkResourceBudget::default();
    budget
        .remaining_by_element_g
        .insert(FragmentElementKind::Iron, 15);
    model.chunk_resource_budgets.insert(chunk, budget);
    for index in 0..fragment_count {
        let id = format!("frag-preview-{index}");
        model.locations.insert(
            id.clone(),
            Location::new_with_profile(
                id,
                "preview fragment".to_string(),
                GeoPos {
                    x_cm: 10_000 + index as i64,
                    y_cm: 10_000,
                    z_cm: 10_000,
                },
                LocationProfile::default(),
            ),
        );
    }

    let mut kernel = WorldKernel::with_model_and_chunk_runtime(
        config,
        model,
        ChunkRuntimeConfig {
            world_seed: 7,
            asteroid_fragment_enabled: true,
            asteroid_fragment_seed_offset: 1,
            min_fragment_spacing_cm: None,
        },
    );
    let mut snapshot = kernel.snapshot();
    snapshot.time = current_tick;
    kernel = WorldKernel::from_snapshot(snapshot, kernel.journal_snapshot())
        .expect("restore preview fixture at requested tick");
    (kernel, chunk)
}

#[test]
fn fragment_refill_preview_explains_disabled_interval_without_promising_replenishment() {
    let (kernel, chunk) = kernel_for_fragment_refill_preview(0, 20, 0, 100);

    let quote = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("disabled interval still has a player-readable preview");

    assert!(!quote.replenishment_enabled);
    assert_eq!(quote.next_replenish_tick, None);
    assert_eq!(quote.estimated_replenished_frag_count, 0);
    assert_eq!(
        quote.recommended_resource_action,
        "move_or_switch_material_route"
    );
}

#[test]
fn fragment_refill_preview_reports_wait_cost_when_replenishment_is_not_yet_due() {
    let (kernel, chunk) = kernel_for_fragment_refill_preview(100, 40, 0, 100);

    let quote = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("not-yet-due preview");

    assert!(quote.replenishment_enabled);
    assert!(!quote.replenishment_due);
    assert_eq!(quote.next_replenish_tick, Some(100));
    assert_eq!(quote.wait_cost_ticks, 60);
    assert_eq!(quote.estimated_replenished_frag_count, 0);
    assert_eq!(quote.recommended_resource_action, "wait_current_chunk");
}

#[test]
fn fragment_refill_preview_quantifies_a_replenishable_deficit_and_resource_state() {
    let (kernel, chunk) = kernel_for_fragment_refill_preview(100, 100, 99, 100);

    let quote = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("due replenish preview");

    assert!(quote.replenishment_due);
    assert_eq!(quote.next_replenish_tick, Some(100));
    assert_eq!(quote.estimated_replenished_frag_count, 1);
    assert_eq!(
        quote.remaining_by_element_g.get(&FragmentElementKind::Iron),
        Some(&15)
    );
    assert!(!quote.next_industrial_goal_relevance.is_empty());
    assert_eq!(quote.recommended_resource_action, "wait_current_chunk");
}

#[test]
fn fragment_refill_preview_does_not_recommend_waiting_for_a_full_chunk() {
    let (kernel, chunk) = kernel_for_fragment_refill_preview(100, 100, 100, 100);

    let quote = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("full chunk preview");

    assert_eq!(quote.estimated_replenished_frag_count, 0);
    assert_eq!(
        quote.recommended_resource_action,
        "move_or_switch_material_route"
    );
}

#[test]
fn fragment_refill_preview_is_deterministic_and_does_not_mutate_kernel_state() {
    let (kernel, chunk) = kernel_for_fragment_refill_preview(100, 100, 99, 100);
    let snapshot_before = kernel.snapshot();
    let journal_before = kernel.journal_snapshot();

    let first = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("first preview");
    let second = kernel
        .quote_fragment_refill_preview(chunk)
        .expect("repeat preview");

    assert_eq!(second, first);
    assert_eq!(kernel.snapshot(), snapshot_before);
    assert_eq!(kernel.journal_snapshot(), journal_before);
}
