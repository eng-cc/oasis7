impl WorldKernel {
    pub fn quote_schedule_recipe(
        &self,
        owner: &ResourceOwner,
        factory_id: &str,
        recipe_id: &str,
        batches: i64,
    ) -> Result<ScheduleQuote, RejectReason> {
        if recipe_id.trim().is_empty() {
            return Err(RejectReason::RuleDenied {
                notes: vec!["recipe_id cannot be empty".to_string()],
            });
        }
        if batches <= 0 {
            return Err(RejectReason::InvalidAmount { amount: batches });
        }
        self.ensure_owner_exists(owner)?;

        let Some(factory) = self.model.factories.get(factory_id) else {
            return Err(RejectReason::FacilityNotFound {
                facility_id: factory_id.to_string(),
            });
        };
        if factory.owner != *owner {
            return Err(RejectReason::RuleDenied {
                notes: vec!["factory owner mismatch".to_string()],
            });
        }
        let site_owner = ResourceOwner::Location {
            location_id: factory.location_id.clone(),
        };
        self.ensure_colocated(owner, &site_owner)?;

        let Some(plan) = self.recipe_plan(recipe_id) else {
            return Err(RejectReason::RuleDenied {
                notes: vec![format!("unsupported recipe_id: {recipe_id}")],
            });
        };
        if !factory.kind.eq_ignore_ascii_case(plan.required_factory_kind) {
            return Err(RejectReason::RuleDenied {
                notes: vec![format!(
                    "recipe {recipe_id} requires factory kind {}, got {}",
                    plan.required_factory_kind, factory.kind
                )],
            });
        }

        let electricity_cost = plan.electricity_per_batch.saturating_mul(batches);
        let hardware_cost = plan.hardware_per_batch.saturating_mul(batches);
        let data_output = plan.data_output_per_batch.saturating_mul(batches);
        let finished_product_units = plan
            .finished_product_units_per_batch
            .saturating_mul(batches);

        let stock = self
            .owner_stock(owner)
            .expect("owner exists after ensure_owner_exists");
        let available_electricity = stock.get(ResourceKind::Electricity);
        if available_electricity < electricity_cost {
            return Err(RejectReason::InsufficientResource {
                owner: owner.clone(),
                kind: ResourceKind::Electricity,
                requested: electricity_cost,
                available: available_electricity,
            });
        }
        let available_hardware = stock.get(ResourceKind::Data);
        if available_hardware < hardware_cost {
            return Err(RejectReason::InsufficientResource {
                owner: owner.clone(),
                kind: ResourceKind::Data,
                requested: hardware_cost,
                available: available_hardware,
            });
        }
        if data_output > 0 {
            let data_after_cost =
                available_hardware
                    .checked_sub(hardware_cost)
                    .ok_or(RejectReason::InsufficientResource {
                        owner: owner.clone(),
                        kind: ResourceKind::Data,
                        requested: hardware_cost,
                        available: available_hardware,
                    })?;
            if data_after_cost.checked_add(data_output).is_none() {
                return Err(RejectReason::InvalidAmount {
                    amount: data_output,
                });
            }
        }

        Ok(ScheduleQuote {
            owner: owner.clone(),
            factory_id: factory_id.to_string(),
            recipe_id: recipe_id.to_string(),
            batches,
            base_duration_ticks: batches,
            electricity_cost,
            hardware_cost,
            data_output,
            finished_product_id: plan.finished_product_id.to_string(),
            finished_product_units,
            local_shortage_delay_ticks: 0,
            shortage_reason: "none".to_string(),
            recommended_pre_step: "schedule_now".to_string(),
            runway_before_ticks: 0,
            runway_after_ticks: 0,
            downtime_threshold_ppm: 0,
            maintenance_pressure_delta: "none".to_string(),
            recommended_maintenance_action: "continue_production".to_string(),
        })
    }
}
