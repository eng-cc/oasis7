use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::super::state::LogisticsRouteV1;
use super::super::util::sha256_hex;
use super::super::{
    DomainEvent, MaterialDefaultPriority, MaterialLedgerId, MaterialTransitPriority,
    MaterialTransportLossClass, RejectReason, WorldError, WorldEvent, WorldEventBody, WorldTime,
};
use super::World;

pub(super) const MATERIAL_TRANSFER_MAX_DISTANCE_KM: i64 = 10_000;
pub(super) const MATERIAL_TRANSFER_LOSS_PER_KM_BPS: i64 = 5;
pub(super) const MATERIAL_TRANSFER_SPEED_KM_PER_TICK: i64 = 100;
pub(super) const MATERIAL_TRANSFER_MAX_INFLIGHT: usize = 2;
pub(super) const LOGISTICS_MAX_HOPS: usize = 8;
pub(super) const LOGISTICS_MAX_PATH_SEARCHES: usize = 4_096;

pub(super) fn normalize_logistics_route_kind(kind: &str) -> Option<String> {
    let normalized = kind.trim().to_ascii_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

pub(super) fn logistics_route_id(
    from_ledger: &MaterialLedgerId,
    to_ledger: &MaterialLedgerId,
    kind: &str,
    distance_km: i64,
    priority: MaterialTransitPriority,
) -> String {
    let tuple = (from_ledger, to_ledger, kind, distance_km, priority);
    let bytes = serde_json::to_vec(&tuple).expect("logistics route tuple is serializable");
    sha256_hex(&bytes)
}

pub(super) fn logistics_route_matches(
    route: &LogisticsRouteV1,
    from_ledger: &MaterialLedgerId,
    to_ledger: &MaterialLedgerId,
    kind: &str,
    distance_km: i64,
    priority: Option<MaterialTransitPriority>,
) -> bool {
    route.from_ledger == *from_ledger
        && route.to_ledger == *to_ledger
        && route.kind == kind
        && route.distance_km == distance_km
        && priority.is_none_or(|priority| priority == route.priority)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogisticsPathSelection {
    pub route_ids: Vec<String>,
    pub path_id: Option<String>,
    pub total_distance_km: i64,
    pub total_tariff_electricity: i64,
    pub total_expected_loss_amount: i64,
    pub reroute_count: u32,
}

pub(super) fn logistics_path_id(route_ids: &[String]) -> Option<String> {
    if route_ids.is_empty() {
        return None;
    }
    let bytes = serde_json::to_vec(route_ids).expect("logistics path ids are serializable");
    Some(sha256_hex(&bytes))
}

fn route_available_for_amount(route: &LogisticsRouteV1, amount: i64) -> bool {
    route.available
        && route.capacity_units > 0
        && route.reserved_capacity_units.saturating_add(amount) <= route.capacity_units
}

fn path_candidate_key(
    world: &World,
    route_ids: &[String],
    amount: i64,
) -> Option<(u64, i64, i64, usize, Vec<String>)> {
    let mut total_distance = 0_i64;
    let mut total_tariff = 0_i64;
    let mut total_loss = 0_i64;
    for route_id in route_ids {
        let route = world.state.logistics_routes.get(route_id)?;
        total_distance = total_distance.checked_add(route.distance_km)?;
        total_tariff =
            total_tariff.checked_add(route.tariff_electricity_per_unit.checked_mul(amount)?)?;
        total_loss = total_loss.checked_add(material_transit_loss_amount(
            amount,
            route.distance_km,
            material_transit_loss_bps_for_kind(world, route.kind.as_str()),
        ))?;
    }
    Some((
        material_transit_ticks(total_distance),
        total_tariff,
        total_loss,
        route_ids.len(),
        route_ids.to_vec(),
    ))
}

fn path_matches_route_tuple(
    world: &World,
    route_ids: &[String],
    from_ledger: &MaterialLedgerId,
    to_ledger: &MaterialLedgerId,
    kind: &str,
) -> bool {
    if route_ids.is_empty() || route_ids.len() > LOGISTICS_MAX_HOPS {
        return false;
    }
    let mut current = from_ledger;
    let mut seen_route_ids = BTreeSet::new();
    let mut seen_ledgers = BTreeSet::new();
    seen_ledgers.insert(from_ledger.clone());
    for route_id in route_ids {
        if !seen_route_ids.insert(route_id) {
            return false;
        }
        let Some(route) = world.state.logistics_routes.get(route_id) else {
            return false;
        };
        if route.from_ledger != *current || route.kind != kind {
            return false;
        }
        if !seen_ledgers.insert(route.to_ledger.clone()) {
            return false;
        }
        current = &route.to_ledger;
    }
    current == to_ledger
}

fn find_paths_dfs(
    world: &World,
    current: &MaterialLedgerId,
    destination: &MaterialLedgerId,
    kind: &str,
    amount: i64,
    path: &mut Vec<String>,
    visited: &mut BTreeSet<MaterialLedgerId>,
    candidates: &mut Vec<Vec<String>>,
    searches: &mut usize,
) {
    if current == destination {
        candidates.push(path.clone());
        return;
    }
    if *searches >= LOGISTICS_MAX_PATH_SEARCHES || path.len() >= LOGISTICS_MAX_HOPS {
        return;
    }
    let edges: Vec<(String, MaterialLedgerId)> = world
        .state
        .logistics_routes
        .iter()
        .filter(|(_, route)| {
            route.from_ledger == *current
                && route.kind == kind
                && route_available_for_amount(route, amount)
                && !visited.contains(&route.to_ledger)
        })
        .map(|(route_id, route)| (route_id.clone(), route.to_ledger.clone()))
        .collect();
    for (route_id, next) in edges {
        *searches = searches.saturating_add(1);
        path.push(route_id);
        visited.insert(next.clone());
        find_paths_dfs(
            world,
            &next,
            destination,
            kind,
            amount,
            path,
            visited,
            candidates,
            searches,
        );
        visited.remove(&next);
        path.pop();
    }
}

pub(super) fn select_logistics_path(
    world: &World,
    from_ledger: &MaterialLedgerId,
    to_ledger: &MaterialLedgerId,
    kind: &str,
    amount: i64,
    requested_route_ids: &[String],
    auto_reroute: bool,
) -> Result<LogisticsPathSelection, RejectReason> {
    let Some(normalized_kind) = normalize_logistics_route_kind(kind) else {
        return Err(RejectReason::RuleDenied {
            notes: vec!["material kind cannot be empty".to_string()],
        });
    };
    let mut reroute_count = 0;
    let mut candidates = Vec::new();
    if !requested_route_ids.is_empty() {
        if !path_matches_route_tuple(
            world,
            requested_route_ids,
            from_ledger,
            to_ledger,
            normalized_kind.as_str(),
        ) {
            return Err(RejectReason::RuleDenied {
                notes: vec![
                    "logistics route path is cyclic, disconnected, or incompatible".to_string(),
                ],
            });
        }
        let requested_available = requested_route_ids.iter().all(|route_id| {
            world
                .state
                .logistics_routes
                .get(route_id)
                .is_some_and(|route| route_available_for_amount(route, amount))
        });
        if requested_available {
            candidates.push(requested_route_ids.to_vec());
        } else if !auto_reroute {
            return Err(RejectReason::RuleDenied {
                notes: vec!["requested logistics path is unavailable or at capacity".to_string()],
            });
        } else {
            reroute_count = 1;
        }
    }
    if candidates.is_empty() {
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let mut searches = 0;
        visited.insert(from_ledger.clone());
        find_paths_dfs(
            world,
            from_ledger,
            to_ledger,
            normalized_kind.as_str(),
            amount,
            &mut path,
            &mut visited,
            &mut candidates,
            &mut searches,
        );
        if reroute_count > 0 {
            candidates.retain(|candidate| candidate != requested_route_ids);
        }
    }
    let route_ids = candidates
        .into_iter()
        .filter_map(|candidate| {
            path_candidate_key(world, &candidate, amount).map(|key| (key, candidate))
        })
        .min_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, candidate)| candidate)
        .ok_or_else(|| RejectReason::RuleDenied {
            notes: vec!["no available logistics path to destination".to_string()],
        })?;
    let key =
        path_candidate_key(world, &route_ids, amount).ok_or_else(|| RejectReason::RuleDenied {
            notes: vec!["logistics path tariff or distance overflow".to_string()],
        })?;
    Ok(LogisticsPathSelection {
        path_id: logistics_path_id(&route_ids),
        total_distance_km: route_ids
            .iter()
            .filter_map(|route_id| world.state.logistics_routes.get(route_id))
            .map(|route| route.distance_km)
            .sum(),
        total_tariff_electricity: key.1,
        total_expected_loss_amount: key.2,
        route_ids,
        reroute_count,
    })
}

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
                    route_id: job.route_id,
                    path_id: job.path_id,
                    route_ids: job.route_ids,
                    tariff_electricity_total: job.tariff_electricity_total,
                    reroute_count: job.reroute_count,
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
