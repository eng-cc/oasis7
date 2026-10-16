use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltinFactoryStartPreview {
    pub consume_ledger: MaterialLedgerId,
    pub consume: Vec<MaterialStack>,
    pub duration_ticks: u32,
    pub site_authority_revision: u64,
    pub site_location_id: String,
    pub location_anchor_revision: u64,
    pub construction_power_obligation: FactoryBuildPowerObligationV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBuiltinFactoryQuote {
    pub action: Action,
    pub quoted_at_tick: u64,
    pub authorization: AgentDelegationAuthorizationV1,
    pub electricity_before: i64,
    pub electricity_after: Option<i64>,
    pub rejection: Option<RejectReason>,
    pub preview: Option<BuiltinFactoryStartPreview>,
}

impl NativeBuiltinFactoryQuote {
    pub fn validate_selection(
        &self,
        agent: &str,
        site: &str,
        kind: &str,
        factory: &str,
    ) -> Result<(), String> {
        let spec =
            builtin_factory_spec(kind, factory).ok_or("unsupported builtin factory selection")?;
        let expected = Action::BuildFactory {
            builder_agent_id: agent.into(),
            site_id: site.into(),
            spec: spec.clone(),
        };
        let a = &self.authorization;
        if self.action != expected
            || a.grant.agent_id != agent
            || a.grant.object_id != factory
            || !a.grant.action_kinds.iter().any(|v| v == "BuildFactory")
            || a.grant.revoked
            || self.quoted_at_tick < a.grant.valid_from_tick
            || self.quoted_at_tick > a.grant.valid_until_tick
            || a.spent_units.checked_add(a.remaining_units) != Some(a.grant.limit_units)
            || a.remaining_units < a.cost_units
            || self.electricity_before < 0
        {
            return Err("native factory quote selection or authority mismatch".into());
        }
        match (&self.rejection, &self.preview, self.electricity_after) {
            (Some(_), None, None) => Ok(()),
            (None, Some(p), Some(after)) => {
                let power = &p.construction_power_obligation;
                let amount =
                    i64::try_from(a.cost_units).map_err(|_| "invalid construction cost")?;
                if after < 0
                    || self.electricity_before.checked_sub(amount) != Some(after)
                    || p.consume_ledger != MaterialLedgerId::agent(agent)
                    || p.consume != spec.build_cost
                    || p.duration_ticks != spec.build_time_ticks.max(1)
                    || p.site_authority_revision == 0
                    || p.location_anchor_revision == 0
                    || p.site_location_id.trim().is_empty()
                    || power.payer_agent_id != agent
                    || power.profile_key != factory
                    || power.profile_revision == 0
                    || power.electricity_amount != amount
                    || power.electricity_before != Some(self.electricity_before)
                    || power.electricity_after != Some(after)
                    || power.material_ledger.as_ref() != Some(&p.consume_ledger)
                {
                    return Err("native factory quote projection mismatch".into());
                }
                let before = power
                    .material_balances_before
                    .as_ref()
                    .ok_or("missing construction material facts")?;
                let after = power
                    .material_balances_after
                    .as_ref()
                    .ok_or("missing construction material facts")?;
                if before.len() != spec.build_cost.len() || after.len() != spec.build_cost.len() {
                    return Err("construction material facts scope mismatch".into());
                }
                for material in &spec.build_cost {
                    let available = before
                        .get(&material.kind)
                        .ok_or("missing required construction material")?;
                    if *available < material.amount
                        || available.checked_sub(material.amount)
                            != after.get(&material.kind).copied()
                    {
                        return Err("construction material facts mismatch".into());
                    }
                }
                Ok(())
            }
            _ => Err("native factory quote projection mismatch".into()),
        }
    }
}
