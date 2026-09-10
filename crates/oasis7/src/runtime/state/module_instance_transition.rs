//! Sparse, owned instance lifecycle state prepared before any canonical write.
use super::*;
use serde::ser::SerializeMap;

#[derive(Debug)]
pub(crate) struct PreparedModuleInstance {
    event: DomainEvent,
    pub(crate) agents: BTreeMap<String, AgentCell>,
    pub(crate) resources: BTreeMap<ResourceKind, i64>,
    instance: ModuleInstanceState,
    instance_key: String,
    target: ModuleInstallTarget,
    next_instance_id: u64,
    world_materials: BTreeMap<String, i64>,
}

impl PreparedModuleInstance {
    pub(crate) fn matches_event(&self, event: &DomainEvent) -> bool {
        &self.event == event
    }

    pub(crate) fn routed_agents(&self) -> BTreeMap<String, AgentCell> {
        let mut agents = self.agents.clone();
        if let Some(agent_id) = self.event.agent_id()
            && let Some(cell) = agents.get_mut(agent_id)
        {
            cell.mailbox.push_back(self.event.clone());
        }
        agents
    }

    pub(crate) fn install_infallible(self, state: &mut WorldState) {
        state.agents.extend(self.agents);
        state.resources.extend(self.resources);
        state
            .installed_module_targets
            .insert(self.instance.module_id.clone(), self.target);
        state
            .module_instances
            .insert(self.instance_key, self.instance);
        state.next_module_instance_id = self.next_instance_id;
        state.materials = self.world_materials.clone();
        state
            .material_ledgers
            .insert(MaterialLedgerId::world(), self.world_materials);
    }

    pub(crate) fn serialize_fields<S: serde::ser::SerializeStruct>(
        &self,
        state: &WorldState,
        output: &mut S,
    ) -> Result<(), S::Error> {
        output.serialize_field(
            "module_instances",
            &SingleEntryProjection {
                base: &state.module_instances,
                key: &self.instance_key,
                value: &self.instance,
            },
        )?;
        Ok(())
    }

    pub(crate) fn serialize_target_fields<S: serde::ser::SerializeStruct>(
        &self,
        state: &WorldState,
        output: &mut S,
    ) -> Result<(), S::Error> {
        output.serialize_field(
            "installed_module_targets",
            &SingleEntryProjection {
                base: &state.installed_module_targets,
                key: &self.instance.module_id,
                value: &self.target,
            },
        )?;
        output.serialize_field("next_module_instance_id", &self.next_instance_id)
    }

    pub(crate) fn serialize_material_fields<S: serde::ser::SerializeStruct>(
        &self,
        state: &WorldState,
        output: &mut S,
    ) -> Result<(), S::Error> {
        output.serialize_field("materials", &self.world_materials)?;
        output.serialize_field(
            "material_ledgers",
            &SingleEntryProjection {
                base: &state.material_ledgers,
                key: &MaterialLedgerId::world(),
                value: &self.world_materials,
            },
        )
    }
}

/// Ordered map replacement/insertion borrowing all unaffected entries.
struct SingleEntryProjection<'a, K, V> {
    base: &'a BTreeMap<K, V>,
    key: &'a K,
    value: &'a V,
}

impl<K: Ord + Serialize, V: Serialize> Serialize for SingleEntryProjection<'_, K, V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(
            self.base.len() + usize::from(!self.base.contains_key(self.key)),
        ))?;
        let mut inserted = false;
        for (key, value) in self.base {
            if !inserted && key >= self.key {
                map.serialize_entry(self.key, self.value)?;
                inserted = true;
            }
            if key != self.key {
                map.serialize_entry(key, value)?;
            }
        }
        if !inserted {
            map.serialize_entry(self.key, self.value)?;
        }
        map.end()
    }
}

impl WorldState {
    fn prepare_module_action_fee(
        &self,
        agent_id: &str,
        fee_kind: ResourceKind,
        fee_amount: i64,
        now: WorldTime,
    ) -> Result<(AgentCell, BTreeMap<ResourceKind, i64>), WorldError> {
        if fee_amount < 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("module action fee must be >= 0, got {}", fee_amount),
            });
        }
        let mut cell = self
            .agents
            .get(agent_id)
            .ok_or_else(|| WorldError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?
            .clone();
        let mut resources = BTreeMap::new();
        if fee_amount > 0 {
            cell.state
                .resources
                .remove(fee_kind, fee_amount)
                .map_err(|err| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "module action fee debit failed: agent={} kind={:?} amount={} err={:?}",
                        agent_id, fee_kind, fee_amount, err
                    ),
                })?;
            resources.insert(
                fee_kind,
                self.resources
                    .get(&fee_kind)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(fee_amount),
            );
        }
        cell.last_active = now;
        Ok((cell, resources))
    }

    pub(crate) fn prepare_module_instance_event(
        &self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<PreparedModuleInstance, WorldError> {
        let (agent_id, fee_kind, fee_amount) = match event {
            DomainEvent::ModuleInstalled {
                installer_agent_id,
                fee_kind,
                fee_amount,
                ..
            } => (installer_agent_id, *fee_kind, *fee_amount),
            DomainEvent::ModuleUpgraded {
                upgrader_agent_id,
                fee_kind,
                fee_amount,
                ..
            } => (upgrader_agent_id, *fee_kind, *fee_amount),
            DomainEvent::ModuleRollbackApplied {
                operator_agent_id,
                fee_kind,
                fee_amount,
                ..
            } => (operator_agent_id, *fee_kind, *fee_amount),
            _ => unreachable!("module instance preparation requires a lifecycle event"),
        };
        let (cell, resources) =
            self.prepare_module_action_fee(agent_id, fee_kind, fee_amount, now)?;
        let (instance, target, next_instance_id) = match event {
            DomainEvent::ModuleInstalled {
                instance_id,
                module_id,
                install_target,
                module_version,
                wasm_hash,
                active,
                ..
            } => {
                let resolved = if instance_id.trim().is_empty() {
                    module_id.clone()
                } else {
                    instance_id.trim().to_string()
                };
                (
                    ModuleInstanceState {
                        instance_id: resolved,
                        module_id: module_id.clone(),
                        module_version: module_version.clone(),
                        wasm_hash: wasm_hash.clone(),
                        owner_agent_id: agent_id.clone(),
                        install_target: install_target.clone(),
                        active: *active,
                        installed_at: now,
                    },
                    install_target.clone(),
                    self.next_module_instance_id.saturating_add(1),
                )
            }
            DomainEvent::ModuleUpgraded {
                instance_id,
                module_id,
                from_module_version,
                to_module_version,
                wasm_hash,
                install_target,
                active,
                ..
            }
            | DomainEvent::ModuleRollbackApplied {
                instance_id,
                module_id,
                from_module_version,
                to_module_version,
                wasm_hash,
                install_target,
                active,
                ..
            } => {
                let (operation, actor_label) =
                    if matches!(event, DomainEvent::ModuleRollbackApplied { .. }) {
                        ("rollback", "operator")
                    } else {
                        ("upgrade", "upgrader")
                    };
                let mut instance = self
                    .module_instances
                    .get(instance_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("module instance missing for {operation} {instance_id}"),
                    })?
                    .clone();
                if instance.owner_agent_id != *agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance owner mismatch for {operation}: instance={} owner={} {actor_label}={}",
                            instance_id, instance.owner_agent_id, agent_id
                        ),
                    });
                }
                if instance.module_id != *module_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance module_id mismatch for {operation}: instance={} state_module_id={} event_module_id={}",
                            instance_id, instance.module_id, module_id
                        ),
                    });
                }
                if instance.module_version != *from_module_version {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "module instance from_version mismatch for {operation}: instance={} state_version={} event_from={}",
                            instance_id, instance.module_version, from_module_version
                        ),
                    });
                }
                instance.module_version = to_module_version.clone();
                instance.wasm_hash = wasm_hash.clone();
                instance.install_target = install_target.clone();
                instance.active = *active;
                (
                    instance,
                    install_target.clone(),
                    self.next_module_instance_id,
                )
            }
            _ => unreachable!(),
        };
        let world_materials = self
            .material_ledgers
            .get(&MaterialLedgerId::world())
            .filter(|ledger| !ledger.is_empty())
            .unwrap_or(&self.materials)
            .clone();
        let instance_key = match event {
            DomainEvent::ModuleUpgraded { instance_id, .. }
            | DomainEvent::ModuleRollbackApplied { instance_id, .. } => instance_id.clone(),
            _ => instance.instance_id.clone(),
        };
        Ok(PreparedModuleInstance {
            event: event.clone(),
            agents: BTreeMap::from([(agent_id.clone(), cell)]),
            resources,
            instance,
            instance_key,
            target,
            next_instance_id,
            world_materials,
        })
    }
}
