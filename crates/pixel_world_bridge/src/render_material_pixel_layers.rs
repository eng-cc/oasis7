use super::*;

pub(super) fn collect(world: &mut World, layers: &mut Vec<PixelLayer>) {
    let mut grounding =
        world.query::<(&grounding::PixelWorldGroundingVisual, &Sprite, &Transform)>();
    layers.extend(
        grounding
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("grounding", sprite, transform)),
    );
    let mut cues = world.query::<(
        &hotspot_cues::PixelWorldHotspotCueVisual,
        &Sprite,
        &Transform,
    )>();
    layers.extend(
        cues.iter(world)
            .map(|(_, sprite, transform)| pixel_layer("hotspot_cue", sprite, transform)),
    );
    let mut links = world.query::<(&PixelWorldLinkVisual, &Sprite, &Transform)>();
    layers.extend(
        links
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("link", sprite, transform)),
    );
    let mut facilities = world.query::<(&PixelWorldMicroDepotVisual, &Sprite, &Transform)>();
    layers.extend(
        facilities
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("facility", sprite, transform)),
    );
}
