use super::*;

impl World {
    pub(super) fn try_apply_module_release_action(
        &mut self,
        action_id: u64,
        action: &Action,
    ) -> Result<Option<bool>, WorldError> {
        match action {
            Action::ModuleReleaseSubmit {
                requester_agent_id,
                manifest,
                activate,
                install_target,
                required_roles,
                profile_changes,
            } => {
                if !self.state.agents.contains_key(requester_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: requester_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if manifest.module_id.trim().is_empty() {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec!["module release submit rejected: module_id is empty"
                                    .to_string()],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if manifest.version.trim().is_empty() {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![
                                    "module release submit rejected: version is empty".to_string()
                                ],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if !self.module_artifacts.contains(&manifest.wasm_hash) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release submit rejected: module artifact missing {}",
                                    manifest.wasm_hash
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if let Some(owner_agent_id) =
                    self.state.module_artifact_owners.get(&manifest.wasm_hash)
                {
                    if owner_agent_id != requester_agent_id {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!(
                                        "module release submit rejected: requester {} does not own {} (owner {})",
                                        requester_agent_id, manifest.wasm_hash, owner_agent_id
                                    )],
                                },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                }

                let request_id = self.peek_next_module_release_request_id();
                let normalized_roles =
                    Self::normalize_module_release_required_roles(required_roles.as_slice());
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseRequested {
                        request_id,
                        requester_agent_id: requester_agent_id.clone(),
                        manifest: manifest.clone(),
                        activate: *activate,
                        install_target: install_target.clone(),
                        required_roles: normalized_roles,
                        profile_changes: profile_changes.clone(),
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseShadow {
                operator_agent_id,
                request_id,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: operator_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(request) = self.state.module_release_requests.get(request_id).cloned()
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release shadow rejected: request not found ({request_id})"
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                if !matches!(request.status, ModuleReleaseRequestStatus::Requested) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release shadow rejected: invalid status {:?} for request {}",
                                    request.status, request_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if let Err(reason) =
                    self.validate_module_release_profile_changes(&request.profile_changes)
                {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("module release shadow rejected: {reason}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let shadow_manifest_hash = match self
                    .evaluate_module_release_shadow_hash(&request.manifest, request.activate)
                {
                    Ok(hash) => hash,
                    Err(reason) => {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![reason],
                                },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                };
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseShadowed {
                        request_id: *request_id,
                        operator_agent_id: operator_agent_id.clone(),
                        manifest_hash: shadow_manifest_hash,
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseSubmitAttestation {
                operator_agent_id,
                request_id,
                signer_node_id,
                platform,
                build_manifest_hash,
                source_hash,
                wasm_hash,
                proof_cid,
                builder_image_digest,
                container_platform,
                canonicalizer_version,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: operator_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(request) = self.state.module_release_requests.get(request_id).cloned()
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release attestation rejected: request not found ({request_id})"
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                if matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Rejected | ModuleReleaseRequestStatus::Applied
                ) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release attestation rejected: invalid status {:?} for request {}",
                                    request.status, request_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let normalized_signer_node_id = signer_node_id.trim().to_string();
                if normalized_signer_node_id.is_empty() {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![
                                    "module release attestation rejected: signer_node_id is empty"
                                        .to_string(),
                                ],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if self
                    .node_identity_public_key(normalized_signer_node_id.as_str())
                    .is_none()
                {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release attestation rejected: signer_node_id is untrusted ({})",
                                    normalized_signer_node_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(normalized_platform) =
                    Self::normalize_module_release_attestation_platform(platform.as_str())
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![
                                    "module release attestation rejected: platform is empty"
                                        .to_string(),
                                ],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                let normalized_build_manifest_hash =
                    match Self::normalize_module_release_attestation_hash(
                        build_manifest_hash.as_str(),
                        "build_manifest_hash",
                    ) {
                        Ok(hash) => hash,
                        Err(note) => {
                            self.append_event(
                                WorldEventBody::Domain(DomainEvent::ActionRejected {
                                    action_id,
                                    reason: RejectReason::RuleDenied { notes: vec![note] },
                                }),
                                Some(CausedBy::Action(action_id)),
                            )?;
                            return Ok(Some(true));
                        }
                    };
                let normalized_source_hash = match Self::normalize_module_release_attestation_hash(
                    source_hash.as_str(),
                    "source_hash",
                ) {
                    Ok(hash) => hash,
                    Err(note) => {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied { notes: vec![note] },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                };
                let normalized_wasm_hash = match Self::normalize_module_release_attestation_hash(
                    wasm_hash.as_str(),
                    "wasm_hash",
                ) {
                    Ok(hash) => hash,
                    Err(note) => {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied { notes: vec![note] },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                };
                if request.manifest.wasm_hash != normalized_wasm_hash {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release attestation rejected: wasm hash mismatch expected {} found {}",
                                    request.manifest.wasm_hash, normalized_wasm_hash
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(normalized_proof_cid) =
                    Self::normalize_module_release_attestation_proof_cid(proof_cid.as_str())
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![
                                    "module release attestation rejected: proof_cid is empty or too long"
                                        .to_string(),
                                ],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                let normalized_builder_image_digest =
                    match Self::normalize_module_release_attestation_builder_image_digest(
                        builder_image_digest.as_str(),
                    ) {
                        Ok(digest) => digest,
                        Err(note) => {
                            self.append_event(
                                WorldEventBody::Domain(DomainEvent::ActionRejected {
                                    action_id,
                                    reason: RejectReason::RuleDenied { notes: vec![note] },
                                }),
                                Some(CausedBy::Action(action_id)),
                            )?;
                            return Ok(Some(true));
                        }
                    };
                let normalized_container_platform =
                    match Self::normalize_module_release_attestation_label(
                        container_platform.as_str(),
                        "container_platform",
                    ) {
                        Ok(value) => value,
                        Err(note) => {
                            self.append_event(
                                WorldEventBody::Domain(DomainEvent::ActionRejected {
                                    action_id,
                                    reason: RejectReason::RuleDenied { notes: vec![note] },
                                }),
                                Some(CausedBy::Action(action_id)),
                            )?;
                            return Ok(Some(true));
                        }
                    };
                let normalized_canonicalizer_version =
                    match Self::normalize_module_release_attestation_label(
                        canonicalizer_version.as_str(),
                        "canonicalizer_version",
                    ) {
                        Ok(value) => value,
                        Err(note) => {
                            self.append_event(
                                WorldEventBody::Domain(DomainEvent::ActionRejected {
                                    action_id,
                                    reason: RejectReason::RuleDenied { notes: vec![note] },
                                }),
                                Some(CausedBy::Action(action_id)),
                            )?;
                            return Ok(Some(true));
                        }
                    };
                let attestation_key = Self::module_release_attestation_key(
                    normalized_signer_node_id.as_str(),
                    normalized_platform.as_str(),
                );
                if let Some(existing) = request.attestations.get(attestation_key.as_str()) {
                    let same_payload = existing.build_manifest_hash
                        == normalized_build_manifest_hash
                        && existing.source_hash == normalized_source_hash
                        && existing.wasm_hash == normalized_wasm_hash
                        && existing.proof_cid == normalized_proof_cid
                        && existing.builder_image_digest == normalized_builder_image_digest
                        && existing.container_platform == normalized_container_platform
                        && existing.canonicalizer_version == normalized_canonicalizer_version;
                    if !same_payload {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!(
                                        "module release attestation rejected: conflicting attestation already exists for signer={} platform={}",
                                        normalized_signer_node_id, normalized_platform
                                    )],
                                },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                } else if request.attestations.len() >= MODULE_RELEASE_ATTESTATION_LIMIT {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release attestation rejected: attestation limit exceeded for request {}",
                                    request_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseAttested {
                        request_id: *request_id,
                        operator_agent_id: operator_agent_id.clone(),
                        signer_node_id: normalized_signer_node_id,
                        platform: normalized_platform,
                        build_manifest_hash: normalized_build_manifest_hash,
                        source_hash: normalized_source_hash,
                        wasm_hash: normalized_wasm_hash,
                        proof_cid: normalized_proof_cid,
                        builder_image_digest: normalized_builder_image_digest,
                        container_platform: normalized_container_platform,
                        canonicalizer_version: normalized_canonicalizer_version,
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseApproveRole {
                approver_agent_id,
                request_id,
                role,
            } => {
                if !self.state.agents.contains_key(approver_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: approver_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(request) = self.state.module_release_requests.get(request_id).cloned()
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release approve_role rejected: request not found ({request_id})"
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                if !matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Shadowed
                        | ModuleReleaseRequestStatus::PartiallyApproved
                        | ModuleReleaseRequestStatus::Approved
                ) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release approve_role rejected: invalid status {:?} for request {}",
                                    request.status, request_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(normalized_role) = Self::normalize_module_release_role(role.as_str())
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec!["module release approve_role rejected: role is empty"
                                    .to_string()],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                let bound_roles = self
                    .state
                    .module_release_role_bindings
                    .get(approver_agent_id)
                    .cloned()
                    .unwrap_or_default();
                if !bound_roles.contains(&normalized_role) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release approve_role rejected: approver role binding missing for {} role {}",
                                    approver_agent_id, normalized_role
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if !request
                    .required_roles
                    .iter()
                    .any(|required| required == &normalized_role)
                {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release approve_role rejected: role not required ({normalized_role})"
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if let Some(existing_approver) = request.role_approvals.get(&normalized_role) {
                    if existing_approver != approver_agent_id {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!(
                                        "module release approve_role rejected: role {} already approved by {}",
                                        normalized_role, existing_approver
                                    )],
                                },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(Some(true));
                    }
                }
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseRoleApproved {
                        request_id: *request_id,
                        approver_agent_id: approver_agent_id.clone(),
                        role: normalized_role,
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseBindRoles {
                operator_agent_id,
                target_agent_id,
                roles,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: operator_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if !self.state.agents.contains_key(target_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: target_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let normalized_roles = Self::normalize_module_release_role_set(roles.as_slice());
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseRolesBound {
                        operator_agent_id: operator_agent_id.clone(),
                        target_agent_id: target_agent_id.clone(),
                        roles: normalized_roles,
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseReject {
                rejector_agent_id,
                request_id,
                reason,
            } => {
                if !self.state.agents.contains_key(rejector_agent_id) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: rejector_agent_id.clone(),
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                let Some(request) = self.state.module_release_requests.get(request_id).cloned()
                else {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release reject rejected: request not found ({request_id})"
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                };
                if matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Rejected | ModuleReleaseRequestStatus::Applied
                ) {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "module release reject rejected: invalid status {:?} for request {}",
                                    request.status, request_id
                                )],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                if reason.trim().is_empty() {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![
                                    "module release reject rejected: reason is empty".to_string()
                                ],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(Some(true));
                }
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ModuleReleaseRejected {
                        request_id: *request_id,
                        rejector_agent_id: rejector_agent_id.clone(),
                        reason: reason.trim().to_string(),
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                Ok(Some(true))
            }
            Action::ModuleReleaseApply {
                operator_agent_id,
                request_id,
            } => self
                .apply_module_release_request_action(
                    action_id,
                    operator_agent_id.as_str(),
                    *request_id,
                    None,
                )
                .map(Some),
            Action::ModuleReleaseApplyWithFinality {
                operator_agent_id,
                request_id,
                finality_certificate,
            } => self
                .apply_module_release_request_action(
                    action_id,
                    operator_agent_id.as_str(),
                    *request_id,
                    Some(finality_certificate),
                )
                .map(Some),
            _ => Ok(None),
        }
    }

    fn apply_module_release_request_action(
        &mut self,
        action_id: u64,
        operator_agent_id: &str,
        request_id: u64,
        finality_certificate: Option<&GovernanceFinalityCertificate>,
    ) -> Result<bool, WorldError> {
        if !self.state.agents.contains_key(operator_agent_id) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::AgentNotFound {
                        agent_id: operator_agent_id.to_string(),
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        let Some(request) = self.state.module_release_requests.get(&request_id).cloned() else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "module release apply rejected: request not found ({request_id})"
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        if matches!(
            request.status,
            ModuleReleaseRequestStatus::Requested
                | ModuleReleaseRequestStatus::Shadowed
                | ModuleReleaseRequestStatus::Rejected
                | ModuleReleaseRequestStatus::Applied
        ) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "module release apply rejected: invalid status {:?} for request {}",
                            request.status, request_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if !Self::module_release_roles_satisfied(
            request.required_roles.as_slice(),
            &request.role_approvals,
        ) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "module release apply rejected: required roles are not fully approved for request {}",
                            request_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        let epoch_id = self.current_governance_epoch();
        let snapshot = self.governance_finality_epoch_snapshot_for_epoch(epoch_id);
        let snapshot_signers: BTreeSet<&str> = snapshot
            .signer_node_ids
            .iter()
            .map(String::as_str)
            .collect();
        let aggregated_signers: BTreeSet<String> = request
            .attestations
            .values()
            .filter_map(|attestation| {
                let signer_node_id = attestation.signer_node_id.trim();
                if snapshot_signers.contains(signer_node_id) {
                    Some(signer_node_id.to_string())
                } else {
                    None
                }
            })
            .collect();
        let eligible_attestations: Vec<_> = request
            .attestations
            .values()
            .filter(|attestation| snapshot_signers.contains(attestation.signer_node_id.trim()))
            .collect();
        let min_unique_signers = snapshot.effective_min_unique_signers();
        let aggregated_stake_bps = if snapshot.signer_node_ids.is_empty() {
            0
        } else {
            (u128::from(aggregated_signers.len() as u64)
                .saturating_mul(10_000)
                .saturating_div(u128::from(snapshot.signer_node_ids.len() as u64)))
            .min(10_000) as u16
        };
        if aggregated_signers.len() < min_unique_signers as usize
            || aggregated_stake_bps < snapshot.threshold_bps
        {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "module release apply rejected: attestation threshold not met epoch_id={} min_unique_signers={} threshold_bps={} aggregated_signers={} aggregated_stake_bps={} request_id={}",
                            epoch_id,
                            min_unique_signers,
                            snapshot.threshold_bps,
                            aggregated_signers.len(),
                            aggregated_stake_bps,
                            request_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        let receipt_evidence_keys: BTreeSet<_> = eligible_attestations
            .iter()
            .map(|attestation| {
                (
                    attestation.wasm_hash.clone(),
                    attestation.source_hash.clone(),
                    attestation.build_manifest_hash.clone(),
                    attestation.builder_image_digest.clone(),
                    attestation.container_platform.clone(),
                    attestation.canonicalizer_version.clone(),
                )
            })
            .collect();
        if receipt_evidence_keys.len() > 1 {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "module release apply rejected: attestation receipt evidence mismatch request_id={} unique_receipt_variants={}",
                            request_id,
                            receipt_evidence_keys.len()
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if let (Some(identity), Some(attestation)) = (
            request.manifest.artifact_identity.as_ref(),
            eligible_attestations.first(),
        ) {
            if attestation.source_hash != identity.source_hash
                || attestation.build_manifest_hash != identity.build_manifest_hash
                || attestation.wasm_hash != request.manifest.wasm_hash
            {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "module release apply rejected: attestation receipt identity mismatch request_id={} expected_wasm_hash={} actual_wasm_hash={} expected_source_hash={} actual_source_hash={} expected_build_manifest_hash={} actual_build_manifest_hash={}",
                                request_id,
                                request.manifest.wasm_hash,
                                attestation.wasm_hash,
                                identity.source_hash,
                                attestation.source_hash,
                                identity.build_manifest_hash,
                                attestation.build_manifest_hash
                            )],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        }
        if let Err(reason) = self.validate_module_release_profile_changes(&request.profile_changes)
        {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("module release apply rejected: {reason}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let installer_agent_id = request.requester_agent_id.clone();
        self.apply_install_module_action(
            action_id,
            installer_agent_id.as_str(),
            &request.manifest,
            request.activate,
            request.install_target.clone(),
            finality_certificate,
        )?;

        let (instance_id, module_id, module_version, proposal_id, manifest_hash) =
            match self.journal.events.last().map(|event| &event.body) {
                Some(WorldEventBody::Domain(DomainEvent::ModuleInstalled {
                    instance_id,
                    module_id,
                    module_version,
                    proposal_id,
                    manifest_hash,
                    ..
                })) => (
                    instance_id.clone(),
                    module_id.clone(),
                    module_version.clone(),
                    *proposal_id,
                    manifest_hash.clone(),
                ),
                _ => return Ok(true),
            };

        self.apply_module_release_profile_changes(
            action_id,
            operator_agent_id,
            proposal_id,
            &request.profile_changes,
        )?;
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ModuleReleaseApplied {
                request_id,
                operator_agent_id: operator_agent_id.to_string(),
                installer_agent_id,
                instance_id,
                module_id,
                module_version,
                proposal_id,
                manifest_hash,
            }),
            Some(CausedBy::Action(action_id)),
        )?;
        Ok(true)
    }

    fn validate_module_release_profile_changes(
        &self,
        changes: &ModuleProfileChanges,
    ) -> Result<(), String> {
        if changes.is_empty() {
            return Ok(());
        }

        let total_changes = changes
            .product_profiles
            .len()
            .saturating_add(changes.recipe_profiles.len())
            .saturating_add(changes.factory_profiles.len());
        if total_changes > MODULE_RELEASE_PROFILE_CHANGE_LIMIT {
            return Err(format!(
                "profile changes exceed limit {} (got {})",
                MODULE_RELEASE_PROFILE_CHANGE_LIMIT, total_changes
            ));
        }

        let product_fields = [
            "product_id",
            "role_tag",
            "maintenance_sink",
            "tradable",
            "unlock_stage",
        ];
        let recipe_fields = [
            "recipe_id",
            "bottleneck_tags",
            "stage_gate",
            "preferred_factory_tags",
        ];
        let factory_fields = ["factory_id", "tier", "recipe_slots", "tags"];

        let mut product_ids = BTreeSet::new();
        for profile in &changes.product_profiles {
            if profile.product_id.trim().is_empty() {
                return Err("product profile product_id cannot be empty".to_string());
            }
            if profile.role_tag.trim().is_empty() {
                return Err(format!(
                    "product profile role_tag cannot be empty: {}",
                    profile.product_id
                ));
            }
            ensure_profile_field_whitelist(profile, product_fields.as_slice(), "product profile")?;
            if !product_ids.insert(profile.product_id.clone()) {
                return Err(format!(
                    "duplicate product profile_id {}",
                    profile.product_id
                ));
            }
            if self
                .state
                .product_profiles
                .contains_key(profile.product_id.as_str())
            {
                return Err(format!(
                    "product profile_id already exists in state {} (module release overwrite is forbidden)",
                    profile.product_id
                ));
            }
        }

        let mut recipe_ids = BTreeSet::new();
        for profile in &changes.recipe_profiles {
            if profile.recipe_id.trim().is_empty() {
                return Err("recipe profile recipe_id cannot be empty".to_string());
            }
            ensure_profile_field_whitelist(profile, recipe_fields.as_slice(), "recipe profile")?;
            if !recipe_ids.insert(profile.recipe_id.clone()) {
                return Err(format!("duplicate recipe profile_id {}", profile.recipe_id));
            }
            if self
                .state
                .recipe_profiles
                .contains_key(profile.recipe_id.as_str())
            {
                return Err(format!(
                    "recipe profile_id already exists in state {} (module release overwrite is forbidden)",
                    profile.recipe_id
                ));
            }
        }

        let mut factory_ids = BTreeSet::new();
        for profile in &changes.factory_profiles {
            if profile.factory_id.trim().is_empty() {
                return Err("factory profile factory_id cannot be empty".to_string());
            }
            if profile.tier == 0 {
                return Err(format!(
                    "factory profile tier must be >= 1: {}",
                    profile.factory_id
                ));
            }
            if profile.recipe_slots == 0 {
                return Err(format!(
                    "factory profile recipe_slots must be > 0: {}",
                    profile.factory_id
                ));
            }
            ensure_profile_field_whitelist(profile, factory_fields.as_slice(), "factory profile")?;
            if !factory_ids.insert(profile.factory_id.clone()) {
                return Err(format!(
                    "duplicate factory profile_id {}",
                    profile.factory_id
                ));
            }
            if self
                .state
                .factory_profiles
                .contains_key(profile.factory_id.as_str())
            {
                return Err(format!(
                    "factory profile_id already exists in state {} (module release overwrite is forbidden)",
                    profile.factory_id
                ));
            }
        }

        Ok(())
    }

    fn apply_module_release_profile_changes(
        &mut self,
        action_id: ActionId,
        operator_agent_id: &str,
        proposal_id: ProposalId,
        changes: &ModuleProfileChanges,
    ) -> Result<(), WorldError> {
        if changes.is_empty() {
            return Ok(());
        }

        let mut product_profiles = changes.product_profiles.clone();
        product_profiles.sort_by(|left, right| left.product_id.cmp(&right.product_id));
        for profile in product_profiles {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ProductProfileGoverned {
                    operator_agent_id: operator_agent_id.to_string(),
                    proposal_id,
                    profile,
                }),
                Some(CausedBy::Action(action_id)),
            )?;
        }

        let mut recipe_profiles = changes.recipe_profiles.clone();
        recipe_profiles.sort_by(|left, right| left.recipe_id.cmp(&right.recipe_id));
        for profile in recipe_profiles {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::RecipeProfileGoverned {
                    operator_agent_id: operator_agent_id.to_string(),
                    proposal_id,
                    profile,
                }),
                Some(CausedBy::Action(action_id)),
            )?;
        }

        let mut factory_profiles = changes.factory_profiles.clone();
        factory_profiles.sort_by(|left, right| left.factory_id.cmp(&right.factory_id));
        for profile in factory_profiles {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::FactoryProfileGoverned {
                    operator_agent_id: operator_agent_id.to_string(),
                    proposal_id,
                    profile,
                }),
                Some(CausedBy::Action(action_id)),
            )?;
        }

        Ok(())
    }
}
