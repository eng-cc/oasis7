struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
}

static CHECKPOINT_PROBE_NONCE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn lock_checkpoint_probe_nonce() -> std::sync::MutexGuard<'static, ()> {
    CHECKPOINT_PROBE_NONCE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) struct CheckpointProbeNonceGuard;

impl CheckpointProbeNonceGuard {
    pub(super) fn install() -> Self {
        // SAFETY: tests serialize access with CHECKPOINT_PROBE_NONCE_LOCK.
        unsafe {
            std::env::set_var(
                "OASIS7_CHECKPOINT_PROBE_NONCE",
                "probe-nonce-0123456789abcdef0123456789abcdef",
            );
        }
        Self
    }
}

impl Drop for CheckpointProbeNonceGuard {
    fn drop(&mut self) {
        // SAFETY: tests serialize access with CHECKPOINT_PROBE_NONCE_LOCK.
        unsafe {
            std::env::remove_var("OASIS7_CHECKPOINT_PROBE_NONCE");
        }
    }
}

impl NodeExecutionHook for CheckpointInstallingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "{} at height {}",
            EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE, context.height
        ))
    }

    fn install_checkpoint_bundle(
        &mut self,
        context: NodeExecutionCheckpointInstallContext,
        bundle: NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCommitResult, String> {
        if bundle.height != context.height
            || bundle.execution_block_hash != context.execution_block_hash
            || bundle.execution_state_root != context.execution_state_root
        {
            return Err("checkpoint bundle mismatch".to_string());
        }
        self.installed.push(context.height);
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: context.execution_block_hash,
            execution_state_root: context.execution_state_root,
        })
    }
}
