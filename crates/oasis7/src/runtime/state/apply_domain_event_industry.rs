use super::*;

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
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                amount,
                route_id,
                ..
            } => {
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
                received_amount,
                route_id,
                path_id,
                route_ids,
                ..
            } => {
                let pending = self.pending_material_transits.remove(job_id);
                if pending.is_none() && self.settled_logistics_transit_ids.contains(job_id) {
                    return Ok(());
                }
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
                let reservation_routes = pending
                    .as_ref()
                    .map(|job| job.route_ids.as_slice())
                    .unwrap_or(route_ids.as_slice());
                if let Some(path_id) = pending
                    .as_ref()
                    .and_then(|job| job.path_id.as_ref())
                    .or(path_id.as_ref())
                {
                    self.completed_logistics_paths
                        .entry(path_id.clone())
                        .or_insert_with(|| LogisticsPathAuthorityV1 {
                            path_id: path_id.clone(),
                            route_ids: reservation_routes.to_vec(),
                            from_ledger: pending
                                .as_ref()
                                .map(|job| job.from_ledger.clone())
                                .unwrap_or_else(|| from_ledger.clone()),
                            to_ledger: pending
                                .as_ref()
                                .map(|job| job.to_ledger.clone())
                                .unwrap_or_else(|| to_ledger.clone()),
                            kind: pending
                                .as_ref()
                                .map(|job| job.kind.clone())
                                .unwrap_or_else(|| kind.clone()),
                        });
                }
                for route_id in reservation_routes {
                    self.completed_logistics_route_ids.insert(route_id.clone());
                }
                if let Some(route_id) = route_id {
                    self.completed_logistics_route_ids.insert(route_id.clone());
                }
                let reserved_amount = pending.as_ref().map(|job| job.amount).unwrap_or(0);
                for route_id in reservation_routes {
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
                    for route_id in reservation_routes {
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
                for stack in &spec.build_cost {
                    remove_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        consume_ledger,
                        stack.kind.as_str(),
                        stack.amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("factory build consume failed: {reason}"),
                    })?;
                }
                self.pending_factory_builds.insert(
                    *job_id,
                    FactoryBuildJobState {
                        job_id: *job_id,
                        builder_agent_id: builder_agent_id.clone(),
                        site_id: site_id.clone(),
                        spec: spec.clone(),
                        consume_ledger: consume_ledger.clone(),
                        ready_at: *ready_at,
                    },
                );
                if let Some(cell) = self.agents.get_mut(builder_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryBuilt {
                job_id,
                builder_agent_id,
                site_id,
                spec,
            } => {
                self.pending_factory_builds.remove(job_id);
                let site_ledger = MaterialLedgerId::site(site_id.clone());
                self.factories.insert(
                    spec.factory_id.clone(),
                    FactoryState {
                        factory_id: spec.factory_id.clone(),
                        site_id: site_id.clone(),
                        builder_agent_id: builder_agent_id.clone(),
                        spec: spec.clone(),
                        input_ledger: site_ledger.clone(),
                        output_ledger: site_ledger,
                        durability_ppm: 1_000_000,
                        production: FactoryProductionState::default(),
                        built_at: now,
                    },
                );
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(builder_agent_id) {
                    cell.last_active = now;
                }
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
                remove_material_balance_for_ledger(
                    &mut self.material_ledgers,
                    consume_ledger,
                    "hardware_part",
                    *consumed_parts,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("factory maintenance consume failed: {reason}"),
                })?;
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.durability_ppm = (*durability_ppm).clamp(0, 1_000_000);
                }
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryRecycled {
                operator_agent_id,
                factory_id,
                recycle_ledger,
                recovered,
                ..
            } => {
                self.factories.remove(factory_id);
                self.pending_recipe_jobs
                    .retain(|_, job| job.factory_id != *factory_id);
                for stack in recovered {
                    add_material_balance_for_ledger(
                        &mut self.material_ledgers,
                        recycle_ledger,
                        stack.kind.as_str(),
                        stack.amount,
                    )
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!("factory recycle material add failed: {reason}"),
                    })?;
                }
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
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
                duration_ticks,
                consume_ledger,
                output_ledger,
                bottleneck_tags,
                market_quotes,
                logistics_route_ids,
                logistics_path_ids,
                ready_at,
            } => {
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
                remove_resource_balance(
                    &mut self.resources,
                    ResourceKind::Electricity,
                    *power_required,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid {
                    reason: format!("recipe power consume failed: {reason}"),
                })?;
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
                    cell.last_active = now;
                }
            }
            DomainEvent::RecipeCompleted {
                job_id,
                requester_agent_id,
                factory_id,
                recipe_id,
                produce,
                byproducts,
                output_ledger,
                logistics_route_ids: _,
                logistics_path_ids: _,
                ..
            } => {
                let Some(completed_snapshot) = self
                    .pending_recipe_jobs
                    .get(job_id)
                    .map(FactoryProductionSnapshot::from_recipe_job)
                else {
                    return Ok(());
                };
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
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryProductionBlocked {
                requester_agent_id,
                factory_id,
                blocker_kind,
                blocker_detail,
                ..
            } => {
                if let Some(factory) = self.factories.get_mut(factory_id) {
                    factory.production.status = FactoryProductionStatus::Blocked;
                    factory.production.last_blocked_at = Some(now);
                    factory.production.current_blocker_kind = Some(blocker_kind.clone());
                    factory.production.current_blocker_detail = Some(blocker_detail.clone());
                    factory.production.current_job_id = None;
                    factory.production.current_recipe_id = None;
                    factory.production.last_completed_recipe_id = None;
                    factory.production.same_recipe_repeat_count = 0;
                    factory.production.last_completed_canonical_snapshot = None;
                }
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
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
