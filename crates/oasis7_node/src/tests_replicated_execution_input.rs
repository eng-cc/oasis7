use std::sync::{Arc as InputArc, Mutex as InputMutex};
use std::time::Duration as InputDuration;

fn replicated_input_action(target_height: u64, payload_cbor: Vec<u8>) -> NodeConsensusAction {
    NodeConsensusAction::from_payload(
        REPLICATED_EXECUTION_INPUT_ACTION_ID,
        REPLICATED_EXECUTION_INPUT_SUBMITTER,
        NodeReplicatedExecutionInputV1::new(
            PROVIDER_BACKED_BOOTSTRAP_EXECUTION_INPUT_KIND,
            target_height,
            payload_cbor,
        )
        .encode()
        .expect("encode replicated execution input"),
    )
    .expect("build replicated execution input action")
}

#[test]
fn replicated_execution_input_binding_is_deterministic_and_height_bound() {
    let producer_action = replicated_input_action(0, vec![1, 2, 3, 5, 8]);
    let mut first = producer_action.clone();
    let mut second = producer_action;

    assert_eq!(
        compute_consensus_action_root(&[first.clone()]).expect("producer action root"),
        compute_consensus_action_root(&[second.clone()]).expect("producer action root clone")
    );
    assert_eq!(
        decode_replicated_execution_input_action(&first)
            .expect("decode producer input")
            .expect("reserved input")
            .target_height,
        0
    );

    assert!(
        bind_replicated_execution_input_action(&mut first, 7).expect("bind first input"),
        "reserved action must be recognized during binding"
    );
    assert!(
        bind_replicated_execution_input_action(&mut second, 7).expect("bind second input"),
        "reserved action clone must be recognized during binding"
    );
    assert_eq!(
        first, second,
        "all proposers must commit identical bound bytes"
    );
    assert_eq!(
        compute_consensus_action_root(&[first.clone()]).expect("bound action root"),
        compute_consensus_action_root(&[second]).expect("bound action root clone")
    );
    assert_eq!(
        decode_replicated_execution_input_action(&first)
            .expect("decode bound input")
            .expect("reserved input")
            .target_height,
        7
    );

    let err = bind_replicated_execution_input_action(&mut first, 8)
        .expect_err("a bound input cannot move to another height");
    assert!(
        err.contains("target height mismatch"),
        "unexpected error: {err}"
    );
}

#[test]
fn player_actions_cannot_claim_replicated_execution_input_identity() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-replicated-input",
            "world-replicated-input",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let err = runtime
        .submit_consensus_action_payload(REPLICATED_EXECUTION_INPUT_ACTION_ID, vec![1, 2, 3])
        .expect_err("reserved action id must be rejected for player input");
    assert!(
        err.to_string()
            .contains("reserved replicated execution input identity"),
        "unexpected error: {err}"
    );
}

#[derive(Clone)]
struct RecordingInputHook {
    contexts: InputArc<InputMutex<Vec<NodeExecutionCommitContext>>>,
}

impl NodeExecutionHook for RecordingInputHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.contexts
        .lock()
            .expect("record input context")
            .push(context.clone());
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("input-exec-block-{}", context.height),
            execution_state_root: format!("input-exec-state-{}", context.height),
        })
    }
}

#[test]
fn replicated_execution_input_binds_to_restored_next_height_before_proposal() {
    let config = NodeConfig::new(
        "node-replicated-input-runtime",
        "world-replicated-input-runtime",
        NodeRole::Sequencer,
    )
    .expect("config")
        .with_tick_interval(InputDuration::from_millis(10))
    .expect("tick interval");
    let contexts = InputArc::new(InputMutex::new(Vec::new()));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(RecordingInputHook {
        contexts: InputArc::clone(&contexts),
    });
    runtime
        .submit_replicated_execution_input(NodeReplicatedExecutionInputV1::new(
            PROVIDER_BACKED_BOOTSTRAP_EXECUTION_INPUT_KIND,
            0,
            vec![9, 7, 5, 3, 1],
        ))
        .expect("queue producer input");
    runtime.start().expect("start runtime");
        std::thread::sleep(InputDuration::from_millis(120));
    runtime.stop().expect("stop runtime");

    let contexts = contexts.lock().expect("read input contexts");
    let context = contexts
        .iter()
        .find(|context| !context.committed_actions.is_empty())
        .expect("producer input must reach a proposal");
    assert_eq!(context.committed_actions.len(), 1);
    let input = decode_replicated_execution_input_action(&context.committed_actions[0])
        .expect("decode committed input")
        .expect("reserved committed input");
    assert_eq!(input.target_height, context.height);
    assert_eq!(
        context.height, 1,
        "fresh engine binds to its first proposal height"
    );
    assert_eq!(
        compute_consensus_action_root(context.committed_actions.as_slice()).expect("action root"),
        context.action_root
    );
}
