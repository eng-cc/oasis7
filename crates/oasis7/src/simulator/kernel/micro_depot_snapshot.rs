use super::WorldKernel;
use super::micro_depot::MicroDepotPlayerFacilitySnapshot;
use super::micro_depot_validation::measured_micro_depot_inventory_depleted;

impl WorldKernel {
    pub fn micro_depot_player_facility_snapshots(&self) -> Vec<MicroDepotPlayerFacilitySnapshot> {
        let mut snapshots: Vec<_> = self
            .model
            .regional_infrastructure
            .values()
            .filter(|facility| facility.kind == "micro_depot")
            .map(|facility| {
                let mut available_actions = Vec::new();
                if measured_micro_depot_inventory_depleted(facility) {
                    available_actions.push("reclaim_micro_depot".to_string());
                } else if facility.status == "active" && facility.upkeep_paid {
                    available_actions.push("service_micro_depot_repair".to_string());
                    available_actions.push("service_micro_depot_logistics".to_string());
                    available_actions.push("suspend_micro_depot".to_string());
                } else if facility.status == "suspended" || !facility.upkeep_paid {
                    available_actions.push("pay_micro_depot_upkeep".to_string());
                    available_actions.push("reclaim_micro_depot".to_string());
                }
                MicroDepotPlayerFacilitySnapshot {
                    facility_id: facility.facility_id.clone(),
                    status: facility.status.clone(),
                    location_id: facility.location_id.clone(),
                    service_radius_cm: facility.service_radius_cm,
                    supported_resource_kinds: facility.supported_resource_kinds.clone(),
                    module_id: facility.module_id.clone(),
                    module_version: facility.module_version.clone(),
                    wasm_hash: facility.wasm_hash.clone(),
                    upkeep_paid: facility.upkeep_paid,
                    last_receipt_id: facility.last_receipt_id.clone(),
                    last_proposal_hash: facility.last_proposal_hash.clone(),
                    available_actions,
                }
            })
            .collect();
        snapshots.sort_by(|left, right| left.facility_id.cmp(&right.facility_id));
        snapshots
    }
}
