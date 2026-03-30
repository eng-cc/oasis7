use super::*;

pub(super) fn parse_bool_flag(args: &mut ArgCursor, flag_name: &str) -> Result<bool, String> {
    let mut enabled = false;
    while matches!(args.peek(), Some(flag) if flag == flag_name) {
        args.next();
        enabled = true;
    }
    while let Some(flag) = args.peek() {
        if flag == "-h" || flag == "--help" {
            return Err(usage());
        }
        if flag.starts_with("--") {
            break;
        }
        return Err(format!("unexpected positional argument `{flag}`"));
    }
    Ok(enabled)
}

pub(super) fn subscribe_for_control(
    conn: &mut ViewerConnection,
    include_events: bool,
    include_metrics: bool,
) -> Result<(), String> {
    let mut streams = vec![ViewerStream::Snapshot];
    if include_events {
        streams.push(ViewerStream::Events);
    }
    if include_metrics {
        streams.push(ViewerStream::Metrics);
    }
    conn.send(&ViewerRequest::Subscribe {
        streams,
        event_kinds: Vec::new(),
    })
}

pub(super) fn maybe_request_snapshot(
    conn: &mut ViewerConnection,
    with_snapshot: bool,
    responses: &mut Vec<CollectedResponse>,
    timeout: Duration,
) -> Result<(), String> {
    if !with_snapshot {
        return Ok(());
    }
    conn.send(&ViewerRequest::RequestSnapshot)?;
    responses.extend(conn.collect_until(
        timeout,
        terminal_snapshot,
        "waiting for snapshot after command",
    )?);
    Ok(())
}

pub(super) fn build_signed_agent_chat_request(
    agent_id: &str,
    player_id: &str,
    message: &str,
    private_key_hex: &str,
    public_key_hex: Option<&str>,
    intent_tick: Option<u64>,
    intent_seq: Option<u64>,
) -> Result<AgentChatRequest, String> {
    let public_key = public_key_hex
        .map(str::to_string)
        .unwrap_or_else(|| derive_public_key_hex(private_key_hex).unwrap_or_default());
    if public_key.is_empty() {
        return Err("failed to derive public key from private key".to_string());
    }
    let nonce = next_u64_id();
    let request = AgentChatRequest {
        agent_id: agent_id.to_string(),
        message: message.to_string(),
        player_id: Some(player_id.to_string()),
        public_key: Some(public_key.clone()),
        auth: None,
        intent_tick,
        intent_seq: Some(intent_seq.unwrap_or(nonce)),
    };
    let proof = sign_agent_chat_auth_proof(&request, nonce, public_key.as_str(), private_key_hex)?;
    Ok(AgentChatRequest {
        auth: Some(proof),
        ..request
    })
}

pub(super) fn build_signed_gameplay_action_request(
    action_id: &str,
    target_agent_id: &str,
    player_id: &str,
    private_key_hex: &str,
    public_key_hex: Option<&str>,
) -> Result<GameplayActionRequest, String> {
    let public_key = resolve_public_key_hex(private_key_hex, public_key_hex)?;
    let nonce = next_u64_id();
    let request = GameplayActionRequest {
        action_id: action_id.to_string(),
        target_agent_id: target_agent_id.to_string(),
        player_id: player_id.to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
    };
    let proof =
        sign_gameplay_action_auth_proof(&request, nonce, public_key.as_str(), private_key_hex)?;
    Ok(GameplayActionRequest {
        auth: Some(proof),
        ..request
    })
}

pub(super) fn build_signed_prompt_apply_request(
    agent_id: &str,
    player_id: &str,
    private_key_hex: &str,
    public_key_hex: Option<&str>,
    expected_version: Option<u64>,
    updated_by: Option<String>,
    system_prompt_override: Option<Option<String>>,
    short_term_goal_override: Option<Option<String>>,
    long_term_goal_override: Option<Option<String>>,
    preview: bool,
) -> Result<PromptControlApplyRequest, String> {
    let public_key = resolve_public_key_hex(private_key_hex, public_key_hex)?;
    let nonce = next_u64_id();
    let request = PromptControlApplyRequest {
        agent_id: agent_id.to_string(),
        player_id: player_id.to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version,
        updated_by,
        system_prompt_override,
        short_term_goal_override,
        long_term_goal_override,
    };
    let intent = if preview {
        PromptControlAuthIntent::Preview
    } else {
        PromptControlAuthIntent::Apply
    };
    let proof = sign_prompt_control_apply_auth_proof(
        intent,
        &request,
        nonce,
        public_key.as_str(),
        private_key_hex,
    )?;
    Ok(PromptControlApplyRequest {
        auth: Some(proof),
        ..request
    })
}

pub(super) fn build_signed_prompt_rollback_request(
    agent_id: &str,
    player_id: &str,
    private_key_hex: &str,
    public_key_hex: Option<&str>,
    to_version: u64,
    expected_version: Option<u64>,
    updated_by: Option<String>,
) -> Result<PromptControlRollbackRequest, String> {
    let public_key = resolve_public_key_hex(private_key_hex, public_key_hex)?;
    let nonce = next_u64_id();
    let request = PromptControlRollbackRequest {
        agent_id: agent_id.to_string(),
        player_id: player_id.to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        to_version,
        expected_version,
        updated_by,
    };
    let proof = sign_prompt_control_rollback_auth_proof(
        &request,
        nonce,
        public_key.as_str(),
        private_key_hex,
    )?;
    Ok(PromptControlRollbackRequest {
        auth: Some(proof),
        ..request
    })
}

pub(super) fn resolve_public_key_hex(
    private_key_hex: &str,
    public_key_hex: Option<&str>,
) -> Result<String, String> {
    match public_key_hex {
        Some(value) => Ok(value.to_string()),
        None => derive_public_key_hex(private_key_hex),
    }
}

pub(super) fn derive_public_key_hex(private_key_hex: &str) -> Result<String, String> {
    let bytes = decode_private_key_hex(private_key_hex)?;
    let signing_key = SigningKey::from_bytes(&bytes);
    Ok(hex::encode(signing_key.verifying_key().to_bytes()))
}

fn decode_private_key_hex(private_key_hex: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(private_key_hex)
        .map_err(|err| format!("decode private key hex failed: {err}"))?;
    let array: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "private key must be exactly 32 bytes (64 hex chars)".to_string())?;
    Ok(array)
}

pub(super) fn keygen_output() -> Result<Value, String> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let public_key = hex::encode(signing_key.verifying_key().to_bytes());
    let private_key = hex::encode(signing_key.to_bytes());
    Ok(json!({
        "player_auth_scheme": "ed25519",
        "public_key_hex": public_key,
        "private_key_hex": private_key,
    }))
}

pub(super) fn command_output(hello_ack: &Value, responses: &[CollectedResponse]) -> Value {
    let latest_snapshot =
        latest_snapshot(responses).and_then(|snapshot| serde_json::to_value(snapshot).ok());
    let player_gameplay = latest_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("player_gameplay").cloned());
    json!({
        "hello_ack": hello_ack,
        "responses": responses.iter().map(|item| item.raw.clone()).collect::<Vec<_>>(),
        "latest_snapshot": latest_snapshot,
        "player_gameplay": player_gameplay,
    })
}

pub(super) fn latest_snapshot<'a>(
    responses: &'a [CollectedResponse],
) -> Option<&'a WorldSnapshot> {
    responses
        .iter()
        .rev()
        .find_map(|item| match &item.response {
            ViewerResponse::Snapshot { snapshot } => Some(snapshot),
            _ => None,
        })
}

pub(super) fn print_json(value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("serialize output failed: {err}"))?;
    println!("{text}");
    Ok(())
}

pub(super) fn next_u64_id() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(1));
    now.as_nanos().min(u128::from(u64::MAX)) as u64
}

pub(super) fn parse_u64_flag(value: String, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|err| format!("parse {flag} failed for `{value}`: {err}"))
}

pub(super) fn parse_usize_flag(value: String, flag: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|err| format!("parse {flag} failed for `{value}`: {err}"))
}

pub(super) fn required_flag<T>(value: Option<T>, flag: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("missing required flag `{flag}`"))
}

pub(super) fn terminal_hello(response: &ViewerResponse) -> bool {
    matches!(response, ViewerResponse::HelloAck { .. })
}

pub(super) fn terminal_snapshot(response: &ViewerResponse) -> bool {
    matches!(response, ViewerResponse::Snapshot { .. })
}

pub(super) fn terminal_control_ack(response: &ViewerResponse, request_id: u64) -> bool {
    matches!(
        response,
        ViewerResponse::ControlCompletionAck { ack } if ack.request_id == request_id
    )
}

pub(super) fn terminal_agent_chat(response: &ViewerResponse) -> bool {
    matches!(
        response,
        ViewerResponse::AgentChatAck { .. } | ViewerResponse::AgentChatError { .. }
    )
}

pub(super) fn terminal_gameplay_action(response: &ViewerResponse) -> bool {
    matches!(
        response,
        ViewerResponse::GameplayActionAck { .. } | ViewerResponse::GameplayActionError { .. }
    )
}

pub(super) fn terminal_prompt_control(response: &ViewerResponse) -> bool {
    matches!(
        response,
        ViewerResponse::PromptControlAck { .. } | ViewerResponse::PromptControlError { .. }
    )
}

pub(super) fn terminal_recovery(response: &ViewerResponse) -> bool {
    matches!(
        response,
        ViewerResponse::AuthoritativeRecoveryAck { .. }
            | ViewerResponse::AuthoritativeRecoveryError { .. }
    )
}

pub(super) fn is_terminal_error(response: &ViewerResponse) -> bool {
    matches!(response, ViewerResponse::Error { .. })
}
