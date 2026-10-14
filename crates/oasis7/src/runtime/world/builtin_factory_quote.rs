//! Conditional native construction preview, using admission on a private clone.
use super::*;
use crate::runtime::{
    Action, AgentDecisionCausalContextV1, AgentDelegationAuthorizationV1, DomainEvent,
    FactoryBuildPowerObligationV1, MaterialLedgerId, RejectReason, WorldEventBody,
    builtin_factory_spec,
};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::MaterialStack;

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

impl World {
    pub fn builtin_factory_quote(
        &self,
        agent: &str,
        site: &str,
        kind: &str,
        factory: &str,
    ) -> Result<NativeBuiltinFactoryQuote, WorldError> {
        let invalid = |reason: &str| WorldError::DistributedValidationFailed {
            reason: reason.into(),
        };
        let spec = builtin_factory_spec(kind, factory)
            .ok_or_else(|| invalid("unsupported builtin factory selection"))?;
        let action = Action::BuildFactory {
            builder_agent_id: agent.into(),
            site_id: site.into(),
            spec: spec.clone(),
        };
        let mut grants = self
            .agent_delegation_authorizations(agent)?
            .into_iter()
            .filter(|a| {
                a.grant.object_id == factory
                    && a.grant.action_kinds.iter().any(|v| v == "BuildFactory")
            });
        let mut authorization = grants
            .next()
            .ok_or_else(|| invalid("native factory grant required"))?;
        if grants.next().is_some() {
            return Err(invalid("native factory grant ambiguous"));
        }
        let mut clone = self.clone();
        let request = "builtin-factory-read-only-preview";
        clone.bind_agent_causal_decision(
            request,
            agent,
            &action,
            AgentDecisionCausalContextV1::default(),
        )?;
        clone.prepare_agent_delegation_commit(request, &action)?;
        let body = clone.action_to_event(&ActionEnvelope {
            id: 0,
            action: action.clone(),
            committed_recipe_origin: None,
        })?;
        let power = self
            .state
            .factory_construction_power_profiles
            .get(factory)
            .ok_or_else(|| invalid("construction power unavailable"))?;
        authorization.cost_units = u64::try_from(power.electricity_amount)
            .map_err(|_| invalid("invalid construction power"))?;
        let electricity_before = self
            .state
            .agents
            .get(agent)
            .ok_or_else(|| invalid("factory agent unavailable"))?
            .state
            .resources
            .get(ResourceKind::Electricity);
        let (rejection, preview, electricity_after) = match body {
            WorldEventBody::Domain(DomainEvent::FactoryBuildStarted {
                consume_ledger,
                ready_at,
                site_authority_revision: Some(site_authority_revision),
                site_location_id: Some(site_location_id),
                location_anchor_revision: Some(location_anchor_revision),
                construction_power_obligation: Some(construction_power_obligation),
                ..
            }) => {
                let duration_ticks = u32::try_from(
                    ready_at
                        .checked_sub(self.state.time)
                        .ok_or_else(|| invalid("invalid factory ready tick"))?,
                )
                .map_err(|_| invalid("factory duration overflow"))?;
                let after = construction_power_obligation.electricity_after;
                (
                    None,
                    Some(BuiltinFactoryStartPreview {
                        consume_ledger,
                        consume: spec.build_cost.clone(),
                        duration_ticks,
                        site_authority_revision,
                        site_location_id,
                        location_anchor_revision,
                        construction_power_obligation,
                    }),
                    after,
                )
            }
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
                (Some(reason), None, None)
            }
            _ => return Err(invalid("native factory preview unavailable")),
        };
        let output = NativeBuiltinFactoryQuote {
            action,
            quoted_at_tick: self.state.time,
            authorization,
            electricity_before,
            electricity_after,
            rejection,
            preview,
        };
        output
            .validate_selection(agent, site, kind, factory)
            .map_err(|_| invalid("native factory quote inconsistent"))?;
        Ok(output)
    }
}
