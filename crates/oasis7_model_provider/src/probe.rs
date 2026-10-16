//! Two fixed, non-world capability requests under one idempotent probe operation.
use crate::{
    connection::{
        ConnectionError, ConnectionManager, Principal, ProbeStart, ProbeState, RequestPermit,
    },
    protocol::{Conversation, Message, ProviderFailure, ProviderResponse, probe_conversation},
    transport::ModelTransport,
};
fn core_error(error: ConnectionError) -> ProviderFailure {
    match error {
        ConnectionError::BudgetExceeded => ProviderFailure::BudgetExhausted,
        ConnectionError::Invalid => ProviderFailure::CapabilityUnsupported,
        _ => ProviderFailure::SessionExpired,
    }
}
/// An existing Pending operation never issues another upstream request. Each
/// internal phase has exactly one atomically reserved physical send slot.
pub fn run_probe(
    manager: &ConnectionManager,
    principal: &Principal,
    id: &str,
    version: u64,
    operation_id: &str,
) -> Result<ProbeState, ProviderFailure> {
    run_with(
        manager,
        principal,
        id,
        version,
        operation_id,
        |permit, conversation| ModelTransport.send(permit, conversation),
    )
}
fn run_with(
    manager: &ConnectionManager,
    principal: &Principal,
    id: &str,
    version: u64,
    operation_id: &str,
    mut send: impl FnMut(&RequestPermit, &Conversation) -> Result<ProviderResponse, ProviderFailure>,
) -> Result<ProbeState, ProviderFailure> {
    let operation = match manager
        .begin_probe(principal, id, version, operation_id)
        .map_err(core_error)?
    {
        ProbeStart::Pending => return Ok(ProbeState::Pending),
        ProbeStart::Ready => return Ok(ProbeState::Ready),
        ProbeStart::Failed => return Ok(ProbeState::Failed),
        ProbeStart::Operation(op) => op,
    };
    let mut conversation = probe_conversation();
    let reserve = manager
        .limits()
        .max_input_tokens
        .checked_add(manager.limits().max_output_tokens)
        .ok_or(ProviderFailure::BudgetExhausted)?;
    let first = operation.reserve_phase(0, reserve).map_err(core_error)?;
    let reply = match send(&first, &conversation) {
        Ok(reply) => reply,
        Err(error) => {
            let _ = first.finish(None, false);
            return Err(error);
        }
    };
    let verified = reply.tool_calls.len() == 1
        && reply.tool_calls[0].name == "protocol_echo"
        && reply.tool_calls[0].arguments == serde_json::json!({"value":"ready"});
    first
        .finish(reply.usage.total_tokens, verified)
        .map_err(core_error)?;
    if !verified {
        return Err(ProviderFailure::CapabilityUnsupported);
    }
    let call_id = reply.tool_calls[0].id.clone();
    conversation.messages.extend(reply.replay);
    conversation.messages.push(Message::ToolResult {
        call_id,
        content: "{\"value\":\"ready\"}".into(),
    });
    conversation.messages.push(Message::User {
        content: "The tool completed. Reply exactly ready, without calling any tool.".into(),
    });
    let second = operation.reserve_phase(1, reserve).map_err(core_error)?;
    let reply = match send(&second, &conversation) {
        Ok(reply) => reply,
        Err(error) => {
            let _ = second.finish(None, false);
            return Err(error);
        }
    };
    let verified = reply.tool_calls.is_empty() && reply.text.trim() == "ready";
    second
        .finish(reply.usage.total_tokens, verified)
        .map_err(core_error)?;
    if !verified {
        return Err(ProviderFailure::CapabilityUnsupported);
    }
    Ok(ProbeState::Ready)
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;
