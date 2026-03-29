use super::*;

impl WorldState {
    pub(super) fn apply_domain_event_core(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::AgentRegistered { agent_id, pos } => {
                let state = AgentState::new(agent_id, *pos);
                self.agents
                    .insert(agent_id.clone(), AgentCell::new(state, now));
            }
            DomainEvent::AgentMoved { agent_id, to, .. } => {
                if let Some(cell) = self.agents.get_mut(agent_id) {
                    cell.state.pos = *to;
                    cell.last_active = now;
                }
            }
            DomainEvent::ActionAccepted { .. } => {}
            DomainEvent::ActionRejected { .. } => {}
            DomainEvent::Observation { .. } => {}
            DomainEvent::BodyAttributesUpdated { agent_id, view, .. } => {
                let cell =
                    self.agents
                        .get_mut(agent_id)
                        .ok_or_else(|| WorldError::AgentNotFound {
                            agent_id: agent_id.clone(),
                        })?;
                cell.state.body_view = view.clone();
                cell.last_active = now;
            }
            DomainEvent::BodyAttributesRejected { agent_id, .. } => {
                if let Some(cell) = self.agents.get_mut(agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: agent_id.clone(),
                    });
                }
            }
            DomainEvent::BodyInterfaceExpanded {
                agent_id,
                slot_capacity,
                expansion_level,
                consumed_item_id,
                new_slot_id,
                slot_type,
                ..
            } => {
                let cell =
                    self.agents
                        .get_mut(agent_id)
                        .ok_or_else(|| WorldError::AgentNotFound {
                            agent_id: agent_id.clone(),
                        })?;
                cell.state
                    .body_state
                    .consume_interface_module_item(consumed_item_id)
                    .map_err(|reason| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "consume interface module item failed for {agent_id}: {reason}"
                        ),
                    })?;
                cell.state.body_state.slot_capacity = *slot_capacity;
                cell.state.body_state.expansion_level = *expansion_level;
                if !cell
                    .state
                    .body_state
                    .slots
                    .iter()
                    .any(|slot| slot.slot_id == *new_slot_id)
                {
                    cell.state
                        .body_state
                        .slots
                        .push(crate::models::BodyModuleSlot {
                            slot_id: new_slot_id.clone(),
                            slot_type: *slot_type,
                            installed_module: None,
                            locked: false,
                        });
                }
                cell.last_active = now;
            }
            DomainEvent::BodyInterfaceExpandRejected { agent_id, .. } => {
                if let Some(cell) = self.agents.get_mut(agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: agent_id.clone(),
                    });
                }
            }
            DomainEvent::ModuleArtifactDeployed {
                publisher_agent_id,
                wasm_hash,
                fee_kind,
                fee_amount,
                ..
            } => {
                self.settle_module_action_fee(
                    publisher_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                self.module_artifact_owners
                    .insert(wasm_hash.clone(), publisher_agent_id.clone());
                self.module_artifact_listings.remove(wasm_hash);
                self.module_artifact_bids.remove(wasm_hash);
            }
            DomainEvent::ModuleInstalled {
                installer_agent_id,
                instance_id,
                module_id,
                install_target,
                module_version,
                wasm_hash,
                active,
                fee_kind,
                fee_amount,
                ..
            } => {
                self.settle_module_action_fee(
                    installer_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                let resolved_instance_id = if instance_id.trim().is_empty() {
                    module_id.clone()
                } else {
                    instance_id.trim().to_string()
                };
                self.module_instances.insert(
                    resolved_instance_id.clone(),
                    ModuleInstanceState {
                        instance_id: resolved_instance_id,
                        module_id: module_id.clone(),
                        module_version: module_version.clone(),
                        wasm_hash: wasm_hash.clone(),
                        owner_agent_id: installer_agent_id.clone(),
                        install_target: install_target.clone(),
                        active: *active,
                        installed_at: now,
                    },
                );
                self.next_module_instance_id = self.next_module_instance_id.saturating_add(1);
                self.installed_module_targets
                    .insert(module_id.clone(), install_target.clone());
            }
            DomainEvent::ModuleUpgraded {
                upgrader_agent_id,
                instance_id,
                module_id,
                from_module_version,
                to_module_version,
                wasm_hash,
                install_target,
                active,
                fee_kind,
                fee_amount,
                ..
            } => {
                self.settle_module_action_fee(
                    upgrader_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                let instance = self.module_instances.get_mut(instance_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("module instance missing for upgrade {instance_id}"),
                    }
                })?;
                if instance.owner_agent_id != *upgrader_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance owner mismatch for upgrade: instance={} owner={} upgrader={}",
                            instance_id, instance.owner_agent_id, upgrader_agent_id
                        ),
                    });
                }
                if instance.module_id != *module_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance module_id mismatch for upgrade: instance={} state_module_id={} event_module_id={}",
                            instance_id, instance.module_id, module_id
                        ),
                    });
                }
                if instance.module_version != *from_module_version {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance from_version mismatch for upgrade: instance={} state_version={} event_from={}",
                            instance_id, instance.module_version, from_module_version
                        ),
                    });
                }
                instance.module_version = to_module_version.clone();
                instance.wasm_hash = wasm_hash.clone();
                instance.install_target = install_target.clone();
                instance.active = *active;
                self.installed_module_targets
                    .insert(module_id.clone(), install_target.clone());
            }
            DomainEvent::ModuleReleaseRequested {
                request_id,
                requester_agent_id,
                manifest,
                activate,
                install_target,
                required_roles,
                profile_changes,
            } => {
                if *request_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "module release request_id must be > 0".to_string(),
                    });
                }
                if !self.agents.contains_key(requester_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: requester_agent_id.clone(),
                    });
                }
                if self.module_release_requests.contains_key(request_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release request already exists: request_id={request_id}"
                        ),
                    });
                }

                let mut normalized_roles: Vec<String> = required_roles
                    .iter()
                    .map(|role| role.trim().to_ascii_lowercase())
                    .filter(|role| !role.is_empty())
                    .collect();
                normalized_roles.sort();
                normalized_roles.dedup();
                if normalized_roles.is_empty() {
                    normalized_roles = default_module_release_required_roles();
                }

                self.module_release_requests.insert(
                    *request_id,
                    ModuleReleaseRequestState {
                        request_id: *request_id,
                        requester_agent_id: requester_agent_id.clone(),
                        manifest: manifest.clone(),
                        activate: *activate,
                        install_target: install_target.clone(),
                        profile_changes: profile_changes.clone(),
                        required_roles: normalized_roles,
                        role_approvals: BTreeMap::new(),
                        attestations: BTreeMap::new(),
                        status: ModuleReleaseRequestStatus::Requested,
                        shadow_manifest_hash: None,
                        applied_manifest_hash: None,
                        applied_proposal_id: None,
                        rejected_reason: None,
                        created_at: now,
                        updated_at: now,
                    },
                );
                self.module_release_manifest_mappings.insert(
                    *request_id,
                    ModuleReleaseManifestMappingState {
                        request_id: *request_id,
                        release_id: format!("release-{request_id}"),
                        module_id: manifest.module_id.clone(),
                        attestation_count: 0,
                        release_wasm_hash: None,
                        release_source_hash: None,
                        release_build_manifest_hash: None,
                        release_builder_image_digest: None,
                        release_container_platform: None,
                        release_canonicalizer_version: None,
                        attestation_platforms: Vec::new(),
                        attestation_proof_cids: Vec::new(),
                        receipt_evidence_conflict: false,
                        shadow_manifest_hash: None,
                        applied_manifest_hash: None,
                        applied_proposal_id: None,
                        status: ModuleReleaseRequestStatus::Requested,
                        created_at: now,
                        updated_at: now,
                    },
                );
                self.next_module_release_request_id = self
                    .next_module_release_request_id
                    .max(request_id.saturating_add(1));
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleReleaseShadowed {
                request_id,
                operator_agent_id,
                manifest_hash,
            } => {
                let request = self
                    .module_release_requests
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release shadow rejected: request not found ({request_id})"
                        ),
                    })?;
                if !matches!(request.status, ModuleReleaseRequestStatus::Requested) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release shadow invalid status for request {}: {:?}",
                            request_id, request.status
                        ),
                    });
                }
                request.status = ModuleReleaseRequestStatus::Shadowed;
                request.shadow_manifest_hash = Some(manifest_hash.clone());
                request.updated_at = now;
                let mapping = self
                    .module_release_manifest_mappings
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release mapping missing for shadow request_id={request_id}"
                        ),
                    })?;
                mapping.status = ModuleReleaseRequestStatus::Shadowed;
                mapping.shadow_manifest_hash = Some(manifest_hash.clone());
                mapping.updated_at = now;
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleReleaseAttested {
                request_id,
                operator_agent_id,
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
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if signer_node_id.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation signer_node_id cannot be empty (request_id={request_id})"
                        ),
                    });
                }
                if !self
                    .node_identity_bindings
                    .contains_key(signer_node_id.trim())
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation signer_node_id is untrusted: {}",
                            signer_node_id
                        ),
                    });
                }
                let normalized_platform = platform.trim().to_ascii_lowercase();
                if normalized_platform.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation platform cannot be empty (request_id={request_id})"
                        ),
                    });
                }
                let request = self
                    .module_release_requests
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation rejected: request not found ({request_id})"
                        ),
                    })?;
                if matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Rejected | ModuleReleaseRequestStatus::Applied
                ) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation invalid status for request {}: {:?}",
                            request_id, request.status
                        ),
                    });
                }
                if request.manifest.wasm_hash != *wasm_hash {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release attestation wasm hash mismatch: request_id={} expected={} found={}",
                            request_id, request.manifest.wasm_hash, wasm_hash
                        ),
                    });
                }

                let attestation_key = format!("{}|{}", signer_node_id.trim(), normalized_platform);
                let next_attestation = ModuleReleaseAttestationState {
                    request_id: *request_id,
                    signer_node_id: signer_node_id.trim().to_string(),
                    platform: normalized_platform,
                    submitted_by_agent_id: operator_agent_id.clone(),
                    build_manifest_hash: build_manifest_hash.clone(),
                    source_hash: source_hash.clone(),
                    wasm_hash: wasm_hash.clone(),
                    proof_cid: proof_cid.clone(),
                    builder_image_digest: builder_image_digest.clone(),
                    container_platform: container_platform.clone(),
                    canonicalizer_version: canonicalizer_version.clone(),
                    submitted_at: now,
                };
                if let Some(existing) = request.attestations.get(attestation_key.as_str()) {
                    let same_payload = existing.request_id == *request_id
                        && existing.signer_node_id == next_attestation.signer_node_id
                        && existing.platform == next_attestation.platform
                        && existing.build_manifest_hash == next_attestation.build_manifest_hash
                        && existing.source_hash == next_attestation.source_hash
                        && existing.wasm_hash == next_attestation.wasm_hash
                        && existing.proof_cid == next_attestation.proof_cid
                        && existing.builder_image_digest == next_attestation.builder_image_digest
                        && existing.container_platform == next_attestation.container_platform
                        && existing.canonicalizer_version == next_attestation.canonicalizer_version;
                    if !same_payload {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "module release attestation conflict: request_id={} signer={} platform={}",
                                request_id, signer_node_id, platform
                            ),
                        });
                    }
                } else {
                    request
                        .attestations
                        .insert(attestation_key, next_attestation.clone());
                }
                request.updated_at = now;
                let mapping = self
                    .module_release_manifest_mappings
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release mapping missing for attestation request_id={request_id}"
                        ),
                    })?;
                mapping.attestation_count = request.attestations.len() as u32;
                if !mapping
                    .attestation_platforms
                    .iter()
                    .any(|platform_name| platform_name == &next_attestation.platform)
                {
                    mapping
                        .attestation_platforms
                        .push(next_attestation.platform.clone());
                    mapping.attestation_platforms.sort();
                }
                if !mapping
                    .attestation_proof_cids
                    .iter()
                    .any(|existing_cid| existing_cid == &next_attestation.proof_cid)
                {
                    mapping
                        .attestation_proof_cids
                        .push(next_attestation.proof_cid.clone());
                    mapping.attestation_proof_cids.sort();
                }
                match mapping.release_wasm_hash.as_ref() {
                    None => {
                        mapping.release_wasm_hash = Some(next_attestation.wasm_hash.clone());
                        mapping.release_source_hash = Some(next_attestation.source_hash.clone());
                        mapping.release_build_manifest_hash =
                            Some(next_attestation.build_manifest_hash.clone());
                        mapping.release_builder_image_digest =
                            Some(next_attestation.builder_image_digest.clone());
                        mapping.release_container_platform =
                            Some(next_attestation.container_platform.clone());
                        mapping.release_canonicalizer_version =
                            Some(next_attestation.canonicalizer_version.clone());
                    }
                    Some(existing_wasm_hash) => {
                        let same_release_evidence = existing_wasm_hash
                            == &next_attestation.wasm_hash
                            && mapping.release_source_hash.as_ref()
                                == Some(&next_attestation.source_hash)
                            && mapping.release_build_manifest_hash.as_ref()
                                == Some(&next_attestation.build_manifest_hash)
                            && mapping.release_builder_image_digest.as_ref()
                                == Some(&next_attestation.builder_image_digest)
                            && mapping.release_container_platform.as_ref()
                                == Some(&next_attestation.container_platform)
                            && mapping.release_canonicalizer_version.as_ref()
                                == Some(&next_attestation.canonicalizer_version);
                        if !same_release_evidence {
                            mapping.receipt_evidence_conflict = true;
                        }
                    }
                }
                mapping.updated_at = now;
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleReleaseRoleApproved {
                request_id,
                approver_agent_id,
                role,
            } => {
                let request = self
                    .module_release_requests
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release approve_role rejected: request not found ({request_id})"
                        ),
                    })?;
                if !matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Shadowed
                        | ModuleReleaseRequestStatus::PartiallyApproved
                        | ModuleReleaseRequestStatus::Approved
                ) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release approve_role invalid status for request {}: {:?}",
                            request_id, request.status
                        ),
                    });
                }
                let normalized_role = role.trim().to_ascii_lowercase();
                if normalized_role.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release approve_role role cannot be empty (request_id={request_id})"
                        ),
                    });
                }
                if !request
                    .required_roles
                    .iter()
                    .any(|item| item == &normalized_role)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release approve_role role not required: request_id={} role={}",
                            request_id, normalized_role
                        ),
                    });
                }
                if let Some(existing_approver) = request.role_approvals.get(&normalized_role) {
                    if existing_approver != approver_agent_id {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "module release approve_role approver mismatch: request_id={} role={} existing={} incoming={}",
                                request_id, normalized_role, existing_approver, approver_agent_id
                            ),
                        });
                    }
                } else {
                    request
                        .role_approvals
                        .insert(normalized_role, approver_agent_id.clone());
                }
                let all_roles_approved = request
                    .required_roles
                    .iter()
                    .all(|required| request.role_approvals.contains_key(required));
                request.status = if all_roles_approved {
                    ModuleReleaseRequestStatus::Approved
                } else {
                    ModuleReleaseRequestStatus::PartiallyApproved
                };
                request.updated_at = now;
                if let Some(mapping) = self.module_release_manifest_mappings.get_mut(request_id) {
                    mapping.status = request.status;
                    mapping.updated_at = now;
                }
                if let Some(cell) = self.agents.get_mut(approver_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleReleaseRolesBound {
                operator_agent_id,
                target_agent_id,
                roles,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(target_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: target_agent_id.clone(),
                    });
                }

                let normalized_roles: BTreeSet<String> = roles
                    .iter()
                    .map(|role| role.trim().to_ascii_lowercase())
                    .filter(|role| !role.is_empty())
                    .collect();
                if normalized_roles.is_empty() {
                    self.module_release_role_bindings.remove(target_agent_id);
                } else {
                    self.module_release_role_bindings
                        .insert(target_agent_id.clone(), normalized_roles);
                }
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
                if operator_agent_id != target_agent_id {
                    if let Some(cell) = self.agents.get_mut(target_agent_id) {
                        cell.last_active = now;
                    }
                }
            }
            DomainEvent::ModuleReleaseRejected {
                request_id,
                rejector_agent_id,
                reason,
            } => {
                let request = self
                    .module_release_requests
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release reject rejected: request not found ({request_id})"
                        ),
                    })?;
                if matches!(
                    request.status,
                    ModuleReleaseRequestStatus::Applied | ModuleReleaseRequestStatus::Rejected
                ) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release reject invalid status for request {}: {:?}",
                            request_id, request.status
                        ),
                    });
                }
                if reason.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release reject reason cannot be empty (request_id={request_id})"
                        ),
                    });
                }
                request.status = ModuleReleaseRequestStatus::Rejected;
                request.rejected_reason = Some(reason.clone());
                request.updated_at = now;
                if let Some(mapping) = self.module_release_manifest_mappings.get_mut(request_id) {
                    mapping.status = ModuleReleaseRequestStatus::Rejected;
                    mapping.updated_at = now;
                }
                if let Some(cell) = self.agents.get_mut(rejector_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleReleaseApplied {
                request_id,
                operator_agent_id,
                manifest_hash,
                proposal_id,
                ..
            } => {
                let request = self
                    .module_release_requests
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release apply rejected: request not found ({request_id})"
                        ),
                    })?;
                if !matches!(request.status, ModuleReleaseRequestStatus::Approved) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release apply invalid status for request {}: {:?}",
                            request_id, request.status
                        ),
                    });
                }
                request.status = ModuleReleaseRequestStatus::Applied;
                request.applied_manifest_hash = Some(manifest_hash.clone());
                request.applied_proposal_id = if *proposal_id == 0 {
                    None
                } else {
                    Some(*proposal_id)
                };
                request.updated_at = now;
                let mapping = self
                    .module_release_manifest_mappings
                    .get_mut(request_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module release mapping missing for apply request_id={request_id}"
                        ),
                    })?;
                mapping.status = ModuleReleaseRequestStatus::Applied;
                mapping.applied_manifest_hash = Some(manifest_hash.clone());
                mapping.applied_proposal_id = if *proposal_id == 0 {
                    None
                } else {
                    Some(*proposal_id)
                };
                mapping.updated_at = now;
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleRollbackApplied {
                operator_agent_id,
                instance_id,
                module_id,
                from_module_version,
                to_module_version,
                wasm_hash,
                install_target,
                active,
                fee_kind,
                fee_amount,
                ..
            } => {
                self.settle_module_action_fee(
                    operator_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                let instance = self.module_instances.get_mut(instance_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("module instance missing for rollback {instance_id}"),
                    }
                })?;
                if instance.owner_agent_id != *operator_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance owner mismatch for rollback: instance={} owner={} operator={}",
                            instance_id, instance.owner_agent_id, operator_agent_id
                        ),
                    });
                }
                if instance.module_id != *module_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance module_id mismatch for rollback: instance={} state_module_id={} event_module_id={}",
                            instance_id, instance.module_id, module_id
                        ),
                    });
                }
                if instance.module_version != *from_module_version {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance from_version mismatch for rollback: instance={} state_version={} event_from={}",
                            instance_id, instance.module_version, from_module_version
                        ),
                    });
                }
                instance.module_version = to_module_version.clone();
                instance.wasm_hash = wasm_hash.clone();
                instance.install_target = install_target.clone();
                instance.active = *active;
                self.installed_module_targets
                    .insert(module_id.clone(), install_target.clone());
            }
            DomainEvent::ModuleArtifactListed {
                seller_agent_id,
                wasm_hash,
                price_kind,
                price_amount,
                order_id,
                fee_kind,
                fee_amount,
            } => {
                if *price_amount <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact listing price must be > 0, got {}",
                            price_amount
                        ),
                    });
                }
                let owner = self.module_artifact_owners.get(wasm_hash).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact owner missing for listing hash {}",
                            wasm_hash
                        ),
                    }
                })?;
                if owner != seller_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact listing seller mismatch: hash={} owner={} seller={}",
                            wasm_hash, owner, seller_agent_id
                        ),
                    });
                }
                self.settle_module_action_fee(
                    seller_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                self.module_artifact_listings.insert(
                    wasm_hash.clone(),
                    ModuleArtifactListingState {
                        order_id: *order_id,
                        seller_agent_id: seller_agent_id.clone(),
                        price_kind: *price_kind,
                        price_amount: *price_amount,
                        listed_at: now,
                    },
                );
                if *order_id > 0 {
                    self.next_module_market_order_id = self
                        .next_module_market_order_id
                        .max(order_id.saturating_add(1));
                }
            }
            DomainEvent::ModuleArtifactDelisted {
                seller_agent_id,
                wasm_hash,
                order_id,
                fee_kind,
                fee_amount,
            } => {
                let listing = self
                    .module_artifact_listings
                    .get(wasm_hash)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("module artifact listing missing for hash {}", wasm_hash),
                    })?;
                if listing.seller_agent_id != *seller_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact delist seller mismatch: hash={} listing_seller={} event_seller={}",
                            wasm_hash, listing.seller_agent_id, seller_agent_id
                        ),
                    });
                }
                if let Some(expected_order_id) = order_id {
                    if listing.order_id != *expected_order_id {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "module artifact delist order mismatch: hash={} listing_order_id={} event_order_id={}",
                                wasm_hash, listing.order_id, expected_order_id
                            ),
                        });
                    }
                }
                let owner = self.module_artifact_owners.get(wasm_hash).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact owner missing for delist hash {}",
                            wasm_hash
                        ),
                    }
                })?;
                if owner != seller_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact delist seller is not owner: hash={} owner={} seller={}",
                            wasm_hash, owner, seller_agent_id
                        ),
                    });
                }
                self.settle_module_action_fee(
                    seller_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                self.module_artifact_listings.remove(wasm_hash);
            }
            DomainEvent::ModuleArtifactDestroyed {
                owner_agent_id,
                wasm_hash,
                reason,
                fee_kind,
                fee_amount,
            } => {
                if reason.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact destroy reason cannot be empty for hash {}",
                            wasm_hash
                        ),
                    });
                }
                let owner = self.module_artifact_owners.get(wasm_hash).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact owner missing for destroy hash {}",
                            wasm_hash
                        ),
                    }
                })?;
                if owner != owner_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact destroy owner mismatch: hash={} owner={} event_owner={}",
                            wasm_hash, owner, owner_agent_id
                        ),
                    });
                }
                self.settle_module_action_fee(
                    owner_agent_id.as_str(),
                    *fee_kind,
                    *fee_amount,
                    now,
                )?;
                self.module_artifact_owners.remove(wasm_hash);
                self.module_artifact_listings.remove(wasm_hash);
                self.module_artifact_bids.remove(wasm_hash);
            }
            DomainEvent::ModuleArtifactBidPlaced {
                bidder_agent_id,
                wasm_hash,
                order_id,
                price_kind,
                price_amount,
            } => {
                if *order_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact bid order_id must be > 0 for hash {}",
                            wasm_hash
                        ),
                    });
                }
                if *price_amount <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact bid price must be > 0, got {}",
                            price_amount
                        ),
                    });
                }
                if !self.agents.contains_key(bidder_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: bidder_agent_id.clone(),
                    });
                }
                self.next_module_market_order_id = self
                    .next_module_market_order_id
                    .max(order_id.saturating_add(1));
                self.module_artifact_bids
                    .entry(wasm_hash.clone())
                    .or_default()
                    .push(ModuleArtifactBidState {
                        order_id: *order_id,
                        bidder_agent_id: bidder_agent_id.clone(),
                        price_kind: *price_kind,
                        price_amount: *price_amount,
                        bid_at: now,
                    });
                if let Some(cell) = self.agents.get_mut(bidder_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleArtifactBidCancelled {
                bidder_agent_id,
                wasm_hash,
                order_id,
                ..
            } => {
                let remove_empty_entry = {
                    let bids = self
                        .module_artifact_bids
                        .get_mut(wasm_hash)
                        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                            reason: format!("module artifact bids missing for hash {}", wasm_hash),
                        })?;
                    let before = bids.len();
                    bids.retain(|entry| {
                        !(entry.order_id == *order_id && entry.bidder_agent_id == *bidder_agent_id)
                    });
                    if before == bids.len() {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "module artifact bid cancel target not found: hash={} order_id={} bidder={}",
                                wasm_hash, order_id, bidder_agent_id
                            ),
                        });
                    }
                    bids.is_empty()
                };
                if remove_empty_entry {
                    self.module_artifact_bids.remove(wasm_hash);
                }
                if let Some(cell) = self.agents.get_mut(bidder_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ModuleArtifactSaleCompleted {
                buyer_agent_id,
                seller_agent_id,
                wasm_hash,
                price_kind,
                price_amount,
                sale_id,
                listing_order_id,
                bid_order_id,
            } => {
                if buyer_agent_id == seller_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact buyer and seller cannot be the same: {}",
                            buyer_agent_id
                        ),
                    });
                }
                if *price_amount <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact sale price must be > 0, got {}",
                            price_amount
                        ),
                    });
                }

                let listing = self
                    .module_artifact_listings
                    .get(wasm_hash)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("module artifact listing missing for hash {}", wasm_hash),
                    })?;
                if listing.seller_agent_id != *seller_agent_id
                    || listing.price_kind != *price_kind
                    || listing.price_amount != *price_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("module artifact listing mismatch for hash {}", wasm_hash),
                    });
                }
                if let Some(expected_listing_order_id) = listing_order_id {
                    if listing.order_id != *expected_listing_order_id {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "module artifact sale listing order mismatch: hash={} listing_order_id={} event_order_id={}",
                                wasm_hash, listing.order_id, expected_listing_order_id
                            ),
                        });
                    }
                }
                let owner = self.module_artifact_owners.get(wasm_hash).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact owner missing for sale hash {}",
                            wasm_hash
                        ),
                    }
                })?;
                if owner != seller_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module artifact sale seller is not owner: hash={} owner={} seller={}",
                            wasm_hash, owner, seller_agent_id
                        ),
                    });
                }

                let mut seller = self.agents.remove(seller_agent_id).ok_or_else(|| {
                    WorldError::AgentNotFound {
                        agent_id: seller_agent_id.clone(),
                    }
                })?;
                let mut buyer = self.agents.remove(buyer_agent_id).ok_or_else(|| {
                    WorldError::AgentNotFound {
                        agent_id: buyer_agent_id.clone(),
                    }
                })?;

                buyer
                    .state
                    .resources
                    .remove(*price_kind, *price_amount)
                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                        reason: format!("module artifact sale buyer debit failed: {err:?}"),
                    })?;
                seller
                    .state
                    .resources
                    .add(*price_kind, *price_amount)
                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                        reason: format!("module artifact sale seller credit failed: {err:?}"),
                    })?;
                seller.last_active = now;
                buyer.last_active = now;

                self.agents.insert(seller_agent_id.clone(), seller);
                self.agents.insert(buyer_agent_id.clone(), buyer);
                self.module_artifact_owners
                    .insert(wasm_hash.clone(), buyer_agent_id.clone());
                self.module_artifact_listings.remove(wasm_hash);
                if let Some(expected_bid_order_id) = bid_order_id {
                    let remove_empty_entry = {
                        let bids =
                            self.module_artifact_bids
                                .get_mut(wasm_hash)
                                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                    reason: format!(
                                        "module artifact sale bid missing for hash {} order_id {}",
                                        wasm_hash, expected_bid_order_id
                                    ),
                                })?;
                        let before = bids.len();
                        bids.retain(|entry| {
                            !(entry.order_id == *expected_bid_order_id
                                && entry.bidder_agent_id == *buyer_agent_id)
                        });
                        if before == bids.len() {
                            return Err(WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "module artifact sale bid not found: hash={} order_id={} buyer={}",
                                    wasm_hash, expected_bid_order_id, buyer_agent_id
                                ),
                            });
                        }
                        bids.is_empty()
                    };
                    if remove_empty_entry {
                        self.module_artifact_bids.remove(wasm_hash);
                    }
                }
                if *sale_id > 0 {
                    self.next_module_market_sale_id = self
                        .next_module_market_sale_id
                        .max(sale_id.saturating_add(1));
                }
            }
            DomainEvent::ResourceTransferred {
                from_agent_id,
                to_agent_id,
                kind,
                amount,
            } => {
                if from_agent_id == to_agent_id {
                    let cell = self.agents.get_mut(from_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: from_agent_id.clone(),
                        }
                    })?;
                    cell.last_active = now;
                } else {
                    // Validate and precompute both sides first so transfer is atomic.
                    let (next_from_resources, next_to_resources) = {
                        let from = self.agents.get(from_agent_id).ok_or_else(|| {
                            WorldError::AgentNotFound {
                                agent_id: from_agent_id.clone(),
                            }
                        })?;
                        let to = self.agents.get(to_agent_id).ok_or_else(|| {
                            WorldError::AgentNotFound {
                                agent_id: to_agent_id.clone(),
                            }
                        })?;

                        let mut next_from = from.state.resources.clone();
                        let mut next_to = to.state.resources.clone();
                        next_from.remove(*kind, *amount).map_err(|err| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!("transfer remove failed: {err:?}"),
                            }
                        })?;
                        next_to.add(*kind, *amount).map_err(|err| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!("transfer add failed: {err:?}"),
                            }
                        })?;
                        (next_from, next_to)
                    };

                    let from = self.agents.get_mut(from_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: from_agent_id.clone(),
                        }
                    })?;
                    from.state.resources = next_from_resources;
                    from.last_active = now;

                    let to = self.agents.get_mut(to_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: to_agent_id.clone(),
                        }
                    })?;
                    to.state.resources = next_to_resources;
                    to.last_active = now;
                }
            }
            DomainEvent::DataCollected {
                collector_agent_id,
                electricity_cost,
                data_amount,
            } => {
                if *electricity_cost <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "data collection electricity_cost must be > 0, got {}",
                            electricity_cost
                        ),
                    });
                }
                if *data_amount <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "data collection data_amount must be > 0, got {}",
                            data_amount
                        ),
                    });
                }
                let next_resources = {
                    let collector = self.agents.get(collector_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: collector_agent_id.clone(),
                        }
                    })?;
                    let mut next = collector.state.resources.clone();
                    next.remove(ResourceKind::Electricity, *electricity_cost)
                        .map_err(|err| WorldError::ResourceBalanceInvalid {
                            reason: format!("data collection electricity debit failed: {err:?}"),
                        })?;
                    next.add(ResourceKind::Data, *data_amount).map_err(|err| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!("data collection data credit failed: {err:?}"),
                        }
                    })?;
                    next
                };
                let collector = self.agents.get_mut(collector_agent_id).ok_or_else(|| {
                    WorldError::AgentNotFound {
                        agent_id: collector_agent_id.clone(),
                    }
                })?;
                collector.state.resources = next_resources;
                collector.last_active = now;
            }
            DomainEvent::DataAccessGranted {
                owner_agent_id,
                grantee_agent_id,
            } => {
                if !self.agents.contains_key(owner_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: owner_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(grantee_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: grantee_agent_id.clone(),
                    });
                }
                if owner_agent_id != grantee_agent_id {
                    self.data_access_permissions
                        .entry(owner_agent_id.clone())
                        .or_default()
                        .insert(grantee_agent_id.clone());
                }
                if let Some(owner) = self.agents.get_mut(owner_agent_id) {
                    owner.last_active = now;
                }
                if owner_agent_id != grantee_agent_id {
                    if let Some(grantee) = self.agents.get_mut(grantee_agent_id) {
                        grantee.last_active = now;
                    }
                }
            }
            DomainEvent::DataAccessRevoked {
                owner_agent_id,
                grantee_agent_id,
            } => {
                if !self.agents.contains_key(owner_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: owner_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(grantee_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: grantee_agent_id.clone(),
                    });
                }
                if owner_agent_id != grantee_agent_id {
                    let remove_owner_entry = if let Some(grantees) =
                        self.data_access_permissions.get_mut(owner_agent_id)
                    {
                        grantees.remove(grantee_agent_id);
                        grantees.is_empty()
                    } else {
                        false
                    };
                    if remove_owner_entry {
                        self.data_access_permissions.remove(owner_agent_id);
                    }
                }
                if let Some(owner) = self.agents.get_mut(owner_agent_id) {
                    owner.last_active = now;
                }
                if owner_agent_id != grantee_agent_id {
                    if let Some(grantee) = self.agents.get_mut(grantee_agent_id) {
                        grantee.last_active = now;
                    }
                }
            }
            DomainEvent::PowerRedeemed {
                node_id,
                target_agent_id,
                burned_credits,
                granted_power_units,
                reserve_remaining,
                nonce,
                ..
            } => {
                if *burned_credits == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "burned_credits must be > 0".to_string(),
                    });
                }
                if *granted_power_units <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "granted_power_units must be > 0, got {}",
                            granted_power_units
                        ),
                    });
                }
                let min_redeem_power_unit = self.reward_asset_config.min_redeem_power_unit;
                if min_redeem_power_unit <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "min_redeem_power_unit must be positive".to_string(),
                    });
                }
                if *granted_power_units < min_redeem_power_unit {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "granted_power_units below minimum: granted={} min={}",
                            granted_power_units, min_redeem_power_unit
                        ),
                    });
                }
                if *nonce == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "nonce must be > 0".to_string(),
                    });
                }
                if let Some(last_nonce) = self.node_redeem_nonces.get(node_id) {
                    if *nonce <= *last_nonce {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "nonce replay detected: node_id={} nonce={} last_nonce={}",
                                node_id, nonce, last_nonce
                            ),
                        });
                    }
                }
                let (next_power_credit_balance, next_total_burned_credits) = {
                    let node_balance = self.node_asset_balances.get(node_id).ok_or_else(|| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "power redeem burn failed: node balance not found: {node_id}"
                            ),
                        }
                    })?;
                    if node_balance.power_credit_balance < *burned_credits {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "power redeem burn failed: insufficient power credits: balance={} burn={}",
                                node_balance.power_credit_balance, burned_credits
                            ),
                        });
                    }
                    let next_total_burned_credits = node_balance
                        .total_burned_credits
                        .checked_add(*burned_credits)
                        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "power redeem burn failed: total_burned_credits overflow: current={} burn={}",
                                node_balance.total_burned_credits, burned_credits
                            ),
                        })?;
                    (
                        node_balance.power_credit_balance - *burned_credits,
                        next_total_burned_credits,
                    )
                };
                if self.protocol_power_reserve.available_power_units < *granted_power_units {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "insufficient protocol power reserve: available={} requested={}",
                            self.protocol_power_reserve.available_power_units, granted_power_units
                        ),
                    });
                }
                let next_reserve =
                    self.protocol_power_reserve.available_power_units - *granted_power_units;
                if next_reserve != *reserve_remaining {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "reserve remaining mismatch: computed={} event={}",
                            next_reserve, reserve_remaining
                        ),
                    });
                }
                let max_redeem_power_per_epoch =
                    self.reward_asset_config.max_redeem_power_per_epoch;
                if max_redeem_power_per_epoch <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "max_redeem_power_per_epoch must be positive".to_string(),
                    });
                }
                let next_redeemed = self
                    .protocol_power_reserve
                    .redeemed_power_units
                    .checked_add(*granted_power_units)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: "redeemed_power_units overflow".to_string(),
                    })?;
                if next_redeemed > max_redeem_power_per_epoch {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "epoch redeem cap exceeded: next={} cap={}",
                            next_redeemed, max_redeem_power_per_epoch
                        ),
                    });
                }
                let next_target_electricity = {
                    let target = self.agents.get(target_agent_id).ok_or_else(|| {
                        WorldError::AgentNotFound {
                            agent_id: target_agent_id.clone(),
                        }
                    })?;
                    let current = target.state.resources.get(ResourceKind::Electricity);
                    current.checked_add(*granted_power_units).ok_or_else(|| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "power redeem add electricity failed: overflow current={current} delta={}",
                                granted_power_units
                            ),
                        }
                    })?
                };

                {
                    let node_balance =
                        self.node_asset_balances.get_mut(node_id).ok_or_else(|| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "power redeem burn failed: node balance not found: {node_id}"
                                ),
                            }
                        })?;
                    node_balance.power_credit_balance = next_power_credit_balance;
                    node_balance.total_burned_credits = next_total_burned_credits;
                }
                self.protocol_power_reserve.available_power_units = next_reserve;
                self.protocol_power_reserve.redeemed_power_units = next_redeemed;
                self.node_redeem_nonces.insert(node_id.clone(), *nonce);

                let target = self.agents.get_mut(target_agent_id).ok_or_else(|| {
                    WorldError::AgentNotFound {
                        agent_id: target_agent_id.clone(),
                    }
                })?;
                if next_target_electricity == 0 {
                    target
                        .state
                        .resources
                        .amounts
                        .remove(&ResourceKind::Electricity);
                } else {
                    target
                        .state
                        .resources
                        .amounts
                        .insert(ResourceKind::Electricity, next_target_electricity);
                }
                target.last_active = now;
                if let Some(cell) = self.agents.get_mut(node_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::PowerRedeemRejected {
                node_id,
                target_agent_id,
                ..
            } => {
                if let Some(cell) = self.agents.get_mut(node_id) {
                    cell.last_active = now;
                }
                if let Some(cell) = self.agents.get_mut(target_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::NodePointsSettlementApplied {
                report,
                signer_node_id,
                settlement_hash,
                minted_records,
                main_token_bridge_total_amount,
                main_token_bridge_distributions,
            } => {
                apply_node_points_settlement_event(
                    self,
                    report,
                    signer_node_id.as_str(),
                    settlement_hash.as_str(),
                    minted_records.as_slice(),
                    *main_token_bridge_total_amount,
                    main_token_bridge_distributions.as_slice(),
                )?;
            }
            event @ DomainEvent::MainTokenGenesisInitialized { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenVestingClaimed { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenTransferred { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenEpochIssued { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenFeeSettled { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenPolicyUpdateScheduled { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MainTokenTreasuryDistributed { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::RestrictedStarterClaimGrantIssued { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::RestrictedStarterClaimGrantExpired { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::RestrictedStarterClaimGrantRevoked { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MaterialTransferred { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::MaterialTransitStarted { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::MaterialTransitCompleted { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryBuildStarted { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryBuilt { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryDurabilityChanged { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryMaintained { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryRecycled { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::RecipeStarted { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::RecipeCompleted { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryProductionBlocked { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            event @ DomainEvent::FactoryProductionResumed { .. } => {
                self.apply_domain_event_industry(event, now)?;
            }
            DomainEvent::MaterialProfileGoverned {
                operator_agent_id,
                proposal_id,
                profile,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "material profile governed proposal_id must be > 0".to_string(),
                    });
                }
                if profile.kind.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "material profile kind cannot be empty".to_string(),
                    });
                }
                if profile.tier == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("material profile tier must be >= 1: {}", profile.kind),
                    });
                }
                if profile.category.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "material profile category cannot be empty: {}",
                            profile.kind
                        ),
                    });
                }
                if profile.stack_limit <= 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "material profile stack_limit must be > 0: {}",
                            profile.kind
                        ),
                    });
                }
                self.material_profiles
                    .insert(profile.kind.clone(), profile.clone());
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ProductProfileGoverned {
                operator_agent_id,
                proposal_id,
                profile,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "product profile governed proposal_id must be > 0".to_string(),
                    });
                }
                if profile.product_id.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "product profile product_id cannot be empty".to_string(),
                    });
                }
                if profile.role_tag.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "product profile role_tag cannot be empty: {}",
                            profile.product_id
                        ),
                    });
                }
                self.product_profiles
                    .insert(profile.product_id.clone(), profile.clone());
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::RecipeProfileGoverned {
                operator_agent_id,
                proposal_id,
                profile,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "recipe profile governed proposal_id must be > 0".to_string(),
                    });
                }
                if profile.recipe_id.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "recipe profile recipe_id cannot be empty".to_string(),
                    });
                }
                self.recipe_profiles
                    .insert(profile.recipe_id.clone(), profile.clone());
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::FactoryProfileGoverned {
                operator_agent_id,
                proposal_id,
                profile,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "factory profile governed proposal_id must be > 0".to_string(),
                    });
                }
                if profile.factory_id.trim().is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "factory profile factory_id cannot be empty".to_string(),
                    });
                }
                if profile.tier == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "factory profile tier must be >= 1: {}",
                            profile.factory_id
                        ),
                    });
                }
                if profile.recipe_slots == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "factory profile recipe_slots must be > 0: {}",
                            profile.factory_id
                        ),
                    });
                }
                self.factory_profiles
                    .insert(profile.factory_id.clone(), profile.clone());
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            _ => unreachable!("apply_domain_event_core received unsupported event variant"),
        }
        Ok(())
    }
}
