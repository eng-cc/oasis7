use super::super::init::{
    generate_chunk_fragments, summarize_chunk_generation, AsteroidFragmentInitConfig,
    WorldInitConfig,
};
use super::super::persist::PersistError;
use super::super::power::PowerEvent;
use super::super::types::{ResourceKind, ResourceOwner, StockError};
use super::super::world_model::{Factory, Location, PowerOrderState};
use super::super::ChunkState;
use super::types::{WorldEvent, WorldEventKind};
use super::WorldKernel;

const LOCATION_ELECTRICITY_POOL_REMOVED_NOTE: &str = "location electricity pool removed";
const FACTORY_KIND_RADIATION_POWER_MK1: &str = "factory.power.radiation.mk1";

impl WorldKernel {
    pub(super) fn apply_event(&mut self, event: &WorldEvent) -> Result<(), PersistError> {
        if event.id != self.next_event_id {
            return Err(PersistError::ReplayConflict {
                message: format!(
                    "event id mismatch: expected {}, got {}",
                    self.next_event_id, event.id
                ),
            });
        }
        if event.time < self.time {
            return Err(PersistError::ReplayConflict {
                message: format!(
                    "event time regression: current {}, got {}",
                    self.time, event.time
                ),
            });
        }
        self.time = event.time;
        self.next_event_id = self.next_event_id.saturating_add(1);

        if let Some(result) = self.replay_module_lifecycle_event(&event.kind) {
            result.map_err(|message| PersistError::ReplayConflict { message })?;
            return Ok(());
        }

        match &event.kind {
            WorldEventKind::ModuleArtifactDeployed { .. }
            | WorldEventKind::ModuleInstalled { .. }
            | WorldEventKind::ModuleArtifactListed { .. }
            | WorldEventKind::ModuleArtifactDelisted { .. }
            | WorldEventKind::ModuleArtifactBidPlaced { .. }
            | WorldEventKind::ModuleArtifactBidCancelled { .. }
            | WorldEventKind::ModuleArtifactSaleCompleted { .. }
            | WorldEventKind::ModuleArtifactDestroyed { .. } => {
                unreachable!("module lifecycle events should be handled before replay match")
            }
            WorldEventKind::LocationRegistered {
                location_id,
                name,
                pos,
                profile,
            } => {
                if self.model.locations.contains_key(location_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("location already exists: {location_id}"),
                    });
                }
                self.model.locations.insert(
                    location_id.clone(),
                    Location::new_with_profile(
                        location_id.clone(),
                        name.clone(),
                        *pos,
                        profile.clone(),
                    ),
                );
            }
            WorldEventKind::AgentRegistered {
                agent_id,
                location_id,
                pos,
            } => {
                if self.model.agents.contains_key(agent_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent already exists: {agent_id}"),
                    });
                }
                if !self.model.locations.contains_key(location_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("location not found: {location_id}"),
                    });
                }
                let mut agent = super::super::world_model::Agent::new_with_power(
                    agent_id.clone(),
                    location_id.clone(),
                    *pos,
                    &self.config.power,
                );
                agent.pos = *pos;
                self.model.agents.insert(agent_id.clone(), agent);
            }
            WorldEventKind::AgentMoved {
                agent_id,
                from,
                to,
                electricity_cost,
                ..
            } => {
                let Some(location) = self.model.locations.get(to) else {
                    return Err(PersistError::ReplayConflict {
                        message: format!("location not found: {to}"),
                    });
                };
                let Some(agent) = self.model.agents.get_mut(agent_id) else {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent not found: {agent_id}"),
                    });
                };
                if &agent.location_id != from {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent {agent_id} not at expected location {from}"),
                    });
                }
                if *electricity_cost > 0 {
                    let available = agent.resources.get(ResourceKind::Electricity);
                    if available < *electricity_cost {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "insufficient electricity for move: requested {electricity_cost}, available {available}"
                            ),
                        });
                    }
                    if let Err(err) = agent
                        .resources
                        .remove(ResourceKind::Electricity, *electricity_cost)
                    {
                        return Err(PersistError::ReplayConflict {
                            message: format!("failed to apply move cost: {err:?}"),
                        });
                    }
                }
                agent.location_id = to.clone();
                agent.pos = location.pos;
            }
            WorldEventKind::AgentSpoke { .. }
            | WorldEventKind::TargetInspected { .. }
            | WorldEventKind::SimpleInteractionPerformed { .. } => {}
            WorldEventKind::ResourceTransferred {
                from,
                to,
                kind,
                amount,
            } => {
                if *amount <= 0 {
                    return Err(PersistError::ReplayConflict {
                        message: "transfer amount must be positive".to_string(),
                    });
                }
                self.ensure_owner_exists(from)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid transfer source: {reason:?}"),
                    })?;
                self.ensure_owner_exists(to)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid transfer target: {reason:?}"),
                    })?;
                self.remove_from_owner_for_replay(from, *kind, *amount)?;
                self.add_to_owner_for_replay(to, *kind, *amount)?;
            }
            WorldEventKind::DebugResourceGranted {
                owner,
                kind,
                amount,
            } => {
                if *amount <= 0 {
                    return Err(PersistError::ReplayConflict {
                        message: "debug grant amount must be positive".to_string(),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid debug grant owner: {reason:?}"),
                    })?;
                self.add_to_owner_for_replay(owner, *kind, *amount)?;
            }
            WorldEventKind::RadiationHarvested {
                agent_id, amount, ..
            } => {
                let Some(agent) = self.model.agents.get_mut(agent_id) else {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent not found: {agent_id}"),
                    });
                };
                agent
                    .resources
                    .add(ResourceKind::Electricity, *amount)
                    .map_err(|err| PersistError::ReplayConflict {
                        message: format!("failed to apply radiation harvest: {err:?}"),
                    })?;
            }
            WorldEventKind::CompoundMined {
                owner,
                location_id,
                compound_mass_g,
                electricity_cost,
                extracted_elements,
            } => {
                if *compound_mass_g <= 0 || *electricity_cost < 0 {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "invalid compound mined event values: mass={}, electricity_cost={}",
                            compound_mass_g, electricity_cost
                        ),
                    });
                }
                if !self.model.locations.contains_key(location_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("mining location not found: {location_id}"),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid mining owner: {reason:?}"),
                    })?;
                self.ensure_colocated(
                    owner,
                    &ResourceOwner::Location {
                        location_id: location_id.clone(),
                    },
                )
                .map_err(|reason| PersistError::ReplayConflict {
                    message: format!("mining owner and location not colocated: {reason:?}"),
                })?;

                let extracted_total: i64 = extracted_elements.values().copied().sum();
                if extracted_total != *compound_mass_g {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "compound mined extracted total mismatch: mass={}, extracted_total={}",
                            compound_mass_g, extracted_total
                        ),
                    });
                }
                for (element, amount) in extracted_elements {
                    if *amount <= 0 {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "compound mined extracted element amount must be positive: {:?}={}",
                                element, amount
                            ),
                        });
                    }
                }

                self.remove_from_owner_for_replay(
                    owner,
                    ResourceKind::Electricity,
                    *electricity_cost,
                )?;
                for (element, amount) in extracted_elements {
                    self.model
                        .consume_fragment_resource(
                            location_id,
                            &self.config.space,
                            *element,
                            *amount,
                        )
                        .map_err(|err| PersistError::ReplayConflict {
                            message: format!(
                                "failed to apply mining fragment consumption at {} for {:?}: {:?}",
                                location_id, element, err
                            ),
                        })?;
                }
                self.add_to_owner_for_replay(owner, ResourceKind::Data, *compound_mass_g)?;
                let location = self.model.locations.get_mut(location_id).ok_or_else(|| {
                    PersistError::ReplayConflict {
                        message: format!("mining location missing after consume: {location_id}"),
                    }
                })?;
                location.mined_compound_g = location
                    .mined_compound_g
                    .max(0)
                    .saturating_add(*compound_mass_g);
            }
            WorldEventKind::CompoundRefined {
                owner,
                compound_mass_g,
                electricity_cost,
                hardware_output,
            } => {
                if *compound_mass_g <= 0 || *electricity_cost < 0 || *hardware_output <= 0 {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "invalid refine event values: mass={}, electricity_cost={}, hardware_output={}",
                            compound_mass_g, electricity_cost, hardware_output
                        ),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid refine owner: {reason:?}"),
                    })?;
                self.remove_from_owner_for_replay(owner, ResourceKind::Data, *compound_mass_g)?;
                self.remove_from_owner_for_replay(
                    owner,
                    ResourceKind::Electricity,
                    *electricity_cost,
                )?;
                self.add_to_owner_for_replay(owner, ResourceKind::Data, *hardware_output)?;
            }
            WorldEventKind::FactoryBuilt {
                owner,
                location_id,
                factory_id,
                factory_kind,
                electricity_cost,
                hardware_cost,
            } => {
                if factory_id.trim().is_empty() || factory_kind.trim().is_empty() {
                    return Err(PersistError::ReplayConflict {
                        message: "invalid factory build event payload".to_string(),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid factory owner: {reason:?}"),
                    })?;
                if !self.model.locations.contains_key(location_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("factory location not found: {location_id}"),
                    });
                }
                if self.model.factories.contains_key(factory_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("factory already exists: {factory_id}"),
                    });
                }
                self.remove_from_owner_for_replay(
                    owner,
                    ResourceKind::Electricity,
                    *electricity_cost,
                )?;
                self.remove_from_owner_for_replay(owner, ResourceKind::Data, *hardware_cost)?;
                self.model.factories.insert(
                    factory_id.clone(),
                    Factory {
                        id: factory_id.clone(),
                        owner: owner.clone(),
                        location_id: location_id.clone(),
                        kind: factory_kind.clone(),
                    },
                );
                if factory_kind.eq_ignore_ascii_case(FACTORY_KIND_RADIATION_POWER_MK1) {
                    if self.model.power_plants.contains_key(factory_id) {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "power facility already exists while replaying factory build: {factory_id}"
                            ),
                        });
                    }
                    self.model.power_plants.insert(
                        factory_id.clone(),
                        super::super::power::PowerPlant {
                            id: factory_id.clone(),
                            location_id: location_id.clone(),
                            owner: owner.clone(),
                            capacity_per_tick: self
                                .config
                                .economy
                                .radiation_power_plant_output_per_tick,
                            current_output: 0,
                            fuel_cost_per_pu: 0,
                            maintenance_cost: 0,
                            status: super::super::power::PlantStatus::Running,
                            efficiency: 1.0,
                            degradation: 0.0,
                        },
                    );
                }
            }
            WorldEventKind::RecipeScheduled {
                owner,
                factory_id,
                recipe_id,
                batches,
                electricity_cost,
                hardware_cost,
                data_output,
                finished_product_id,
                finished_product_units,
            } => {
                if recipe_id.trim().is_empty()
                    || finished_product_id.trim().is_empty()
                    || *batches <= 0
                    || *finished_product_units < 0
                {
                    return Err(PersistError::ReplayConflict {
                        message: "invalid recipe scheduled event payload".to_string(),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid recipe owner: {reason:?}"),
                    })?;
                let Some(factory) = self.model.factories.get(factory_id) else {
                    return Err(PersistError::ReplayConflict {
                        message: format!("factory not found for recipe: {factory_id}"),
                    });
                };
                if &factory.owner != owner {
                    return Err(PersistError::ReplayConflict {
                        message: format!("factory owner mismatch: {factory_id}"),
                    });
                }

                self.remove_from_owner_for_replay(
                    owner,
                    ResourceKind::Electricity,
                    *electricity_cost,
                )?;
                self.remove_from_owner_for_replay(owner, ResourceKind::Data, *hardware_cost)?;
                if *data_output > 0 {
                    self.add_to_owner_for_replay(owner, ResourceKind::Data, *data_output)?;
                }
            }
            WorldEventKind::ChunkGenerated {
                coord,
                seed,
                fragment_count,
                block_count,
                chunk_budget,
                ..
            } => {
                if !self.model.chunks.contains_key(coord) {
                    self.model.chunks.insert(*coord, ChunkState::Unexplored);
                }

                let actual = if self.chunk_runtime.asteroid_fragment_enabled {
                    let init = WorldInitConfig {
                        seed: self.chunk_runtime.world_seed,
                        asteroid_fragment: AsteroidFragmentInitConfig {
                            enabled: self.chunk_runtime.asteroid_fragment_enabled,
                            seed_offset: self.chunk_runtime.asteroid_fragment_seed_offset,
                            min_fragment_spacing_cm: self.chunk_runtime.min_fragment_spacing_cm,
                            bootstrap_chunks: Vec::new(),
                        },
                        ..WorldInitConfig::default()
                    };
                    generate_chunk_fragments(
                        &mut self.model,
                        &self.config,
                        &init,
                        *coord,
                        Some(self.chunk_runtime.asteroid_fragment_seed()),
                    )
                    .map_err(|err| PersistError::ReplayConflict {
                        message: format!(
                            "chunk generation failed during replay at ({}, {}, {}): {err:?}",
                            coord.x, coord.y, coord.z
                        ),
                    })?
                } else {
                    self.model.chunks.insert(*coord, ChunkState::Generated);
                    self.model.chunk_resource_budgets.entry(*coord).or_default();
                    summarize_chunk_generation(&self.model, &self.config, *coord, *seed)
                };

                if actual.seed != *seed
                    || actual.fragment_count != *fragment_count
                    || actual.block_count != *block_count
                    || actual.chunk_budget != *chunk_budget
                {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "chunk replay mismatch at ({}, {}, {}): expected seed={}, fragments={}, blocks={}, budget={:?}; actual seed={}, fragments={}, blocks={}, budget={:?}",
                            coord.x,
                            coord.y,
                            coord.z,
                            seed,
                            fragment_count,
                            block_count,
                            chunk_budget,
                            actual.seed,
                            actual.fragment_count,
                            actual.block_count,
                            actual.chunk_budget
                        ),
                    });
                }
            }
            WorldEventKind::FragmentsReplenished { entries } => {
                self.apply_fragment_replenished_entries(entries)
                    .map_err(|err| PersistError::ReplayConflict {
                        message: format!("failed to apply fragment replenish event: {err}"),
                    })?;
            }
            WorldEventKind::AgentPromptUpdated { profile, .. } => {
                if !self.model.agents.contains_key(&profile.agent_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "agent not found for prompt profile: {}",
                            profile.agent_id
                        ),
                    });
                }
                self.model
                    .agent_prompt_profiles
                    .insert(profile.agent_id.clone(), profile.clone());
            }
            WorldEventKind::AgentPlayerBound {
                agent_id,
                player_id,
                public_key,
            } => {
                if !self.model.agents.contains_key(agent_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent not found for player binding: {}", agent_id),
                    });
                }
                if player_id.trim().is_empty() {
                    return Err(PersistError::ReplayConflict {
                        message: format!("empty player_id for agent binding: {}", agent_id),
                    });
                }
                let normalized_public_key = public_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                if public_key.is_some() && normalized_public_key.is_none() {
                    return Err(PersistError::ReplayConflict {
                        message: format!("empty public_key for agent binding: {}", agent_id),
                    });
                }
                self.model
                    .agent_player_bindings
                    .insert(agent_id.clone(), player_id.clone());
                match normalized_public_key {
                    Some(value) => {
                        self.model
                            .agent_player_public_key_bindings
                            .insert(agent_id.clone(), value);
                    }
                    None => {
                        self.model.agent_player_public_key_bindings.remove(agent_id);
                    }
                }
            }
            WorldEventKind::AgentPlayerUnbound {
                agent_id,
                player_id,
                public_key,
            } => {
                if !self.model.agents.contains_key(agent_id) {
                    return Err(PersistError::ReplayConflict {
                        message: format!("agent not found for player unbinding: {}", agent_id),
                    });
                }
                if player_id.trim().is_empty() {
                    return Err(PersistError::ReplayConflict {
                        message: format!("empty player_id for agent unbinding: {}", agent_id),
                    });
                }
                let normalized_public_key = public_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                if public_key.is_some() && normalized_public_key.is_none() {
                    return Err(PersistError::ReplayConflict {
                        message: format!("empty public_key for agent unbinding: {}", agent_id),
                    });
                }
                if let Some(bound_player_id) = self.model.agent_player_bindings.get(agent_id) {
                    if bound_player_id != player_id {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "player unbinding mismatch for agent {}: expected={} actual={}",
                                agent_id, bound_player_id, player_id
                            ),
                        });
                    }
                }
                if let Some(existing_public_key) =
                    self.model.agent_player_public_key_bindings.get(agent_id)
                {
                    if normalized_public_key.as_deref() != Some(existing_public_key.as_str()) {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "public_key mismatch for agent unbinding: {}",
                                agent_id
                            ),
                        });
                    }
                }
                self.model.agent_player_bindings.remove(agent_id);
                self.model.agent_player_public_key_bindings.remove(agent_id);
            }
            WorldEventKind::SocialFactPublished { fact } => {
                self.replay_social_fact_published(fact)
                    .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialFactChallenged {
                fact_id,
                challenger,
                reason,
                challenged_at_tick,
                stake,
            } => {
                self.replay_social_fact_challenged(
                    *fact_id,
                    challenger,
                    reason,
                    *challenged_at_tick,
                    stake.clone(),
                )
                .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialFactAdjudicated {
                fact_id,
                adjudicator,
                decision,
                adjudicated_at_tick,
                ..
            } => {
                self.replay_social_fact_adjudicated(
                    *fact_id,
                    adjudicator,
                    *decision,
                    *adjudicated_at_tick,
                )
                .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialFactRevoked {
                fact_id,
                actor,
                revoked_at_tick,
                ..
            } => {
                self.replay_social_fact_revoked(*fact_id, actor, *revoked_at_tick)
                    .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialFactExpired {
                fact_id,
                expired_at_tick,
            } => {
                self.replay_social_fact_expired(*fact_id, *expired_at_tick)
                    .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialEdgeDeclared { edge } => {
                self.replay_social_edge_declared(edge)
                    .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::SocialEdgeExpired {
                edge_id,
                expired_at_tick,
                ..
            } => {
                self.replay_social_edge_expired(*edge_id, *expired_at_tick)
                    .map_err(|message| PersistError::ReplayConflict { message })?;
            }
            WorldEventKind::PowerOrderPlaced {
                order_id,
                owner,
                side,
                requested_amount,
                remaining_amount,
                limit_price_per_pu,
                fills,
                auto_cancelled_order_ids,
            } => {
                if *order_id == 0 {
                    return Err(PersistError::ReplayConflict {
                        message: "power order id must be > 0".to_string(),
                    });
                }
                if *requested_amount <= 0 {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power requested amount must be > 0, got {}",
                            requested_amount
                        ),
                    });
                }
                if *remaining_amount < 0 || *remaining_amount > *requested_amount {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "invalid power remaining amount {} for requested {}",
                            remaining_amount, requested_amount
                        ),
                    });
                }
                if *limit_price_per_pu < 0 {
                    return Err(PersistError::ReplayConflict {
                        message: format!("invalid power order limit price {}", limit_price_per_pu),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid power order owner: {reason:?}"),
                    })?;
                if matches!(owner, ResourceOwner::Location { .. }) {
                    return Err(PersistError::ReplayConflict {
                        message: LOCATION_ELECTRICITY_POOL_REMOVED_NOTE.to_string(),
                    });
                }
                if self
                    .model
                    .power_order_book
                    .open_orders
                    .iter()
                    .any(|entry| entry.order_id == *order_id)
                {
                    return Err(PersistError::ReplayConflict {
                        message: format!("power order already exists: {order_id}"),
                    });
                }
                self.model.power_order_book.next_order_id = self
                    .model
                    .power_order_book
                    .next_order_id
                    .max(order_id.saturating_add(1));

                for cancelled_order_id in auto_cancelled_order_ids {
                    let Some(cancelled_index) = self
                        .model
                        .power_order_book
                        .open_orders
                        .iter()
                        .position(|entry| entry.order_id == *cancelled_order_id)
                    else {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "power order auto-cancel target not found: {}",
                                cancelled_order_id
                            ),
                        });
                    };
                    self.model
                        .power_order_book
                        .open_orders
                        .remove(cancelled_index);
                }

                let mut incoming_filled_amount = 0_i64;
                for fill in fills {
                    if fill.amount <= 0 || fill.loss < 0 || fill.loss >= fill.amount {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "invalid power order fill values: amount {} loss {}",
                                fill.amount, fill.loss
                            ),
                        });
                    }
                    if fill.buy_order_id == fill.sell_order_id {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "power order fill buy/sell id cannot be equal: {}",
                                fill.buy_order_id
                            ),
                        });
                    }
                    if fill.buy_order_id == *order_id || fill.sell_order_id == *order_id {
                        incoming_filled_amount = incoming_filled_amount.saturating_add(fill.amount);
                    }

                    self.ensure_owner_exists(&fill.seller).map_err(|reason| {
                        PersistError::ReplayConflict {
                            message: format!("invalid power fill seller: {reason:?}"),
                        }
                    })?;
                    self.ensure_owner_exists(&fill.buyer).map_err(|reason| {
                        PersistError::ReplayConflict {
                            message: format!("invalid power fill buyer: {reason:?}"),
                        }
                    })?;
                    self.ensure_owner_chunks_generated(&fill.seller, &fill.buyer)
                        .map_err(|reason| PersistError::ReplayConflict {
                            message: format!(
                                "power order fill owner chunk generation failed: {reason:?}"
                            ),
                        })?;
                    if matches!(fill.seller, ResourceOwner::Location { .. })
                        || matches!(fill.buyer, ResourceOwner::Location { .. })
                    {
                        return Err(PersistError::ReplayConflict {
                            message: LOCATION_ELECTRICITY_POOL_REMOVED_NOTE.to_string(),
                        });
                    }

                    self.remove_from_owner_for_replay(
                        &fill.seller,
                        ResourceKind::Electricity,
                        fill.amount,
                    )?;
                    let delivered = fill.amount.saturating_sub(fill.loss);
                    if delivered <= 0 {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "power order fill delivered amount must be positive: {}",
                                delivered
                            ),
                        });
                    }
                    self.add_to_owner_for_replay(
                        &fill.buyer,
                        ResourceKind::Electricity,
                        delivered,
                    )?;

                    for resting_order_id in [fill.buy_order_id, fill.sell_order_id] {
                        if resting_order_id == *order_id {
                            continue;
                        }
                        let Some(resting_index) = self
                            .model
                            .power_order_book
                            .open_orders
                            .iter()
                            .position(|entry| entry.order_id == resting_order_id)
                        else {
                            return Err(PersistError::ReplayConflict {
                                message: format!(
                                    "power order fill resting order not found: {}",
                                    resting_order_id
                                ),
                            });
                        };
                        let resting = &mut self.model.power_order_book.open_orders[resting_index];
                        if resting.remaining_amount < fill.amount {
                            return Err(PersistError::ReplayConflict {
                                message: format!(
                                    "power order fill exceeds resting order remaining: order {} remaining {} fill {}",
                                    resting_order_id, resting.remaining_amount, fill.amount
                                ),
                            });
                        }
                        resting.remaining_amount =
                            resting.remaining_amount.saturating_sub(fill.amount);
                        if resting.remaining_amount == 0 {
                            self.model
                                .power_order_book
                                .open_orders
                                .remove(resting_index);
                        }
                    }
                }

                let expected_remaining = requested_amount.saturating_sub(incoming_filled_amount);
                if expected_remaining != *remaining_amount {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power order remaining mismatch: expected {}, got {}",
                            expected_remaining, remaining_amount
                        ),
                    });
                }
                if *remaining_amount > 0 {
                    self.model
                        .power_order_book
                        .open_orders
                        .push(PowerOrderState {
                            order_id: *order_id,
                            owner: owner.clone(),
                            side: *side,
                            remaining_amount: *remaining_amount,
                            limit_price_per_pu: *limit_price_per_pu,
                            created_at: event.time,
                        });
                }
            }
            WorldEventKind::PowerOrderCancelled {
                owner,
                order_id,
                side,
                remaining_amount,
            } => {
                if *order_id == 0 {
                    return Err(PersistError::ReplayConflict {
                        message: "power order cancel id must be > 0".to_string(),
                    });
                }
                if *remaining_amount <= 0 {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power order cancel remaining amount must be > 0, got {}",
                            remaining_amount
                        ),
                    });
                }
                self.ensure_owner_exists(owner)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!("invalid power order cancel owner: {reason:?}"),
                    })?;
                if matches!(owner, ResourceOwner::Location { .. }) {
                    return Err(PersistError::ReplayConflict {
                        message: LOCATION_ELECTRICITY_POOL_REMOVED_NOTE.to_string(),
                    });
                }
                let Some(order_index) = self
                    .model
                    .power_order_book
                    .open_orders
                    .iter()
                    .position(|entry| entry.order_id == *order_id && entry.owner == *owner)
                else {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power order cancel target not found: order_id={} owner={:?}",
                            order_id, owner
                        ),
                    });
                };
                let removed = self.model.power_order_book.open_orders.remove(order_index);
                if removed.side != *side {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power order cancel side mismatch: order {} expected {:?} got {:?}",
                            order_id, removed.side, side
                        ),
                    });
                }
                if removed.remaining_amount != *remaining_amount {
                    return Err(PersistError::ReplayConflict {
                        message: format!(
                            "power order cancel remaining mismatch: order {} expected {} got {}",
                            order_id, removed.remaining_amount, remaining_amount
                        ),
                    });
                }
            }
            WorldEventKind::ActionRejected { .. } => {}
            WorldEventKind::ModuleVisualEntityUpserted { entity } => {
                if entity.entity_id.trim().is_empty() || entity.module_id.trim().is_empty() {
                    return Err(PersistError::ReplayConflict {
                        message: "invalid module visual entity payload".to_string(),
                    });
                }
                self.ensure_module_visual_anchor_exists(&entity.anchor)
                    .map_err(|reason| PersistError::ReplayConflict {
                        message: format!(
                            "module visual entity anchor missing for {}: {reason:?}",
                            entity.entity_id
                        ),
                    })?;
                self.model
                    .module_visual_entities
                    .insert(entity.entity_id.clone(), entity.clone());
            }
            WorldEventKind::ModuleVisualEntityRemoved { entity_id } => {
                if self
                    .model
                    .module_visual_entities
                    .remove(entity_id)
                    .is_none()
                {
                    return Err(PersistError::ReplayConflict {
                        message: format!("module visual entity not found: {entity_id}"),
                    });
                }
            }
            WorldEventKind::Power(power_event) => match power_event {
                PowerEvent::PowerPlantRegistered { plant } => {
                    if self.model.power_plants.contains_key(&plant.id) {
                        return Err(PersistError::ReplayConflict {
                            message: format!("power plant already exists: {}", plant.id),
                        });
                    }
                    if !self.model.locations.contains_key(&plant.location_id) {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "location not found for power plant: {}",
                                plant.location_id
                            ),
                        });
                    }
                    self.ensure_owner_exists(&plant.owner).map_err(|reason| {
                        PersistError::ReplayConflict {
                            message: format!("invalid power plant owner: {reason:?}"),
                        }
                    })?;
                    self.model
                        .power_plants
                        .insert(plant.id.clone(), plant.clone());
                }
                PowerEvent::PowerGenerated {
                    plant_id,
                    location_id,
                    amount,
                } => {
                    if *amount < 0 {
                        return Err(PersistError::ReplayConflict {
                            message: format!("invalid power generated amount: {amount}"),
                        });
                    }
                    let owner = {
                        let plant = self.model.power_plants.get_mut(plant_id).ok_or_else(|| {
                            PersistError::ReplayConflict {
                                message: format!("power plant not found: {plant_id}"),
                            }
                        })?;
                        if &plant.location_id != location_id {
                            return Err(PersistError::ReplayConflict {
                                message: format!(
                                    "power plant location mismatch: expected {}, got {}",
                                    plant.location_id, location_id
                                ),
                            });
                        }
                        plant.current_output = *amount;
                        plant.owner.clone()
                    };
                    self.add_to_owner_for_replay(&owner, ResourceKind::Electricity, *amount)?;
                }
                PowerEvent::PowerConsumed {
                    agent_id, amount, ..
                } => {
                    if let Some(agent) = self.model.agents.get_mut(agent_id) {
                        let power_config = self.config.power.clone();
                        agent.power.consume(*amount, &power_config);
                    }
                }
                PowerEvent::PowerCharged {
                    agent_id, amount, ..
                } => {
                    if let Some(agent) = self.model.agents.get_mut(agent_id) {
                        let power_config = self.config.power.clone();
                        agent.power.charge(*amount, &power_config);
                    }
                }
                PowerEvent::PowerStateChanged { .. } => {}
                PowerEvent::PowerTransferred {
                    from,
                    to,
                    amount,
                    loss,
                    ..
                } => {
                    if *amount < 0 || *loss < 0 || *loss > *amount {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "invalid power transfer values: amount {amount}, loss {loss}"
                            ),
                        });
                    }
                    self.ensure_owner_exists(from).map_err(|reason| {
                        PersistError::ReplayConflict {
                            message: format!("invalid power transfer source: {reason:?}"),
                        }
                    })?;
                    self.ensure_owner_exists(to).map_err(|reason| {
                        PersistError::ReplayConflict {
                            message: format!("invalid power transfer target: {reason:?}"),
                        }
                    })?;
                    if matches!(from, ResourceOwner::Location { .. })
                        || matches!(to, ResourceOwner::Location { .. })
                    {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "{LOCATION_ELECTRICITY_POOL_REMOVED_NOTE}: location power transfer unsupported"
                            ),
                        });
                    }
                    self.remove_from_owner_for_replay(from, ResourceKind::Electricity, *amount)?;
                    let delivered = amount.saturating_sub(*loss);
                    if delivered <= 0 {
                        return Err(PersistError::ReplayConflict {
                            message: format!(
                                "power transfer delivered amount must be positive: {delivered}"
                            ),
                        });
                    }
                    self.add_to_owner_for_replay(to, ResourceKind::Electricity, delivered)?;
                }
            },
            WorldEventKind::LlmEffectQueued { .. } => {}
            WorldEventKind::LlmReceiptAppended { .. } => {}
            WorldEventKind::RuntimeEvent { .. } => {}
        }

        Ok(())
    }

    pub(super) fn remove_from_owner_for_replay(
        &mut self,
        owner: &ResourceOwner,
        kind: ResourceKind,
        amount: i64,
    ) -> Result<(), PersistError> {
        if matches!(owner, ResourceOwner::Location { .. })
            && matches!(kind, ResourceKind::Electricity)
        {
            return Err(PersistError::ReplayConflict {
                message: LOCATION_ELECTRICITY_POOL_REMOVED_NOTE.to_string(),
            });
        }
        let stock = match owner {
            ResourceOwner::Agent { agent_id } => self
                .model
                .agents
                .get_mut(agent_id)
                .map(|agent| &mut agent.resources)
                .ok_or_else(|| PersistError::ReplayConflict {
                    message: format!("agent not found: {agent_id}"),
                })?,
            ResourceOwner::Location { location_id } => self
                .model
                .locations
                .get_mut(location_id)
                .map(|location| &mut location.resources)
                .ok_or_else(|| PersistError::ReplayConflict {
                    message: format!("location not found: {location_id}"),
                })?,
        };

        stock.remove(kind, amount).map_err(|err| match err {
            StockError::NegativeAmount { amount } => PersistError::ReplayConflict {
                message: format!("invalid transfer amount: {amount}"),
            },
            StockError::Insufficient {
                requested,
                available,
                ..
            } => PersistError::ReplayConflict {
                message: format!(
                    "insufficient resource {:?}: requested {requested}, available {available}",
                    kind
                ),
            },
            StockError::Overflow { current, delta, .. } => PersistError::ReplayConflict {
                message: format!(
                    "resource overflow for {:?}: current={current} delta={delta}",
                    kind
                ),
            },
        })
    }

    pub(super) fn add_to_owner_for_replay(
        &mut self,
        owner: &ResourceOwner,
        kind: ResourceKind,
        amount: i64,
    ) -> Result<(), PersistError> {
        if matches!(owner, ResourceOwner::Location { .. })
            && matches!(kind, ResourceKind::Electricity)
        {
            return Err(PersistError::ReplayConflict {
                message: LOCATION_ELECTRICITY_POOL_REMOVED_NOTE.to_string(),
            });
        }
        let stock = match owner {
            ResourceOwner::Agent { agent_id } => self
                .model
                .agents
                .get_mut(agent_id)
                .map(|agent| &mut agent.resources)
                .ok_or_else(|| PersistError::ReplayConflict {
                    message: format!("agent not found: {agent_id}"),
                })?,
            ResourceOwner::Location { location_id } => self
                .model
                .locations
                .get_mut(location_id)
                .map(|location| &mut location.resources)
                .ok_or_else(|| PersistError::ReplayConflict {
                    message: format!("location not found: {location_id}"),
                })?,
        };

        stock.add(kind, amount).map_err(|err| match err {
            StockError::NegativeAmount { amount } => PersistError::ReplayConflict {
                message: format!("invalid transfer amount: {amount}"),
            },
            StockError::Insufficient { .. } => PersistError::ReplayConflict {
                message: format!("invalid transfer amount: {amount}"),
            },
            StockError::Overflow { current, delta, .. } => PersistError::ReplayConflict {
                message: format!(
                    "resource overflow for {:?}: current={current} delta={delta}",
                    kind
                ),
            },
        })
    }
}
