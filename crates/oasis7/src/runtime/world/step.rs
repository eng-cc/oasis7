use oasis7_wasm_abi::ModuleSandbox;

use super::super::{
    Action, ActionEnvelope, CausedBy, DomainEvent, FactoryProductionStatus,
    ModuleSubscriptionStage, RejectReason, RuleVerdict, WorldError, WorldEvent, WorldEventBody,
    WorldTime,
};
use super::economy::EconomyActionResolution;
use super::World;
use crate::simulator::ResourceKind;

#[derive(Debug, Clone, Default)]
struct FactoryProductionFollowupContext {
    active_jobs: u16,
    current_status: Option<FactoryProductionStatus>,
    blocked_at: Option<WorldTime>,
    blocker_kind: Option<String>,
    blocker_detail: Option<String>,
}

impl World {
    fn should_emit_action_accepted(action: &Action) -> bool {
        matches!(
            action,
            Action::FormAlliance { .. }
                | Action::JoinAlliance { .. }
                | Action::LeaveAlliance { .. }
                | Action::DissolveAlliance { .. }
                | Action::DeclareWar { .. }
                | Action::OpenGovernanceProposal { .. }
                | Action::CastGovernanceVote { .. }
                | Action::ResolveCrisis { .. }
                | Action::GrantMetaProgress { .. }
                | Action::UpdateGameplayPolicy { .. }
                | Action::UpdateRestrictedStarterClaimAdminRegistry { .. }
                | Action::OpenEconomicContract { .. }
                | Action::AcceptEconomicContract { .. }
                | Action::SettleEconomicContract { .. }
                | Action::ClaimAgent { .. }
                | Action::ReleaseAgentClaim { .. }
                | Action::SubmitFirstAgentClaimApprovalRequest { .. }
                | Action::ApproveFirstAgentClaimApprovalRequest { .. }
                | Action::RejectFirstAgentClaimApprovalRequest { .. }
                | Action::IssueRestrictedStarterClaimGrant { .. }
                | Action::RevokeRestrictedStarterClaimGrant { .. }
        )
    }

    fn append_action_accepted_event(
        &mut self,
        envelope: &ActionEnvelope,
    ) -> Result<(), WorldError> {
        if !Self::should_emit_action_accepted(&envelope.action) {
            return Ok(());
        }
        let actor_id = envelope.action.actor_id().unwrap_or("system");
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ActionAccepted {
                action_id: envelope.id,
                action_kind: super::module_runtime_labels::action_kind_label(&envelope.action)
                    .to_string(),
                actor_id: actor_id.to_string(),
                eta_ticks: 0,
                notes: vec!["accepted_for_gameplay_processing".to_string()],
            }),
            Some(CausedBy::Action(envelope.id)),
        )?;
        Ok(())
    }

    fn preflight_domain_event(&self, body: &WorldEventBody) -> Result<(), WorldError> {
        let WorldEventBody::Domain(event) = body else {
            return Ok(());
        };
        let mut preview_state = self.state.clone();
        preview_state.apply_domain_event(event, self.state.time)
    }

    fn factory_production_followup_context(
        &self,
        envelope: &ActionEnvelope,
    ) -> Option<FactoryProductionFollowupContext> {
        let Action::ScheduleRecipe { factory_id, .. } = &envelope.action else {
            return None;
        };
        self.state
            .factories
            .get(factory_id)
            .map(|factory| FactoryProductionFollowupContext {
                active_jobs: factory.production.active_jobs,
                current_status: Some(factory.production.status),
                blocked_at: factory.production.last_blocked_at,
                blocker_kind: factory.production.current_blocker_kind.clone(),
                blocker_detail: factory.production.current_blocker_detail.clone(),
            })
    }

    fn classify_factory_production_block(reason: &RejectReason) -> Option<(String, String)> {
        match reason {
            RejectReason::InsufficientMaterial { material_kind, .. } => Some((
                "material_shortage".to_string(),
                format!("material_shortage:{material_kind}"),
            )),
            RejectReason::InsufficientResource { kind, .. }
                if *kind == ResourceKind::Electricity =>
            {
                Some((
                    "power_shortage".to_string(),
                    "power_shortage:electricity".to_string(),
                ))
            }
            RejectReason::InsufficientResources { deficits }
                if deficits.contains_key(&ResourceKind::Electricity) =>
            {
                Some((
                    "power_shortage".to_string(),
                    "power_shortage:electricity".to_string(),
                ))
            }
            RejectReason::RuleDenied { notes } => {
                let joined = notes.join(" | ");
                let lowercase = joined.to_ascii_lowercase();
                if lowercase.contains("stage gate denied")
                    || lowercase.contains("unlock_stage denied")
                    || lowercase.contains("preferred_factory_tags mismatch")
                    || lowercase.contains("recipe module denied")
                    || lowercase.contains("recipe plan rejected")
                {
                    return Some(("governance_gate".to_string(), joined));
                }
                None
            }
            _ => None,
        }
    }

    fn derive_factory_production_followup_event(
        &self,
        envelope: &ActionEnvelope,
        event_body: &WorldEventBody,
        context: Option<&FactoryProductionFollowupContext>,
    ) -> Option<DomainEvent> {
        match (&envelope.action, event_body) {
            (
                Action::ScheduleRecipe {
                    requester_agent_id,
                    factory_id,
                    recipe_id,
                    ..
                },
                WorldEventBody::Domain(DomainEvent::ActionRejected { action_id, reason }),
            ) => {
                let context = context?;
                if context.active_jobs > 0 {
                    return None;
                }
                let (blocker_kind, blocker_detail) =
                    Self::classify_factory_production_block(reason)?;
                Some(DomainEvent::FactoryProductionBlocked {
                    action_id: *action_id,
                    requester_agent_id: requester_agent_id.clone(),
                    factory_id: factory_id.clone(),
                    recipe_id: recipe_id.clone(),
                    blocker_kind,
                    blocker_detail,
                })
            }
            (
                Action::ScheduleRecipe {
                    requester_agent_id, ..
                },
                WorldEventBody::Domain(DomainEvent::RecipeStarted {
                    job_id,
                    factory_id,
                    recipe_id,
                    ..
                }),
            ) => {
                let context = context?;
                if context.current_status != Some(FactoryProductionStatus::Blocked) {
                    return None;
                }
                Some(DomainEvent::FactoryProductionResumed {
                    job_id: *job_id,
                    requester_agent_id: requester_agent_id.clone(),
                    factory_id: factory_id.clone(),
                    recipe_id: recipe_id.clone(),
                    previous_blocked_at: context.blocked_at,
                    previous_blocker_kind: context.blocker_kind.clone(),
                    previous_blocker_detail: context.blocker_detail.clone(),
                })
            }
            _ => None,
        }
    }

    fn append_factory_production_followup_event(
        &mut self,
        envelope: &ActionEnvelope,
        event_body: &WorldEventBody,
        context: Option<&FactoryProductionFollowupContext>,
    ) -> Result<Option<WorldEvent>, WorldError> {
        let Some(event) =
            self.derive_factory_production_followup_event(envelope, event_body, context)
        else {
            return Ok(None);
        };
        let body = WorldEventBody::Domain(event);
        self.preflight_domain_event(&body)?;
        self.append_event(body, Some(CausedBy::Action(envelope.id)))?;
        Ok(self.journal.events.last().cloned())
    }

    // ---------------------------------------------------------------------
    // Simulation step
    // ---------------------------------------------------------------------

    pub fn step(&mut self) -> Result<(), WorldError> {
        self.state.time = self.state.time.saturating_add(1);
        let _ = self.process_factory_depreciation()?;
        while let Some(envelope) = self.pending_actions.pop_front() {
            if self.try_apply_runtime_module_action(&envelope)? {
                continue;
            }
            let followup_context = self.factory_production_followup_context(&envelope);
            let event_body = self.action_to_event(&envelope)?;
            self.preflight_domain_event(&event_body)?;
            self.append_action_accepted_event(&envelope)?;
            self.append_event(event_body.clone(), Some(CausedBy::Action(envelope.id)))?;
            let _ = self.append_factory_production_followup_event(
                &envelope,
                &event_body,
                followup_context.as_ref(),
            )?;
        }
        let _ = self.process_due_economy_jobs()?;
        let _ = self.process_due_material_transits()?;
        let _ = self.process_gameplay_cycles()?;
        let _ = self.process_restricted_starter_claim_grant_epochs()?;
        let _ = self.process_agent_claim_epochs()?;
        self.refresh_threat_heatmap();
        self.record_tick_consensus()?;
        Ok(())
    }

    pub fn step_with_modules(&mut self, sandbox: &mut dyn ModuleSandbox) -> Result<(), WorldError> {
        self.state.time = self.state.time.saturating_add(1);
        for event in self.process_factory_depreciation()? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        while let Some(envelope) = self.pending_actions.pop_front() {
            let mut action_envelope = envelope.clone();
            let mut post_action_result_event: Option<WorldEvent> = None;
            match self.resolve_module_backed_economy_action(&envelope, sandbox)? {
                EconomyActionResolution::Resolved(action) => {
                    action_envelope.action = action;
                }
                EconomyActionResolution::Rejected(reason) => {
                    let followup_context = self.factory_production_followup_context(&envelope);
                    self.append_action_accepted_event(&envelope)?;
                    let rejection_body =
                        WorldEventBody::Domain(super::super::DomainEvent::ActionRejected {
                            action_id: envelope.id,
                            reason,
                        });
                    self.append_event(rejection_body.clone(), Some(CausedBy::Action(envelope.id)))?;
                    post_action_result_event = self.journal.events.last().cloned();
                    self.route_action_to_modules_with_stage_and_event(
                        &envelope,
                        ModuleSubscriptionStage::PostAction,
                        post_action_result_event.as_ref(),
                        sandbox,
                    )?;
                    if let Some(event) = self.journal.events.last() {
                        let event = event.clone();
                        self.route_event_to_modules(&event, sandbox)?;
                    }
                    if let Some(event) = self.append_factory_production_followup_event(
                        &envelope,
                        &rejection_body,
                        followup_context.as_ref(),
                    )? {
                        self.route_event_to_modules(&event, sandbox)?;
                    }
                    continue;
                }
            }

            let decision = self.evaluate_rule_decisions(&action_envelope, sandbox)?;
            if decision.verdict == RuleVerdict::Modify {
                if let Some(override_action) = decision.override_action.clone() {
                    self.record_action_override(
                        super::super::ActionOverrideRecord {
                            action_id: envelope.id,
                            original_action: envelope.action.clone(),
                            override_action: override_action.clone(),
                        },
                        Some(CausedBy::Action(envelope.id)),
                    )?;
                    action_envelope = ActionEnvelope {
                        id: envelope.id,
                        action: override_action,
                    };
                }
            }

            if decision.verdict == RuleVerdict::Deny {
                self.append_action_accepted_event(&envelope)?;
                self.append_event(
                    WorldEventBody::Domain(super::super::DomainEvent::ActionRejected {
                        action_id: envelope.id,
                        reason: RejectReason::RuleDenied {
                            notes: decision.notes.clone(),
                        },
                    }),
                    Some(CausedBy::Action(envelope.id)),
                )?;
                post_action_result_event = self.journal.events.last().cloned();
            } else {
                let deficits = decision.cost.deficits(&self.state.resources);
                if !deficits.is_empty() {
                    self.append_action_accepted_event(&envelope)?;
                    self.append_event(
                        WorldEventBody::Domain(super::super::DomainEvent::ActionRejected {
                            action_id: envelope.id,
                            reason: RejectReason::InsufficientResources { deficits },
                        }),
                        Some(CausedBy::Action(envelope.id)),
                    )?;
                    post_action_result_event = self.journal.events.last().cloned();
                } else {
                    match self.apply_resource_delta(&decision.cost) {
                        Ok(()) => {
                            if !self.try_apply_runtime_module_action(&action_envelope)? {
                                let followup_context =
                                    self.factory_production_followup_context(&envelope);
                                let event_body = self.action_to_event(&action_envelope)?;
                                self.preflight_domain_event(&event_body)?;
                                self.append_action_accepted_event(&envelope)?;
                                self.append_event(
                                    event_body.clone(),
                                    Some(CausedBy::Action(envelope.id)),
                                )?;
                                post_action_result_event = self.journal.events.last().cloned();
                                if let Some(event) = self.journal.events.last() {
                                    let event = event.clone();
                                    self.route_event_to_modules(&event, sandbox)?;
                                }
                                if let Some(event) = self.append_factory_production_followup_event(
                                    &envelope,
                                    &event_body,
                                    followup_context.as_ref(),
                                )? {
                                    self.route_event_to_modules(&event, sandbox)?;
                                }
                            }
                        }
                        Err(err) => {
                            self.append_action_accepted_event(&envelope)?;
                            self.append_event(
                                WorldEventBody::Domain(super::super::DomainEvent::ActionRejected {
                                    action_id: envelope.id,
                                    reason: RejectReason::RuleDenied {
                                        notes: vec![format!(
                                            "rule decision cost apply rejected: {err:?}"
                                        )],
                                    },
                                }),
                                Some(CausedBy::Action(envelope.id)),
                            )?;
                            post_action_result_event = self.journal.events.last().cloned();
                        }
                    }
                }
            }

            self.route_action_to_modules_with_stage_and_event(
                &action_envelope,
                ModuleSubscriptionStage::PostAction,
                post_action_result_event.as_ref(),
                sandbox,
            )?;
            if let Some(event) = self.journal.events.last() {
                let event = event.clone();
                self.route_event_to_modules(&event, sandbox)?;
            }
        }
        for event in self.process_due_economy_jobs_with_modules(sandbox)? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        for event in self.process_due_material_transits()? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        for event in self.process_gameplay_cycles_with_modules(sandbox)? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        for event in self.process_restricted_starter_claim_grant_epochs()? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        for event in self.process_agent_claim_epochs()? {
            self.route_event_to_modules(&event, sandbox)?;
        }
        self.refresh_threat_heatmap();
        self.record_tick_consensus()?;
        Ok(())
    }
}
