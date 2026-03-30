use super::*;

impl WorldState {
    pub(super) fn apply_domain_event_core_late(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
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
            event @ DomainEvent::MainTokenGenesisInitialized { .. }
            | event @ DomainEvent::MainTokenVestingClaimed { .. }
            | event @ DomainEvent::MainTokenTransferred { .. }
            | event @ DomainEvent::MainTokenEpochIssued { .. }
            | event @ DomainEvent::MainTokenFeeSettled { .. }
            | event @ DomainEvent::MainTokenPolicyUpdateScheduled { .. }
            | event @ DomainEvent::MainTokenTreasuryDistributed { .. }
            | event @ DomainEvent::RestrictedStarterClaimGrantIssued { .. }
            | event @ DomainEvent::RestrictedStarterClaimGrantExpired { .. }
            | event @ DomainEvent::RestrictedStarterClaimGrantRevoked { .. } => {
                self.apply_domain_event_main_token(event, now)?;
            }
            event @ DomainEvent::MaterialTransferred { .. }
            | event @ DomainEvent::MaterialTransitStarted { .. }
            | event @ DomainEvent::MaterialTransitCompleted { .. }
            | event @ DomainEvent::FactoryBuildStarted { .. }
            | event @ DomainEvent::FactoryBuilt { .. }
            | event @ DomainEvent::FactoryDurabilityChanged { .. }
            | event @ DomainEvent::FactoryMaintained { .. }
            | event @ DomainEvent::FactoryRecycled { .. }
            | event @ DomainEvent::RecipeStarted { .. }
            | event @ DomainEvent::RecipeCompleted { .. }
            | event @ DomainEvent::FactoryProductionBlocked { .. }
            | event @ DomainEvent::FactoryProductionResumed { .. } => {
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
            _ => unreachable!("apply_domain_event_core_late received unsupported event"),
        }
        Ok(())
    }
}
