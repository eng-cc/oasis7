use crate::geometry::{space_distance_cm, GeoPos};
use std::collections::BTreeMap;

use super::super::chunking::CHUNK_SIZE_X_CM;
use super::super::module_visual::ModuleVisualAnchor;
use super::super::power::{PlantStatus, PowerEvent, PowerPlant};
use super::super::types::{
    Action, ElementBudgetError, FragmentElementKind, PowerOrderSide, ResourceKind, ResourceOwner,
    StockError, CM_PER_KM, PPM_BASE,
};
use super::super::world_model::{Agent, Factory, FragmentResourceError, Location, PowerOrderState};
use super::types::{ChunkGenerationCause, PowerOrderFill, RejectReason, WorldEventKind};
use super::WorldKernel;

#[derive(Debug, Clone, Copy)]
struct RecipePlan {
    required_factory_kind: &'static str,
    electricity_per_batch: i64,
    hardware_per_batch: i64,
    data_output_per_batch: i64,
    finished_product_id: &'static str,
    finished_product_units_per_batch: i64,
}

const LOCATION_ELECTRICITY_POOL_REMOVED_NOTE: &str = "location electricity pool removed";
const FACTORY_KIND_SMELTER_MK1: &str = "factory.smelter.mk1";
const FACTORY_KIND_ASSEMBLER_MK1: &str = "factory.assembler.mk1";
const FACTORY_KIND_RADIATION_POWER_MK1: &str = "factory.power.radiation.mk1";

#[derive(Debug, Clone, Copy)]
struct PreparedPowerTransfer {
    loss: i64,
    quoted_price_per_pu: i64,
}

include!("actions_core.rs");
include!("actions_resolution.rs");
include!("actions_regressions.rs");
