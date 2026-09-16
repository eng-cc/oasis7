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
