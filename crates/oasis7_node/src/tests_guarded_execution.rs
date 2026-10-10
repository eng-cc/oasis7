use super::*;

struct RetainedHook {
    contexts: Vec<NodeExecutionCommitContext>,
    fail: bool,
}
impl NodeExecutionHook for RetainedHook {
    fn guarded_local_execution(&self) -> bool {
        true
    }
    fn prepare_local_continuation(
        &mut self,
        _: &NodeLocalExecutionContinuation,
    ) -> Result<(), String> {
        Ok(())
    }
    fn on_commit(
        &mut self,
        _: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        panic!("guarded caller must use typed outcome")
    }
    fn on_commit_outcome_with_expected(
        &mut self,
        context: NodeExecutionCommitContext,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<NodeExecutionCommitOutcome, String> {
        self.contexts.push(context.clone());
        if self.fail {
            return Err(format!(
                "{EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE}: retained proof missing"
            ));
        }
        Ok(NodeExecutionCommitOutcome::Applied(
            NodeExecutionCommitResult {
                execution_height: context.height,
                execution_block_hash: "retained-execution".into(),
                execution_state_root: "retained-state".into(),
            },
        ))
    }
}

#[test]
fn guarded_same_height_revalidates_original_hook_and_never_uses_gap_fallback() {
    let config = NodeConfig::new("node-a", "guarded-fixture", NodeRole::Sequencer)
        .unwrap()
        .with_require_execution_on_commit(false);
    let mut engine = PosNodeEngine::new(&config).unwrap();
    let decision = PosDecision {
        height: 1,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".into(),
        status: PosConsensusStatus::Committed,
        block_hash: "original-node-block".into(),
        action_root: empty_action_root(),
        committed_actions: vec![],
        approved_stake: 1,
        rejected_stake: 0,
        required_stake: 1,
        total_stake: 1,
    };
    let mut hook = RetainedHook {
        contexts: vec![],
        fail: false,
    };
    assert!(
        engine
            .apply_committed_execution("node-a", "guarded-fixture", 123, &decision, Some(&mut hook))
            .unwrap()
    );
    assert_eq!(engine.last_execution_height, 1);
    hook.fail = true;
    assert!(
        engine
            .apply_committed_execution("node-a", "guarded-fixture", 123, &decision, Some(&mut hook))
            .is_err()
    );
    assert_eq!(hook.contexts.len(), 2);
    assert_eq!(hook.contexts[0], hook.contexts[1]);
    assert_eq!(
        engine.last_execution_block_hash.as_deref(),
        Some("retained-execution")
    );
    assert_eq!(engine.committed_height, 0);
    let mut alien = decision;
    alien.proposer_id = "network-peer".into();
    assert!(
        engine
            .apply_committed_execution("node-a", "guarded-fixture", 123, &alien, Some(&mut hook))
            .is_err()
    );
    assert_eq!(hook.contexts.len(), 2);
}
