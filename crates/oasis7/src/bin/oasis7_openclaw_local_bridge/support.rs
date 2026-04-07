use super::*;

pub(super) fn should_fallback_to_local_agent(error: &str) -> bool {
    error.to_ascii_lowercase().contains("gateway timeout")
}

pub(super) fn local_session_id_from_session_key(session_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_key.as_bytes());
    format!("ws-{}", hex::encode(hasher.finalize()))
}

pub(super) fn agent_output_from_json(
    prompt: String,
    payload: &str,
    route_note: Option<String>,
) -> Result<AgentInvocationOutput, String> {
    let parsed = parse_openclaw_agent_command_output(payload)?;
    let text = parsed
        .payloads
        .into_iter()
        .find_map(|entry| entry.text)
        .ok_or_else(|| "openclaw agent json did not contain payloads[].text".to_string())?;
    Ok(AgentInvocationOutput {
        prompt,
        text,
        provider_version: parsed
            .meta
            .as_ref()
            .and_then(|meta| meta.agent_meta.as_ref())
            .map(
                |agent_meta| match (&agent_meta.provider, &agent_meta.model) {
                    (Some(provider), Some(model)) => format!("{provider}/{model}"),
                    (Some(provider), None) => provider.clone(),
                    (None, Some(model)) => model.clone(),
                    (None, None) => DEFAULT_PROVIDER_ID.to_string(),
                },
            ),
        duration_ms: parsed.meta.as_ref().and_then(|meta| meta.duration_ms),
        prompt_tokens: parsed
            .meta
            .as_ref()
            .and_then(|meta| meta.agent_meta.as_ref())
            .and_then(|agent_meta| agent_meta.prompt_tokens),
        completion_tokens: parsed
            .meta
            .as_ref()
            .and_then(|meta| meta.agent_meta.as_ref())
            .and_then(|agent_meta| agent_meta.usage.as_ref())
            .and_then(|usage| usage.output),
        total_tokens: parsed
            .meta
            .as_ref()
            .and_then(|meta| meta.agent_meta.as_ref())
            .and_then(|agent_meta| agent_meta.usage.as_ref())
            .and_then(|usage| usage.total),
        route_note,
    })
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentCliOutput {
    result: OpenClawAgentCliResult,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenClawAgentCommandOutput {
    Gateway(OpenClawAgentCliOutput),
    Local(OpenClawAgentCliResult),
}

fn parse_openclaw_agent_command_output(payload: &str) -> Result<OpenClawAgentCliResult, String> {
    match serde_json::from_str::<OpenClawAgentCommandOutput>(payload) {
        Ok(OpenClawAgentCommandOutput::Gateway(output)) => Ok(output.result),
        Ok(OpenClawAgentCommandOutput::Local(result)) => Ok(result),
        Err(err) => Err(format!("parse openclaw agent json failed: {err}")),
    }
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentCliResult {
    payloads: Vec<OpenClawAgentPayload>,
    #[serde(default)]
    meta: Option<OpenClawAgentMeta>,
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentPayload {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentMeta {
    #[serde(rename = "durationMs", default)]
    duration_ms: Option<u64>,
    #[serde(rename = "agentMeta", default)]
    agent_meta: Option<OpenClawAgentMetaDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentMetaDetails {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(rename = "promptTokens", default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    usage: Option<OpenClawAgentUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenClawAgentUsage {
    #[serde(default)]
    output: Option<u64>,
    #[serde(default)]
    total: Option<u64>,
}

pub(super) fn estimated_current_location_id(
    observation: &oasis7::simulator::ProviderObservation,
) -> Option<&str> {
    if observation.self_state.location_ref.trim().is_empty() {
        None
    } else {
        Some(observation.self_state.location_ref.as_str())
    }
}

pub(super) fn nearest_reachable_non_current_location_id(
    observation: &oasis7::simulator::ProviderObservation,
) -> Option<String> {
    let current_location_id = estimated_current_location_id(observation);
    if !observation.local_navigation_graph.is_empty() {
        return observation
            .local_navigation_graph
            .iter()
            .find(|node| {
                node.traversable
                    && node.relation == "reachable_location"
                    && Some(node.node_ref.as_str()) != current_location_id
            })
            .map(|node| node.node_ref.clone());
    }
    observation
        .nearby_entities
        .iter()
        .filter(|entity| {
            entity.kind == "location"
                && entity.relation == "reachable_location"
                && Some(entity.entity_ref.as_str()) != current_location_id
        })
        .map(|entity| entity.entity_ref.clone())
        .next()
}
