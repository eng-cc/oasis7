use super::apply_domain_event_industry_helpers::{
    aggregate_recipe_material_stacks, validate_recipe_material_stacks,
    validate_recipe_output_capacity,
};
use super::*;
use crate::runtime::AgentActivityV1;

impl WorldState {
    pub(super) fn apply_domain_event_industry(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::LogisticsRouteRegistered {
                requester_agent_id,
                route_id,
                from_ledger,
                to_ledger,
                kind,
                distance_km,
                priority,
                owner_agent_id,
                available,
                capacity_units,
                tariff_electricity_per_unit,
            } => {
                if let Some(route_id) = route_id {
                    self.logistics_routes.insert(
                        route_id.clone(),
                        LogisticsRouteV1 {
                            route_id: route_id.clone(),
                            from_ledger: from_ledger.clone(),
                            to_ledger: to_ledger.clone(),
                            kind: kind.clone(),
                            distance_km: *distance_km,
                            priority: *priority,
                            owner_agent_id: if owner_agent_id.is_empty() {
                                requester_agent_id.clone()
                            } else {
                                owner_agent_id.clone()
                            },
                            available: *available,
                            capacity_units: *capacity_units,
                            reserved_capacity_units: 0,
                            tariff_electricity_per_unit: *tariff_electricity_per_unit,
                        },
                    );
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::LogisticsRouteAvailabilityChanged {
                requester_agent_id,
                route_id,
                available,
                owner_agent_id,
            } => {
                let route = self.logistics_routes.get_mut(route_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("logistics route not found: route_id={route_id}"),
                    }
                })?;
                let owner = if owner_agent_id.is_empty() {
                    route.owner_agent_id.as_str()
                } else {
                    owner_agent_id.as_str()
                };
                if !owner.is_empty() && owner != requester_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "logistics route availability owner mismatch: route_id={route_id} owner={owner} requester={requester_agent_id}"
                        ),
                    });
                }
                route.available = *available;
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::LogisticsPathRerouted { .. } => {}
            DomainEvent::MaterialTransferred {
                transfer_id,
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                amount,
                distance_km,
                priority,
                route_id,
            } => {
                let receipt = transfer_id.map(|transfer_id| MaterialTransferReceiptV1 {
                    transfer_id,
                    requester_agent_id: requester_agent_id.clone(),
                    from_ledger: from_ledger.clone(),
                    to_ledger: to_ledger.clone(),
                    kind: kind.clone(),
                    amount: *amount,
                    distance_km: *distance_km,
                    priority: *priority,
                    route_id: route_id.clone(),
                });
                if let Some(receipt) = receipt.as_ref() {
                    if let Some(existing) = self
                        .direct_material_transfer_receipts
                        .get(&receipt.transfer_id)
                    {
                        if existing == receipt {
                            return Ok(());
                        }
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "direct material transfer receipt mismatch: transfer_id={}",
                                receipt.transfer_id
                            ),
                        });
                    }
                }
                preflight_material_balance_additions(
                    &self.material_ledgers,
                    to_ledger,
                    &[MaterialStack::new(kind.clone(), *amount)],
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("material transfer add preflight failed: {reason}"),
                })?;
                remove_material_balance_for_ledger(
                    &mut self.material_ledgers,
                    from_ledger,
                    kind.as_str(),
                    *amount,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("material transfer remove failed: {reason}"),
                })?;
                add_material_balance_for_ledger(
                    &mut self.material_ledgers,
                    to_ledger,
                    kind.as_str(),
                    *amount,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("material transfer add failed: {reason}"),
                })?;
                if let Some(route_id) = route_id {
                    self.completed_logistics_route_ids.insert(route_id.clone());
                }
                if let Some(receipt) = receipt {
                    self.direct_material_transfer_receipts
                        .insert(receipt.transfer_id, receipt);
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::MaterialTransitStarted {
                job_id,
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                amount,
                distance_km,
                loss_bps,
                priority,
                route_id,
                path_id,
                route_ids,
                tariff_electricity_total,
                reroute_count,
                ready_at,
            } => {
                if self.pending_material_transits.contains_key(job_id)
                    || self.settled_logistics_transit_ids.contains(job_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "material transit job already settled or pending: job_id={job_id}"
                        ),
                    });
                }
                for route_id in route_ids {
                    let route = self.logistics_routes.get(route_id).ok_or_else(|| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "logistics route not found for transit: route_id={route_id}"
                            ),
                        }
                    })?;
                    if !route.available
                        || route.capacity_units <= 0
                        || route.reserved_capacity_units.saturating_add(*amount)
                            > route.capacity_units
                    {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "logistics route capacity unavailable for transit: route_id={route_id}"
                            ),
                        });
                    }
                }
                if *tariff_electricity_total > 0 {
                    let payer = self.agents.get(requester_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: requester_agent_id.clone(),
                        }
                    })?;
                    let available = payer.state.resources.get(ResourceKind::Electricity);
                    if available < *tariff_electricity_total {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "logistics tariff escrow insufficient: requested={} available={} agent={requester_agent_id}",
                                tariff_electricity_total, available
                            ),
                        });
                    }
                }
                remove_material_balance_for_ledger(
                    &mut self.material_ledgers,
                    from_ledger,
                    kind.as_str(),
                    *amount,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("material transit reserve failed: {reason}"),
                })?;
                self.pending_material_transits.insert(
                    *job_id,
                    MaterialTransitJobState {
                        job_id: *job_id,
                        requester_agent_id: requester_agent_id.clone(),
                        from_ledger: from_ledger.clone(),
                        to_ledger: to_ledger.clone(),
                        kind: kind.clone(),
                        amount: *amount,
                        distance_km: *distance_km,
                        loss_bps: *loss_bps,
                        priority: *priority,
                        route_id: route_id.clone(),
                        path_id: path_id.clone(),
                        route_ids: route_ids.clone(),
                        tariff_electricity_total: *tariff_electricity_total,
                        reroute_count: *reroute_count,
                        ready_at: *ready_at,
                    },
                );
                if *tariff_electricity_total > 0 {
                    let payer = self.agents.get_mut(requester_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: requester_agent_id.clone(),
                        }
                    })?;
                    payer
                        .state
                        .resources
                        .remove(ResourceKind::Electricity, *tariff_electricity_total)
                        .map_err(|err| WorldError::ResourceBalanceInvalid {
                            reason: format!("logistics tariff escrow failed: {err:?}"),
                        })?;
                }
                for route_id in route_ids {
                    if let Some(route) = self.logistics_routes.get_mut(route_id) {
                        route.reserved_capacity_units =
                            route.reserved_capacity_units.saturating_add(*amount);
                    }
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::MaterialTransitCompleted {
                job_id,
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                sent_amount,
                received_amount,
                loss_amount,
                distance_km,
                priority,
                route_id,
                path_id,
                route_ids,
                tariff_electricity_total,
                reroute_count,
                ..
            } => {
                if self.settled_logistics_transit_ids.contains(job_id) {
                    return Ok(());
                }
                let Some(pending) = self.pending_material_transits.get(job_id).cloned() else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "material transit completion has no pending job: job_id={job_id}"
                        ),
                    });
                };
                let expected_loss = {
                    let amount = pending.amount.max(0);
                    ((amount as i128)
                        .saturating_mul(pending.distance_km as i128)
                        .saturating_mul(pending.loss_bps as i128)
                        / 10_000)
                        .clamp(0, amount as i128) as i64
                };
                let conservation_valid = *sent_amount >= 0
                    && *received_amount >= 0
                    && *loss_amount >= 0
                    && *received_amount <= *sent_amount
                    && sent_amount.checked_sub(*received_amount) == Some(*loss_amount)
                    && *loss_amount == expected_loss
                    && *received_amount == pending.amount.saturating_sub(expected_loss);
                let identity_valid = pending.requester_agent_id == *requester_agent_id
                    && pending.from_ledger == *from_ledger
                    && pending.to_ledger == *to_ledger
                    && pending.kind == *kind
                    && pending.amount == *sent_amount
                    && pending.distance_km == *distance_km
                    && pending.priority == *priority
                    && pending.route_id == *route_id
                    && pending.path_id == *path_id
                    && pending.route_ids == *route_ids
                    && pending.tariff_electricity_total == *tariff_electricity_total
                    && pending.reroute_count == *reroute_count;
                if !identity_valid || !conservation_valid {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "material transit completion does not match pending commitment: job_id={job_id}"
                        ),
                    });
                }
                if *received_amount > 0 {
                    preflight_material_balance_additions(
                        &self.material_ledgers,
                        to_ledger,
                        &[MaterialStack::new(kind.clone(), *received_amount)],
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("material transit completion preflight failed: {reason}"),
                    })?;
                }
                let reservation_routes = pending.route_ids.clone();
                let completed_path_id = pending.path_id.clone().or_else(|| path_id.clone());
                let settled_path_amount = (*received_amount).max(0);
                let path_authority = match completed_path_id.as_ref() {
                    None => None,
                    Some(path_id) => {
                        let mut authority = if let Some(existing) =
                            self.completed_logistics_paths.get(path_id)
                        {
                            if existing.route_ids != reservation_routes
                                || existing.from_ledger != pending.from_ledger
                                || existing.to_ledger != pending.to_ledger
                                || existing.kind != pending.kind
                                || existing.settled_amount < 0
                                || existing.remaining_recipe_amount < 0
                                || existing.remaining_recipe_amount > existing.settled_amount
                            {
                                return Err(WorldError::ResourceBalanceInvalid {
                                    reason: format!(
                                        "material transit path authority does not match settlement: path_id={path_id}"
                                    ),
                                });
                            }
                            existing.clone()
                        } else {
                            LogisticsPathAuthorityV1 {
                                path_id: path_id.clone(),
                                route_ids: reservation_routes.clone(),
                                from_ledger: pending.from_ledger.clone(),
                                to_ledger: pending.to_ledger.clone(),
                                kind: pending.kind.clone(),
                                settled_amount: 0,
                                remaining_recipe_amount: 0,
                            }
                        };
                        authority.settled_amount = authority
                            .settled_amount
                            .checked_add(settled_path_amount)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "material transit path settled quantity overflow: path_id={path_id}"
                                ),
                            })?;
                        authority.remaining_recipe_amount = authority
                            .remaining_recipe_amount
                            .checked_add(settled_path_amount)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "material transit path recipe quantity overflow: path_id={path_id}"
                                ),
                            })?;
                        Some(authority)
                    }
                };
                self.pending_material_transits.remove(job_id);
                let pending = Some(pending);
                if *received_amount > 0 {
                    add_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        to_ledger,
                        kind.as_str(),
                        *received_amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("material transit completion failed: {reason}"),
                    })?;
                }
                if let Some(authority) = path_authority {
                    self.completed_logistics_paths
                        .insert(authority.path_id.clone(), authority);
                }
                for route_id in &reservation_routes {
                    self.completed_logistics_route_ids.insert(route_id.clone());
                }
                if let Some(route_id) = route_id {
                    self.completed_logistics_route_ids.insert(route_id.clone());
                }
                let reserved_amount = pending.as_ref().map(|job| job.amount).unwrap_or(0);
                for route_id in &reservation_routes {
                    if let Some(route) = self.logistics_routes.get_mut(route_id) {
                        route.reserved_capacity_units = route
                            .reserved_capacity_units
                            .saturating_sub(reserved_amount);
                    }
                }
                if self.settled_logistics_transit_ids.insert(*job_id) {
                    let tariff_total = pending
                        .as_ref()
                        .map(|job| job.tariff_electricity_total)
                        .unwrap_or(0)
                        .max(0);
                    let mut owner_fees = BTreeMap::<String, i64>::new();
                    let amount = pending
                        .as_ref()
                        .map(|job| job.amount)
                        .unwrap_or(*received_amount);
                    for route_id in &reservation_routes {
                        if let Some(route) = self.logistics_routes.get(route_id) {
                            let edge_fee = route
                                .tariff_electricity_per_unit
                                .saturating_mul(amount)
                                .max(0);
                            if edge_fee > 0 {
                                let owner_fee =
                                    owner_fees.entry(route.owner_agent_id.clone()).or_default();
                                *owner_fee = owner_fee.saturating_add(edge_fee);
                            }
                        }
                    }
                    if tariff_total > 0 {
                        let mut remaining = tariff_total;
                        for (owner, owner_fee) in &owner_fees {
                            if remaining == 0 {
                                break;
                            }
                            let credit = (*owner_fee).min(remaining);
                            if let Some(cell) = self.agents.get_mut(owner) {
                                cell.state
                                    .resources
                                    .add(ResourceKind::Electricity, credit)
                                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                                        reason: format!("logistics tariff credit failed: {err:?}"),
                                    })?;
                            }
                            remaining = remaining.saturating_sub(credit);
                        }
                    }
                    self.logistics_settlement_receipts.insert(
                        *job_id,
                        LogisticsSettlementReceiptV1 {
                            job_id: *job_id,
                            path_id: pending
                                .as_ref()
                                .and_then(|job| job.path_id.clone())
                                .or_else(|| path_id.clone()),
                            route_ids: reservation_routes.to_vec(),
                            tariff_electricity_total: tariff_total,
                            owner_payouts: owner_fees,
                            governance_tax_electricity: 0,
                        },
                    );
                }
                self.industry_progress.completed_material_transits = self
                    .industry_progress
                    .completed_material_transits
                    .saturating_add(1);
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryBuildStarted {
                job_id,
                builder_agent_id,
                site_id,
                spec,
                consume_ledger,
                ready_at,
            } => {
                self.apply_factory_build_started(
                    now,
                    job_id,
                    builder_agent_id,
                    site_id,
                    spec,
                    consume_ledger,
                    ready_at,
                )?;
            }
            DomainEvent::FactoryBuilt {
                job_id,
                builder_agent_id,
                site_id,
                spec,
            } => {
                self.apply_factory_built(now, job_id, builder_agent_id, site_id, spec)?;
            }
            DomainEvent::FactoryDurabilityChanged {
                factory_id,
                durability_ppm,
                ..
            } => {
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.durability_ppm = (*durability_ppm).clamp(0, 1_000_000);
                }
            }
            DomainEvent::FactoryMaintained {
                operator_agent_id,
                factory_id,
                consume_ledger,
                consumed_parts,
                durability_ppm,
            } => {
                self.apply_factory_maintained(
                    now,
                    operator_agent_id,
                    factory_id,
                    consume_ledger,
                    consumed_parts,
                    durability_ppm,
                )?;
            }
            DomainEvent::FactoryRecycled {
                operator_agent_id,
                factory_id,
                recycle_ledger,
                recovered,
                ..
            } => {
                self.apply_factory_recycled(
                    now,
                    operator_agent_id,
                    factory_id,
                    recycle_ledger,
                    recovered,
                )?;
            }
            DomainEvent::RecipeStarted {
                job_id,
                requester_agent_id,
                factory_id,
                recipe_id,
                accepted_batches,
                consume,
                produce,
                byproducts,
                power_required,
                power_owner_agent_id,
                duration_ticks,
                consume_ledger,
                output_ledger,
                bottleneck_tags,
                market_quotes,
                logistics_route_ids,
                logistics_path_ids,
                ready_at,
            } => {
                if self.pending_recipe_jobs.contains_key(job_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe start job is already pending: job_id={job_id}"),
                    });
                }
                if self.settled_recipe_job_ids.contains(job_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe start job identity is already settled: job_id={job_id}"
                        ),
                    });
                }
                if self.retired_factory_ids.contains(factory_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe scheduled for retired factory: factory_id={factory_id}"
                        ),
                    });
                }
                let Some(factory) = self.factories.get(factory_id).cloned() else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe factory not found: factory_id={factory_id}"),
                    });
                };
                let Some(power_owner_agent_id) = power_owner_agent_id.as_deref() else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power owner is missing: job_id={job_id} factory_id={factory_id}"
                        ),
                    });
                };
                if power_owner_agent_id != factory.builder_agent_id
                    || power_owner_agent_id != requester_agent_id
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power owner mismatch: job_id={job_id} payer={power_owner_agent_id} requester={requester_agent_id} builder={} ",
                            factory.builder_agent_id
                        ),
                    });
                }
                if !self.agents.contains_key(power_owner_agent_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power owner agent not found: job_id={job_id} payer={power_owner_agent_id}"
                        ),
                    });
                }
                if factory.builder_agent_id != *requester_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe requester is not factory builder: factory_id={factory_id} requester={requester_agent_id} builder={}",
                            factory.builder_agent_id
                        ),
                    });
                }
                if factory.spec.recipe_slots == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe factory has no execution slots: factory_id={factory_id}"
                        ),
                    });
                }
                let pending_factory_jobs = self
                    .pending_recipe_jobs
                    .values()
                    .filter(|job| job.factory_id == *factory_id)
                    .count();
                if pending_factory_jobs >= usize::from(factory.spec.recipe_slots)
                    || usize::from(factory.production.active_jobs)
                        >= usize::from(factory.spec.recipe_slots)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe factory has no free execution slot: factory_id={factory_id} active_jobs={} pending_jobs={} recipe_slots={}",
                            factory.production.active_jobs,
                            pending_factory_jobs,
                            factory.spec.recipe_slots
                        ),
                    });
                }
                let expected_output_ledger = if *consume_ledger == MaterialLedgerId::world() {
                    MaterialLedgerId::world()
                } else if *consume_ledger == factory.input_ledger {
                    factory.output_ledger.clone()
                } else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe consume ledger does not match factory input or world fallback: factory_id={factory_id} consume_ledger={consume_ledger} input_ledger={}",
                            factory.input_ledger
                        ),
                    });
                };
                if *output_ledger != expected_output_ledger {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe output ledger does not match committed factory route: factory_id={factory_id} output_ledger={output_ledger} expected={expected_output_ledger}"
                        ),
                    });
                }
                if *accepted_batches == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe accepted_batches must be positive: job_id={job_id}"
                        ),
                    });
                }
                if *duration_ticks == 0 || *ready_at <= now {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe timing commitment is invalid: job_id={job_id} duration_ticks={duration_ticks} ready_at={ready_at} now={now}"
                        ),
                    });
                }
                validate_recipe_material_stacks("consume", consume).map_err(|reason| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe consume stack invalid: {reason}"),
                    }
                })?;
                validate_recipe_material_stacks("produce", produce).map_err(|reason| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe produce stack invalid: {reason}"),
                    }
                })?;
                validate_recipe_material_stacks("byproduct", byproducts).map_err(|reason| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe byproduct stack invalid: {reason}"),
                    }
                })?;
                let required_inputs =
                    aggregate_recipe_material_stacks(consume).map_err(|reason| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!("recipe consume aggregation failed: {reason}"),
                        }
                    })?;
                let path_allocations = self
                    .allocate_recipe_path_amounts(
                        consume_ledger,
                        logistics_route_ids,
                        logistics_path_ids,
                        consume,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe logistics path authority failed: {reason}"),
                    })?;
                for (kind, amount) in required_inputs {
                    let available = self
                        .material_ledgers
                        .get(consume_ledger)
                        .and_then(|ledger| ledger.get(&kind))
                        .copied()
                        .unwrap_or(0);
                    if available < amount {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "recipe consume failed: insufficient material {kind}: requested={amount} available={available}"
                            ),
                        });
                    }
                }
                if *power_required < 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power_required must be non-negative: job_id={job_id} power_required={power_required}"
                        ),
                    });
                }
                let available_power = self
                    .agents
                    .get(power_owner_agent_id)
                    .map(|cell| cell.state.resources.get(ResourceKind::Electricity))
                    .unwrap_or(0);
                if available_power < *power_required {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power consume failed: insufficient electricity: payer={power_owner_agent_id} requested={power_required} available={available_power}"
                        ),
                    });
                }
                for stack in consume {
                    remove_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        consume_ledger,
                        stack.kind.as_str(),
                        stack.amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe consume failed: {reason}"),
                    })?;
                }
                self.agents
                    .get_mut(power_owner_agent_id)
                    .expect("recipe power owner existence prevalidated")
                    .state
                    .resources
                    .remove(ResourceKind::Electricity, *power_required)
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe power consume failed for payer {power_owner_agent_id}: {reason:?}"
                        ),
                    })?;
                for (path_id, amount) in path_allocations {
                    let path = self
                        .completed_logistics_paths
                        .get_mut(&path_id)
                        .expect("recipe path authority existence prevalidated");
                    path.remaining_recipe_amount = path
                        .remaining_recipe_amount
                        .checked_sub(amount)
                        .expect("recipe path authority quantity prevalidated");
                }
                self.pending_recipe_jobs.insert(
                    *job_id,
                    RecipeJobState {
                        job_id: *job_id,
                        requester_agent_id: requester_agent_id.clone(),
                        factory_id: factory_id.clone(),
                        recipe_id: recipe_id.clone(),
                        accepted_batches: *accepted_batches,
                        consume: consume.clone(),
                        produce: produce.clone(),
                        byproducts: byproducts.clone(),
                        power_required: *power_required,
                        power_owner_agent_id: Some(power_owner_agent_id.to_string()),
                        duration_ticks: *duration_ticks,
                        consume_ledger: consume_ledger.clone(),
                        output_ledger: output_ledger.clone(),
                        bottleneck_tags: bottleneck_tags.clone(),
                        logistics_route_ids: logistics_route_ids.clone(),
                        logistics_path_ids: logistics_path_ids.clone(),
                        ready_at: *ready_at,
                    },
                );
                for quote in market_quotes {
                    self.industry_progress
                        .latest_market_quotes
                        .insert(quote.kind.clone(), quote.clone());
                }
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.production.active_jobs =
                        factory.production.active_jobs.saturating_add(1);
                    factory.production.current_job_id = Some(*job_id);
                    factory.production.current_recipe_id = Some(recipe_id.clone());
                    factory.production.last_started_at = Some(now);
                    factory.production.status = FactoryProductionStatus::Running;
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.activity = Some(AgentActivityV1::executing(
                        "recipe",
                        *job_id,
                        factory_id.clone(),
                        now,
                    ));
                    cell.last_active = now;
                }
            }
            DomainEvent::RecipeCompleted {
                job_id,
                requester_agent_id,
                factory_id,
                recipe_id,
                accepted_batches,
                produce,
                byproducts,
                output_ledger,
                bottleneck_tags,
                logistics_route_ids,
                logistics_path_ids,
                ..
            } => {
                if self.settled_recipe_job_ids.contains(job_id) {
                    return Ok(());
                }
                let Some(pending) = self.pending_recipe_jobs.get(job_id).cloned() else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe completion has no pending job: job_id={job_id}"),
                    });
                };
                if now < pending.ready_at {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe completion is early: job_id={job_id} ready_at={} now={now}",
                            pending.ready_at
                        ),
                    });
                }
                if pending.requester_agent_id != *requester_agent_id
                    || pending.factory_id != *factory_id
                    || pending.recipe_id != *recipe_id
                    || pending.accepted_batches != *accepted_batches
                    || pending.produce != *produce
                    || pending.byproducts != *byproducts
                    || pending.output_ledger != *output_ledger
                    || pending.bottleneck_tags != *bottleneck_tags
                    || pending.logistics_route_ids != *logistics_route_ids
                    || pending.logistics_path_ids != *logistics_path_ids
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "recipe completion does not match pending commitment: job_id={job_id}"
                        ),
                    });
                }
                validate_recipe_output_capacity(
                    &self.material_ledgers,
                    output_ledger,
                    produce,
                    byproducts,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("recipe output preflight failed: {reason}"),
                })?;
                let completed_snapshot = FactoryProductionSnapshot::from_recipe_job(&pending);
                self.pending_recipe_jobs.remove(job_id);
                for stack in produce {
                    add_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        output_ledger,
                        stack.kind.as_str(),
                        stack.amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe produce failed: {reason}"),
                    })?;
                }
                for stack in byproducts {
                    add_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        output_ledger,
                        stack.kind.as_str(),
                        stack.amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("recipe byproduct failed: {reason}"),
                    })?;
                }
                self.settled_recipe_job_ids.insert(*job_id);
                self.industry_progress.completed_recipe_jobs = self
                    .industry_progress
                    .completed_recipe_jobs
                    .saturating_add(1);
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.production.active_jobs =
                        factory.production.active_jobs.saturating_sub(1);
                    if factory.production.current_job_id == Some(*job_id) {
                        factory.production.current_job_id = None;
                    }
                    factory.production.current_recipe_id = None;
                    factory.production.last_completed_at = Some(now);
                    factory.production.completed_jobs =
                        factory.production.completed_jobs.saturating_add(1);
                    let same_canonical_line = factory
                        .production
                        .last_completed_canonical_snapshot
                        .as_ref()
                        == Some(&completed_snapshot);
                    factory.production.same_recipe_repeat_count = if same_canonical_line {
                        factory
                            .production
                            .same_recipe_repeat_count
                            .saturating_add(1)
                    } else {
                        1
                    };
                    factory.production.last_completed_recipe_id = Some(recipe_id.clone());
                    factory.production.last_completed_canonical_snapshot = Some(completed_snapshot);
                    if factory.production.active_jobs == 0 {
                        factory.production.status = FactoryProductionStatus::Idle;
                    }
                }
                self.refresh_industry_progress_stage(now);
                let no_active_recipe_jobs = self
                    .factories
                    .get(factory_id)
                    .map(|factory| factory.production.active_jobs == 0)
                    .unwrap_or(false);
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    if no_active_recipe_jobs {
                        cell.activity = Some(AgentActivityV1::idle(now));
                    }
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryProductionBlocked {
                action_id,
                requester_agent_id,
                factory_id,
                recipe_id,
                blocker_kind,
                blocker_detail,
                ..
            } => {
                let product_validation_failure = blocker_kind == "product_validation";
                if product_validation_failure {
                    let Some(pending) = self.pending_recipe_jobs.get(action_id) else {
                        // A validation disposition already applied is a
                        // terminal replay and must remain byte-stable.
                        return Ok(());
                    };
                    if pending.requester_agent_id != *requester_agent_id
                        || pending.factory_id != *factory_id
                        || pending.recipe_id != *recipe_id
                    {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "product-validation blocker does not match pending commitment: job_id={action_id}"
                            ),
                        });
                    }
                }
                let terminated_job = if product_validation_failure {
                    self.pending_recipe_jobs.remove(action_id)
                } else {
                    None
                };
                // A replayed validation-failure disposition is already
                // terminal; do not mutate counters or stable-line state twice.
                if product_validation_failure && terminated_job.is_none() {
                    return Ok(());
                }
                if product_validation_failure {
                    self.settled_recipe_job_ids.insert(*action_id);
                }
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    if terminated_job.is_some() {
                        factory.production.active_jobs =
                            factory.production.active_jobs.saturating_sub(1);
                        if factory.production.current_job_id == Some(*action_id) {
                            factory.production.current_job_id = None;
                        }
                        factory.production.current_recipe_id = None;
                    }
                    factory.production.status = FactoryProductionStatus::Blocked;
                    factory.production.last_blocked_at = Some(now);
                    factory.production.current_blocker_kind = Some(blocker_kind.clone());
                    factory.production.current_blocker_detail = Some(blocker_detail.clone());
                    factory.production.current_job_id = None;
                    factory.production.current_recipe_id = None;
                    if !product_validation_failure {
                        factory.production.last_completed_recipe_id = None;
                        factory.production.same_recipe_repeat_count = 0;
                        factory.production.last_completed_canonical_snapshot = None;
                    }
                }
                if !product_validation_failure {
                    self.refresh_industry_progress_stage(now);
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.activity = Some(AgentActivityV1::blocked(
                        "recipe",
                        *action_id,
                        factory_id.clone(),
                        blocker_kind.clone(),
                        blocker_detail.clone(),
                        now,
                    ));
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryProductionResumed {
                requester_agent_id,
                factory_id,
                ..
            } => {
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.production.status = if factory.production.active_jobs > 0 {
                        FactoryProductionStatus::Running
                    } else {
                        FactoryProductionStatus::Idle
                    };
                    factory.production.last_resumed_at = Some(now);
                    factory.production.current_blocker_kind = None;
                    factory.production.current_blocker_detail = None;
                }
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryProductionPaused {
                requester_agent_id,
                factory_id,
                ..
            } => {
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.production.status = FactoryProductionStatus::Paused;
                    factory.production.current_job_id = None;
                    factory.production.current_recipe_id = None;
                    factory.production.current_blocker_kind = None;
                    factory.production.current_blocker_detail = None;
                    factory.production.last_completed_recipe_id = None;
                    factory.production.same_recipe_repeat_count = 0;
                    factory.production.last_completed_canonical_snapshot = None;
                }
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            _ => unreachable!("apply_domain_event_industry received unsupported event variant"),
        }
        Ok(())
    }

    pub(super) fn refresh_industry_progress_stage(&mut self, now: WorldTime) {
        let current = self.industry_progress.stage;
        let active_completed_jobs = self
            .factories
            .values()
            .map(|factory| factory.production.completed_jobs)
            .sum::<u64>();
        let mut next = IndustryStage::Bootstrap;

        let stable_canonical_line_ready = self
            .factories
            .values()
            .any(|factory| factory.production.same_recipe_repeat_count >= 3);
        if stable_canonical_line_ready {
            next = IndustryStage::ScaleOut;
        }

        let governance_enabled =
            self.gameplay_policy.electricity_tax_bps > 0 || self.gameplay_policy.data_tax_bps > 0;
        let governance_throughput_ready =
            active_completed_jobs >= 6 || self.industry_progress.completed_material_transits >= 3;
        if next == IndustryStage::ScaleOut && governance_enabled && governance_throughput_ready {
            next = IndustryStage::Governance;
        }

        if next != current {
            self.industry_progress.stage = next;
            self.industry_progress.stage_updated_at = now;
        }
    }
}
