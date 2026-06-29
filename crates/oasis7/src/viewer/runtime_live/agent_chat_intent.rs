use crate::viewer::protocol::AgentChatRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ResolvedAgentChatIntent {
    pub(super) intent_tick: Option<u64>,
    pub(super) intent_seq: u64,
}

pub(super) fn resolve_agent_chat_intent(
    request: &AgentChatRequest,
    verified_nonce: u64,
) -> Result<ResolvedAgentChatIntent, String> {
    let intent_seq = match request.intent_seq {
        Some(0) => {
            return Err("intent_seq must be greater than zero".to_string());
        }
        Some(seq) if seq != verified_nonce => {
            return Err(format!(
                "intent_seq {} must match auth nonce {}",
                seq, verified_nonce
            ));
        }
        Some(seq) => seq,
        None => verified_nonce,
    };
    Ok(ResolvedAgentChatIntent {
        intent_tick: request.intent_tick,
        intent_seq,
    })
}
