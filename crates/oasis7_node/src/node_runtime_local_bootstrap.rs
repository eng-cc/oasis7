use super::*;

impl NodeRuntime {
    pub(super) fn initialize_local_execution_bootstrap(
        &self,
        engine: &mut PosNodeEngine,
    ) -> Result<(), NodeError> {
        if let Some(bootstrap) = self.local_execution_bootstrap.as_ref()
            && let Err(err) = engine.apply_local_execution_bootstrap(bootstrap)
        {
            self.running.store(false, Ordering::SeqCst);
            return Err(err);
        }
        if let Some(hook) = self.execution_hook.as_ref() {
            let mut hook = hook.lock().map_err(|_| NodeError::Execution {
                reason: "guarded bootstrap hook lock unavailable".into(),
            })?;
            if hook.guarded_local_execution() {
                if self.config.replication.is_some() || self.replication_network.is_some() {
                    return Err(NodeError::Execution {
                        reason: "guarded local bootstrap rejects ordinary replication recovery"
                            .into(),
                    });
                }
                if let Some(saved) = hook
                    .pending_local_continuation()
                    .map_err(|reason| NodeError::Execution { reason })?
                {
                    if self
                        .local_execution_bootstrap
                        .as_ref()
                        .is_none_or(|b| b.height != saved.predecessor_committed_height)
                        || saved.next_height != saved.context.height
                    {
                        return Err(NodeError::Execution {
                            reason: "guarded original counter boundary changed".into(),
                        });
                    }
                    engine.next_slot = saved.next_slot;
                    engine.restore_guarded_local_decision(
                        &saved,
                        &self.config.node_id,
                        &self.config.world_id,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_local_execution_bootstrap_after_restore(
        &self,
        engine: &PosNodeEngine,
    ) -> Result<(), NodeError> {
        if let Some(bootstrap) = self.local_execution_bootstrap.as_ref()
            && let Err(err) = engine.validate_local_execution_bootstrap(bootstrap)
        {
            self.running.store(false, Ordering::SeqCst);
            return Err(err);
        }
        Ok(())
    }
}
