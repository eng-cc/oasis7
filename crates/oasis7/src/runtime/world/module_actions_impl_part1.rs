impl World {
    fn module_deploy_fee_amount(bytes_len: usize) -> i64 {
        let bytes_len = bytes_len as i64;
        (bytes_len.saturating_add(MODULE_DEPLOY_FEE_BYTES_PER_ELECTRICITY - 1)
            / MODULE_DEPLOY_FEE_BYTES_PER_ELECTRICITY)
            .max(1)
    }

    fn module_compile_fee_amount(source_bytes_len: usize, wasm_bytes_len: usize) -> i64 {
        let total_bytes = source_bytes_len.saturating_add(wasm_bytes_len) as i64;
        (total_bytes.saturating_add(MODULE_COMPILE_FEE_BYTES_PER_ELECTRICITY - 1)
            / MODULE_COMPILE_FEE_BYTES_PER_ELECTRICITY)
            .max(2)
    }

    fn module_install_fee_amount(manifest: &oasis7_wasm_abi::ModuleManifest) -> i64 {
        let export_cost = manifest.exports.len() as i64;
        let subscription_cost = manifest.subscriptions.len() as i64;
        1_i64
            .saturating_add(export_cost)
            .saturating_add(subscription_cost)
            .max(1)
    }

    fn next_module_instance_id(&self, module_id: &str) -> String {
        let seq = self.state.next_module_instance_id.max(1);
        format!("{module_id}#{seq}")
    }

    fn validate_upgrade_interface_compatible(
        current: &oasis7_wasm_abi::ModuleManifest,
        next: &oasis7_wasm_abi::ModuleManifest,
    ) -> Result<(), String> {
        if current.interface_version != next.interface_version {
            return Err(format!(
                "upgrade interface_version mismatch: from={} to={}",
                current.interface_version, next.interface_version
            ));
        }

        let missing_exports: Vec<String> = current
            .exports
            .iter()
            .filter(|export_name| !next.exports.contains(*export_name))
            .cloned()
            .collect();
        if !missing_exports.is_empty() {
            return Err(format!(
                "upgrade exports incompatible: missing {:?}",
                missing_exports
            ));
        }

        for subscription in &current.subscriptions {
            if !next.subscriptions.contains(subscription) {
                return Err(
                    "upgrade subscriptions incompatible: existing subscription removed or modified"
                        .to_string(),
                );
            }
        }

        if current.abi_contract.abi_version != next.abi_contract.abi_version {
            return Err("upgrade abi_version mismatch".to_string());
        }
        if current.abi_contract.input_schema != next.abi_contract.input_schema
            || current.abi_contract.output_schema != next.abi_contract.output_schema
        {
            return Err("upgrade abi input/output schema mismatch".to_string());
        }
        for (slot, cap_ref) in &current.abi_contract.cap_slots {
            match next.abi_contract.cap_slots.get(slot) {
                Some(next_cap_ref) if next_cap_ref == cap_ref => {}
                _ => {
                    return Err(format!("upgrade abi cap slot mismatch for slot {}", slot));
                }
            }
        }
        for hook in &current.abi_contract.policy_hooks {
            if !next.abi_contract.policy_hooks.contains(hook) {
                return Err(format!(
                    "upgrade abi policy_hooks incompatible: missing {}",
                    hook
                ));
            }
        }
        for required_cap in &current.required_caps {
            if !next.required_caps.contains(required_cap) {
                return Err(format!(
                    "upgrade required_caps incompatible: missing {}",
                    required_cap
                ));
            }
        }
        Ok(())
    }

    fn ensure_module_fee_affordable(
        &mut self,
        action_id: u64,
        agent_id: &str,
        fee_kind: ResourceKind,
        fee_amount: i64,
    ) -> Result<bool, WorldError> {
        if fee_amount <= 0 {
            return Ok(true);
        }
        let available = self
            .state
            .agents
            .get(agent_id)
            .map(|cell| cell.state.resources.get(fee_kind))
            .unwrap_or(0);
        if available < fee_amount {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::InsufficientResource {
                        agent_id: agent_id.to_string(),
                        kind: fee_kind,
                        requested: fee_amount,
                        available,
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(false);
        }
        Ok(true)
    }

    fn has_active_module_using_artifact(&self, wasm_hash: &str) -> bool {
        if self
            .state
            .module_instances
            .values()
            .any(|instance| instance.active && instance.wasm_hash == wasm_hash)
        {
            return true;
        }
        self.module_registry
            .active
            .iter()
            .any(|(module_id, version)| {
                if self
                    .state
                    .module_instances
                    .values()
                    .any(|instance| instance.module_id == *module_id)
                {
                    return false;
                }
                let key = oasis7_wasm_abi::ModuleRegistry::record_key(module_id, version);
                self.module_registry
                    .records
                    .get(&key)
                    .map(|record| record.manifest.wasm_hash == wasm_hash)
                    .unwrap_or(false)
            })
    }

    fn peek_next_module_market_order_id(&self) -> u64 {
        self.state.next_module_market_order_id.max(1)
    }

    fn peek_next_module_market_sale_id(&self) -> u64 {
        self.state.next_module_market_sale_id.max(1)
    }

    fn peek_next_module_release_request_id(&self) -> u64 {
        self.state.next_module_release_request_id.max(1)
    }

    fn normalize_module_release_required_roles(required_roles: &[String]) -> Vec<String> {
        let mut normalized: Vec<String> = required_roles
            .iter()
            .map(|role| role.trim().to_ascii_lowercase())
            .filter(|role| !role.is_empty())
            .collect();
        normalized.sort();
        normalized.dedup();
        if normalized.is_empty() {
            normalized = MODULE_RELEASE_DEFAULT_REQUIRED_ROLES
                .iter()
                .map(|role| role.to_string())
                .collect();
        }
        normalized
    }

    fn normalize_module_release_role_set(roles: &[String]) -> Vec<String> {
        let mut normalized: Vec<String> = roles
            .iter()
            .map(|role| role.trim().to_ascii_lowercase())
            .filter(|role| !role.is_empty())
            .collect();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_module_release_role(role: &str) -> Option<String> {
        let normalized = role.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    fn module_release_roles_satisfied(
        required_roles: &[String],
        role_approvals: &std::collections::BTreeMap<String, String>,
    ) -> bool {
        required_roles
            .iter()
            .all(|required| role_approvals.contains_key(required))
    }

    fn module_release_attestation_key(signer_node_id: &str, platform: &str) -> String {
        format!(
            "{}|{}",
            signer_node_id.trim(),
            platform.trim().to_ascii_lowercase()
        )
    }

    fn normalize_module_release_attestation_platform(platform: &str) -> Option<String> {
        let normalized = platform.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    fn normalize_module_release_attestation_hash(raw: &str, field: &str) -> Result<String, String> {
        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(format!(
                "module release attestation rejected: {field} must be 64-char hex"
            ));
        }
        Ok(normalized)
    }

    fn normalize_module_release_attestation_builder_image_digest(
        raw: &str,
    ) -> Result<String, String> {
        let normalized = raw.trim().to_ascii_lowercase();
        let Some(digest_hex) = normalized.strip_prefix("sha256:") else {
            return Err(
                "module release attestation rejected: builder_image_digest must be sha256:<64-hex>"
                    .to_string(),
            );
        };
        if digest_hex.len() != 64 || !digest_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(
                "module release attestation rejected: builder_image_digest must be sha256:<64-hex>"
                    .to_string(),
            );
        }
        Ok(normalized)
    }

    fn normalize_module_release_attestation_label(
        raw: &str,
        field: &str,
    ) -> Result<String, String> {
        let normalized = raw.trim().to_string();
        if normalized.is_empty() {
            return Err(format!(
                "module release attestation rejected: {field} is empty"
            ));
        }
        if normalized.len() > 128 {
            return Err(format!(
                "module release attestation rejected: {field} exceeds 128 chars"
            ));
        }
        Ok(normalized)
    }

    fn normalize_module_release_attestation_proof_cid(proof_cid: &str) -> Option<String> {
        let normalized = proof_cid.trim().to_string();
        if normalized.is_empty() {
            return None;
        }
        if normalized.len() > 256 {
            return None;
        }
        Some(normalized)
    }

    fn evaluate_module_release_shadow_hash(
        &self,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        activate: bool,
    ) -> Result<String, String> {
        let mut changes = ModuleChangeSet::default();
        let record_key = oasis7_wasm_abi::ModuleRegistry::record_key(
            manifest.module_id.as_str(),
            manifest.version.as_str(),
        );
        if let Some(record) = self.module_registry.records.get(record_key.as_str()) {
            if record.manifest != *manifest {
                return Err(format!(
                    "module release shadow rejected: existing manifest mismatch for {}",
                    record_key
                ));
            }
        } else {
            changes.register.push(manifest.clone());
        }

        if activate {
            let already_active_same = self
                .module_registry
                .active
                .get(&manifest.module_id)
                .map(|version| version == &manifest.version)
                .unwrap_or(false);
            if !already_active_same {
                changes.activate.push(ModuleActivation {
                    module_id: manifest.module_id.clone(),
                    version: manifest.version.clone(),
                });
            }
        }

        if changes.is_empty() {
            return self
                .current_manifest_hash()
                .map_err(|err| format!("module release shadow hash failed: {err:?}"));
        }

        self.validate_module_changes(&changes)
            .map_err(|err| format!("module release shadow validate failed: {err:?}"))?;
        self.shadow_validate_module_changes(&changes)
            .map_err(|err| format!("module release shadow dry-run failed: {err:?}"))?;

        let module_changes_value = serde_json::to_value(&changes)
            .map_err(|err| format!("module release shadow serialize failed: {err}"))?;
        let mut manifest_update = self.manifest.clone();
        manifest_update.version = manifest_update.version.saturating_add(1);
        let serde_json::Value::Object(content) = &mut manifest_update.content else {
            return Err(
                "module release shadow rejected: current manifest content must be object"
                    .to_string(),
            );
        };
        content.insert("module_changes".to_string(), module_changes_value);
        super::super::util::hash_json(&manifest_update)
            .map_err(|err| format!("module release shadow hash failed: {err:?}"))
    }

    fn best_bid_for_listing(
        &self,
        wasm_hash: &str,
        listing: &ModuleArtifactListingState,
    ) -> Option<ModuleArtifactBidState> {
        let bids = self.state.module_artifact_bids.get(wasm_hash)?;
        let mut best: Option<ModuleArtifactBidState> = None;
        for bid in bids {
            if bid.price_kind != listing.price_kind {
                continue;
            }
            if bid.price_amount < listing.price_amount {
                continue;
            }
            if bid.bidder_agent_id == listing.seller_agent_id {
                continue;
            }
            let available = self
                .state
                .agents
                .get(&bid.bidder_agent_id)
                .map(|cell| cell.state.resources.get(listing.price_kind))
                .unwrap_or(0);
            if available < listing.price_amount {
                continue;
            }
            let replace = match &best {
                Some(current) => {
                    bid.price_amount > current.price_amount
                        || (bid.price_amount == current.price_amount
                            && bid.order_id < current.order_id)
                }
                None => true,
            };
            if replace {
                best = Some(bid.clone());
            }
        }
        best
    }

    fn try_match_module_listing(
        &mut self,
        wasm_hash: &str,
        action_id: u64,
    ) -> Result<(), WorldError> {
        let Some(listing) = self.state.module_artifact_listings.get(wasm_hash).cloned() else {
            return Ok(());
        };
        let Some(best_bid) = self.best_bid_for_listing(wasm_hash, &listing) else {
            return Ok(());
        };
        let sale_id = self.peek_next_module_market_sale_id();
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ModuleArtifactSaleCompleted {
                buyer_agent_id: best_bid.bidder_agent_id,
                seller_agent_id: listing.seller_agent_id,
                wasm_hash: wasm_hash.to_string(),
                price_kind: listing.price_kind,
                price_amount: listing.price_amount,
                sale_id,
                listing_order_id: if listing.order_id > 0 {
                    Some(listing.order_id)
                } else {
                    None
                },
                bid_order_id: Some(best_bid.order_id),
            }),
            Some(CausedBy::Action(action_id)),
        )?;
        Ok(())
    }

    fn apply_module_governance_proposal(
        &mut self,
        proposal_id: ProposalId,
        finality_certificate: Option<&GovernanceFinalityCertificate>,
    ) -> Result<String, WorldError> {
        match finality_certificate {
            Some(certificate) => self.apply_proposal_with_finality(proposal_id, certificate),
            None => self.apply_proposal(proposal_id),
        }
    }

    fn apply_install_module_action(
        &mut self,
        action_id: u64,
        installer_agent_id: &str,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        activate: bool,
        install_target: ModuleInstallTarget,
        finality_certificate: Option<&GovernanceFinalityCertificate>,
    ) -> Result<bool, WorldError> {
        if !self.state.agents.contains_key(installer_agent_id) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::AgentNotFound {
                        agent_id: installer_agent_id.to_string(),
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if let Some(owner_agent_id) = self.state.module_artifact_owners.get(&manifest.wasm_hash) {
            if owner_agent_id != installer_agent_id {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "install module artifact rejected: installer {} does not own {} (owner {})",
                                installer_agent_id, manifest.wasm_hash, owner_agent_id
                            )],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        }
        let fee_kind = ResourceKind::Electricity;
        let fee_amount = Self::module_install_fee_amount(manifest);
        if !self.ensure_module_fee_affordable(
            action_id,
            installer_agent_id,
            fee_kind,
            fee_amount,
        )? {
            return Ok(true);
        }

        let mut changes = ModuleChangeSet::default();
        let record_key = oasis7_wasm_abi::ModuleRegistry::record_key(
            manifest.module_id.as_str(),
            manifest.version.as_str(),
        );
        if let Some(record) = self.module_registry.records.get(record_key.as_str()) {
            if record.manifest != *manifest {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "install module rejected: existing manifest mismatch for {}",
                                record_key
                            )],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        } else {
            changes.register.push(manifest.clone());
        }
        if activate {
            let already_active_same = self
                .module_registry
                .active
                .get(&manifest.module_id)
                .map(|version| version == &manifest.version)
                .unwrap_or(false);
            if !already_active_same {
                changes.activate.push(ModuleActivation {
                    module_id: manifest.module_id.clone(),
                    version: manifest.version.clone(),
                });
            }
        }

        let (proposal_id, manifest_hash) = if changes.is_empty() {
            (0, self.current_manifest_hash()?)
        } else {
            let module_changes_value = match serde_json::to_value(&changes) {
                Ok(value) => value,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("serialize module changes failed: {err}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };

            let mut manifest_update = self.manifest.clone();
            manifest_update.version = manifest_update.version.saturating_add(1);
            let serde_json::Value::Object(content) = &mut manifest_update.content else {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["current manifest content must be object".to_string()],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            };
            content.insert("module_changes".to_string(), module_changes_value);

            let proposal_id = match self
                .propose_manifest_update(manifest_update, installer_agent_id.to_string())
            {
                Ok(proposal_id) => proposal_id,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("propose module install rejected: {err:?}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };

            if let Err(err) = self.shadow_proposal(proposal_id) {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("shadow module install rejected: {err:?}")],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }

            if let Err(err) = self.approve_proposal(
                proposal_id,
                installer_agent_id.to_string(),
                ProposalDecision::Approve,
            ) {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("approve module install rejected: {err:?}")],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }

            let manifest_hash =
                match self.apply_module_governance_proposal(proposal_id, finality_certificate) {
                    Ok(hash) => hash,
                    Err(err) => {
                        self.append_event(
                            WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!("apply module install rejected: {err:?}")],
                                },
                            }),
                            Some(CausedBy::Action(action_id)),
                        )?;
                        return Ok(true);
                    }
                };
            (proposal_id, manifest_hash)
        };

        let instance_id = self.next_module_instance_id(manifest.module_id.as_str());

        self.append_event(
            WorldEventBody::Domain(DomainEvent::ModuleInstalled {
                installer_agent_id: installer_agent_id.to_string(),
                instance_id,
                module_id: manifest.module_id.clone(),
                module_version: manifest.version.clone(),
                wasm_hash: manifest.wasm_hash.clone(),
                install_target,
                active: activate,
                proposal_id,
                manifest_hash,
                fee_kind,
                fee_amount,
            }),
            Some(CausedBy::Action(action_id)),
        )?;
        Ok(true)
    }

    fn apply_upgrade_module_action(
        &mut self,
        action_id: u64,
        upgrader_agent_id: &str,
        instance_id: &str,
        from_module_version: &str,
        manifest: &oasis7_wasm_abi::ModuleManifest,
        activate: bool,
        finality_certificate: Option<&GovernanceFinalityCertificate>,
    ) -> Result<bool, WorldError> {
        if !self.state.agents.contains_key(upgrader_agent_id) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::AgentNotFound {
                        agent_id: upgrader_agent_id.to_string(),
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let Some(instance) = self.state.module_instances.get(instance_id).cloned() else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: instance not found {}",
                            instance_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        if instance.owner_agent_id != upgrader_agent_id {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: upgrader {} does not own instance {} (owner {})",
                            upgrader_agent_id, instance_id, instance.owner_agent_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if instance.module_version != from_module_version {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: from_version mismatch for instance {} expected {} got {}",
                            instance_id, instance.module_version, from_module_version
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if manifest.module_id != instance.module_id {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: manifest module_id mismatch for instance {} expected {} got {}",
                            instance_id, instance.module_id, manifest.module_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if manifest.version == instance.module_version {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: target version equals current version {}",
                            manifest.version
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if let Some(owner_agent_id) = self.state.module_artifact_owners.get(&manifest.wasm_hash) {
            if owner_agent_id != upgrader_agent_id {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "upgrade module artifact rejected: upgrader {} does not own {} (owner {})",
                                upgrader_agent_id, manifest.wasm_hash, owner_agent_id
                            )],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        }

        let current_key = oasis7_wasm_abi::ModuleRegistry::record_key(
            instance.module_id.as_str(),
            instance.module_version.as_str(),
        );
        let Some(current_record) = self.module_registry.records.get(current_key.as_str()) else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "upgrade module rejected: current module record missing {}",
                            current_key
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        if let Err(reason) =
            Self::validate_upgrade_interface_compatible(&current_record.manifest, manifest)
        {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("upgrade module rejected: {reason}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let fee_kind = ResourceKind::Electricity;
        let fee_amount = Self::module_install_fee_amount(manifest);
        if !self.ensure_module_fee_affordable(action_id, upgrader_agent_id, fee_kind, fee_amount)? {
            return Ok(true);
        }

        let mut changes = ModuleChangeSet {
            upgrade: vec![ModuleUpgrade {
                module_id: instance.module_id.clone(),
                from_version: instance.module_version.clone(),
                to_version: manifest.version.clone(),
                wasm_hash: manifest.wasm_hash.clone(),
                manifest: manifest.clone(),
            }],
            ..ModuleChangeSet::default()
        };
        if activate {
            changes.activate.push(ModuleActivation {
                module_id: manifest.module_id.clone(),
                version: manifest.version.clone(),
            });
        }

        let module_changes_value = match serde_json::to_value(&changes) {
            Ok(value) => value,
            Err(err) => {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("serialize module changes failed: {err}")],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        };

        let mut manifest_update = self.manifest.clone();
        manifest_update.version = manifest_update.version.saturating_add(1);
        let serde_json::Value::Object(content) = &mut manifest_update.content else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["current manifest content must be object".to_string()],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        content.insert("module_changes".to_string(), module_changes_value);

        let proposal_id =
            match self.propose_manifest_update(manifest_update, upgrader_agent_id.to_string()) {
                Ok(proposal_id) => proposal_id,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("propose module upgrade rejected: {err:?}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };

        if let Err(err) = self.shadow_proposal(proposal_id) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("shadow module upgrade rejected: {err:?}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        if let Err(err) = self.approve_proposal(
            proposal_id,
            upgrader_agent_id.to_string(),
            ProposalDecision::Approve,
        ) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("approve module upgrade rejected: {err:?}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let manifest_hash =
            match self.apply_module_governance_proposal(proposal_id, finality_certificate) {
                Ok(hash) => hash,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("apply module upgrade rejected: {err:?}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };

        self.append_event(
            WorldEventBody::Domain(DomainEvent::ModuleUpgraded {
                upgrader_agent_id: upgrader_agent_id.to_string(),
                instance_id: instance.instance_id,
                module_id: instance.module_id,
                from_module_version: from_module_version.to_string(),
                to_module_version: manifest.version.clone(),
                wasm_hash: manifest.wasm_hash.clone(),
                install_target: instance.install_target,
                active: activate,
                proposal_id,
                manifest_hash,
                fee_kind,
                fee_amount,
            }),
            Some(CausedBy::Action(action_id)),
        )?;
        Ok(true)
    }

    fn apply_rollback_module_instance_action(
        &mut self,
        action_id: u64,
        operator_agent_id: &str,
        instance_id: &str,
        target_module_version: &str,
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
        let target_module_version = target_module_version.trim();
        if target_module_version.is_empty() {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![
                            "rollback module rejected: target_module_version is empty".to_string()
                        ],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let Some(instance) = self.state.module_instances.get(instance_id).cloned() else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "rollback module rejected: instance not found {}",
                            instance_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        if instance.owner_agent_id != operator_agent_id {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "rollback module rejected: operator {} does not own instance {} (owner {})",
                            operator_agent_id, instance_id, instance.owner_agent_id
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if instance.module_version == target_module_version {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "rollback module rejected: target version equals current version {}",
                            target_module_version
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }

        let current_key = oasis7_wasm_abi::ModuleRegistry::record_key(
            instance.module_id.as_str(),
            instance.module_version.as_str(),
        );
        let Some(current_record) = self.module_registry.records.get(current_key.as_str()) else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "rollback module rejected: current module record missing {}",
                            current_key
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        let target_key = oasis7_wasm_abi::ModuleRegistry::record_key(
            instance.module_id.as_str(),
            target_module_version,
        );
        let Some(target_record) = self.module_registry.records.get(target_key.as_str()) else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "rollback module rejected: target version not found {}",
                            target_key
                        )],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        if let Err(reason) = Self::validate_upgrade_interface_compatible(
            &current_record.manifest,
            &target_record.manifest,
        ) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("rollback module rejected: {reason}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        let target_manifest = target_record.manifest.clone();
        let fee_kind = ResourceKind::Electricity;
        let fee_amount = Self::module_install_fee_amount(&target_manifest);
        if !self.ensure_module_fee_affordable(action_id, operator_agent_id, fee_kind, fee_amount)? {
            return Ok(true);
        }

        let mut changes = ModuleChangeSet::default();
        if instance.active {
            changes.activate.push(ModuleActivation {
                module_id: target_manifest.module_id.clone(),
                version: target_manifest.version.clone(),
            });
        }

        let module_changes_value = match serde_json::to_value(&changes) {
            Ok(value) => value,
            Err(err) => {
                self.append_event(
                    WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("serialize module rollback changes failed: {err}")],
                        },
                    }),
                    Some(CausedBy::Action(action_id)),
                )?;
                return Ok(true);
            }
        };
        let mut manifest_update = self.manifest.clone();
        manifest_update.version = manifest_update.version.saturating_add(1);
        let serde_json::Value::Object(content) = &mut manifest_update.content else {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec!["current manifest content must be object".to_string()],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        };
        content.insert("module_changes".to_string(), module_changes_value);
        let proposal_id =
            match self.propose_manifest_update(manifest_update, operator_agent_id.to_string()) {
                Ok(proposal_id) => proposal_id,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("propose module rollback rejected: {err:?}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };
        if let Err(err) = self.shadow_proposal(proposal_id) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("shadow module rollback rejected: {err:?}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        if let Err(err) = self.approve_proposal(
            proposal_id,
            operator_agent_id.to_string(),
            ProposalDecision::Approve,
        ) {
            self.append_event(
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("approve module rollback rejected: {err:?}")],
                    },
                }),
                Some(CausedBy::Action(action_id)),
            )?;
            return Ok(true);
        }
        let manifest_hash =
            match self.apply_module_governance_proposal(proposal_id, finality_certificate) {
                Ok(hash) => hash,
                Err(err) => {
                    self.append_event(
                        WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("apply module rollback rejected: {err:?}")],
                            },
                        }),
                        Some(CausedBy::Action(action_id)),
                    )?;
                    return Ok(true);
                }
            };
        self.append_event(
            WorldEventBody::Domain(DomainEvent::ModuleRollbackApplied {
                operator_agent_id: operator_agent_id.to_string(),
                instance_id: instance.instance_id.clone(),
                module_id: instance.module_id.clone(),
                from_module_version: instance.module_version,
                to_module_version: target_module_version.to_string(),
                wasm_hash: target_manifest.wasm_hash,
                install_target: instance.install_target,
                active: instance.active,
                proposal_id,
                manifest_hash,
                fee_kind,
                fee_amount,
            }),
            Some(CausedBy::Action(action_id)),
        )?;
        Ok(true)
    }
}
