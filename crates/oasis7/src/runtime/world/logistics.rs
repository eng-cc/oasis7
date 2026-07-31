use serde::{Deserialize, Serialize};

use super::super::{
    DomainEvent, MaterialDefaultPriority, MaterialLedgerId, MaterialTransitPriority,
    MaterialTransportLossClass, RejectReason, WorldError, WorldEvent, WorldEventBody, WorldTime,
};
use super::World;

pub(super) const MATERIAL_TRANSFER_MAX_DISTANCE_KM: i64 = 10_000;
pub(super) const MATERIAL_TRANSFER_LOSS_PER_KM_BPS: i64 = 5;
pub(super) const MATERIAL_TRANSFER_SPEED_KM_PER_TICK: i64 = 100;
pub(super) const MATERIAL_TRANSFER_MAX_INFLIGHT: usize = 2;

const MATERIAL_TRANSIT_URGENT_KEYWORDS: &[&str] = &[
    "survival",
    "lifeline",
    "critical",
    "repair",
    "maintenance",
    "oxygen",
    "water",
    "emergency",
];

/// A non-reserving projection of a material transfer at the current world state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogisticsTransferQuote {
    pub requester_agent_id: String,
    pub from_ledger: MaterialLedgerId,
    pub to_ledger: MaterialLedgerId,
    pub kind: String,
    pub requested_amount: i64,
    /// Whether the transfer can be submitted against the currently observed state.
    /// Submission still revalidates before it reserves materials or capacity.
    pub submission_feasible: bool,
    /// The source-ledger amount currently available for this material.
    pub max_transferable_amount: i64,
    pub sent_amount: i64,
    pub distance_km: i64,
    pub loss_bps: i64,
    pub expected_loss_amount: i64,
    pub expected_received_amount: i64,
    pub source_amount_before: i64,
    pub source_amount_after: i64,
    pub destination_amount_before: i64,
    pub destination_expected_amount_after: i64,
    pub ticks_until_arrival: u64,
    pub ready_at: WorldTime,
    pub effective_priority: MaterialTransitPriority,
    pub priority_reason: String,
    pub inflight_before: usize,
    pub inflight_capacity: usize,
    pub recommendation: String,
    /// Quotes do not reserve balance or transit capacity; submission revalidates both.
    pub conditional: bool,
}

pub(super) fn material_transit_priority_for_kind(
    world: &World,
    kind: &str,
) -> MaterialTransitPriority {
    if let Some(profile) = world.material_profile(kind) {
        return match profile.default_priority {
            MaterialDefaultPriority::Urgent => MaterialTransitPriority::Urgent,
            MaterialDefaultPriority::Standard => MaterialTransitPriority::Standard,
        };
    }

    let normalized = kind.to_ascii_lowercase();
    if MATERIAL_TRANSIT_URGENT_KEYWORDS
        .iter()
        .any(|keyword| normalized.contains(keyword))
    {
        MaterialTransitPriority::Urgent
    } else {
        MaterialTransitPriority::Standard
    }
}

pub(super) fn material_transit_loss_bps_for_kind(world: &World, kind: &str) -> i64 {
    let base = MATERIAL_TRANSFER_LOSS_PER_KM_BPS.max(0);
    let factor = world
        .material_profile(kind)
        .map(|profile| match profile.transport_loss_class {
            MaterialTransportLossClass::Low => 1_i64,
            MaterialTransportLossClass::Medium => 2_i64,
            MaterialTransportLossClass::High => 4_i64,
        })
        .unwrap_or(1);
    base.saturating_mul(factor)
}

pub(super) fn material_transit_ticks(distance_km: i64) -> u64 {
    if distance_km == 0 {
        0
    } else {
        ((distance_km + MATERIAL_TRANSFER_SPEED_KM_PER_TICK - 1)
            / MATERIAL_TRANSFER_SPEED_KM_PER_TICK)
            .max(1) as u64
    }
}

pub(super) fn material_transit_loss_amount(amount: i64, distance_km: i64, loss_bps: i64) -> i64 {
    ((amount as i128)
        .saturating_mul(distance_km as i128)
        .saturating_mul(loss_bps as i128)
        / 10_000)
        .clamp(0, amount as i128) as i64
}

impl World {
    pub fn pending_material_transits_len(&self) -> usize {
        self.state.pending_material_transits.len()
    }

    /// Derives the existing transfer priority, loss, and timing rules without
    /// reserving materials or consuming a transit slot.
    pub fn logistics_transfer_quote(
        &self,
        requester_agent_id: &str,
        from_ledger: &MaterialLedgerId,
        to_ledger: &MaterialLedgerId,
        kind: &str,
        amount: i64,
        distance_km: i64,
        requested_priority: Option<MaterialTransitPriority>,
    ) -> Result<LogisticsTransferQuote, RejectReason> {
        if !self.state.agents.contains_key(requester_agent_id) {
            return Err(RejectReason::AgentNotFound {
                agent_id: requester_agent_id.to_string(),
            });
        }
        if from_ledger == to_ledger {
            return Err(RejectReason::RuleDenied {
                notes: vec!["from_ledger and to_ledger cannot be the same".to_string()],
            });
        }
        if kind.trim().is_empty() {
            return Err(RejectReason::RuleDenied {
                notes: vec!["material kind cannot be empty".to_string()],
            });
        }
        if amount <= 0 {
            return Err(RejectReason::InvalidAmount { amount });
        }
        if distance_km < 0 {
            return Err(RejectReason::RuleDenied {
                notes: vec!["distance_km must be >= 0".to_string()],
            });
        }
        if distance_km > MATERIAL_TRANSFER_MAX_DISTANCE_KM {
            return Err(RejectReason::MaterialTransferDistanceExceeded {
                distance_km,
                max_distance_km: MATERIAL_TRANSFER_MAX_DISTANCE_KM,
            });
        }

        let source_amount_before = self.ledger_material_balance(from_ledger, kind);
        let destination_amount_before = self.ledger_material_balance(to_ledger, kind);
        let max_transferable_amount = source_amount_before.max(0);
        let (effective_priority, priority_reason) = match requested_priority {
            Some(priority) => (priority, "explicit_priority".to_string()),
            None => (
                material_transit_priority_for_kind(self, kind),
                "material_default_priority".to_string(),
            ),
        };
        let loss_bps = if distance_km == 0 {
            0
        } else {
            material_transit_loss_bps_for_kind(self, kind)
        };
        let expected_loss_amount = material_transit_loss_amount(amount, distance_km, loss_bps);
        let expected_received_amount = amount.saturating_sub(expected_loss_amount);
        let ticks_until_arrival = material_transit_ticks(distance_km);
        let inflight_before = self.state.pending_material_transits.len();
        let submission_feasible = source_amount_before >= amount
            && (distance_km == 0 || inflight_before < MATERIAL_TRANSFER_MAX_INFLIGHT);
        let (sent_amount, expected_loss_amount, expected_received_amount) = if submission_feasible {
            (amount, expected_loss_amount, expected_received_amount)
        } else {
            (0, 0, 0)
        };
        let recommendation = if source_amount_before < amount {
            "reduce_amount_or_source_materials"
        } else if distance_km > 0 && inflight_before >= MATERIAL_TRANSFER_MAX_INFLIGHT {
            "wait_for_transit_capacity"
        } else if distance_km == 0 {
            "submit_immediate_transfer"
        } else {
            "submit_transfer"
        };

        Ok(LogisticsTransferQuote {
            requester_agent_id: requester_agent_id.to_string(),
            from_ledger: from_ledger.clone(),
            to_ledger: to_ledger.clone(),
            kind: kind.to_string(),
            requested_amount: amount,
            submission_feasible,
            max_transferable_amount,
            sent_amount,
            distance_km,
            loss_bps,
            expected_loss_amount,
            expected_received_amount,
            source_amount_before,
            source_amount_after: source_amount_before.saturating_sub(sent_amount),
            destination_amount_before,
            destination_expected_amount_after: destination_amount_before
                .saturating_add(expected_received_amount),
            ticks_until_arrival,
            ready_at: self
                .state
                .time
                .saturating_add(1)
                .saturating_add(ticks_until_arrival),
            effective_priority,
            priority_reason,
            inflight_before,
            inflight_capacity: MATERIAL_TRANSFER_MAX_INFLIGHT,
            recommendation: recommendation.to_string(),
            conditional: true,
        })
    }

    pub(super) fn process_due_material_transits(&mut self) -> Result<Vec<WorldEvent>, WorldError> {
        let now = self.state.time;
        let mut emitted = Vec::new();

        let mut due_jobs: Vec<_> = self
            .state
            .pending_material_transits
            .values()
            .filter(|job| job.ready_at <= now)
            .cloned()
            .collect();
        due_jobs.sort_by_key(|job| (job.ready_at, job.priority, job.job_id));

        for job in due_jobs {
            let loss_amount =
                material_transit_loss_amount(job.amount, job.distance_km, job.loss_bps);
            let received_amount = job.amount.saturating_sub(loss_amount);
            self.record_logistics_sla_completion(job.ready_at, now, job.priority);

            self.append_event(
                WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted {
                    job_id: job.job_id,
                    requester_agent_id: job.requester_agent_id,
                    from_ledger: job.from_ledger,
                    to_ledger: job.to_ledger,
                    kind: job.kind,
                    sent_amount: job.amount,
                    received_amount,
                    loss_amount,
                    distance_km: job.distance_km,
                    priority: job.priority,
                }),
                None,
            )?;
            if let Some(event) = self.journal.events.last() {
                emitted.push(event.clone());
            }
        }

        Ok(emitted)
    }
}
