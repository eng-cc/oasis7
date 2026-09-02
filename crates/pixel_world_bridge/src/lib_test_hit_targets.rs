use super::*;

pub(super) fn hotspot_test_hit_targets(hit_regions: &[HitRegion]) -> Vec<HotspotTestHitTarget> {
    hit_regions
        .iter()
        .filter(|region| region.kind == "hotspot")
        .map(|region| HotspotTestHitTarget {
            kind: region.kind,
            id: region.id.clone(),
            canvas_x: (region.left + region.right) / 2.0,
            canvas_y: (region.top + region.bottom) / 2.0,
        })
        .collect()
}

pub(super) fn publish_hotspot_test_hit_targets(hit_regions: &[HitRegion]) {
    let targets = hotspot_test_hit_targets(hit_regions);
    BRIDGE_SHARED.with(|shared| shared.borrow_mut().hotspot_test_targets = targets);
}

pub(super) fn publish_location_test_hit_targets(hit_regions: &[HitRegion]) {
    let targets = hit_regions
        .iter()
        .filter(|region| region.kind == "location")
        .map(|region| LocationTestHitTarget {
            id: region.id.clone(),
            canvas_x: (region.left + region.right) / 2.0,
            canvas_y: (region.top + region.bottom) / 2.0,
        })
        .collect();
    BRIDGE_SHARED.with(|shared| shared.borrow_mut().location_test_targets = targets);
}
