use std::collections::HashSet;

use super::location_fragment_render;
use super::scene_helpers::{
    agent_module_counts_in_snapshot, location_visual_radius_cm, spawn_agent_entity,
    spawn_chunk_entity, spawn_location_entity_with_radiation,
};
use super::*;

pub(super) fn scene_requires_full_rebuild(scene: &Viewer3dScene, snapshot: &WorldSnapshot) -> bool {
    if scene.origin.is_none() || scene.space.is_none() {
        return true;
    }
    if scene
        .last_snapshot_time
        .map(|last| snapshot.time < last)
        .unwrap_or(false)
    {
        return true;
    }
    if scene.origin != Some(space_origin(&snapshot.config.space)) {
        return true;
    }
    scene.space.as_ref() != Some(&snapshot.config.space)
}

pub(super) fn refresh_scene_dirty_objects(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
) {
    let Some(origin) = scene.origin else {
        return;
    };

    prune_absent_locations(commands, scene, snapshot);
    prune_absent_agents(commands, scene, snapshot);

    for (coord, state) in snapshot.model.chunks.iter() {
        spawn_chunk_entity(
            commands,
            config,
            assets,
            scene,
            origin,
            *coord,
            *state,
            &snapshot.config.space,
        );
    }

    for (location_id, location) in snapshot.model.locations.iter() {
        let visual_radius_cm = location_visual_radius_cm(
            location.profile.radius_cm,
            location.fragment_budget.as_ref(),
        );
        if !location_needs_refresh(scene, location_id, location.pos, visual_radius_cm) {
            continue;
        }

        spawn_location_entity_with_radiation(
            commands,
            config,
            assets,
            scene,
            origin,
            location_id,
            &location.name,
            location.pos,
            location.profile.material,
            visual_radius_cm,
            location.profile.radiation_emission_per_tick,
            location.fragment_budget.as_ref(),
        );

        if let (Some(fragment_profile), Some(entity)) = (
            location.fragment_profile.as_ref(),
            scene.location_entities.get(location_id).copied(),
        ) {
            commands.entity(entity).with_children(|parent| {
                location_fragment_render::spawn_location_fragment_elements(
                    parent,
                    assets,
                    location_id,
                    visual_radius_cm,
                    fragment_profile,
                );
            });
        }
    }

    let module_counts = agent_module_counts_in_snapshot(snapshot);
    for (agent_id, agent) in snapshot.model.agents.iter() {
        let module_count = module_counts
            .get(agent_id.as_str())
            .copied()
            .unwrap_or_else(default_agent_module_count_estimate_local);
        if !agent_needs_refresh(
            scene,
            agent_id,
            agent.pos,
            agent.body.height_cm,
            agent.location_id.as_str(),
            module_count,
            &agent.kinematics,
        ) {
            continue;
        }

        spawn_agent_entity(
            commands,
            config,
            assets,
            scene,
            origin,
            agent_id,
            Some(agent.location_id.as_str()),
            agent.pos,
            agent.body.height_cm,
            module_count,
            Some(&agent.kinematics),
        );
    }
}

fn prune_absent_locations(
    commands: &mut Commands,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
) {
    let expected: HashSet<&str> = snapshot
        .model
        .locations
        .keys()
        .map(std::string::String::as_str)
        .collect();
    let to_remove: Vec<String> = scene
        .location_entities
        .keys()
        .filter(|id| !expected.contains(id.as_str()))
        .cloned()
        .collect();
    for location_id in to_remove {
        if let Some(entity) = scene.location_entities.remove(location_id.as_str()) {
            commands.entity(entity).despawn();
        }
        scene.location_positions.remove(location_id.as_str());
        scene.location_radii_cm.remove(location_id.as_str());
    }
}

fn prune_absent_agents(
    commands: &mut Commands,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
) {
    let expected: HashSet<&str> = snapshot
        .model
        .agents
        .keys()
        .map(std::string::String::as_str)
        .collect();
    let to_remove: Vec<String> = scene
        .agent_entities
        .keys()
        .filter(|id| !expected.contains(id.as_str()))
        .cloned()
        .collect();
    for agent_id in to_remove {
        if let Some(entity) = scene.agent_entities.remove(agent_id.as_str()) {
            commands.entity(entity).despawn();
        }
        scene.agent_positions.remove(agent_id.as_str());
        scene.agent_heights_cm.remove(agent_id.as_str());
        scene.agent_location_ids.remove(agent_id.as_str());
        scene.agent_module_counts.remove(agent_id.as_str());
        scene.agent_kinematics.remove(agent_id.as_str());
    }
}

fn location_needs_refresh(
    scene: &Viewer3dScene,
    location_id: &str,
    pos: GeoPos,
    visual_radius_cm: i64,
) -> bool {
    scene.location_entities.get(location_id).is_none()
        || scene.location_positions.get(location_id).copied() != Some(pos)
        || scene.location_radii_cm.get(location_id).copied() != Some(visual_radius_cm)
}

fn agent_needs_refresh(
    scene: &Viewer3dScene,
    agent_id: &str,
    pos: GeoPos,
    height_cm: i64,
    location_id: &str,
    module_count: usize,
    kinematics: &AgentKinematics,
) -> bool {
    let normalized_height = height_cm.max(1);
    scene.agent_entities.get(agent_id).is_none()
        || scene.agent_positions.get(agent_id).copied() != Some(pos)
        || scene.agent_heights_cm.get(agent_id).copied() != Some(normalized_height)
        || scene
            .agent_location_ids
            .get(agent_id)
            .map(std::string::String::as_str)
            != Some(location_id)
        || scene.agent_module_counts.get(agent_id).copied() != Some(module_count)
        || scene.agent_kinematics.get(agent_id) != Some(kinematics)
}

fn default_agent_module_count_estimate_local() -> usize {
    oasis7::models::AgentBodyState::default()
        .slots
        .iter()
        .filter(|slot| slot.installed_module.is_some())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::simulator::{Agent, Location, WorldConfig, WorldModel, WorldSnapshot};

    #[test]
    fn scene_requires_full_rebuild_only_for_topology_or_time_regression() {
        let snapshot = sample_snapshot(10);
        let origin = space_origin(&snapshot.config.space);
        let mut scene = Viewer3dScene {
            origin: Some(origin),
            space: Some(snapshot.config.space.clone()),
            last_snapshot_time: Some(9),
            ..Viewer3dScene::default()
        };

        assert!(!scene_requires_full_rebuild(&scene, &snapshot));

        scene.last_snapshot_time = Some(11);
        assert!(scene_requires_full_rebuild(&scene, &snapshot));
    }

    #[test]
    fn location_needs_refresh_detects_cached_delta() {
        let mut scene = Viewer3dScene::default();
        let location_id = "loc-1";
        scene
            .location_entities
            .insert(location_id.to_string(), Entity::from_bits(1));
        let pos = GeoPos {
            x_cm: 1,
            y_cm: 2,
            z_cm: 3,
        };
        scene
            .location_positions
            .insert(location_id.to_string(), pos);
        scene.location_radii_cm.insert(location_id.to_string(), 120);

        assert!(!location_needs_refresh(&scene, location_id, pos, 120));
        assert!(location_needs_refresh(&scene, location_id, pos, 130));
        assert!(location_needs_refresh(
            &scene,
            location_id,
            GeoPos {
                x_cm: 4,
                y_cm: 2,
                z_cm: 3
            },
            120
        ));
    }

    #[test]
    fn agent_needs_refresh_detects_module_and_location_delta() {
        let mut scene = Viewer3dScene::default();
        let agent_id = "agent-1";
        let kinematics = AgentKinematics::default();
        let pos = GeoPos {
            x_cm: 1,
            y_cm: 2,
            z_cm: 3,
        };
        scene
            .agent_entities
            .insert(agent_id.to_string(), Entity::from_bits(2));
        scene.agent_positions.insert(agent_id.to_string(), pos);
        scene.agent_heights_cm.insert(agent_id.to_string(), 170);
        scene
            .agent_location_ids
            .insert(agent_id.to_string(), "loc-a".to_string());
        scene.agent_module_counts.insert(agent_id.to_string(), 2);
        scene
            .agent_kinematics
            .insert(agent_id.to_string(), kinematics.clone());

        assert!(!agent_needs_refresh(
            &scene,
            agent_id,
            pos,
            170,
            "loc-a",
            2,
            &kinematics
        ));
        assert!(agent_needs_refresh(
            &scene,
            agent_id,
            pos,
            170,
            "loc-b",
            2,
            &kinematics
        ));
        assert!(agent_needs_refresh(
            &scene,
            agent_id,
            pos,
            170,
            "loc-a",
            3,
            &kinematics
        ));

        let mut moving_kinematics = kinematics.clone();
        moving_kinematics.move_remaining_cm = 100;
        assert!(agent_needs_refresh(
            &scene,
            agent_id,
            pos,
            170,
            "loc-a",
            2,
            &moving_kinematics
        ));
    }

    fn sample_snapshot(time: u64) -> WorldSnapshot {
        let mut model = WorldModel::default();
        model.locations.insert(
            "loc-1".to_string(),
            Location::new(
                "loc-1",
                "Alpha",
                GeoPos {
                    x_cm: 0,
                    y_cm: 0,
                    z_cm: 0,
                },
            ),
        );
        model.agents.insert(
            "agent-1".to_string(),
            Agent::new(
                "agent-1",
                "loc-1",
                GeoPos {
                    x_cm: 0,
                    y_cm: 0,
                    z_cm: 0,
                },
            ),
        );
        WorldSnapshot {
            version: oasis7::simulator::SNAPSHOT_VERSION,
            chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
            time,
            config: WorldConfig::default(),
            model,
            chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
            next_event_id: 1,
            next_action_id: 1,
            pending_actions: Vec::new(),
            journal_len: 0,
            runtime_snapshot: None,
            player_gameplay: None,
        }
    }
}
