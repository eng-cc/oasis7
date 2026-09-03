use super::*;

// Settled industry payloads are only needed for a bounded replay/audit
// window. Unsettled jobs retain their attempt and receipt payloads so a
// checkpoint can still recover the in-flight decision without re-running a
// module.
const SETTLED_INDUSTRY_HISTORY_LIMIT: usize = 64;

impl WorldState {
    pub(super) fn allocate_industry_settlement_order(
        &mut self,
        job_id: ActionId,
    ) -> Result<u64, WorldError> {
        if let Some(order) = self.industry_settlement_orders.get(&job_id) {
            return Ok(*order);
        }
        let order = self.next_industry_settlement_order;
        let next_order =
            order
                .checked_add(1)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "industry settlement order exhausted: next_order={order} job_id={job_id}"
                    ),
                })?;
        self.industry_settlement_orders.insert(job_id, order);
        self.next_industry_settlement_order = next_order;
        Ok(order)
    }

    fn is_current_factory_failure_disposition(&self, job_id: ActionId) -> bool {
        let Some(disposition) = self.factory_production_failure_dispositions.get(&job_id) else {
            return false;
        };
        let Some(factory) = self.factories.get(&disposition.factory_id) else {
            return false;
        };
        let production = &factory.production;
        production.current_job_id.is_none()
            && production.current_recipe_id.is_none()
            && production
                .current_blocker_action_id
                .is_none_or(|action_id| action_id == job_id)
            && production.current_blocker_kind.as_deref() == Some(disposition.blocker_kind.as_str())
            && production.current_blocker_detail.as_deref()
                == Some(disposition.blocker_detail.as_str())
            && !self.pending_recipe_jobs.contains_key(&job_id)
    }

    pub(super) fn compact_settled_industry_history(&mut self) {
        let mut settled_industry_job_ids = BTreeSet::new();
        let mut protected_failure_job_ids = BTreeSet::new();
        for job_id in self
            .product_validation_attempts
            .keys()
            .chain(self.product_validation_receipts.keys())
            .chain(self.recipe_completion_receipts.keys())
            .chain(self.factory_production_failure_dispositions.keys())
        {
            if self.settled_recipe_job_ids.contains(job_id)
                && !self.pending_recipe_jobs.contains_key(job_id)
            {
                if self.is_current_factory_failure_disposition(*job_id) {
                    protected_failure_job_ids.insert(*job_id);
                } else {
                    settled_industry_job_ids.insert(*job_id);
                }
            }
        }

        // Only entries with a durable settlement order are eligible for
        // eviction. Legacy payloads without an order remain conservatively
        // retained because ActionId order is not a settlement-age authority.
        let mut ordered_job_ids: Vec<(u64, ActionId)> = settled_industry_job_ids
            .iter()
            .filter_map(|job_id| {
                self.industry_settlement_orders
                    .get(job_id)
                    .copied()
                    .map(|order| (order, *job_id))
            })
            .collect();
        ordered_job_ids.sort_unstable();
        let unprotected_limit =
            SETTLED_INDUSTRY_HISTORY_LIMIT.saturating_sub(protected_failure_job_ids.len());
        while ordered_job_ids.len() > unprotected_limit {
            let (_, job_id) = ordered_job_ids.remove(0);
            self.product_validation_attempts.remove(&job_id);
            self.product_validation_receipts.remove(&job_id);
            self.recipe_completion_receipts.remove(&job_id);
            self.factory_production_failure_dispositions.remove(&job_id);
            self.industry_settlement_orders.remove(&job_id);
        }
    }

    pub(super) fn prepare_product_validation_blocker(
        &mut self,
        action_id: &ActionId,
        requester_agent_id: &str,
        factory_id: &str,
        recipe_id: &str,
        blocker_kind: &str,
        blocker_detail: &str,
    ) -> Result<bool, WorldError> {
        if let Some(existing) = self.factory_production_failure_dispositions.get(action_id) {
            if existing.requester_agent_id == requester_agent_id
                && existing.factory_id == factory_id
                && existing.recipe_id == recipe_id
                && existing.blocker_kind == blocker_kind
                && existing.blocker_detail == blocker_detail
            {
                return Ok(false);
            }
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "product-validation blocker conflicts with persisted disposition: job_id={action_id}"
                ),
            });
        }
        let Some(pending) = self.pending_recipe_jobs.get(action_id).cloned() else {
            return Ok(false);
        };
        if pending.requester_agent_id != requester_agent_id
            || pending.factory_id != factory_id
            || pending.recipe_id != recipe_id
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "product-validation blocker does not match pending commitment: job_id={action_id}"
                ),
            });
        }
        self.factory_production_failure_dispositions.insert(
            *action_id,
            FactoryProductionFailureDispositionV1 {
                action_id: *action_id,
                requester_agent_id: pending.requester_agent_id,
                factory_id: pending.factory_id,
                recipe_id: pending.recipe_id,
                blocker_kind: blocker_kind.to_string(),
                blocker_detail: blocker_detail.to_string(),
                disposition_kind: "consumed_lost".to_string(),
                consumed_inputs: pending.consume.clone(),
                lost_inputs: pending.consume,
                consumed_power: pending.power_required,
                lost_power: pending.power_required,
                next_action: "inspect_product_validation_and_reschedule".to_string(),
                next_recheck: None,
            },
        );
        Ok(true)
    }

    pub(super) fn apply_factory_build_started(
        &mut self,
        now: WorldTime,
        job_id: &ActionId,
        builder_agent_id: &str,
        site_id: &str,
        spec: &FactoryModuleSpec,
        consume_ledger: &MaterialLedgerId,
        ready_at: &WorldTime,
        contract_version: Option<u8>,
        site_authority_revision: Option<&u64>,
        site_location_id: Option<&str>,
        location_anchor_revision: Option<&u64>,
        construction_power_obligation: Option<&FactoryBuildPowerObligationV1>,
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

        // Keep an omitted version distinct from an explicitly classified
        // historical version zero. Do not let a wire omission turn an
        // otherwise authority-bound event into a legacy replay: the presence
        // of any modern admission fact is the structural discriminator.
        // Genuine historical events have none of these fields and continue
        // through the explicitly legacy path.
        let has_modern_admission_facts = site_authority_revision.is_some()
            || site_location_id.is_some()
            || location_anchor_revision.is_some()
            || construction_power_obligation.is_some();
        let modern_contract = match contract_version {
            None if has_modern_admission_facts => {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "modern factory build event is missing its contract discriminator: factory_id={}",
                        spec.factory_id
                    ),
                });
            }
            None | Some(0) => false,
            Some(FACTORY_BUILD_STARTED_MODERN_VERSION) => true,
            Some(unsupported) => {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "unsupported factory build event contract version: {unsupported}"
                    ),
                });
            }
        };
        if modern_contract
            && (site_authority_revision.is_none()
                || site_location_id.is_none()
                || location_anchor_revision.is_none()
                || construction_power_obligation.is_none())
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "modern factory build event is missing authority or construction facts: factory_id={}",
                    spec.factory_id
                ),
            });
        }

        // New authority-bound events carry all facts needed for deterministic
        // admission and replay. Legacy events are the only events allowed to
        // retain the historical material-only semantics.
        if let Some(obligation) = construction_power_obligation {
            let site_revision =
                site_authority_revision.ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: "factory build power obligation requires site authority revision"
                        .to_string(),
                })?;
            let site_location =
                site_location_id.ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: "factory build power obligation requires site location".to_string(),
                })?;
            let site = self.factory_site_authorities.get(site_id).ok_or_else(|| {
                WorldError::ResourceBalanceInvalid {
                    reason: format!("factory build site authority missing: site_id={site_id}"),
                }
            })?;
            let location = self
                .agent_location_authorities
                .get(builder_agent_id)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build builder location authority missing: builder_agent_id={builder_agent_id}"
                    ),
                })?;
            let anchor_matches = location_anchor_revision.map_or(!modern_contract, |revision| {
                self.location_anchors
                    .get(site_location)
                    .is_some_and(|anchor| {
                        anchor.location_id == site_location
                            && anchor.authority_revision == *revision
                            && anchor.active
                            && anchor.effective_at <= now
                    })
            });
            if site.site_id != site_id
                || location.agent_id != builder_agent_id
                || *site_revision != site.authority_revision
                || site_location != site.location_id
                || !site.active
                || !site.chunk_ready
                || !location.active
                || location.location_id != site.location_id
                || location.effective_at > now
                || !anchor_matches
                || (site.owner_agent_id != builder_agent_id
                    && !site
                        .authorized_agent_ids
                        .iter()
                        .any(|agent_id| agent_id == builder_agent_id))
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build authority changed or is unavailable: site_id={site_id} builder_agent_id={builder_agent_id}"
                    ),
                });
            }
            if obligation.payer_agent_id != builder_agent_id
                || obligation.profile_key != spec.factory_id
                || obligation.profile_revision == 0
                || obligation.electricity_amount < 0
                || obligation.mode != FactoryConstructionPowerMode::StartOnlySink
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build power obligation is invalid: factory_id={} builder_agent_id={}",
                        spec.factory_id, builder_agent_id
                    ),
                });
            }
            if modern_contract && obligation.material_ledger.as_ref() != Some(consume_ledger) {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "modern factory build construction ledger does not match consume ledger: factory_id={}",
                        spec.factory_id
                    ),
                });
            }
            if modern_contract {
                let Some(profile) = self.factory_profiles.get(spec.factory_id.as_str()) else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "modern factory build canonical profile missing: factory_id={}",
                            spec.factory_id
                        ),
                    });
                };
                if let Err(reason) = canonical_factory_profile_matches_spec(profile, spec) {
                    return Err(WorldError::ResourceBalanceInvalid { reason });
                }
            }
            let profile = self
                .factory_construction_power_profiles
                .get(spec.factory_id.as_str())
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build construction power profile missing: factory_id={}",
                        spec.factory_id
                    ),
                })?;
            if !profile.active
                || profile.authority_revision != obligation.profile_revision
                || profile.factory_id != spec.factory_id
                || profile.electricity_amount != obligation.electricity_amount
                || profile.mode != obligation.mode
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build construction power profile changed: factory_id={}",
                        spec.factory_id
                    ),
                });
            }
            let available_power = self
                .agents
                .get(builder_agent_id)
                .map(|cell| cell.state.resources.get(ResourceKind::Electricity))
                .unwrap_or(0);
            if available_power < obligation.electricity_amount {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build consume failed: insufficient construction electricity requested={} available={}",
                        obligation.electricity_amount, available_power
                    ),
                });
            }
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
        if let Some(obligation) = construction_power_obligation {
            if let (Some(before), Some(after)) =
                (obligation.electricity_before, obligation.electricity_after)
            {
                let available = self
                    .agents
                    .get(builder_agent_id)
                    .map(|cell| cell.state.resources.get(ResourceKind::Electricity))
                    .unwrap_or(0);
                let expected_after = before
                    .checked_sub(obligation.electricity_amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "factory build construction electricity facts underflow: factory_id={}",
                            spec.factory_id
                        ),
                    })?;
                if available != before || after != expected_after {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "factory build construction electricity facts mismatch: factory_id={} before={} current={} after={} expected_after={}",
                            spec.factory_id, before, available, after, expected_after
                        ),
                    });
                }
            } else if obligation.electricity_before.is_some()
                || obligation.electricity_after.is_some()
                || modern_contract
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build construction electricity facts must include before and after: factory_id={}",
                        spec.factory_id
                    ),
                });
            }
            if let (Some(before), Some(after)) = (
                obligation.material_balances_before.as_ref(),
                obligation.material_balances_after.as_ref(),
            ) {
                if before.len() != required_by_kind.len() || after.len() != required_by_kind.len() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "factory build material facts do not cover build cost: factory_id={}",
                            spec.factory_id
                        ),
                    });
                }
                for (kind, required) in &required_by_kind {
                    let current = self
                        .material_ledgers
                        .get(consume_ledger)
                        .and_then(|ledger| ledger.get(kind))
                        .copied()
                        .unwrap_or(0);
                    let Some(before_balance) = before.get(kind) else {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "factory build material facts missing before balance: factory_id={} kind={kind}",
                                spec.factory_id
                            ),
                        });
                    };
                    let Some(after_balance) = after.get(kind) else {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "factory build material facts missing after balance: factory_id={} kind={kind}",
                                spec.factory_id
                            ),
                        });
                    };
                    let expected_after = before_balance.checked_sub(*required).ok_or_else(
                        || WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "factory build material facts underflow: factory_id={} kind={kind}",
                                spec.factory_id
                            ),
                        },
                    )?;
                    if current != *before_balance || *after_balance != expected_after {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "factory build material facts mismatch: factory_id={} kind={kind} before={} current={} after={} expected_after={expected_after}",
                                spec.factory_id, before_balance, current, after_balance
                            ),
                        });
                    }
                }
            } else if obligation.material_balances_before.is_some()
                || obligation.material_balances_after.is_some()
                || modern_contract
            {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "factory build material facts must include before and after: factory_id={}",
                        spec.factory_id
                    ),
                });
            }
            if let Some(cell) = self.agents.get_mut(builder_agent_id) {
                cell.state
                    .resources
                    .remove(ResourceKind::Electricity, obligation.electricity_amount)
                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                        reason: format!("factory construction electricity debit failed: {err:?}"),
                    })?;
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
                contract_version: contract_version.unwrap_or(0),
                site_authority_revision: site_authority_revision.copied(),
                site_location_id: site_location_id.map(ToOwned::to_owned),
                location_anchor_revision: location_anchor_revision.copied(),
                construction_power_obligation: construction_power_obligation.cloned(),
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
                site_authority_revision: pending.site_authority_revision,
                site_location_id: pending.site_location_id.clone(),
                location_anchor_revision: pending.location_anchor_revision,
                construction_power_profile_key: pending
                    .construction_power_obligation
                    .as_ref()
                    .map(|obligation| obligation.profile_key.clone()),
                construction_power_profile_revision: pending
                    .construction_power_obligation
                    .as_ref()
                    .map(|obligation| obligation.profile_revision),
                built_at: now,
            },
        );
        if let Some(obligation) = pending.construction_power_obligation {
            self.factory_construction_receipts
                .insert(spec.factory_id.clone(), obligation);
        }
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
        durability_ppm: &i64,
    ) -> Result<(), WorldError> {
        let recycle_receipt = FactoryRecycleReceiptV1 {
            operator_agent_id: operator_agent_id.to_string(),
            factory_id: factory_id.to_string(),
            recycle_ledger: recycle_ledger.clone(),
            recovered: recovered.to_vec(),
            durability_ppm: *durability_ppm,
        };
        if let Some(existing) = self.factory_recycle_receipts.get(factory_id) {
            if existing == &recycle_receipt {
                return Ok(());
            }
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "factory recycle conflicts with persisted receipt: factory_id={factory_id}"
                ),
            });
        }
        if self.retired_factory_ids.contains(factory_id) {
            // Pre-receipt snapshots only have the tombstone. Preserve their
            // old idempotent replay behavior; new events always write the
            // full payload receipt below.
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
        self.factory_recycle_receipts
            .insert(factory_id.to_string(), recycle_receipt);
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

fn canonical_factory_profile_matches_spec(
    profile: &FactoryProfileV1,
    spec: &FactoryModuleSpec,
) -> Result<(), String> {
    let normalize = |tags: &[String]| {
        tags.iter()
            .map(|tag| tag.trim().to_ascii_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect::<BTreeSet<_>>()
    };
    if profile.factory_id != spec.factory_id {
        return Err(format!(
            "modern factory build canonical profile identity mismatch: profile={} spec={}",
            profile.factory_id, spec.factory_id
        ));
    }
    if profile.tier != spec.tier {
        return Err(format!(
            "modern factory build canonical profile tier mismatch: factory_id={} profile={} spec={}",
            spec.factory_id, profile.tier, spec.tier
        ));
    }
    if profile.recipe_slots != spec.recipe_slots {
        return Err(format!(
            "modern factory build canonical profile recipe_slots mismatch: factory_id={} profile={} spec={}",
            spec.factory_id, profile.recipe_slots, spec.recipe_slots
        ));
    }
    if normalize(&profile.tags) != normalize(&spec.tags) {
        return Err(format!(
            "modern factory build canonical profile tags mismatch: factory_id={}",
            spec.factory_id
        ));
    }
    Ok(())
}

pub(super) fn recipe_completion_receipt(
    job_id: &ActionId,
    requester_agent_id: &str,
    factory_id: &str,
    recipe_id: &str,
    accepted_batches: u32,
    produce: &[MaterialStack],
    byproducts: &[MaterialStack],
    output_ledger: &MaterialLedgerId,
    bottleneck_tags: &[String],
    logistics_route_ids: &[String],
    logistics_path_ids: &[String],
) -> RecipeCompletionReceiptV1 {
    RecipeCompletionReceiptV1 {
        job_id: *job_id,
        requester_agent_id: requester_agent_id.to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        accepted_batches,
        produce: produce.to_vec(),
        byproducts: byproducts.to_vec(),
        output_ledger: output_ledger.clone(),
        bottleneck_tags: bottleneck_tags.to_vec(),
        logistics_route_ids: logistics_route_ids.to_vec(),
        logistics_path_ids: logistics_path_ids.to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(job_id: ActionId) -> ProductValidationAttemptV1 {
        ProductValidationAttemptV1 {
            job_id,
            validation_index: Some(0),
            requester_agent_id: "builder-a".to_string(),
            module_id: "m4.product.test".to_string(),
            stack: MaterialStack::new("test_product", 1),
        }
    }

    fn receipt(job_id: ActionId) -> ProductValidationReceiptV1 {
        ProductValidationReceiptV1 {
            job_id,
            validation_index: Some(0),
            requester_agent_id: "builder-a".to_string(),
            module_id: "m4.product.test".to_string(),
            stack: MaterialStack::new("test_product", 1),
            decision: ProductValidationDecision::accepted("test_product", 1, true, Vec::new()),
            failure_detail: None,
        }
    }

    fn completion_receipt(job_id: ActionId) -> RecipeCompletionReceiptV1 {
        RecipeCompletionReceiptV1 {
            job_id,
            ..RecipeCompletionReceiptV1::default()
        }
    }

    fn failure_disposition(job_id: ActionId) -> FactoryProductionFailureDispositionV1 {
        FactoryProductionFailureDispositionV1 {
            action_id: job_id,
            requester_agent_id: "builder-a".to_string(),
            factory_id: "factory.test".to_string(),
            recipe_id: "recipe.test".to_string(),
            blocker_kind: "product_validation".to_string(),
            blocker_detail: "test rejection".to_string(),
            disposition_kind: "consumed_lost".to_string(),
            consumed_inputs: Vec::new(),
            lost_inputs: Vec::new(),
            consumed_power: 0,
            lost_power: 0,
            next_action: "retry".to_string(),
            next_recheck: None,
        }
    }

    #[test]
    fn settled_industry_history_is_bounded_and_restart_stable() {
        let mut state = WorldState::default();
        for job_id in 1..=96 {
            state.settled_recipe_job_ids.insert(job_id);
            state.industry_settlement_orders.insert(job_id, job_id);
            state
                .product_validation_attempts
                .insert(job_id, vec![attempt(job_id)]);
            state
                .product_validation_receipts
                .insert(job_id, vec![receipt(job_id)]);
            state
                .recipe_completion_receipts
                .insert(job_id, completion_receipt(job_id));
            state
                .factory_production_failure_dispositions
                .insert(job_id, failure_disposition(job_id));
        }
        let active_job_id = 10_000;
        state
            .product_validation_attempts
            .insert(active_job_id, vec![attempt(active_job_id)]);
        state
            .product_validation_receipts
            .insert(active_job_id, vec![receipt(active_job_id)]);

        state.compact_settled_industry_history();

        assert_eq!(state.product_validation_attempts.len(), 65);
        assert_eq!(state.product_validation_receipts.len(), 65);
        assert_eq!(state.recipe_completion_receipts.len(), 64);
        assert_eq!(state.factory_production_failure_dispositions.len(), 64);
        assert_eq!(state.industry_settlement_orders.len(), 64);
        assert!((1..=32).all(|job_id| {
            !state.product_validation_attempts.contains_key(&job_id)
                && !state.product_validation_receipts.contains_key(&job_id)
                && !state.recipe_completion_receipts.contains_key(&job_id)
                && !state
                    .factory_production_failure_dispositions
                    .contains_key(&job_id)
        }));
        assert!(
            state
                .product_validation_attempts
                .contains_key(&active_job_id)
        );
        assert!(
            state
                .product_validation_receipts
                .contains_key(&active_job_id)
        );

        let encoded = serde_json::to_vec(&state).expect("serialize compacted state");
        let restored: WorldState =
            serde_json::from_slice(&encoded).expect("restore compacted state");
        assert_eq!(
            restored, state,
            "restart must preserve compacted state exactly"
        );
    }

    #[test]
    fn settled_product_validation_replay_does_not_repopulate_compacted_history() {
        let job_id = 77;
        let mut state = WorldState::default();
        state.settled_recipe_job_ids.insert(job_id);
        state.compact_settled_industry_history();
        let next_order = state.next_industry_settlement_order;

        state
            .apply_domain_event(
                &DomainEvent::ProductValidationAttemptStarted {
                    attempt: attempt(job_id),
                },
                0,
            )
            .expect("settled attempt replay is a no-op");
        state
            .apply_domain_event(
                &DomainEvent::ProductValidationRecorded {
                    receipt: receipt(job_id),
                },
                0,
            )
            .expect("settled receipt replay is a no-op");
        state
            .apply_domain_event(
                &DomainEvent::RecipeCompleted {
                    job_id,
                    requester_agent_id: "builder-a".to_string(),
                    factory_id: "factory.test".to_string(),
                    recipe_id: "recipe.test".to_string(),
                    accepted_batches: 1,
                    produce: Vec::new(),
                    byproducts: Vec::new(),
                    output_ledger: MaterialLedgerId::world(),
                    bottleneck_tags: Vec::new(),
                    logistics_route_ids: Vec::new(),
                    logistics_path_ids: Vec::new(),
                },
                0,
            )
            .expect("settled completion replay is a no-op");
        state
            .apply_domain_event(
                &DomainEvent::FactoryProductionBlocked {
                    action_id: job_id,
                    requester_agent_id: "builder-a".to_string(),
                    factory_id: "factory.test".to_string(),
                    recipe_id: "recipe.test".to_string(),
                    blocker_kind: "product_validation".to_string(),
                    blocker_detail: "test rejection".to_string(),
                },
                0,
            )
            .expect("settled failure replay is a no-op");

        assert!(!state.product_validation_attempts.contains_key(&job_id));
        assert!(!state.product_validation_receipts.contains_key(&job_id));
        assert!(!state.recipe_completion_receipts.contains_key(&job_id));
        assert!(
            !state
                .factory_production_failure_dispositions
                .contains_key(&job_id)
        );
        assert_eq!(state.next_industry_settlement_order, next_order);
        assert!(state.industry_settlement_orders.is_empty());
    }
}
