//! Tests for the simulator module.

use super::*;
use crate::geometry::GeoPos;
use crate::models::DEFAULT_AGENT_HEIGHT_CM;
use std::fs;

fn pos(x: i64, y: i64) -> GeoPos {
    GeoPos {
        x_cm: x,
        y_cm: y,
        z_cm: 0,
    }
}

fn seed_owner_resource(
    kernel: &mut WorldKernel,
    owner: ResourceOwner,
    kind: ResourceKind,
    amount: i64,
) {
    let mut snapshot = kernel.snapshot();
    match owner {
        ResourceOwner::Agent { agent_id } => {
            let stock = snapshot
                .model
                .agents
                .get_mut(&agent_id)
                .expect("agent exists in snapshot");
            stock
                .resources
                .add(kind, amount)
                .expect("seed agent resource");
        }
        ResourceOwner::Location { location_id } => {
            let stock = snapshot
                .model
                .locations
                .get_mut(&location_id)
                .expect("location exists in snapshot");
            stock
                .resources
                .add(kind, amount)
                .expect("seed location resource");
        }
    }
    let journal = kernel.journal_snapshot();
    *kernel =
        WorldKernel::from_snapshot(snapshot, journal).expect("rebuild kernel from seeded snapshot");
}

mod asteroid_fragment;
mod basics;
mod boundary_extremes;
mod chunking;
mod conservation;
mod consistency;
mod decision_provider;
mod fragment_physics;
mod init;
mod init_agent_frag_spawn;
mod init_position_contract;
mod kernel;
mod kernel_rule_decisions;
mod kernel_rule_invariants;
mod kernel_wasm_rule_bridge;
mod kernel_wasm_sandbox_bridge;
mod memory;
mod module_lifecycle;
mod module_visual;
mod monotonicity;
mod native_resolution_contract;
mod persist;
mod physics_parameters;
mod power;
#[cfg(not(target_arch = "wasm32"))]
mod provider_loopback_adapter;
#[cfg(not(target_arch = "wasm32"))]
mod provider_loopback_http;
mod runner;
mod social;
mod social_persist;
mod submitter_access;
