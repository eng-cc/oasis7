use oasis7::consensus_action_payload::{
    ConsensusActionPayloadBody, decode_consensus_action_payload,
};
use oasis7::runtime::Action as RuntimeAction;
use oasis7::runtime::ProviderBackedBootstrapAuthorityV1;
use oasis7::simulator::{Action as SimulatorAction, ActionSubmitter};
use oasis7_node::{
    NodeExecutionCommitContext, decode_replicated_execution_input_action,
    validate_replicated_execution_input_actions,
};

pub(super) fn decode_committed_actions(
    context: &NodeExecutionCommitContext,
) -> Result<
    (
        Vec<RuntimeAction>,
        Vec<(SimulatorAction, ActionSubmitter)>,
        Option<Vec<ProviderBackedBootstrapAuthorityV1>>,
    ),
    String,
> {
    validate_replicated_execution_input_actions(
        context.committed_actions.as_slice(),
        context.height,
    )
    .map_err(|err| {
        format!(
            "execution driver replicated execution input validation failed at height {}: {}",
            context.height, err
        )
    })?;

    let mut decoded_runtime_actions = Vec::with_capacity(context.committed_actions.len());
    let mut decoded_simulator_actions = Vec::with_capacity(context.committed_actions.len());
    let mut replicated_provider_backed_bootstrap = None;
    for action in &context.committed_actions {
        if let Some(input) = decode_replicated_execution_input_action(action).map_err(|err| {
            format!(
                "execution driver decode replicated execution input failed action_id={} err={err}",
                action.action_id
            )
        })? {
            if input.target_height != context.height {
                return Err(format!(
                    "execution driver replicated execution input height mismatch: action_height={} context_height={}",
                    input.target_height, context.height
                ));
            }
            if replicated_provider_backed_bootstrap.is_some() {
                return Err(format!(
                    "execution driver received duplicate replicated ProviderBacked bootstrap inputs at height {}",
                    context.height
                ));
            }
            replicated_provider_backed_bootstrap = Some(
                super::provider_bootstrap::decode_provider_backed_bootstrap_execution_input(
                    &input,
                )?,
            );
            continue;
        }
        match decode_consensus_action_payload(action.payload_cbor.as_slice()) {
            Ok(ConsensusActionPayloadBody::RuntimeAction { action: decoded }) => {
                decoded_runtime_actions.push(decoded);
            }
            Ok(ConsensusActionPayloadBody::SimulatorAction { action, submitter }) => {
                decoded_simulator_actions.push((action, submitter));
            }
            Err(err) => {
                return Err(format!(
                    "execution driver decode committed action failed action_id={} err={}",
                    action.action_id, err
                ));
            }
        }
    }

    Ok((
        decoded_runtime_actions,
        decoded_simulator_actions,
        replicated_provider_backed_bootstrap,
    ))
}
