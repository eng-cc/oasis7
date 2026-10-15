use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltinRecipeStartPreview {
    pub consume_ledger: MaterialLedgerId,
    pub output_ledger: MaterialLedgerId,
    pub consume: Vec<MaterialStack>,
    pub produce: Vec<MaterialStack>,
    pub byproducts: Vec<MaterialStack>,
    pub power_required: i64,
    pub duration_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBuiltinRecipeQuote {
    pub action: Action,
    pub quoted_at_tick: u64,
    pub authorization: AgentDelegationAuthorizationV1,
    pub electricity_before: i64,
    pub electricity_after: Option<i64>,
    pub rejection: Option<RejectReason>,
    pub preview: Option<BuiltinRecipeStartPreview>,
}

impl NativeBuiltinRecipeQuote {
    pub fn validate_selection(
        &self,
        agent: &str,
        factory: &str,
        recipe: &str,
        batches: u32,
    ) -> Result<(), String> {
        let plan =
            builtin_recipe_plan(recipe, batches).ok_or("unsupported builtin recipe selection")?;
        let expected = Action::ScheduleRecipe {
            requester_agent_id: agent.into(),
            factory_id: factory.into(),
            recipe_id: recipe.into(),
            plan: plan.clone(),
            logistics_route_ids: vec![],
            logistics_path_ids: vec![],
        };
        let a = &self.authorization;
        if self.action != expected
            || a.grant.agent_id != agent
            || a.grant.object_id != factory
            || !a.grant.action_kinds.iter().any(|k| k == "ScheduleRecipe")
            || a.grant.revoked
            || self.quoted_at_tick < a.grant.valid_from_tick
            || self.quoted_at_tick > a.grant.valid_until_tick
            || a.cost_units
                != u64::try_from(plan.power_required).map_err(|_| "invalid canonical power")?
            || a.spent_units.checked_add(a.remaining_units) != Some(a.grant.limit_units)
            || a.remaining_units < a.cost_units
            || self.electricity_before < 0
        {
            return Err("native recipe quote selection or authority mismatch".into());
        }
        match (&self.rejection, &self.preview, self.electricity_after) {
            (None, Some(preview), Some(after))
                if after >= 0
                    && self.electricity_before.checked_sub(plan.power_required) == Some(after)
                    && preview.produce == plan.produce
                    && preview.byproducts == plan.byproducts
                    && preview.power_required == plan.power_required
                    && preview.duration_ticks >= plan.duration_ticks.max(1) =>
            {
                Ok(())
            }
            (Some(_), None, None) => Ok(()),
            _ => Err("native recipe quote projection mismatch".into()),
        }
    }
}
