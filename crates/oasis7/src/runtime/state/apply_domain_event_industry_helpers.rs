use super::*;

impl WorldState {
    pub(super) fn apply_factory_build_started(
        &mut self,
        now: WorldTime,
        job_id: &ActionId,
        builder_agent_id: &str,
        site_id: &str,
        spec: &FactoryModuleSpec,
        consume_ledger: &MaterialLedgerId,
        ready_at: &WorldTime,
    ) -> Result<(), WorldError> {
        if self.pending_factory_builds.contains_key(job_id)
            || self.settled_factory_build_ids.contains(job_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("factory build job already settled or pending: job_id={job_id}"),
            });
        }
        if builder_agent_id.trim().is_empty() || !self.agents.contains_key(builder_agent_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build builder agent is missing or empty: builder_agent_id={builder_agent_id}"
                ),
            });
        }
        if site_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "factory build site_id cannot be empty".to_string(),
            });
        }
        if spec.factory_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "factory build factory_id cannot be empty".to_string(),
            });
        }
        if *ready_at <= now {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build ready_at must be in the future: now={now} ready_at={ready_at}"
                ),
            });
        }
        if self.retired_factory_ids.contains(&spec.factory_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build started for retired identity: factory_id={}",
                    spec.factory_id
                ),
            });
        }
        if self.factories.contains_key(&spec.factory_id)
            || self
                .pending_factory_builds
                .values()
                .any(|job| job.spec.factory_id == spec.factory_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build identity is already active: factory_id={}",
                    spec.factory_id
                ),
            });
        }

        // Validate and aggregate every cost before debiting any material.
        // A build event may contain duplicate material stacks; checking each
        // stack independently would allow the second debit to fail after the
        // first one had already mutated state.
        let mut required_by_kind = BTreeMap::<String, i64>::new();
        for stack in &spec.build_cost {
            if stack.kind.trim().is_empty() || stack.amount <= 0 {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build cost must be positive and named: {}={}",
                        stack.kind, stack.amount
                    ),
                });
            }
            let required = required_by_kind.entry(stack.kind.clone()).or_default();
            *required = required.checked_add(stack.amount).ok_or_else(|| {
                WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build cost overflow: kind={} amount={}",
                        stack.kind, stack.amount
                    ),
                }
            })?;
        }
        for (kind, required) in &required_by_kind {
            let available = self
                .material_ledgers
                .get(consume_ledger)
                .and_then(|ledger| ledger.get(kind))
                .copied()
                .unwrap_or(0);
            if available < *required {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build consume failed: insufficient material {kind}: requested={required} available={available}"
                    ),
                });
            }
        }
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
                builder_agent_id: builder_agent_id.to_string(),
                site_id: site_id.to_string(),
                spec: spec.clone(),
                consume_ledger: consume_ledger.clone(),
                ready_at: *ready_at,
            },
        );
        if let Some(cell) = self.agents.get_mut(builder_agent_id) {
            cell.last_active = now;
        }
        Ok(())
    }

    pub(super) fn apply_factory_built(
        &mut self,
        now: WorldTime,
        job_id: &ActionId,
        builder_agent_id: &str,
        site_id: &str,
        spec: &FactoryModuleSpec,
    ) -> Result<(), WorldError> {
        if self.settled_factory_build_ids.contains(job_id) {
            if self.factories.get(&spec.factory_id).is_some_and(|factory| {
                factory.builder_agent_id == builder_agent_id
                    && factory.site_id == site_id
                    && factory.spec == *spec
            }) {
                return Ok(());
            }
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "settled factory build completion does not match receipt: job_id={job_id}"
                ),
            });
        }
        let Some(pending) = self.pending_factory_builds.get(job_id).cloned() else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("factory build completion has no pending job: job_id={job_id}"),
            });
        };
        if self.retired_factory_ids.contains(&spec.factory_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory built for retired identity: factory_id={}",
                    spec.factory_id
                ),
            });
        }
        if pending.builder_agent_id != builder_agent_id
            || pending.site_id != site_id
            || pending.spec != *spec
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build completion does not match pending commitment: job_id={job_id}"
                ),
            });
        }
        if now < pending.ready_at {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build completion is early: job_id={job_id} ready_at={} now={now}",
                    pending.ready_at
                ),
            });
        }
        if self.factories.contains_key(&spec.factory_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory build completion would overwrite active factory: factory_id={}",
                    spec.factory_id
                ),
            });
        }
        self.pending_factory_builds.remove(job_id);
        let site_ledger = MaterialLedgerId::site(site_id.to_string());
        self.factories.insert(
            spec.factory_id.clone(),
            FactoryState {
                factory_id: spec.factory_id.clone(),
                site_id: site_id.to_string(),
                builder_agent_id: builder_agent_id.to_string(),
                spec: spec.clone(),
                input_ledger: site_ledger.clone(),
                output_ledger: site_ledger,
                durability_ppm: 1_000_000,
                production: FactoryProductionState::default(),
                built_at: now,
            },
        );
        self.settled_factory_build_ids.insert(*job_id);
        self.refresh_industry_progress_stage(now);
        if let Some(cell) = self.agents.get_mut(builder_agent_id) {
            cell.last_active = now;
        }
        Ok(())
    }

    pub(super) fn apply_factory_maintained(
        &mut self,
        now: WorldTime,
        operator_agent_id: &str,
        factory_id: &str,
        consume_ledger: &MaterialLedgerId,
        consumed_parts: &i64,
        durability_ppm: &i64,
    ) -> Result<(), WorldError> {
        if self.retired_factory_ids.contains(factory_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance targets retired identity: factory_id={factory_id}"
                ),
            });
        }
        let Some(factory) = self.factories.get(factory_id).cloned() else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance targets unknown factory: factory_id={factory_id}"
                ),
            });
        };
        if factory.builder_agent_id != operator_agent_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance operator mismatch: factory_id={factory_id} operator={operator_agent_id} builder={} ",
                    factory.builder_agent_id
                ),
            });
        }
        if *consumed_parts <= 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance consumed_parts must be positive: factory_id={factory_id} consumed_parts={consumed_parts}"
                ),
            });
        }
        if !(0..=1_000_000).contains(durability_ppm) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance durability is out of range: factory_id={factory_id} durability_ppm={durability_ppm}"
                ),
            });
        }
        let available = self
            .material_ledgers
            .get(consume_ledger)
            .and_then(|ledger| ledger.get("hardware_part"))
            .copied()
            .unwrap_or(0);
        if available < *consumed_parts {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory maintenance consume failed: insufficient material hardware_part: requested={consumed_parts} available={available}"
                ),
            });
        }
        remove_material_balance_for_ledger(
            &mut self.material_ledgers,
            consume_ledger,
            "hardware_part",
            *consumed_parts,
        )
        .map_err(|reason| WorldError::ResourceBalanceInvalid {
            reason: format!("factory maintenance consume failed: {reason}"),
        })?;
        self.factories
            .get_mut(factory_id)
            .expect("factory was validated before maintenance mutation")
            .durability_ppm = *durability_ppm;
        if let Some(cell) = self.agents.get_mut(operator_agent_id) {
            cell.last_active = now;
        }
        Ok(())
    }

    pub(super) fn apply_factory_recycled(
        &mut self,
        now: WorldTime,
        operator_agent_id: &str,
        factory_id: &str,
        recycle_ledger: &MaterialLedgerId,
        recovered: &[MaterialStack],
    ) -> Result<(), WorldError> {
        if self.retired_factory_ids.contains(factory_id) {
            return Ok(());
        }
        let Some(factory) = self.factories.get(factory_id).cloned() else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("factory recycle targets unknown factory: factory_id={factory_id}"),
            });
        };
        if factory.builder_agent_id != operator_agent_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory recycle operator mismatch: factory_id={factory_id} operator={operator_agent_id} builder={} ",
                    factory.builder_agent_id
                ),
            });
        }
        if self
            .pending_recipe_jobs
            .values()
            .any(|job| job.factory_id == factory_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory recycle targets factory with active recipe: factory_id={factory_id}"
                ),
            });
        }
        preflight_material_balance_additions(&self.material_ledgers, recycle_ledger, recovered)
            .map_err(|reason| WorldError::ResourceBalanceInvalid {
                reason: format!("factory recycle material preflight failed: {reason}"),
            })?;
        self.factories.remove(factory_id);
        self.retired_factory_ids.insert(factory_id.to_string());
        self.pending_recipe_jobs
            .retain(|_, job| job.factory_id != factory_id);
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
        Ok(())
    }
}

pub(super) fn validate_recipe_material_stacks(
    label: &str,
    stacks: &[MaterialStack],
) -> Result<(), String> {
    for stack in stacks {
        if stack.kind.trim().is_empty() || stack.amount <= 0 {
            return Err(format!(
                "{label} stack must have a non-empty kind and amount > 0"
            ));
        }
    }
    Ok(())
}

pub(super) fn aggregate_recipe_material_stacks(
    stacks: &[MaterialStack],
) -> Result<BTreeMap<String, i64>, String> {
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for stack in stacks {
        let entry = totals.entry(stack.kind.clone()).or_insert(0);
        *entry = entry
            .checked_add(stack.amount)
            .ok_or_else(|| format!("material amount overflow for kind={}", stack.kind))?;
    }
    Ok(totals)
}

pub(super) fn validate_recipe_output_capacity(
    ledgers: &BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    ledger: &MaterialLedgerId,
    produce: &[MaterialStack],
    byproducts: &[MaterialStack],
) -> Result<(), String> {
    validate_recipe_material_stacks("produce", produce)?;
    validate_recipe_material_stacks("byproduct", byproducts)?;

    let mut totals = aggregate_recipe_material_stacks(produce)?;
    for (kind, amount) in aggregate_recipe_material_stacks(byproducts)? {
        let entry = totals.entry(kind).or_insert(0);
        *entry = entry
            .checked_add(amount)
            .ok_or_else(|| "combined output amount overflow".to_string())?;
    }

    let current_balances = ledgers.get(ledger);
    for (kind, amount) in totals {
        let current = current_balances
            .and_then(|balances| balances.get(&kind))
            .copied()
            .unwrap_or(0);
        if current < 0 {
            return Err(format!(
                "negative existing material balance: ledger={ledger} kind={kind} balance={current}"
            ));
        }
        current.checked_add(amount).ok_or_else(|| {
            format!("material output balance overflow: ledger={ledger} kind={kind}")
        })?;
    }
    Ok(())
}
