use super::*;

#[derive(Component)]
pub(crate) struct PixelWorldLinkVisual {
    pub(crate) id: String,
}

pub(super) fn reconcile_links(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.link_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.link_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for link in &render_state.links {
        let Some((from_x, from_y)) =
            to_canvas_point(&link.from, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let Some((to_x, to_y)) =
            to_canvas_point(&link.to, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(link.id.clone());
        let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2))
            .sqrt()
            .max(4.0);
        let emphasis = clamp(link.emphasis.unwrap_or(0.7), 0.25, 1.0);
        let sprite = sprite_for_rect(
            Color::srgba(0.49, 0.83, 0.98, (0.18 + (emphasis * 0.34)) as f32),
            length as f32,
            (1.4 + (emphasis * 2.2)) as f32,
        );
        let transform = transform_for_line(from_x, from_y, to_x, to_y, width, height, 0.5);

        if let Some(entity) = runtime.link_entities.get(&link.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldLinkVisual {
                        id: link.id.clone(),
                    },
                ))
                .id();
            runtime.link_entities.insert(link.id.clone(), entity);
        }
    }

    despawn_stale_entities(commands, &mut runtime.link_entities, &active_ids);
}
