use super::*;

#[derive(Component)]
pub(crate) struct SpatialOverlayVisual {
    pub kind: &'static str,
    pub id: String,
    pub part: u8,
}

pub(super) fn clear(commands: &mut Commands, existing: &Query<(Entity, &SpatialOverlayVisual)>) {
    for (entity, _) in existing.iter() {
        commands.entity(entity).despawn();
    }
}

pub(super) fn reconcile(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    existing: &Query<(Entity, &SpatialOverlayVisual)>,
    width: f64,
    height: f64,
    rebuild_hits: bool,
) {
    let Some(state) = runtime.render_state.as_ref() else {
        clear(commands, existing);
        return;
    };
    let Some(bounds) = state.world_bounds.as_ref() else {
        clear(commands, existing);
        return;
    };
    let old: HashMap<_, _> = existing
        .iter()
        .map(|(entity, visual)| ((visual.kind, visual.id.clone(), visual.part), entity))
        .collect();
    let mut active = HashSet::new();
    for (kind, visuals) in [
        ("industry_aggregate", &state.industry_aggregates),
        ("world_event", &state.world_events),
    ] {
        for visual in visuals {
            let Some((x, y)) = to_canvas_point(&visual.pos, bounds, width, height, &runtime.camera)
            else {
                continue;
            };
            if x < -20.0 || y < -20.0 || x > width + 20.0 || y > height + 20.0 {
                continue;
            }
            let aggregate = kind == "industry_aggregate";
            let half = if aggregate { 11.0 } else { 7.0 };
            if rebuild_hits {
                runtime.hit_regions.push(HitRegion {
                    kind,
                    id: visual.id.clone(),
                    left: x - half,
                    top: y - half,
                    right: x + half,
                    bottom: y + half,
                });
            }
            let color = if aggregate {
                Color::srgb_u8(34, 211, 238)
            } else if visual.historical {
                Color::srgb_u8(148, 163, 184)
            } else {
                Color::srgb_u8(251, 191, 36)
            };
            for part in 0..3 {
                let key = (kind, visual.id.clone(), part);
                active.insert(key.clone());
                let marker = SpatialOverlayVisual {
                    kind,
                    id: visual.id.clone(),
                    part,
                };
                let transform = Transform::from_translation(to_bevy_translation(
                    x,
                    y,
                    width,
                    height,
                    2.85 + f32::from(part) * 0.01,
                ));
                let entity = old
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| commands.spawn_empty().id());
                if part == 2 {
                    let name = visual.label.chars().take(24).collect::<String>();
                    let label = visual
                        .member_count
                        .as_ref()
                        .map_or(name.clone(), |count| format!("{name} · {count}"));
                    let mut text_transform = transform;
                    text_transform.translation.y -= 18.0;
                    text_transform.translation.z = 5.05;
                    commands.entity(entity).insert((
                        marker,
                        Text2d::new(label),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(color),
                        text_transform,
                    ));
                } else {
                    let mut t = transform;
                    let sprite = if part == 0 {
                        if !aggregate {
                            t.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
                        }
                        sprite_for_rect(
                            color,
                            if aggregate { 20.0 } else { 10.0 },
                            if aggregate { 14.0 } else { 10.0 },
                        )
                    } else {
                        // Aggregate bars and historical event dashes are shape cues,
                        // independent of marker color and unrelated to business state.
                        sprite_for_rect(
                            Color::srgb_u8(15, 23, 42),
                            if aggregate {
                                12.0
                            } else if visual.historical {
                                7.0
                            } else {
                                2.0
                            },
                            if aggregate {
                                3.0
                            } else if visual.historical {
                                2.0
                            } else {
                                7.0
                            },
                        )
                    };
                    commands.entity(entity).insert((marker, sprite, t));
                }
            }
        }
    }
    for (key, entity) in old {
        if !active.contains(&key) {
            commands.entity(entity).despawn();
        }
    }
}
