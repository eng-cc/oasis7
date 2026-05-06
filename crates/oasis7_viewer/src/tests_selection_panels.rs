use super::tests_ui_text::{build_selection_details_text, default_locale};
use super::*;

#[test]
fn update_ui_populates_asset_selection_details() {
    let selection = ViewerSelection {
        current: Some(SelectionInfo {
            entity: Entity::from_raw_u32(1).expect("entity"),
            kind: SelectionKind::Asset,
            id: "asset-1".to_string(),
            name: None,
        }),
    };

    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Alpha", oasis7::geometry::GeoPos::new(0, 0, 0)),
    );
    model.assets.insert(
        "asset-1".to_string(),
        oasis7::simulator::Asset {
            id: "asset-1".to_string(),
            owner: oasis7::simulator::ResourceOwner::Location {
                location_id: "loc-1".to_string(),
            },
            kind: oasis7::simulator::AssetKind::Resource {
                kind: oasis7::simulator::ResourceKind::Electricity,
            },
            quantity: 25,
        },
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 8,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 2,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let events = vec![WorldEvent {
        id: 1,
        time: 8,
        kind: oasis7::simulator::WorldEventKind::ResourceTransferred {
            from: oasis7::simulator::ResourceOwner::Location {
                location_id: "loc-1".to_string(),
            },
            to: oasis7::simulator::ResourceOwner::Location {
                location_id: "loc-1".to_string(),
            },
            kind: oasis7::simulator::ResourceKind::Electricity,
            amount: 3,
        },
        runtime_event: None,
    }];

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events,
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let details_text = build_selection_details_text(&selection, &state, None, locale);

    assert!(details_text.contains("Details: asset asset-1"));
    assert!(details_text.contains("Owner: location::loc-1"));
    assert!(details_text.contains("Recent Owner Events"));
}

#[test]
fn update_ui_populates_power_plant_selection_details() {
    let selection = ViewerSelection {
        current: Some(SelectionInfo {
            entity: Entity::from_raw_u32(2).expect("entity"),
            kind: SelectionKind::PowerPlant,
            id: "plant-1".to_string(),
            name: None,
        }),
    };

    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Alpha", oasis7::geometry::GeoPos::new(0, 0, 0)),
    );
    model.power_plants.insert(
        "plant-1".to_string(),
        oasis7::simulator::PowerPlant {
            id: "plant-1".to_string(),
            location_id: "loc-1".to_string(),
            owner: oasis7::simulator::ResourceOwner::Location {
                location_id: "loc-1".to_string(),
            },
            capacity_per_tick: 30,
            current_output: 12,
            fuel_cost_per_pu: 2,
            maintenance_cost: 1,
            status: oasis7::simulator::PlantStatus::Running,
            efficiency: 0.9,
            degradation: 0.1,
        },
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 9,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 3,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let events = vec![WorldEvent {
        id: 2,
        time: 9,
        kind: oasis7::simulator::WorldEventKind::Power(
            oasis7::simulator::PowerEvent::PowerGenerated {
                plant_id: "plant-1".to_string(),
                location_id: "loc-1".to_string(),
                amount: 7,
            },
        ),
        runtime_event: None,
    }];

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events,
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let details_text = build_selection_details_text(&selection, &state, None, locale);

    assert!(details_text.contains("Details: power_plant plant-1"));
    assert!(details_text.contains("Output: current=12"));
    assert!(details_text.contains("generated 7"));
}

#[test]
fn update_ui_populates_chunk_selection_details() {
    let selection = ViewerSelection {
        current: Some(SelectionInfo {
            entity: Entity::from_raw_u32(3).expect("entity"),
            kind: SelectionKind::Chunk,
            id: "0,0,0".to_string(),
            name: Some("generated".to_string()),
        }),
    };

    let mut model = oasis7::simulator::WorldModel::default();
    model.chunks.insert(
        oasis7::simulator::ChunkCoord { x: 0, y: 0, z: 0 },
        oasis7::simulator::ChunkState::Generated,
    );

    let mut budget = oasis7::simulator::ChunkResourceBudget::default();
    budget
        .total_by_element_g
        .insert(oasis7::simulator::FragmentElementKind::Iron, 120);
    budget
        .remaining_by_element_g
        .insert(oasis7::simulator::FragmentElementKind::Iron, 90);
    model
        .chunk_resource_budgets
        .insert(oasis7::simulator::ChunkCoord { x: 0, y: 0, z: 0 }, budget);

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 12,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 3,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let events = vec![WorldEvent {
        id: 2,
        time: 12,
        kind: oasis7::simulator::WorldEventKind::ChunkGenerated {
            coord: oasis7::simulator::ChunkCoord { x: 0, y: 0, z: 0 },
            seed: 11,
            fragment_count: 4,
            block_count: 18,
            chunk_budget: oasis7::simulator::ChunkResourceBudget::default(),
            cause: oasis7::simulator::ChunkGenerationCause::Action,
        },
        runtime_event: None,
    }];

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events,
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let details_text = build_selection_details_text(&selection, &state, None, locale);

    assert!(details_text.contains("Details: chunk 0,0,0"));
    assert!(details_text.contains("State: generated"));
    assert!(details_text.contains("Budget (remaining top):"));
    assert!(details_text.contains("generated fragments=4 blocks=18"));
}

#[test]
fn update_ui_populates_fragment_selection_details_with_owner_location() {
    let selection = ViewerSelection {
        current: Some(SelectionInfo {
            entity: Entity::from_raw_u32(4).expect("entity"),
            kind: SelectionKind::Fragment,
            id: "loc-1#0".to_string(),
            name: Some("loc-1".to_string()),
        }),
    };

    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Alpha", oasis7::geometry::GeoPos::new(0, 0, 0)),
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 12,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events: Vec::new(),
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let details_text = build_selection_details_text(&selection, &state, None, locale);

    assert!(details_text.contains("Details: fragment loc-1#0"));
    assert!(details_text.contains("Location: loc-1"));
    assert!(details_text.contains("Location Name: Alpha"));
}
