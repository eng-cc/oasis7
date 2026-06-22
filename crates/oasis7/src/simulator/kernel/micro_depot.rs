use oasis7_wasm_abi::{
    ModuleCallInput, ModuleCallOrigin, ModuleCallRequest, ModuleContext, ModuleLimits,
    ModuleOutput, ModuleSandbox,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use crate::geometry::space_distance_cm;

use super::super::types::{ActionId, AgentId, FacilityId, LocationId, ResourceKind, ResourceOwner};
use super::super::world_model::RegionalInfrastructure;
use super::types::{MicroDepotResourceDebit, RejectReason, WorldEventKind};
use super::{WorldKernel, to_canonical_cbor};

pub const MICRO_DEPOT_PROPOSAL_EMIT_KIND: &str = "micro_depot.proposal";
pub const MICRO_DEPOT_DEBIT_AMOUNT_NONE: i64 = 0;
pub const MICRO_DEPOT_DEBIT_AMOUNT_LOW: i64 = 1;
pub const MICRO_DEPOT_DEBIT_AMOUNT_MEDIUM: i64 = 3;
pub const MICRO_DEPOT_DEBIT_AMOUNT_HIGH: i64 = 5;
pub const MICRO_DEPOT_DEBIT_AMOUNT_CRITICAL: i64 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotEvalInput {
    pub schema_version: String,
    pub module_id: String,
    pub module_version: String,
    pub action: MicroDepotActionContext,
    pub player: MicroDepotPlayerContext,
    pub depot: MicroDepotFacilityContext,
    pub context: MicroDepotRegionContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotActionContext {
    pub action_id: ActionId,
    pub kind: MicroDepotActionKind,
    pub target_id: String,
    pub base_cost_class: MicroDepotPressureClass,
    pub base_risk_class: MicroDepotPressureClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroDepotActionKind {
    Repair,
    Logistics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroDepotPressureClass {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotPlayerContext {
    pub account_id: String,
    pub claim_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotFacilityContext {
    pub facility_id: String,
    pub status: MicroDepotStatus,
    pub owner_claim_id: String,
    pub capacity_class: MicroDepotPressureClass,
    #[serde(default)]
    pub supported_resource_kinds: Vec<String>,
    pub service_radius_cm: i64,
    pub upkeep_paid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroDepotStatus {
    Active,
    Degraded,
    UpkeepGrace,
    Suspended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotRegionContext {
    pub distance_to_target_cm: i64,
    pub resource_gap_class: MicroDepotPressureClass,
    pub route_pressure_class: MicroDepotPressureClass,
    pub region_pressure_class: MicroDepotPressureClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotProposal {
    pub action_id: ActionId,
    pub facility_id: String,
    pub decision: MicroDepotDecision,
    pub cost_delta_class: MicroDepotDeltaClass,
    pub risk_delta_class: MicroDepotDeltaClass,
    pub wait_delta_class: MicroDepotDeltaClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_change: Option<String>,
    #[serde(default)]
    pub consumed_resource_classes: Vec<MicroDepotConsumedResourceClass>,
    pub explanation_code: String,
    pub proposal_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroDepotDecision {
    Applicable,
    NotApplicable,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroDepotDeltaClass {
    MajorDecrease,
    MinorDecrease,
    None,
    MinorIncrease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotConsumedResourceClass {
    pub resource_kind: String,
    pub amount_class: MicroDepotPressureClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotQuotePreview {
    pub module_id: String,
    pub wasm_hash: String,
    pub entrypoint: String,
    pub proposal: MicroDepotProposal,
    pub effect: MicroDepotEffectPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotEffectPreview {
    pub cost_delta_class: MicroDepotDeltaClass,
    pub risk_delta_class: MicroDepotDeltaClass,
    pub wait_delta_class: MicroDepotDeltaClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_change: Option<String>,
    #[serde(default)]
    pub consumed_resource_classes: Vec<MicroDepotConsumedResourceClass>,
    pub explanation_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotPlayerFacilitySnapshot {
    pub facility_id: FacilityId,
    pub status: String,
    pub location_id: LocationId,
    pub service_radius_cm: i64,
    #[serde(default)]
    pub supported_resource_kinds: Vec<String>,
    pub module_id: String,
    pub module_version: String,
    pub wasm_hash: String,
    pub upkeep_paid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_proposal_hash: Option<String>,
    #[serde(default)]
    pub available_actions: Vec<String>,
}

#[derive(Serialize)]
struct MicroDepotProposalHashPayload<'a> {
    input: &'a MicroDepotEvalInput,
    proposal: MicroDepotProposal,
}

pub fn compute_micro_depot_proposal_hash(
    input: &MicroDepotEvalInput,
    proposal: &MicroDepotProposal,
) -> Result<String, String> {
    let mut canonical_proposal = proposal.clone();
    canonical_proposal.proposal_hash.clear();
    let payload = MicroDepotProposalHashPayload {
        input,
        proposal: canonical_proposal,
    };
    let bytes = to_canonical_cbor(&payload)?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

pub fn evaluate_micro_depot_quote_with_module<S>(
    input: &MicroDepotEvalInput,
    module_id: impl Into<String>,
    wasm_hash: impl Into<String>,
    entrypoint: impl Into<String>,
    wasm_bytes: Vec<u8>,
    limits: ModuleLimits,
    sandbox: Arc<Mutex<S>>,
) -> Result<MicroDepotQuotePreview, String>
where
    S: ModuleSandbox + Send + 'static,
{
    validate_micro_depot_input(input)?;

    let module_id = module_id.into();
    let wasm_hash = wasm_hash.into();
    let entrypoint = entrypoint.into();
    if module_id.trim().is_empty() {
        return Err("micro_depot module_id is empty".to_string());
    }
    if wasm_hash.trim().is_empty() {
        return Err("micro_depot wasm_hash is empty".to_string());
    }
    if entrypoint.trim().is_empty() {
        return Err("micro_depot entrypoint is empty".to_string());
    }
    if wasm_bytes.is_empty() {
        return Err(format!(
            "micro_depot wasm bytes are empty for hash {wasm_hash}"
        ));
    }

    let request = build_micro_depot_wasm_call_request(
        input,
        &module_id,
        &wasm_hash,
        &entrypoint,
        wasm_bytes,
        limits,
    )?;
    let output = {
        let mut locked = sandbox
            .lock()
            .map_err(|_| "micro_depot wasm sandbox mutex poisoned".to_string())?;
        locked.call(&request).map_err(|failure| {
            format!(
                "micro_depot module call failed {:?}: {}",
                failure.code, failure.detail
            )
        })?
    };
    let proposal = parse_micro_depot_proposal(input, &output)?;
    let effect = MicroDepotEffectPreview {
        cost_delta_class: proposal.cost_delta_class,
        risk_delta_class: proposal.risk_delta_class,
        wait_delta_class: proposal.wait_delta_class,
        blocker_change: proposal.blocker_change.clone(),
        consumed_resource_classes: proposal.consumed_resource_classes.clone(),
        explanation_code: proposal.explanation_code.clone(),
    };

    Ok(MicroDepotQuotePreview {
        module_id,
        wasm_hash,
        entrypoint,
        proposal,
        effect,
    })
}

impl WorldKernel {
    pub fn set_micro_depot_wasm_module_evaluator<S>(
        &mut self,
        module_id: impl Into<String>,
        wasm_hash: impl Into<String>,
        entrypoint: impl Into<String>,
        wasm_bytes: Vec<u8>,
        limits: ModuleLimits,
        sandbox: Arc<Mutex<S>>,
    ) where
        S: ModuleSandbox + Send + 'static,
    {
        let module_id = module_id.into();
        let wasm_hash = wasm_hash.into();
        let entrypoint = entrypoint.into();
        let wasm_bytes = Arc::<[u8]>::from(wasm_bytes);
        self.rule_hooks.micro_depot_wasm = Some(Arc::new(move |input| {
            evaluate_micro_depot_quote_with_module(
                input,
                module_id.clone(),
                wasm_hash.clone(),
                entrypoint.clone(),
                wasm_bytes.to_vec(),
                limits.clone(),
                Arc::clone(&sandbox),
            )
        }));
    }

    pub fn register_micro_depot_wasm_artifact(
        &mut self,
        wasm_hash: impl Into<String>,
        wasm_bytes: Vec<u8>,
    ) -> Result<(), String> {
        let wasm_hash = wasm_hash.into();
        if wasm_hash.trim().is_empty() {
            return Err("micro_depot wasm hash is empty".to_string());
        }
        if wasm_bytes.is_empty() {
            return Err(format!(
                "micro_depot wasm bytes are empty for hash {wasm_hash}"
            ));
        }
        if let Some(existing) = self.rule_hooks.micro_depot_wasm_artifacts.get(&wasm_hash) {
            if existing != &wasm_bytes {
                return Err(format!(
                    "micro_depot artifact hash {wasm_hash} already registered with different bytes"
                ));
            }
            return Ok(());
        }
        self.rule_hooks
            .micro_depot_wasm_artifacts
            .insert(wasm_hash, wasm_bytes);
        Ok(())
    }

    pub fn set_micro_depot_wasm_module_from_registry<S>(
        &mut self,
        module_id: impl Into<String>,
        wasm_hash: impl Into<String>,
        entrypoint: impl Into<String>,
        limits: ModuleLimits,
        sandbox: Arc<Mutex<S>>,
    ) -> Result<(), String>
    where
        S: ModuleSandbox + Send + 'static,
    {
        let wasm_hash = wasm_hash.into();
        let wasm_bytes = self
            .rule_hooks
            .micro_depot_wasm_artifacts
            .get(&wasm_hash)
            .cloned()
            .ok_or_else(|| format!("micro_depot wasm artifact missing for hash {wasm_hash}"))?;
        self.set_micro_depot_wasm_module_evaluator(
            module_id, wasm_hash, entrypoint, wasm_bytes, limits, sandbox,
        );
        Ok(())
    }

    pub fn micro_depot_player_facility_snapshots(&self) -> Vec<MicroDepotPlayerFacilitySnapshot> {
        let mut snapshots: Vec<_> = self
            .model
            .regional_infrastructure
            .values()
            .filter(|facility| facility.kind == "micro_depot")
            .map(|facility| {
                let mut available_actions = Vec::new();
                if facility.status == "active" && facility.upkeep_paid {
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

    pub(super) fn apply_install_micro_depot(
        &mut self,
        installer_agent_id: AgentId,
        facility_id: FacilityId,
        location_id: LocationId,
        owner_claim_id: String,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        entrypoint: String,
        service_radius_cm: i64,
        supported_resource_kinds: Vec<String>,
    ) -> WorldEventKind {
        if self
            .model
            .regional_infrastructure
            .contains_key(&facility_id)
        {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::FacilityAlreadyExists { facility_id },
            };
        }
        if !self.model.agents.contains_key(&installer_agent_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound {
                    agent_id: installer_agent_id,
                },
            };
        }
        if !self.model.locations.contains_key(&location_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::LocationNotFound { location_id },
            };
        }
        if owner_claim_id.trim().is_empty()
            || module_id.trim().is_empty()
            || module_version.trim().is_empty()
            || wasm_hash.trim().is_empty()
            || entrypoint.trim().is_empty()
            || service_radius_cm <= 0
        {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "install_micro_depot requires owner claim, module identity, entrypoint, and positive radius"
                            .to_string(),
                    ],
                },
            };
        }
        let owner = ResourceOwner::Agent {
            agent_id: installer_agent_id.clone(),
        };
        let facility = RegionalInfrastructure {
            facility_id: facility_id.clone(),
            kind: "micro_depot".to_string(),
            location_id: location_id.clone(),
            owner: owner.clone(),
            owner_claim_id: owner_claim_id.clone(),
            status: "active".to_string(),
            module_id: module_id.clone(),
            module_version: module_version.clone(),
            wasm_hash: wasm_hash.clone(),
            entrypoint: entrypoint.clone(),
            service_radius_cm,
            supported_resource_kinds: supported_resource_kinds.clone(),
            upkeep_paid: true,
            last_proposal_hash: None,
            last_receipt_id: None,
            last_serviced_target_id: None,
        };
        self.model
            .regional_infrastructure
            .insert(facility_id.clone(), facility);
        WorldEventKind::MicroDepotInstalled {
            facility_id,
            installer_agent_id,
            location_id,
            owner,
            owner_claim_id,
            module_id,
            module_version,
            wasm_hash,
            entrypoint,
            service_radius_cm,
            supported_resource_kinds,
        }
    }

    pub(super) fn apply_service_micro_depot_repair(
        &mut self,
        action_id: ActionId,
        agent_id: AgentId,
        facility_id: FacilityId,
        target_id: String,
        base_cost_class: MicroDepotPressureClass,
        base_risk_class: MicroDepotPressureClass,
        blocker_type: Option<String>,
    ) -> WorldEventKind {
        self.apply_service_micro_depot(
            action_id,
            agent_id,
            facility_id,
            target_id,
            MicroDepotActionKind::Repair,
            base_cost_class,
            base_risk_class,
            blocker_type,
        )
    }

    pub(super) fn apply_service_micro_depot_logistics(
        &mut self,
        action_id: ActionId,
        agent_id: AgentId,
        facility_id: FacilityId,
        target_id: String,
        base_cost_class: MicroDepotPressureClass,
        base_risk_class: MicroDepotPressureClass,
        blocker_type: Option<String>,
    ) -> WorldEventKind {
        self.apply_service_micro_depot(
            action_id,
            agent_id,
            facility_id,
            target_id,
            MicroDepotActionKind::Logistics,
            base_cost_class,
            base_risk_class,
            blocker_type,
        )
    }

    fn apply_service_micro_depot(
        &mut self,
        action_id: ActionId,
        agent_id: AgentId,
        facility_id: FacilityId,
        target_id: String,
        action_kind: MicroDepotActionKind,
        base_cost_class: MicroDepotPressureClass,
        base_risk_class: MicroDepotPressureClass,
        blocker_type: Option<String>,
    ) -> WorldEventKind {
        let Some(evaluator) = self.rule_hooks.micro_depot_wasm.clone() else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec!["micro_depot wasm evaluator is not configured".to_string()],
                },
            };
        };
        if !self.model.agents.contains_key(&agent_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound { agent_id },
            };
        }
        let Some(facility) = self
            .model
            .regional_infrastructure
            .get(&facility_id)
            .cloned()
        else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::FacilityNotFound { facility_id },
            };
        };
        if facility.kind != "micro_depot" {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("facility {} is not a micro_depot", facility_id)],
                },
            };
        }
        if facility.status != "active" || !facility.upkeep_paid {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("micro_depot {} is not active", facility_id)],
                },
            };
        }
        let Some(distance_to_target_cm) =
            self.micro_depot_distance_to_target(&facility, &target_id)
        else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("micro_depot target not found: {target_id}")],
                },
            };
        };
        let input = MicroDepotEvalInput {
            schema_version: "micro_depot.eval.v1".to_string(),
            module_id: facility.module_id.clone(),
            module_version: facility.module_version.clone(),
            action: MicroDepotActionContext {
                action_id,
                kind: action_kind,
                target_id: target_id.clone(),
                base_cost_class,
                base_risk_class,
                blocker_type,
            },
            player: MicroDepotPlayerContext {
                account_id: self
                    .player_binding_for_agent(&agent_id)
                    .unwrap_or(agent_id.as_str())
                    .to_string(),
                claim_id: facility.owner_claim_id.clone(),
            },
            depot: MicroDepotFacilityContext {
                facility_id: facility.facility_id.clone(),
                status: MicroDepotStatus::Active,
                owner_claim_id: facility.owner_claim_id.clone(),
                capacity_class: MicroDepotPressureClass::Medium,
                supported_resource_kinds: facility.supported_resource_kinds.clone(),
                service_radius_cm: facility.service_radius_cm,
                upkeep_paid: facility.upkeep_paid,
            },
            context: MicroDepotRegionContext {
                distance_to_target_cm,
                resource_gap_class: base_cost_class,
                route_pressure_class: MicroDepotPressureClass::Medium,
                region_pressure_class: MicroDepotPressureClass::Medium,
            },
        };
        let preview = match evaluator(&input) {
            Ok(preview) => preview,
            Err(err) => {
                return WorldEventKind::ActionRejected {
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("micro_depot wasm quote failed: {err}")],
                    },
                };
            }
        };
        if preview.proposal.decision != MicroDepotDecision::Applicable {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "micro_depot proposal was not applicable: {:?}",
                        preview.proposal.decision
                    )],
                },
            };
        }
        let consumed_resources = match micro_depot_resource_debits(&preview.proposal) {
            Ok(debits) => debits,
            Err(note) => {
                return WorldEventKind::ActionRejected {
                    reason: RejectReason::RuleDenied { notes: vec![note] },
                };
            }
        };
        let owner = facility.owner.clone();
        for debit in &consumed_resources {
            if let Err(reason) = self.remove_from_owner(&owner, debit.kind, debit.amount) {
                return WorldEventKind::ActionRejected { reason };
            }
        }

        let receipt_id = format!("micro-depot:{facility_id}:{action_id}");
        if let Some(stored) = self.model.regional_infrastructure.get_mut(&facility_id) {
            stored.last_proposal_hash = Some(preview.proposal.proposal_hash.clone());
            stored.last_receipt_id = Some(receipt_id.clone());
            stored.last_serviced_target_id = Some(target_id.clone());
        }
        WorldEventKind::MicroDepotServiceApplied {
            facility_id,
            action_id,
            agent_id,
            target_id,
            action_kind: micro_depot_action_kind_label(action_kind).to_string(),
            proposal_hash: preview.proposal.proposal_hash,
            receipt_id,
            cost_delta_class: micro_depot_delta_class_label(preview.effect.cost_delta_class)
                .to_string(),
            risk_delta_class: micro_depot_delta_class_label(preview.effect.risk_delta_class)
                .to_string(),
            wait_delta_class: micro_depot_delta_class_label(preview.effect.wait_delta_class)
                .to_string(),
            blocker_change: preview.effect.blocker_change,
            consumed_resources,
            explanation_code: preview.effect.explanation_code,
        }
    }

    pub(super) fn apply_pay_micro_depot_upkeep(
        &mut self,
        agent_id: AgentId,
        facility_id: FacilityId,
    ) -> WorldEventKind {
        match self.ensure_micro_depot_owner(&facility_id, &agent_id) {
            Ok(()) => {}
            Err(reason) => return WorldEventKind::ActionRejected { reason },
        }
        let Some(facility) = self.model.regional_infrastructure.get_mut(&facility_id) else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::FacilityNotFound { facility_id },
            };
        };
        facility.status = "active".to_string();
        facility.upkeep_paid = true;
        let receipt_id = format!("micro-depot-upkeep:{facility_id}:{}", self.time);
        facility.last_receipt_id = Some(receipt_id.clone());
        WorldEventKind::MicroDepotUpkeepPaid {
            facility_id,
            agent_id,
            receipt_id,
        }
    }

    pub(super) fn apply_suspend_micro_depot(
        &mut self,
        agent_id: AgentId,
        facility_id: FacilityId,
        reason: String,
    ) -> WorldEventKind {
        match self.ensure_micro_depot_owner(&facility_id, &agent_id) {
            Ok(()) => {}
            Err(reason) => return WorldEventKind::ActionRejected { reason },
        }
        let Some(facility) = self.model.regional_infrastructure.get_mut(&facility_id) else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::FacilityNotFound { facility_id },
            };
        };
        facility.status = "suspended".to_string();
        facility.upkeep_paid = false;
        WorldEventKind::MicroDepotSuspended {
            facility_id,
            agent_id,
            reason,
        }
    }

    pub(super) fn apply_reclaim_micro_depot(
        &mut self,
        agent_id: AgentId,
        facility_id: FacilityId,
    ) -> WorldEventKind {
        match self.ensure_micro_depot_owner(&facility_id, &agent_id) {
            Ok(()) => {}
            Err(reason) => return WorldEventKind::ActionRejected { reason },
        }
        self.model.regional_infrastructure.remove(&facility_id);
        WorldEventKind::MicroDepotReclaimed {
            facility_id,
            agent_id,
        }
    }

    fn ensure_micro_depot_owner(
        &self,
        facility_id: &str,
        agent_id: &str,
    ) -> Result<(), RejectReason> {
        let Some(facility) = self.model.regional_infrastructure.get(facility_id) else {
            return Err(RejectReason::FacilityNotFound {
                facility_id: facility_id.to_string(),
            });
        };
        match &facility.owner {
            ResourceOwner::Agent {
                agent_id: owner_agent_id,
            } if owner_agent_id == agent_id => Ok(()),
            _ => Err(RejectReason::RuleDenied {
                notes: vec![format!(
                    "agent {agent_id} does not own micro_depot {facility_id}"
                )],
            }),
        }
    }

    fn micro_depot_distance_to_target(
        &self,
        facility: &RegionalInfrastructure,
        target_id: &str,
    ) -> Option<i64> {
        let facility_pos = self.model.locations.get(&facility.location_id)?.pos;
        let target_pos = if let Some(location) = self.model.locations.get(target_id) {
            location.pos
        } else if let Some(factory) = self.model.factories.get(target_id) {
            self.model.locations.get(&factory.location_id)?.pos
        } else {
            return None;
        };
        Some(space_distance_cm(facility_pos, target_pos))
    }
}

fn micro_depot_resource_debits(
    proposal: &MicroDepotProposal,
) -> Result<Vec<MicroDepotResourceDebit>, String> {
    let mut debits = Vec::new();
    for consumed in &proposal.consumed_resource_classes {
        let kind = micro_depot_resource_kind(&consumed.resource_kind)?;
        let amount = micro_depot_amount_for_pressure(consumed.amount_class);
        if amount > 0 {
            debits.push(MicroDepotResourceDebit { kind, amount });
        }
    }
    Ok(debits)
}

fn micro_depot_resource_kind(value: &str) -> Result<ResourceKind, String> {
    match value.trim() {
        "data" => Ok(ResourceKind::Data),
        "electricity" => Ok(ResourceKind::Electricity),
        other => Err(format!("unsupported micro_depot resource kind: {other}")),
    }
}

fn micro_depot_amount_for_pressure(class: MicroDepotPressureClass) -> i64 {
    match class {
        MicroDepotPressureClass::None => MICRO_DEPOT_DEBIT_AMOUNT_NONE,
        MicroDepotPressureClass::Low => MICRO_DEPOT_DEBIT_AMOUNT_LOW,
        MicroDepotPressureClass::Medium => MICRO_DEPOT_DEBIT_AMOUNT_MEDIUM,
        MicroDepotPressureClass::High => MICRO_DEPOT_DEBIT_AMOUNT_HIGH,
        MicroDepotPressureClass::Critical => MICRO_DEPOT_DEBIT_AMOUNT_CRITICAL,
    }
}

fn micro_depot_action_kind_label(kind: MicroDepotActionKind) -> &'static str {
    match kind {
        MicroDepotActionKind::Repair => "repair",
        MicroDepotActionKind::Logistics => "logistics",
    }
}

fn micro_depot_delta_class_label(class: MicroDepotDeltaClass) -> &'static str {
    match class {
        MicroDepotDeltaClass::MajorDecrease => "major_decrease",
        MicroDepotDeltaClass::MinorDecrease => "minor_decrease",
        MicroDepotDeltaClass::None => "none",
        MicroDepotDeltaClass::MinorIncrease => "minor_increase",
    }
}

fn build_micro_depot_wasm_call_request(
    input: &MicroDepotEvalInput,
    module_id: &str,
    wasm_hash: &str,
    entrypoint: &str,
    wasm_bytes: Vec<u8>,
    limits: ModuleLimits,
) -> Result<ModuleCallRequest, String> {
    let action_bytes = to_canonical_cbor(input)?;
    let trace_id = format!(
        "micro-depot-quote-{}-{}",
        input.depot.facility_id, input.action.action_id
    );
    let call_input = ModuleCallInput {
        ctx: ModuleContext {
            v: "wasm-1".to_string(),
            module_id: module_id.to_string(),
            trace_id: trace_id.clone(),
            time: 0,
            origin: ModuleCallOrigin {
                kind: "micro_depot_quote".to_string(),
                id: input.action.action_id.to_string(),
            },
            limits: limits.clone(),
            stage: Some("micro_depot_evaluate_quote".to_string()),
            world_config_hash: None,
            manifest_hash: None,
            journal_height: None,
            module_version: Some(input.module_version.clone()),
            module_kind: Some("pure".to_string()),
            module_role: Some("gameplay".to_string()),
        },
        event: None,
        action: Some(action_bytes),
        state: None,
    };
    let input_bytes = to_canonical_cbor(&call_input)?;

    Ok(ModuleCallRequest {
        module_id: module_id.to_string(),
        wasm_hash: wasm_hash.to_string(),
        trace_id,
        entrypoint: entrypoint.to_string(),
        input: input_bytes,
        limits,
        wasm_bytes: wasm_bytes.into(),
    })
}

fn parse_micro_depot_proposal(
    input: &MicroDepotEvalInput,
    output: &ModuleOutput,
) -> Result<MicroDepotProposal, String> {
    let mut proposal = None;
    for emit in &output.emits {
        if emit.kind != MICRO_DEPOT_PROPOSAL_EMIT_KIND {
            continue;
        }
        if proposal.is_some() {
            return Err("multiple micro_depot.proposal emits in wasm module output".to_string());
        }
        let parsed: MicroDepotProposal = serde_json::from_value(emit.payload.clone())
            .map_err(|err| format!("failed to decode micro_depot.proposal payload: {err}"))?;
        proposal = Some(parsed);
    }

    let proposal = proposal
        .ok_or_else(|| "missing micro_depot.proposal emit in wasm module output".to_string())?;
    validate_micro_depot_proposal(input, &proposal)?;
    Ok(proposal)
}

fn validate_micro_depot_input(input: &MicroDepotEvalInput) -> Result<(), String> {
    if input.schema_version != "micro_depot.eval.v1" {
        return Err(format!(
            "unsupported micro_depot schema_version {}",
            input.schema_version
        ));
    }
    if input.module_id.trim().is_empty() {
        return Err("micro_depot input module_id is empty".to_string());
    }
    if input.module_version.trim().is_empty() {
        return Err("micro_depot input module_version is empty".to_string());
    }
    if input.action.target_id.trim().is_empty() {
        return Err("micro_depot action target_id is empty".to_string());
    }
    if input.player.account_id.trim().is_empty() || input.player.claim_id.trim().is_empty() {
        return Err("micro_depot player identity is incomplete".to_string());
    }
    if input.depot.facility_id.trim().is_empty() {
        return Err("micro_depot facility_id is empty".to_string());
    }
    if input.depot.owner_claim_id != input.player.claim_id {
        return Err("micro_depot owner_claim_id must match player claim_id".to_string());
    }
    if input.depot.status != MicroDepotStatus::Active {
        return Err("micro_depot facility must be active for quote evaluation".to_string());
    }
    if !input.depot.upkeep_paid {
        return Err("micro_depot upkeep must be paid for quote evaluation".to_string());
    }
    if input.depot.service_radius_cm <= 0 {
        return Err("micro_depot service_radius_cm must be positive".to_string());
    }
    if input.context.distance_to_target_cm < 0 {
        return Err("micro_depot distance_to_target_cm must be non-negative".to_string());
    }
    if input.context.distance_to_target_cm > input.depot.service_radius_cm {
        return Err("micro_depot target is outside service radius".to_string());
    }
    Ok(())
}

fn validate_micro_depot_proposal(
    input: &MicroDepotEvalInput,
    proposal: &MicroDepotProposal,
) -> Result<(), String> {
    if proposal.action_id != input.action.action_id {
        return Err(format!(
            "micro_depot proposal action_id mismatch expected {} got {}",
            input.action.action_id, proposal.action_id
        ));
    }
    if proposal.facility_id != input.depot.facility_id {
        return Err(format!(
            "micro_depot proposal facility_id mismatch expected {} got {}",
            input.depot.facility_id, proposal.facility_id
        ));
    }
    if proposal.explanation_code.trim().is_empty() {
        return Err("micro_depot proposal explanation_code is empty".to_string());
    }
    if proposal.proposal_hash.trim().is_empty() {
        return Err("micro_depot proposal_hash is empty".to_string());
    }
    let expected_hash = compute_micro_depot_proposal_hash(input, proposal)?;
    if proposal.proposal_hash != expected_hash {
        return Err(format!(
            "micro_depot proposal_hash mismatch expected {expected_hash} got {}",
            proposal.proposal_hash
        ));
    }
    Ok(())
}
