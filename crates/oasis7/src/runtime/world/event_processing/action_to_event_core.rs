use super::*;
use crate::runtime::main_token::{
    is_main_token_treasury_distribution_bucket, MainTokenTreasuryDistribution,
    RestrictedStarterClaimGrantStatus, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
    RESTRICTED_STARTER_CLAIM_GRANT_SPEND_SCOPE_SLOT_1_ONLY,
};
use std::collections::BTreeSet;

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

impl World {
    fn ensure_restricted_starter_claim_admin(&self, issuer_account_id: &str) -> Result<(), String> {
        let registry = self
            .governance_main_token_controller_registry()
            .ok_or_else(|| "restricted grant admin registry is not configured".to_string())?;
        if registry
            .restricted_starter_claim_admin_account_ids
            .is_empty()
        {
            return Err("restricted grant admin registry is empty".to_string());
        }
        if !registry
            .restricted_starter_claim_admin_account_ids
            .contains(issuer_account_id)
        {
            return Err(format!(
                "restricted grant issuer is not allowlisted admin: issuer_account_id={issuer_account_id}"
            ));
        }
        if !registry
            .controller_signer_policies
            .contains_key(issuer_account_id)
        {
            return Err(format!(
                "restricted grant admin signer policy is not configured: issuer_account_id={issuer_account_id}"
            ));
        }
        Ok(())
    }

    pub(super) fn action_to_event_core(
        &self,
        action_id: ActionId,
        action: &Action,
    ) -> Result<WorldEventBody, WorldError> {
        match action {
            Action::RegisterAgent { agent_id, pos } => {
                if self.state.agents.contains_key(agent_id) {
                    Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentAlreadyExists {
                            agent_id: agent_id.clone(),
                        },
                    }))
                } else {
                    Ok(WorldEventBody::Domain(DomainEvent::AgentRegistered {
                        agent_id: agent_id.clone(),
                        pos: *pos,
                    }))
                }
            }
            Action::MoveAgent { agent_id, to } => match self.state.agents.get(agent_id) {
                Some(cell) => {
                    let target_location_id = format!(
                        "{}:{}:{}",
                        to.x_cm.round() as i64,
                        to.y_cm.round() as i64,
                        to.z_cm.round() as i64
                    );
                    if self
                        .state
                        .gameplay_policy
                        .forbidden_location_ids
                        .iter()
                        .any(|value| value == &target_location_id)
                    {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "move denied by gameplay forbidden_location_ids: {target_location_id}"
                                )],
                            },
                        }));
                    }
                    Ok(WorldEventBody::Domain(DomainEvent::AgentMoved {
                        agent_id: agent_id.clone(),
                        from: cell.state.pos,
                        to: *to,
                    }))
                }
                None => Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::AgentNotFound {
                        agent_id: agent_id.clone(),
                    },
                })),
            },
            Action::QueryObservation { agent_id } => {
                if self.state.agents.contains_key(agent_id) {
                    Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["observation requires rule module".to_string()],
                        },
                    }))
                } else {
                    Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: agent_id.clone(),
                        },
                    }))
                }
            }
            Action::EmitObservation { observation } => {
                Ok(WorldEventBody::Domain(DomainEvent::Observation {
                    observation: observation.clone(),
                }))
            }
            Action::BodyAction { agent_id, .. } => {
                if self.state.agents.contains_key(agent_id) {
                    Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["body action requires body module".to_string()],
                        },
                    }))
                } else {
                    Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: agent_id.clone(),
                        },
                    }))
                }
            }
            Action::EmitBodyAttributes {
                agent_id,
                view,
                reason,
            } => {
                let Some(cell) = self.state.agents.get(agent_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: agent_id.clone(),
                        },
                    }));
                };
                if let Err(reason) = validate_body_kernel_view(&cell.state.body_view, view) {
                    return Ok(WorldEventBody::Domain(
                        DomainEvent::BodyAttributesRejected {
                            agent_id: agent_id.clone(),
                            reason,
                        },
                    ));
                }
                Ok(WorldEventBody::Domain(DomainEvent::BodyAttributesUpdated {
                    agent_id: agent_id.clone(),
                    view: view.clone(),
                    reason: reason.clone(),
                }))
            }
            Action::ExpandBodyInterface {
                agent_id,
                interface_module_item_id,
            } => Ok(evaluate_expand_body_interface(
                self,
                action_id,
                agent_id,
                interface_module_item_id,
            )),
            Action::DeployModuleArtifact { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "deploy_module_artifact requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::CompileModuleArtifactFromSource { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "compile_module_artifact_from_source requires runtime action loop"
                                .to_string(),
                        ],
                    },
                }))
            }
            Action::InstallModuleFromArtifact { .. }
            | Action::InstallModuleFromArtifactWithFinality { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "install_module_from_artifact requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::InstallModuleToTargetFromArtifact { .. }
            | Action::InstallModuleToTargetFromArtifactWithFinality { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "install_module_to_target_from_artifact requires runtime action loop"
                                .to_string(),
                        ],
                    },
                }))
            }
            Action::UpgradeModuleFromArtifact { .. }
            | Action::UpgradeModuleFromArtifactWithFinality { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "upgrade_module_from_artifact requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::RollbackModuleInstance { .. }
            | Action::RollbackModuleInstanceWithFinality { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "rollback_module_instance requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseSubmit { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "module_release_submit requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseShadow { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "module_release_shadow requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseApproveRole { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "module_release_approve_role requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseBindRoles { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "module_release_bind_roles requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseReject { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "module_release_reject requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::ModuleReleaseApply { .. } | Action::ModuleReleaseApplyWithFinality { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["module_release_apply requires runtime action loop".to_string()],
                    },
                }))
            }
            Action::ListModuleArtifactForSale { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["list_module_artifact_for_sale requires runtime action loop"
                            .to_string()],
                    },
                }))
            }
            Action::BuyModuleArtifact { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["buy_module_artifact requires runtime action loop".to_string()],
                    },
                }))
            }
            Action::DelistModuleArtifact { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "delist_module_artifact requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::DestroyModuleArtifact { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "destroy_module_artifact requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::PlaceModuleArtifactBid { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "place_module_artifact_bid requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::CancelModuleArtifactBid { .. } => {
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "cancel_module_artifact_bid requires runtime action loop".to_string()
                        ],
                    },
                }))
            }
            Action::TransferResource {
                from_agent_id,
                to_agent_id,
                ..
            } => {
                if !self.state.agents.contains_key(from_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: from_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(to_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: to_agent_id.clone(),
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["transfer requires rule module".to_string()],
                    },
                }))
            }
            Action::RedeemPower {
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
            } => Ok(WorldEventBody::Domain(self.evaluate_redeem_power_action(
                node_id.as_str(),
                target_agent_id.as_str(),
                *redeem_credits,
                *nonce,
                None,
            ))),
            Action::RedeemPowerSigned {
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                signer_node_id,
                signature,
            } => Ok(WorldEventBody::Domain(self.evaluate_redeem_power_action(
                node_id.as_str(),
                target_agent_id.as_str(),
                *redeem_credits,
                *nonce,
                Some((signer_node_id.as_str(), signature.as_str())),
            ))),
            Action::ApplyNodePointsSettlementSigned {
                report,
                signer_node_id,
                mint_records,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_apply_node_points_settlement_action(
                    action_id,
                    report,
                    signer_node_id.as_str(),
                    mint_records.as_slice(),
                ),
            )),
            Action::InitializeMainTokenGenesis { allocations } => Ok(WorldEventBody::Domain(
                self.evaluate_initialize_main_token_genesis_action(
                    action_id,
                    allocations.as_slice(),
                ),
            )),
            Action::ClaimMainTokenVesting {
                bucket_id,
                beneficiary,
                nonce,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_claim_main_token_vesting_action(
                    action_id,
                    bucket_id.as_str(),
                    beneficiary.as_str(),
                    *nonce,
                ),
            )),
            Action::TransferMainToken {
                from_account_id,
                to_account_id,
                amount,
                nonce,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_transfer_main_token_action(
                    action_id,
                    from_account_id.as_str(),
                    to_account_id.as_str(),
                    *amount,
                    *nonce,
                ),
            )),
            Action::ApplyMainTokenEpochIssuance {
                epoch_index,
                actual_stake_ratio_bps,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_apply_main_token_epoch_issuance_action(
                    action_id,
                    *epoch_index,
                    *actual_stake_ratio_bps,
                ),
            )),
            Action::SettleMainTokenFee { fee_kind, amount } => Ok(WorldEventBody::Domain(
                self.evaluate_settle_main_token_fee_action(action_id, *fee_kind, *amount),
            )),
            Action::UpdateMainTokenPolicy { proposal_id, next } => Ok(WorldEventBody::Domain(
                self.evaluate_update_main_token_policy_action(action_id, *proposal_id, next),
            )),
            Action::DistributeMainTokenTreasury {
                proposal_id,
                distribution_id,
                bucket_id,
                distributions,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_distribute_main_token_treasury_action(
                    action_id,
                    *proposal_id,
                    distribution_id.as_str(),
                    bucket_id.as_str(),
                    distributions.as_slice(),
                ),
            )),
            Action::TopUpRestrictedStarterClaimLiveopsPool {
                controller_account_id,
                top_up_id,
                amount,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_top_up_restricted_starter_claim_liveops_pool_action(
                    action_id,
                    controller_account_id.as_str(),
                    top_up_id.as_str(),
                    *amount,
                ),
            )),
            Action::IssueRestrictedStarterClaimGrant {
                issuer_account_id,
                beneficiary_account_id,
                amount,
                issuance_reason,
                expires_at_epoch,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_issue_restricted_starter_claim_grant_action(
                    action_id,
                    issuer_account_id.as_str(),
                    beneficiary_account_id.as_str(),
                    *amount,
                    issuance_reason.as_str(),
                    *expires_at_epoch,
                ),
            )),
            Action::RevokeRestrictedStarterClaimGrant {
                issuer_account_id,
                beneficiary_account_id,
                revoke_reason,
            } => Ok(WorldEventBody::Domain(
                self.evaluate_revoke_restricted_starter_claim_grant_action(
                    action_id,
                    issuer_account_id.as_str(),
                    beneficiary_account_id.as_str(),
                    revoke_reason.as_str(),
                ),
            )),
            Action::TransferMaterial {
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                amount,
                distance_km,
                priority,
            } => {
                if !self.state.agents.contains_key(requester_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: requester_agent_id.clone(),
                        },
                    }));
                }
                if from_ledger == to_ledger {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["from_ledger and to_ledger cannot be the same".to_string()],
                        },
                    }));
                }
                if kind.trim().is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["material kind cannot be empty".to_string()],
                        },
                    }));
                }
                if *amount <= 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InvalidAmount { amount: *amount },
                    }));
                }
                if *distance_km < 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["distance_km must be >= 0".to_string()],
                        },
                    }));
                }
                if *distance_km > MATERIAL_TRANSFER_MAX_DISTANCE_KM {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::MaterialTransferDistanceExceeded {
                            distance_km: *distance_km,
                            max_distance_km: MATERIAL_TRANSFER_MAX_DISTANCE_KM,
                        },
                    }));
                }
                let available = self.ledger_material_balance(from_ledger, kind.as_str());
                if available < *amount {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InsufficientMaterial {
                            material_kind: kind.clone(),
                            requested: *amount,
                            available,
                        },
                    }));
                }
                let priority = priority
                    .as_ref()
                    .copied()
                    .unwrap_or_else(|| material_transit_priority_for_kind(self, kind.as_str()));
                let loss_bps = material_transit_loss_bps_for_kind(self, kind.as_str());

                if *distance_km == 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::MaterialTransferred {
                        requester_agent_id: requester_agent_id.clone(),
                        from_ledger: from_ledger.clone(),
                        to_ledger: to_ledger.clone(),
                        kind: kind.clone(),
                        amount: *amount,
                        distance_km: *distance_km,
                        priority,
                    }));
                }

                if self.state.pending_material_transits.len() >= MATERIAL_TRANSFER_MAX_INFLIGHT {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::MaterialTransitCapacityExceeded {
                            in_flight: self.state.pending_material_transits.len(),
                            max_in_flight: MATERIAL_TRANSFER_MAX_INFLIGHT,
                        },
                    }));
                }

                let transit_ticks = ((*distance_km + MATERIAL_TRANSFER_SPEED_KM_PER_TICK - 1)
                    / MATERIAL_TRANSFER_SPEED_KM_PER_TICK)
                    .max(1) as u64;
                let ready_at = self.state.time.saturating_add(transit_ticks);
                Ok(WorldEventBody::Domain(
                    DomainEvent::MaterialTransitStarted {
                        job_id: action_id,
                        requester_agent_id: requester_agent_id.clone(),
                        from_ledger: from_ledger.clone(),
                        to_ledger: to_ledger.clone(),
                        kind: kind.clone(),
                        amount: *amount,
                        distance_km: *distance_km,
                        loss_bps,
                        ready_at,
                        priority,
                    },
                ))
            }
            _ => unreachable!("action_to_event_core received unsupported action variant"),
        }
    }

    fn evaluate_transfer_main_token_action(
        &self,
        action_id: ActionId,
        from_account_id: &str,
        to_account_id: &str,
        amount: u64,
        nonce: u64,
    ) -> DomainEvent {
        let from_account_id = from_account_id.trim();
        let to_account_id = to_account_id.trim();
        if from_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["from_account_id cannot be empty".to_string()],
                },
            };
        }
        if to_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["to_account_id cannot be empty".to_string()],
                },
            };
        }
        if from_account_id == to_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["from_account_id and to_account_id cannot be the same".to_string()],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["amount must be > 0".to_string()],
                },
            };
        }
        if nonce == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["nonce must be > 0".to_string()],
                },
            };
        }

        let event = DomainEvent::MainTokenTransferred {
            from_account_id: from_account_id.to_string(),
            to_account_id: to_account_id.to_string(),
            amount,
            nonce,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("main token transfer rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_distribute_main_token_treasury_action(
        &self,
        action_id: ActionId,
        proposal_id: ProposalId,
        distribution_id: &str,
        bucket_id: &str,
        distributions: &[MainTokenTreasuryDistribution],
    ) -> DomainEvent {
        if self.state.main_token_genesis_buckets.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token genesis is not initialized".to_string()],
                },
            };
        }
        if proposal_id == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["proposal_id must be > 0".to_string()],
                },
            };
        }
        let Some(proposal) = self.proposals.get(&proposal_id) else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: governance proposal not found ({proposal_id})"
                    )],
                },
            };
        };
        match proposal.status {
            ProposalStatus::Approved { .. } | ProposalStatus::Applied { .. } => {}
            _ => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: governance proposal must be approved or applied ({proposal_id})"
                        )],
                    },
                };
            }
        }

        let distribution_id = distribution_id.trim();
        if distribution_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["distribution_id cannot be empty".to_string()],
                },
            };
        }
        if self
            .state
            .main_token_treasury_distribution_records
            .contains_key(distribution_id)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: distribution_id already exists ({distribution_id})"
                    )],
                },
            };
        }

        let bucket_id = bucket_id.trim();
        if !is_main_token_treasury_distribution_bucket(bucket_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: unsupported bucket ({bucket_id})"
                    )],
                },
            };
        }
        if distributions.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["distributions cannot be empty".to_string()],
                },
            };
        }

        let mut normalized_distributions = Vec::with_capacity(distributions.len());
        let mut seen_accounts = BTreeSet::new();
        let mut total_amount = 0_u64;
        for item in distributions {
            let account_id = item.account_id.trim();
            if account_id.is_empty() {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: account_id cannot be empty (distribution_id={distribution_id})"
                        )],
                    },
                };
            }
            if item.amount == 0 {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: amount must be > 0 (distribution_id={distribution_id} account_id={account_id})"
                        )],
                    },
                };
            }
            if !seen_accounts.insert(account_id.to_string()) {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: duplicate account_id ({account_id})"
                        )],
                    },
                };
            }
            total_amount = match total_amount.checked_add(item.amount) {
                Some(value) => value,
                None => {
                    return DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "distribute main token treasury rejected: total_amount overflow (distribution_id={distribution_id})"
                            )],
                        },
                    };
                }
            };
            normalized_distributions.push(MainTokenTreasuryDistribution {
                account_id: account_id.to_string(),
                amount: item.amount,
            });
        }

        let bucket_balance = self
            .state
            .main_token_treasury_balances
            .get(bucket_id)
            .copied()
            .unwrap_or(0);
        if bucket_balance < total_amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: treasury bucket insufficient (bucket={bucket_id} balance={bucket_balance} total={total_amount})"
                    )],
                },
            };
        }

        let event = DomainEvent::MainTokenTreasuryDistributed {
            proposal_id,
            distribution_id: distribution_id.to_string(),
            bucket_id: bucket_id.to_string(),
            total_amount,
            distributions: normalized_distributions,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("distribute main token treasury rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_top_up_restricted_starter_claim_liveops_pool_action(
        &self,
        action_id: ActionId,
        controller_account_id: &str,
        top_up_id: &str,
        amount: u64,
    ) -> DomainEvent {
        let Some(registry) = self.governance_main_token_controller_registry() else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up rejected: main token controller registry is not configured"
                            .to_string(),
                    ],
                },
            };
        };
        let controller_account_id = controller_account_id.trim();
        if controller_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up controller_account_id cannot be empty"
                            .to_string(),
                    ],
                },
            };
        }
        let expected_controller_account_id = match Self::ecosystem_treasury_controller_account_id(
            registry,
            "restricted claim liveops pool top-up",
        ) {
            Ok(account_id) => account_id,
            Err(err) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "restricted claim liveops pool top-up rejected: {err:?}"
                        )],
                    },
                };
            }
        };
        if controller_account_id != expected_controller_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: controller_account_id does not match ecosystem treasury controller slot expected={} actual={}",
                        expected_controller_account_id, controller_account_id
                    )],
                },
            };
        }
        let top_up_id = top_up_id.trim();
        if top_up_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top_up_id cannot be empty".to_string(),
                    ],
                },
            };
        }
        if self
            .state
            .restricted_starter_claim_liveops_pool_top_up_records
            .contains_key(top_up_id)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: top_up_id already exists ({top_up_id})"
                    )],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up amount must be > 0".to_string(),
                    ],
                },
            };
        }
        let source_bucket_balance =
            self.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL);
        if source_bucket_balance < amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up treasury insufficient: bucket={} balance={} amount={amount}",
                        MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, source_bucket_balance
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimLiveopsPoolToppedUp {
            controller_account_id: controller_account_id.to_string(),
            top_up_id: top_up_id.to_string(),
            source_treasury_bucket_id: MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
            target_treasury_bucket_id:
                MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL.to_string(),
            amount,
            topped_up_at_epoch: self.current_governance_epoch(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: {err:?}"
                    )],
                },
            };
        }
        event
    }

    fn evaluate_issue_restricted_starter_claim_grant_action(
        &self,
        action_id: ActionId,
        issuer_account_id: &str,
        beneficiary_account_id: &str,
        amount: u64,
        issuance_reason: &str,
        expires_at_epoch: u64,
    ) -> DomainEvent {
        let issuer_account_id = issuer_account_id.trim();
        if issuer_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuer_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(reason) = self.ensure_restricted_starter_claim_admin(issuer_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![reason],
                },
            };
        }
        let beneficiary_account_id = beneficiary_account_id.trim();
        if beneficiary_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["beneficiary_account_id cannot be empty".to_string()],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["restricted grant amount must be > 0".to_string()],
                },
            };
        }
        let issuance_reason = issuance_reason.trim();
        if issuance_reason.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuance_reason cannot be empty".to_string()],
                },
            };
        }
        let current_epoch = self.current_governance_epoch();
        if expires_at_epoch <= current_epoch {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant expires_at_epoch must be > current_epoch ({expires_at_epoch} <= {current_epoch})"
                    )],
                },
            };
        }
        let source_bucket_id = MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL;
        let source_bucket_balance = self.main_token_treasury_balance(source_bucket_id);
        if source_bucket_balance < amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant treasury insufficient: bucket={source_bucket_id} balance={source_bucket_balance} amount={amount}"
                    )],
                },
            };
        }
        if !self.restricted_starter_claim_grant_can_be_reissued(beneficiary_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant already active or pending settlement: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        }
        if self.main_token_restricted_starter_claim_balance(beneficiary_account_id) > 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant beneficiary already has seeded restricted balance without reusable grant slot: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimGrantIssued {
            issuer_id: issuer_account_id.to_string(),
            beneficiary_account_id: beneficiary_account_id.to_string(),
            source_treasury_bucket_id: source_bucket_id.to_string(),
            amount,
            issuance_reason: issuance_reason.to_string(),
            spend_scope: RESTRICTED_STARTER_CLAIM_GRANT_SPEND_SCOPE_SLOT_1_ONLY.to_string(),
            issued_at_epoch: current_epoch,
            expires_at_epoch,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("issue restricted grant rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_revoke_restricted_starter_claim_grant_action(
        &self,
        action_id: ActionId,
        issuer_account_id: &str,
        beneficiary_account_id: &str,
        revoke_reason: &str,
    ) -> DomainEvent {
        let issuer_account_id = issuer_account_id.trim();
        if issuer_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuer_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(reason) = self.ensure_restricted_starter_claim_admin(issuer_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![reason],
                },
            };
        }
        let beneficiary_account_id = beneficiary_account_id.trim();
        if beneficiary_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["beneficiary_account_id cannot be empty".to_string()],
                },
            };
        }
        let revoke_reason = revoke_reason.trim();
        if revoke_reason.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["revoke_reason cannot be empty".to_string()],
                },
            };
        }

        let Some(grant) = self
            .state
            .restricted_starter_claim_grants
            .get(beneficiary_account_id)
        else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant not found: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        };
        if grant.status != RestrictedStarterClaimGrantStatus::Issued {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant is no longer active: beneficiary={} status={:?}",
                        beneficiary_account_id, grant.status
                    )],
                },
            };
        }
        if grant.issuer_id != issuer_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant issuer mismatch: beneficiary={} expected={} actual={}",
                        beneficiary_account_id, grant.issuer_id, issuer_account_id
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimGrantRevoked {
            beneficiary_account_id: beneficiary_account_id.to_string(),
            issuer_id: issuer_account_id.to_string(),
            issuance_reason: grant.issuance_reason.clone(),
            spend_scope: grant.spend_scope.clone(),
            source_treasury_bucket_id: grant.source_treasury_bucket_id.clone(),
            issued_amount: grant.issued_amount,
            revoked_amount: self
                .main_token_restricted_starter_claim_balance(beneficiary_account_id),
            issued_at_epoch: grant.issued_at_epoch,
            revoked_at_epoch: self.current_governance_epoch(),
            configured_expires_at_epoch: grant.expires_at_epoch,
            revoke_reason: revoke_reason.to_string(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("revoke restricted grant rejected: {err:?}")],
                },
            };
        }
        event
    }
}

fn material_transit_priority_for_kind(world: &World, kind: &str) -> MaterialTransitPriority {
    if let Some(profile) = world.material_profile(kind) {
        return match profile.default_priority {
            crate::runtime::MaterialDefaultPriority::Urgent => MaterialTransitPriority::Urgent,
            crate::runtime::MaterialDefaultPriority::Standard => MaterialTransitPriority::Standard,
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

fn material_transit_loss_bps_for_kind(world: &World, kind: &str) -> i64 {
    let base = MATERIAL_TRANSFER_LOSS_PER_KM_BPS.max(0);
    let factor = world
        .material_profile(kind)
        .map(|profile| match profile.transport_loss_class {
            crate::runtime::MaterialTransportLossClass::Low => 1_i64,
            crate::runtime::MaterialTransportLossClass::Medium => 2_i64,
            crate::runtime::MaterialTransportLossClass::High => 4_i64,
        })
        .unwrap_or(1);
    base.saturating_mul(factor)
}
