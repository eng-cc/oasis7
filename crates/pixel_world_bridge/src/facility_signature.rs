use super::*;

/// Passive visuals invalidate content without changing camera or hit authority.
pub(super) fn hash_visuals(hasher: &mut DefaultHasher, facilities: &[MicroDepotFacility]) {
    facilities.len().hash(hasher);
    for facility in facilities {
        facility.id.hash(hasher);
        hash_position(hasher, &facility.pos);
        facility.status.hash(hasher);
        hash_f64(hasher, facility.service_radius_cm);
        facility.available_units_by_kind.hash(hasher);
        facility.throughput_remaining_units.hash(hasher);
        facility.throughput_limit_units_per_epoch.hash(hasher);
    }
}
