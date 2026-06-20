use super::super::ChunkState;
use super::super::init::{
    AsteroidFragmentInitConfig, WorldInitConfig, generate_chunk_fragments,
    summarize_chunk_generation,
};
use super::super::persist::PersistError;
use super::super::types::{ResourceKind, ResourceOwner, StockError};
use super::super::world_model::{Factory, Location};
use super::WorldKernel;
use super::types::{WorldEvent, WorldEventKind};

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
                self.replay_power_order_placed(
                    event.time,
                    *order_id,
                    owner,
                    *side,
                    *requested_amount,
                    *remaining_amount,
                    *limit_price_per_pu,
                    fills,
                    auto_cancelled_order_ids,
                )?;
            }
            WorldEventKind::PowerOrderCancelled {
                owner,
                order_id,
                side,
                remaining_amount,
            } => {
                self.replay_power_order_cancelled(owner, *order_id, *side, *remaining_amount)?;
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
            WorldEventKind::Power(power_event) => self.replay_power_event(power_event)?,
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
