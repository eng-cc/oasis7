use super::*;

impl PosNodeEngine {
    pub(super) fn apply_local_execution_genesis_bootstrap(
        &mut self,
        configured_world_id: &str,
        bootstrap: &crate::NodeExecutionGenesisBootstrap,
    ) -> Result<(), NodeError> {
        let boundary = bootstrap.boundary();
        let fresh = self.committed_height == 0
            && self.network_committed_height == 0
            && self.replication_persisted_height == 0
            && self.next_height == 1
            && self.next_slot == 0
            && self.last_committed_block_hash.is_none()
            && self.last_execution_height == 0
            && self.last_execution_block_hash.is_none()
            && self.last_execution_state_root.is_none()
            && self.execution_bindings.is_empty()
            && self.pending.is_none()
            && self.pending_consensus_actions.is_empty()
            && self.pending_consensus_action_reservation_bytes == 0
            && !self.pending_consensus_action_reservation_owned
            && self.last_committed_at_ms.is_none()
            && self.last_observed_tick == 0
            && self.last_observed_slot == 0
            && self.missed_tick_count == 0
            && self.missed_slot_count == 0;
        if !fresh {
            return Err(NodeError::InvalidConfig {
                reason: "qualified local genesis bootstrap requires a fresh Node engine".into(),
            });
        }
        if configured_world_id.trim().is_empty()
            || bootstrap.world_id() != configured_world_id
            || bootstrap.initial_record_id().trim().is_empty()
            || boundary.height != 0
            || boundary.consensus_block_hash.trim().is_empty()
            || boundary.execution_block_hash.trim().is_empty()
            || boundary.execution_state_root.trim().is_empty()
        {
            return Err(NodeError::InvalidConfig {
                reason: "qualified local genesis bootstrap does not match Node world".into(),
            });
        }

        // Height zero stays the genesis height. The qualified InitialActivation
        // supplies the parent hashes for the first actual consensus execution.
        self.last_committed_block_hash = Some(boundary.consensus_block_hash.clone());
        self.last_execution_block_hash = Some(boundary.execution_block_hash.clone());
        self.last_execution_state_root = Some(boundary.execution_state_root.clone());
        self.remember_execution_binding_for_height(0);
        Ok(())
    }

    pub(super) fn apply_local_execution_bootstrap(
        &mut self,
        bootstrap: &crate::NodeExecutionBootstrap,
    ) -> Result<(), NodeError> {
        if bootstrap.height == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "local execution bootstrap height must be > 0".to_string(),
            });
        }
        if bootstrap.consensus_block_hash.trim().is_empty()
            || bootstrap.execution_block_hash.trim().is_empty()
            || bootstrap.execution_state_root.trim().is_empty()
        {
            return Err(NodeError::InvalidConfig {
                reason: "local execution bootstrap hashes must be non-empty".to_string(),
            });
        }
        let next_height =
            bootstrap
                .height
                .checked_add(1)
                .ok_or_else(|| NodeError::InvalidConfig {
                    reason: format!(
                        "local execution bootstrap height {} has no successor",
                        bootstrap.height
                    ),
                })?;
        self.committed_height = bootstrap.height;
        self.network_committed_height = bootstrap.height;
        self.replication_persisted_height = bootstrap.height;
        self.next_height = next_height;
        self.last_broadcast_proposal_height = bootstrap.height;
        self.last_broadcast_local_attestation_height = bootstrap.height;
        self.last_broadcast_gossip_committed_height = bootstrap.height;
        self.last_broadcast_network_committed_height = bootstrap.height;
        self.last_broadcast_gossip_replicated_committed_height = bootstrap.height;
        self.last_broadcast_network_replicated_committed_height = bootstrap.height;
        self.last_committed_block_hash = Some(bootstrap.consensus_block_hash.clone());
        self.last_execution_height = bootstrap.height;
        self.last_execution_block_hash = Some(bootstrap.execution_block_hash.clone());
        self.last_execution_state_root = Some(bootstrap.execution_state_root.clone());
        self.execution_bindings.clear();
        self.remember_execution_binding_for_height(bootstrap.height);
        Ok(())
    }

    pub(super) fn validate_local_execution_bootstrap(
        &self,
        bootstrap: &crate::NodeExecutionBootstrap,
    ) -> Result<(), NodeError> {
        let matches = self.committed_height == bootstrap.height
            && self.network_committed_height >= bootstrap.height
            && self.next_height == bootstrap.height.saturating_add(1)
            && self.last_committed_block_hash.as_deref()
                == Some(bootstrap.consensus_block_hash.as_str())
            && self.last_execution_height >= bootstrap.height
            && self.last_execution_block_hash.as_deref()
                == Some(bootstrap.execution_block_hash.as_str())
            && self.last_execution_state_root.as_deref()
                == Some(bootstrap.execution_state_root.as_str());
        if matches {
            Ok(())
        } else {
            Err(NodeError::Replication {
                reason: format!(
                    "persisted node state does not match local execution bootstrap boundary at height {}",
                    bootstrap.height
                ),
            })
        }
    }
}

#[cfg(test)]
mod genesis_bootstrap_tests {
    use super::*;

    struct GuardedHookWithoutGenesisCapability;

    impl crate::NodeExecutionHook for GuardedHookWithoutGenesisCapability {
        fn on_commit(
            &mut self,
            _: crate::NodeExecutionCommitContext,
        ) -> Result<crate::NodeExecutionCommitResult, String> {
            Err("test hook does not execute commits".into())
        }

        fn guarded_local_execution(&self) -> bool {
            true
        }
    }

    fn genesis_bootstrap() -> crate::NodeExecutionGenesisBootstrap {
        crate::NodeExecutionGenesisBootstrap::from_qualified_initial_activation(
            "world-genesis-test",
            "qualified-initial-record",
            crate::NodeExecutionBootstrap {
                height: 0,
                consensus_block_hash: "consensus-genesis".into(),
                execution_block_hash: "execution-genesis".into(),
                execution_state_root: "state-root-genesis".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn qualified_genesis_bootstrap_seeds_only_a_fresh_zero_height_engine() {
        let config = crate::NodeConfig::new(
            "node-genesis-test",
            "world-genesis-test",
            crate::NodeRole::Sequencer,
        )
        .unwrap();
        let mut engine = PosNodeEngine::new(&config).unwrap();

        engine
            .apply_local_execution_genesis_bootstrap("world-genesis-test", &genesis_bootstrap())
            .unwrap();

        assert_eq!(engine.committed_height, 0);
        assert_eq!(engine.next_height, 1);
        assert_eq!(
            engine.last_committed_block_hash.as_deref(),
            Some("consensus-genesis")
        );
        assert_eq!(engine.last_execution_height, 0);
        assert_eq!(
            engine.last_execution_block_hash.as_deref(),
            Some("execution-genesis")
        );
        assert_eq!(
            engine.last_execution_state_root.as_deref(),
            Some("state-root-genesis")
        );
        assert!(engine.execution_bindings.contains_key(&0));

        assert!(
            engine
                .apply_local_execution_genesis_bootstrap("world-genesis-test", &genesis_bootstrap())
                .is_err()
        );
    }

    #[test]
    fn zero_height_cannot_use_normal_bootstrap_or_cross_world_genesis_token() {
        let config = crate::NodeConfig::new(
            "node-genesis-test",
            "world-genesis-test",
            crate::NodeRole::Sequencer,
        )
        .unwrap();
        let mut engine = PosNodeEngine::new(&config).unwrap();
        assert!(
            engine
                .apply_local_execution_bootstrap(&crate::NodeExecutionBootstrap {
                    height: 0,
                    consensus_block_hash: "consensus-genesis".into(),
                    execution_block_hash: "execution-genesis".into(),
                    execution_state_root: "state-root-genesis".into(),
                })
                .is_err()
        );
        assert!(
            engine
                .apply_local_execution_genesis_bootstrap("different-world", &genesis_bootstrap())
                .is_err()
        );
    }

    #[test]
    fn genesis_token_rejects_nonzero_or_incomplete_boundaries() {
        assert!(
            crate::NodeExecutionGenesisBootstrap::from_qualified_initial_activation(
                "world",
                "record",
                crate::NodeExecutionBootstrap {
                    height: 1,
                    consensus_block_hash: "consensus".into(),
                    execution_block_hash: "execution".into(),
                    execution_state_root: "root".into(),
                },
            )
            .is_err()
        );
        assert!(
            crate::NodeExecutionGenesisBootstrap::from_qualified_initial_activation(
                "world",
                "record",
                crate::NodeExecutionBootstrap {
                    height: 0,
                    consensus_block_hash: "consensus".into(),
                    execution_block_hash: "execution".into(),
                    execution_state_root: String::new(),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn guarded_hook_without_current_qualified_initial_token_cannot_start_genesis_node() {
        let config = crate::NodeConfig::new(
            "node-genesis-test",
            "world-genesis-test",
            crate::NodeRole::Sequencer,
        )
        .unwrap();
        let mut runtime = crate::NodeRuntime::new(config)
            .with_execution_hook(GuardedHookWithoutGenesisCapability)
            .with_local_execution_genesis_bootstrap(genesis_bootstrap());

        assert!(runtime.start().is_err());
        assert!(!runtime.running.load(std::sync::atomic::Ordering::SeqCst));
    }
}
