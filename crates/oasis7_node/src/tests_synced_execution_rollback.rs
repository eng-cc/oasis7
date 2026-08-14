use super::*;

struct FailingSyncedRollbackExecutionHook {
    commits: Arc<Mutex<Vec<u64>>>,
    restores: Arc<Mutex<Vec<(String, u64)>>>,
}

impl NodeExecutionHook for FailingSyncedRollbackExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.commits
            .lock()
            .expect("record execution commits")
            .push(context.height);
        Err(format!(
            "forced synced execution failure at height {}",
            context.height
        ))
    }

    fn restore_to_height(&mut self, world_id: &str, height: u64) -> Result<bool, String> {
        self.restores
            .lock()
            .expect("record restore calls")
            .push((world_id.to_string(), height));
        Ok(true)
    }
}

#[test]
fn synced_replication_commit_rolls_back_execution_on_apply_error() {
    let config = NodeConfig::new(
        "node-b",
        "world-synced-exec-apply-error-rollback",
        NodeRole::Storage,
    )
    .expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.last_execution_height = 7;
    engine.last_execution_block_hash = Some("exec-block-7".to_string());
    engine.last_execution_state_root = Some("exec-state-7".to_string());
    engine.remember_execution_binding_for_height(7);
    let payload = super::replication_state_reconcile::ReplicationCommitPayload {
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        proposer_id: None,
        height: 8,
        slot: 8,
        epoch: 0,
        block_hash: "block-8".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: 8_000,
        execution_block_hash: Some("peer-exec-block-8".to_string()),
        execution_state_root: Some("peer-exec-state-8".to_string()),
        execution_checkpoint: None,
    lineage_envelope: None,
    };
    let commits = Arc::new(Mutex::new(Vec::new()));
    let restores = Arc::new(Mutex::new(Vec::new()));
    let mut hook = FailingSyncedRollbackExecutionHook {
        commits: Arc::clone(&commits),
        restores: Arc::clone(&restores),
    };

    let err = engine
        .apply_synced_replication_commit(&config.world_id, &payload, Some(&mut hook))
        .expect_err("ordinary synced execution error should fail");

    assert!(
        matches!(err, NodeError::Execution { ref reason } if reason.contains("forced synced execution failure at height 8")),
        "unexpected error: {err:?}"
    );
    assert_eq!(commits.lock().expect("commit calls").as_slice(), &[8]);
    assert_eq!(
        restores.lock().expect("restore calls").as_slice(),
        &[(config.world_id.clone(), 7)]
    );
    assert_eq!(engine.last_execution_height, 7);
    assert_eq!(
        engine.last_execution_block_hash.as_deref(),
        Some("exec-block-7")
    );
    assert_eq!(
        engine.last_execution_state_root.as_deref(),
        Some("exec-state-7")
    );
    assert!(engine.execution_binding_for_height(8).is_none());
}
