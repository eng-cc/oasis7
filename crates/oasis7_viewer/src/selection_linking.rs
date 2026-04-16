use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use oasis7::simulator::{
    ChunkCoord, PowerEvent, RejectReason, ResourceOwner, WorldEvent, WorldEventKind,
};

use super::location_fragment_render::FragmentElementMarker;
use crate::i18n::{locale_or_default, UiI18n, UiLocale};
use crate::ui_locale_text::{
    jump_selection_label, link_ready, locate_focus_label, map_link_message_for_locale,
    quick_locate_agent_label,
};

use super::*;
#[path = "selection_linking_event_matchers.rs"]
mod selection_linking_event_matchers;
use selection_linking_event_matchers::*;

#[derive(Resource)]
pub(super) struct EventObjectLinkState {
    pub message: String,
}

impl Default for EventObjectLinkState {
    fn default() -> Self {
        Self {
            message: link_ready(UiLocale::EnUs).to_string(),
        }
    }
}

#[derive(Component)]
pub(super) struct LocateFocusEventButton;

#[derive(Component)]
pub(super) struct QuickLocateAgentButton;

#[derive(Component)]
pub(super) struct JumpSelectionEventsButton;

#[derive(Component)]
pub(super) struct EventObjectLinkText;

#[allow(dead_code)]
#[derive(Component)]
pub(super) struct LocateFocusEventButtonLabel;

#[allow(dead_code)]
#[derive(Component)]
pub(super) struct QuickLocateAgentButtonLabel;

#[allow(dead_code)]
#[derive(Component)]
pub(super) struct JumpSelectionEventsButtonLabel;

#[derive(Clone)]
pub(super) struct SelectionTarget {
    pub(super) kind: SelectionKind,
    pub(super) id: String,
    pub(super) name: Option<String>,
}

#[allow(dead_code)]
pub(super) fn spawn_event_object_link_controls(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    locale: UiLocale,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.13, 0.18)),
            BorderColor::all(Color::srgb(0.23, 0.25, 0.32)),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(24.0),
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|buttons| {
                spawn_quick_locate_agent_button(buttons, &font, locale);
                spawn_locate_focus_button(buttons, &font, locale);
                spawn_jump_selection_button(buttons, &font, locale);
            });

            root.spawn((
                Text::new(link_ready(locale)),
                TextFont {
                    font,
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.82, 0.9)),
                EventObjectLinkText,
            ));
        });
}

#[allow(dead_code)]
fn spawn_locate_focus_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    locale: UiLocale,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(120.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(Color::srgb(0.22, 0.32, 0.24)),
            LocateFocusEventButton,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(locate_focus_label(locale)),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                LocateFocusEventButtonLabel,
            ));
        });
}

#[allow(dead_code)]
fn spawn_jump_selection_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    locale: UiLocale,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(120.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(Color::srgb(0.22, 0.24, 0.34)),
            JumpSelectionEventsButton,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(jump_selection_label(locale)),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                JumpSelectionEventsButtonLabel,
            ));
        });
}

#[allow(dead_code)]
fn spawn_quick_locate_agent_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    locale: UiLocale,
) {
    buttons
        .spawn((
            Button,
            Node {
                min_width: Val::Px(120.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(Color::srgb(0.21, 0.30, 0.40)),
            QuickLocateAgentButton,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(quick_locate_agent_label(locale)),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                QuickLocateAgentButtonLabel,
            ));
        });
}

pub(super) fn handle_locate_focus_event_button(
    mut interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<LocateFocusEventButton>,
        ),
    >,
    state: Res<ViewerState>,
    scene: Res<Viewer3dScene>,
    config: Res<Viewer3dConfig>,
    mut selection: ResMut<ViewerSelection>,
    mut link_state: ResMut<EventObjectLinkState>,
    mut transforms: Query<(&mut Transform, Option<&BaseScale>)>,
    mut timeline: Option<ResMut<TimelineUiState>>,
) {
    for interaction in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        locate_focus_event_action(
            &state,
            &scene,
            &config,
            &mut selection,
            &mut link_state,
            &mut transforms,
            timeline.as_deref_mut(),
        );
    }
}

pub(super) fn handle_quick_locate_agent_button(
    mut interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<QuickLocateAgentButton>,
        ),
    >,
    scene: Res<Viewer3dScene>,
    config: Res<Viewer3dConfig>,
    mut selection: ResMut<ViewerSelection>,
    mut link_state: ResMut<EventObjectLinkState>,
    mut transforms: Query<(&mut Transform, Option<&BaseScale>)>,
) {
    for interaction in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        quick_locate_agent_action(
            &scene,
            &config,
            &mut selection,
            &mut link_state,
            &mut transforms,
        );
    }
}

pub(super) fn handle_jump_selection_events_button(
    mut interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<JumpSelectionEventsButton>,
        ),
    >,
    state: Res<ViewerState>,
    selection: Res<ViewerSelection>,
    mut link_state: ResMut<EventObjectLinkState>,
    mut timeline: Option<ResMut<TimelineUiState>>,
) {
    for interaction in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        jump_selection_events_action(&state, &selection, &mut link_state, timeline.as_deref_mut());
    }
}

pub(super) fn quick_locate_agent_action(
    scene: &Viewer3dScene,
    config: &Viewer3dConfig,
    selection: &mut ViewerSelection,
    link_state: &mut EventObjectLinkState,
    transforms: &mut Query<(&mut Transform, Option<&BaseScale>)>,
) {
    let target_agent_id = selection
        .current
        .as_ref()
        .filter(|current| {
            current.kind == SelectionKind::Agent
                && scene.agent_entities.contains_key(current.id.as_str())
        })
        .map(|current| current.id.clone())
        .or_else(|| first_sorted_agent_id(scene));

    let Some(agent_id) = target_agent_id else {
        link_state.message = "Link: no agents available".to_string();
        return;
    };

    let Some(entity) = scene.agent_entities.get(agent_id.as_str()).copied() else {
        link_state.message = format!("Link: target agent {} is not in current scene", agent_id);
        return;
    };

    apply_selection(
        selection,
        transforms,
        config,
        entity,
        SelectionKind::Agent,
        agent_id.clone(),
        None,
    );
    link_state.message = format!("Link: located agent {agent_id}");
}

fn first_sorted_agent_id(scene: &Viewer3dScene) -> Option<String> {
    let mut ids = scene.agent_entities.keys().cloned().collect::<Vec<_>>();
    ids.sort_unstable();
    ids.into_iter().next()
}

pub(super) fn locate_focus_event_action(
    state: &ViewerState,
    scene: &Viewer3dScene,
    config: &Viewer3dConfig,
    selection: &mut ViewerSelection,
    link_state: &mut EventObjectLinkState,
    transforms: &mut Query<(&mut Transform, Option<&BaseScale>)>,
    timeline: Option<&mut TimelineUiState>,
) {
    let focus_tick = focus_tick(state, timeline.as_deref());
    let Some(event) = nearest_event_to_tick(&state.events, focus_tick) else {
        link_state.message = "Link: no events available".to_string();
        return;
    };

    let Some(target) = event_primary_target(event, state.snapshot.as_ref()) else {
        link_state.message = format!(
            "Link: event #{} t{} has no mappable object",
            event.id, event.time
        );
        return;
    };

    let Some(entity) = target_entity(scene, &target) else {
        link_state.message = format!(
            "Link: target {} {} is not in current scene",
            selection_kind_label(target.kind),
            target.id
        );
        return;
    };

    apply_selection(
        selection,
        transforms,
        config,
        entity,
        target.kind,
        target.id.clone(),
        target.name.clone(),
    );

    if let Some(timeline) = timeline {
        timeline.target_tick = event.time;
        timeline.manual_override = true;
        timeline.drag_active = false;
    }

    link_state.message = format!(
        "Link: event #{} t{} -> {} {}",
        event.id,
        event.time,
        selection_kind_label(target.kind),
        target.id
    );
}

pub(super) fn jump_selection_events_action(
    state: &ViewerState,
    selection: &ViewerSelection,
    link_state: &mut EventObjectLinkState,
    timeline: Option<&mut TimelineUiState>,
) {
    let Some(current) = selection.current.as_ref() else {
        link_state.message = "Link: no selection".to_string();
        return;
    };

    let related_ticks = selection_related_ticks(current, &state.events, state.snapshot.as_ref());
    if related_ticks.is_empty() {
        link_state.message = format!(
            "Link: {} {} has no related events",
            selection_kind_label(current.kind),
            current.id
        );
        return;
    }

    let pivot = focus_tick(state, timeline.as_deref());
    let Some(next_tick) = select_next_tick(&related_ticks, pivot) else {
        link_state.message = "Link: no target tick".to_string();
        return;
    };

    if let Some(timeline) = timeline {
        timeline.target_tick = next_tick;
        timeline.manual_override = true;
        timeline.drag_active = false;
    }

    link_state.message = format!(
        "Link: {} {} -> t{}",
        selection_kind_label(current.kind),
        current.id,
        next_tick
    );
}

pub(super) fn update_event_object_link_text(
    link_state: Res<EventObjectLinkState>,
    i18n: Option<Res<UiI18n>>,
    mut query: Query<&mut Text, With<EventObjectLinkText>>,
) {
    let locale_changed = i18n
        .as_ref()
        .map(|value| value.is_changed())
        .unwrap_or(false);
    if !link_state.is_changed() && !locale_changed {
        return;
    }
    let locale = locale_or_default(i18n.as_deref());
    if let Ok(mut text) = query.single_mut() {
        text.0 = map_link_message_for_locale(&link_state.message, locale);
    }
}

#[cfg(test)]
pub(super) fn update_event_object_link_button_labels(
    i18n: Option<Res<UiI18n>>,
    mut labels: ParamSet<(
        Query<&mut Text, With<LocateFocusEventButtonLabel>>,
        Query<&mut Text, With<QuickLocateAgentButtonLabel>>,
        Query<&mut Text, With<JumpSelectionEventsButtonLabel>>,
    )>,
) {
    let Some(i18n) = i18n else {
        return;
    };
    if !i18n.is_changed() {
        return;
    }

    let locale = i18n.locale;
    if let Ok(mut text) = labels.p0().single_mut() {
        text.0 = locate_focus_label(locale).to_string();
    }
    if let Ok(mut text) = labels.p1().single_mut() {
        text.0 = quick_locate_agent_label(locale).to_string();
    }
    if let Ok(mut text) = labels.p2().single_mut() {
        text.0 = jump_selection_label(locale).to_string();
    }
}

pub(super) fn pick_3d_selection(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Viewer3dCamera>>,
    agents: Query<(Entity, &GlobalTransform, &AgentMarker)>,
    locations: Query<(Entity, &GlobalTransform, &LocationMarker), With<Mesh3d>>,
    fragments: Query<(Entity, &GlobalTransform, &FragmentElementMarker)>,
    assets: Query<(Entity, &GlobalTransform, &AssetMarker)>,
    power_plants: Query<(Entity, &GlobalTransform, &PowerPlantMarker)>,
    chunks: Query<(Entity, &GlobalTransform, &ChunkMarker)>,
    config: Res<Viewer3dConfig>,
    panel_width: Option<Res<RightPanelWidthState>>,
    mut selection: ResMut<ViewerSelection>,
    mut transforms: Query<(&mut Transform, Option<&BaseScale>)>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let right_panel_width = panel_width
        .as_deref()
        .map(|state| state.width_px)
        .unwrap_or(UI_PANEL_WIDTH);
    let right_bound = (window.width() - right_panel_width).max(0.0);
    if cursor_position.x > right_bound {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    let mut best: Option<(Entity, SelectionKind, String, Option<String>, f32)> = None;

    for (entity, transform, marker) in agents.iter() {
        if let Some(distance) = ray_point_distance(ray, transform.translation()) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::Agent,
                    marker.id.clone(),
                    Some(format!("modules={}", marker.module_count)),
                    distance,
                ));
            }
        }
    }

    for (entity, transform, marker) in locations.iter() {
        if !marker.id.starts_with("frag-") {
            continue;
        }
        let _material = marker.material;
        let _radiation_emission_per_tick = marker.radiation_emission_per_tick;
        if let Some(distance) = ray_point_distance(ray, transform.translation()) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::Location,
                    marker.id.clone(),
                    Some(marker.name.clone()),
                    distance,
                ));
            }
        }
    }

    for (entity, transform, marker) in fragments.iter() {
        if let Some(distance) = ray_point_distance(ray, transform.translation()) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::Fragment,
                    marker.id.clone(),
                    Some(marker.location_id.clone()),
                    distance,
                ));
            }
        }
    }

    for (entity, transform, marker) in assets.iter() {
        if let Some(distance) = ray_point_distance(ray, transform.translation()) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::Asset,
                    marker.id.clone(),
                    None,
                    distance,
                ));
            }
        }
    }

    for (entity, transform, marker) in power_plants.iter() {
        if let Some(distance) = ray_point_distance(ray, transform.translation()) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::PowerPlant,
                    marker.id.clone(),
                    None,
                    distance,
                ));
            }
        }
    }

    for (entity, _transform, marker) in chunks.iter() {
        if let Some(distance) = ray_chunk_grid_hit_distance(ray, marker) {
            if distance <= PICK_MAX_DISTANCE
                && best
                    .as_ref()
                    .map(|(_, _, _, _, best_dist)| distance < *best_dist)
                    .unwrap_or(true)
            {
                best = Some((
                    entity,
                    SelectionKind::Chunk,
                    marker.id.clone(),
                    Some(marker.state.clone()),
                    distance,
                ));
            }
        }
    }

    if let Some((entity, kind, id, name, _)) = best {
        apply_selection(
            &mut selection,
            &mut transforms,
            &config,
            entity,
            kind,
            id,
            name,
        );
    } else if selection.current.is_some() {
        if let Some(current) = selection.current.take() {
            reset_entity_scale(&mut transforms, current.entity);
        }
    }
}

fn ray_chunk_grid_hit_distance(ray: Ray3d, marker: &ChunkMarker) -> Option<f32> {
    let direction = ray.direction.as_vec3();
    if direction.y.abs() <= f32::EPSILON {
        return None;
    }
    let t = (marker.pick_y - ray.origin.y) / direction.y;
    if t < 0.0 {
        return None;
    }
    let hit = ray.origin + direction * t;
    if hit.x < marker.min_x || hit.x > marker.max_x || hit.z < marker.min_z || hit.z > marker.max_z
    {
        return None;
    }
    Some(t)
}

pub(super) fn apply_selection(
    selection: &mut ViewerSelection,
    transforms: &mut Query<(&mut Transform, Option<&BaseScale>)>,
    config: &Viewer3dConfig,
    entity: Entity,
    kind: SelectionKind,
    id: String,
    name: Option<String>,
) {
    if let Some(current) = selection.current.take() {
        reset_entity_scale(transforms, current.entity);
    }
    selection.current = Some(SelectionInfo {
        entity,
        kind,
        id,
        name,
    });
    if config.highlight_selected && should_apply_scale_highlight(kind) {
        apply_entity_highlight(transforms, entity);
    }
}

pub(super) fn focus_tick(state: &ViewerState, timeline: Option<&TimelineUiState>) -> u64 {
    match timeline {
        Some(timeline) if timeline.manual_override || timeline.drag_active => timeline.target_tick,
        _ => current_tick_from_state(state),
    }
}

fn current_tick_from_state(state: &ViewerState) -> u64 {
    state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.time)
        .or_else(|| state.metrics.as_ref().map(|metrics| metrics.total_ticks))
        .unwrap_or(0)
}

fn nearest_event_to_tick(events: &[WorldEvent], tick: u64) -> Option<&WorldEvent> {
    events.iter().min_by_key(|event| event.time.abs_diff(tick))
}

pub(super) fn select_next_tick(ticks: &[u64], pivot: u64) -> Option<u64> {
    ticks
        .iter()
        .copied()
        .find(|tick| *tick > pivot)
        .or_else(|| ticks.first().copied())
}

pub(super) fn selection_related_ticks(
    selection: &SelectionInfo,
    events: &[WorldEvent],
    snapshot: Option<&WorldSnapshot>,
) -> Vec<u64> {
    let mut ticks = Vec::new();
    for event in events {
        if event_matches_selection(event, selection, snapshot) {
            ticks.push(event.time);
        }
    }
    ticks.sort_unstable();
    ticks.dedup();
    ticks
}

fn event_matches_selection(
    event: &WorldEvent,
    selection: &SelectionInfo,
    snapshot: Option<&WorldSnapshot>,
) -> bool {
    match selection.kind {
        SelectionKind::Agent => event_matches_agent(event, selection.id.as_str()),
        SelectionKind::Location => event_matches_location(event, selection.id.as_str()),
        SelectionKind::Fragment => selection
            .name
            .as_deref()
            .map(|location_id| event_matches_location(event, location_id))
            .unwrap_or(false),
        SelectionKind::PowerPlant => event_matches_power_plant(event, selection.id.as_str()),
        SelectionKind::Chunk => selection
            .id
            .parse::<ChunkCoordId>()
            .ok()
            .map(|coord| event_matches_chunk(event, coord.coord))
            .unwrap_or(false),
        SelectionKind::Asset => snapshot
            .and_then(|snapshot| snapshot.model.assets.get(selection.id.as_str()))
            .map(|asset| event_matches_owner(event, &asset.owner))
            .unwrap_or(false),
    }
}

pub(super) fn event_primary_target(
    event: &WorldEvent,
    snapshot: Option<&WorldSnapshot>,
) -> Option<SelectionTarget> {
    match &event.kind {
        WorldEventKind::LocationRegistered {
            location_id, name, ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Location,
            id: location_id.clone(),
            name: Some(name.clone()),
        }),
        WorldEventKind::AgentRegistered { agent_id, .. }
        | WorldEventKind::AgentMoved { agent_id, .. }
        | WorldEventKind::AgentSpoke { agent_id, .. }
        | WorldEventKind::TargetInspected { agent_id, .. }
        | WorldEventKind::SimpleInteractionPerformed { agent_id, .. }
        | WorldEventKind::RadiationHarvested { agent_id, .. }
        | WorldEventKind::LlmEffectQueued { agent_id, .. }
        | WorldEventKind::LlmReceiptAppended { agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        WorldEventKind::AgentPromptUpdated { profile, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: profile.agent_id.clone(),
            name: None,
        }),
        WorldEventKind::AgentPlayerBound { agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        WorldEventKind::AgentPlayerUnbound { agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ResourceTransferred { from, to, .. }
        | WorldEventKind::Power(PowerEvent::PowerTransferred { from, to, .. }) => {
            owner_to_target(from, snapshot).or_else(|| owner_to_target(to, snapshot))
        }
        WorldEventKind::DebugResourceGranted { owner, .. }
        | WorldEventKind::CompoundMined { owner, .. }
        | WorldEventKind::CompoundRefined { owner, .. } => owner_to_target(owner, snapshot),
        WorldEventKind::FactoryBuilt { owner, .. }
        | WorldEventKind::RecipeScheduled { owner, .. } => owner_to_target(owner, snapshot),
        WorldEventKind::PowerOrderPlaced { owner, .. }
        | WorldEventKind::PowerOrderCancelled { owner, .. } => owner_to_target(owner, snapshot),
        WorldEventKind::SocialFactPublished { fact } => owner_to_target(&fact.actor, snapshot)
            .or_else(|| owner_to_target(&fact.subject, snapshot))
            .or_else(|| {
                fact.object
                    .as_ref()
                    .and_then(|object| owner_to_target(object, snapshot))
            }),
        WorldEventKind::SocialFactChallenged { challenger, .. } => {
            owner_to_target(challenger, snapshot)
        }
        WorldEventKind::SocialFactAdjudicated { adjudicator, .. } => {
            owner_to_target(adjudicator, snapshot)
        }
        WorldEventKind::SocialFactRevoked { actor, .. } => owner_to_target(actor, snapshot),
        WorldEventKind::SocialFactExpired { fact_id, .. } => snapshot.and_then(|snapshot| {
            snapshot
                .model
                .social_facts
                .get(fact_id)
                .and_then(|fact| owner_to_target(&fact.actor, Some(snapshot)))
        }),
        WorldEventKind::SocialEdgeDeclared { edge } => owner_to_target(&edge.declarer, snapshot)
            .or_else(|| owner_to_target(&edge.from, snapshot))
            .or_else(|| owner_to_target(&edge.to, snapshot)),
        WorldEventKind::SocialEdgeExpired { edge_id, .. } => snapshot.and_then(|snapshot| {
            snapshot
                .model
                .social_edges
                .get(edge_id)
                .and_then(|edge| owner_to_target(&edge.declarer, Some(snapshot)))
        }),
        WorldEventKind::ChunkGenerated { coord, .. } => Some(SelectionTarget {
            kind: SelectionKind::Chunk,
            id: chunk_id(*coord),
            name: None,
        }),
        WorldEventKind::FragmentsReplenished { entries } => {
            entries.first().map(|entry| SelectionTarget {
                kind: SelectionKind::Chunk,
                id: chunk_id(entry.coord),
                name: None,
            })
        }
        WorldEventKind::ModuleArtifactDeployed {
            publisher_agent_id, ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: publisher_agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleInstalled {
            installer_agent_id, ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: installer_agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleArtifactListed {
            seller_agent_id, ..
        }
        | WorldEventKind::ModuleArtifactDelisted {
            seller_agent_id, ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: seller_agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleArtifactBidPlaced {
            bidder_agent_id, ..
        }
        | WorldEventKind::ModuleArtifactBidCancelled {
            bidder_agent_id, ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: bidder_agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleArtifactSaleCompleted {
            buyer_agent_id,
            seller_agent_id,
            ..
        } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: buyer_agent_id.clone(),
            name: Some(seller_agent_id.clone()),
        }),
        WorldEventKind::ModuleArtifactDestroyed { owner_agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: owner_agent_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleVisualEntityUpserted { entity } => Some(SelectionTarget {
            kind: SelectionKind::Asset,
            id: entity.entity_id.clone(),
            name: None,
        }),
        WorldEventKind::ModuleVisualEntityRemoved { entity_id } => Some(SelectionTarget {
            kind: SelectionKind::Asset,
            id: entity_id.clone(),
            name: None,
        }),
        WorldEventKind::ActionRejected { reason } => reject_reason_to_target(reason, snapshot),
        WorldEventKind::Power(power_event) => power_event_target(power_event, snapshot),
        WorldEventKind::RuntimeEvent { .. } => None,
    }
}

fn power_event_target(
    power_event: &PowerEvent,
    snapshot: Option<&WorldSnapshot>,
) -> Option<SelectionTarget> {
    match power_event {
        PowerEvent::PowerPlantRegistered { plant } => Some(SelectionTarget {
            kind: SelectionKind::PowerPlant,
            id: plant.id.clone(),
            name: None,
        }),
        PowerEvent::PowerGenerated { plant_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::PowerPlant,
            id: plant_id.clone(),
            name: None,
        }),
        PowerEvent::PowerConsumed { agent_id, .. }
        | PowerEvent::PowerStateChanged { agent_id, .. }
        | PowerEvent::PowerCharged { agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        PowerEvent::PowerTransferred { from, to, .. } => {
            owner_to_target(from, snapshot).or_else(|| owner_to_target(to, snapshot))
        }
    }
}

fn reject_reason_to_target(
    reason: &RejectReason,
    snapshot: Option<&WorldSnapshot>,
) -> Option<SelectionTarget> {
    match reason {
        RejectReason::AgentAlreadyExists { agent_id }
        | RejectReason::AgentNotFound { agent_id }
        | RejectReason::AgentAlreadyAtLocation { agent_id, .. }
        | RejectReason::AgentNotAtLocation { agent_id, .. }
        | RejectReason::AgentShutdown { agent_id } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        RejectReason::AgentsNotCoLocated { agent_id, .. } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        RejectReason::LocationAlreadyExists { location_id }
        | RejectReason::LocationNotFound { location_id }
        | RejectReason::RadiationUnavailable { location_id } => {
            location_target(location_id.as_str(), snapshot)
        }
        RejectReason::FacilityAlreadyExists { facility_id }
        | RejectReason::FacilityNotFound { facility_id } => facility_target(facility_id, snapshot),
        RejectReason::InsufficientResource { owner, .. } => owner_to_target(owner, snapshot),
        RejectReason::LocationTransferNotAllowed { from, .. } => {
            location_target(from.as_str(), snapshot)
        }
        RejectReason::ChunkGenerationFailed { x, y, z } => Some(SelectionTarget {
            kind: SelectionKind::Chunk,
            id: format!("{x},{y},{z}"),
            name: None,
        }),
        _ => None,
    }
}

fn owner_to_target(
    owner: &ResourceOwner,
    snapshot: Option<&WorldSnapshot>,
) -> Option<SelectionTarget> {
    match owner {
        ResourceOwner::Agent { agent_id } => Some(SelectionTarget {
            kind: SelectionKind::Agent,
            id: agent_id.clone(),
            name: None,
        }),
        ResourceOwner::Location { location_id } => location_target(location_id, snapshot),
    }
}

fn location_target(location_id: &str, snapshot: Option<&WorldSnapshot>) -> Option<SelectionTarget> {
    let name = snapshot
        .and_then(|snapshot| snapshot.model.locations.get(location_id))
        .map(|location| location.name.clone());
    Some(SelectionTarget {
        kind: SelectionKind::Location,
        id: location_id.to_string(),
        name,
    })
}

fn facility_target(facility_id: &str, snapshot: Option<&WorldSnapshot>) -> Option<SelectionTarget> {
    if let Some(snapshot) = snapshot {
        if snapshot.model.power_plants.contains_key(facility_id) {
            return Some(SelectionTarget {
                kind: SelectionKind::PowerPlant,
                id: facility_id.to_string(),
                name: None,
            });
        }
    }
    None
}

pub(super) fn target_entity(scene: &Viewer3dScene, target: &SelectionTarget) -> Option<Entity> {
    match target.kind {
        SelectionKind::Agent => scene.agent_entities.get(target.id.as_str()).copied(),
        SelectionKind::Location => scene.location_entities.get(target.id.as_str()).copied(),
        SelectionKind::Fragment => None,
        SelectionKind::Asset => scene
            .asset_entities
            .get(target.id.as_str())
            .copied()
            .or_else(|| {
                scene
                    .module_visual_entities
                    .get(target.id.as_str())
                    .copied()
            }),
        SelectionKind::PowerPlant => scene.power_plant_entities.get(target.id.as_str()).copied(),
        SelectionKind::Chunk => scene.chunk_entities.get(target.id.as_str()).copied(),
    }
}

pub(super) fn selection_kind_label(kind: SelectionKind) -> &'static str {
    match kind {
        SelectionKind::Agent => "agent",
        SelectionKind::Location => "location",
        SelectionKind::Fragment => "fragment",
        SelectionKind::Asset => "asset",
        SelectionKind::PowerPlant => "power_plant",
        SelectionKind::Chunk => "chunk",
    }
}

fn chunk_id(coord: ChunkCoord) -> String {
    format!("{},{},{}", coord.x, coord.y, coord.z)
}

#[derive(Clone, Copy)]
struct ChunkCoordId {
    coord: ChunkCoord,
}

impl std::str::FromStr for ChunkCoordId {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(',');
        let x = parts.next().and_then(|part| part.parse::<i32>().ok());
        let y = parts.next().and_then(|part| part.parse::<i32>().ok());
        let z = parts.next().and_then(|part| part.parse::<i32>().ok());
        if x.is_none() || y.is_none() || z.is_none() || parts.next().is_some() {
            return Err(());
        }
        Ok(Self {
            coord: ChunkCoord {
                x: x.ok_or(())?,
                y: y.ok_or(())?,
                z: z.ok_or(())?,
            },
        })
    }
}

#[cfg(test)]
mod tests;
