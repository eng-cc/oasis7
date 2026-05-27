use super::*;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const GUI_AGENT_API_VERSION: &str = "2026-03-08";

const ACTION_START_GAME: &str = "start_game";
const ACTION_STOP_GAME: &str = "stop_game";
const ACTION_START_CHAIN: &str = "start_chain";
const ACTION_STOP_CHAIN: &str = "stop_chain";
const ACTION_RECOVER_CHAIN: &str = "recover_chain";
const ACTION_SET_CHAIN_NETWORK_TIER: &str = "set_chain_network_tier";
const ACTION_SUBMIT_TRANSFER: &str = "submit_transfer";
const ACTION_SUBMIT_FEEDBACK: &str = "submit_feedback";

const GUI_AGENT_ACTIONS: &[&str] = &[
    ACTION_START_GAME,
    ACTION_STOP_GAME,
    ACTION_START_CHAIN,
    ACTION_RECOVER_CHAIN,
    ACTION_STOP_CHAIN,
    ACTION_SET_CHAIN_NETWORK_TIER,
    ACTION_SUBMIT_TRANSFER,
    ACTION_SUBMIT_FEEDBACK,
    "query_transfer_accounts",
    "query_transfer_status",
    "query_transfer_history",
    "query_explorer_overview",
    "query_explorer_transactions",
    "query_explorer_transaction",
    "query_explorer_blocks",
    "query_explorer_block",
    "query_explorer_txs",
    "query_explorer_tx",
    "query_explorer_search",
    "query_explorer_address",
    "query_explorer_contracts",
    "query_explorer_contract",
    "query_explorer_assets",
    "query_explorer_mempool",
];

#[derive(Debug, Clone, Copy)]
struct QueryActionSpec {
    action: &'static str,
    query_target: &'static str,
    runtime_target: &'static str,
}

const QUERY_ACTION_SPECS: &[QueryActionSpec] = &[
    QueryActionSpec {
        action: "query_transfer_accounts",
        query_target: "transfer.accounts",
        runtime_target: "/v1/chain/transfer/accounts",
    },
    QueryActionSpec {
        action: "query_transfer_status",
        query_target: "transfer.status",
        runtime_target: "/v1/chain/transfer/status",
    },
    QueryActionSpec {
        action: "query_transfer_history",
        query_target: "transfer.history",
        runtime_target: "/v1/chain/transfer/history",
    },
    QueryActionSpec {
        action: "query_explorer_overview",
        query_target: "explorer.overview",
        runtime_target: "/v1/chain/explorer/overview",
    },
    QueryActionSpec {
        action: "query_explorer_transactions",
        query_target: "explorer.transactions",
        runtime_target: "/v1/chain/explorer/transactions",
    },
    QueryActionSpec {
        action: "query_explorer_transaction",
        query_target: "explorer.transaction",
        runtime_target: "/v1/chain/explorer/transaction",
    },
    QueryActionSpec {
        action: "query_explorer_blocks",
        query_target: "explorer.blocks",
        runtime_target: "/v1/chain/explorer/blocks",
    },
    QueryActionSpec {
        action: "query_explorer_block",
        query_target: "explorer.block",
        runtime_target: "/v1/chain/explorer/block",
    },
    QueryActionSpec {
        action: "query_explorer_txs",
        query_target: "explorer.txs",
        runtime_target: "/v1/chain/explorer/txs",
    },
    QueryActionSpec {
        action: "query_explorer_tx",
        query_target: "explorer.tx",
        runtime_target: "/v1/chain/explorer/tx",
    },
    QueryActionSpec {
        action: "query_explorer_search",
        query_target: "explorer.search",
        runtime_target: "/v1/chain/explorer/search",
    },
    QueryActionSpec {
        action: "query_explorer_address",
        query_target: "explorer.address",
        runtime_target: "/v1/chain/explorer/address",
    },
    QueryActionSpec {
        action: "query_explorer_contracts",
        query_target: "explorer.contracts",
        runtime_target: "/v1/chain/explorer/contracts",
    },
    QueryActionSpec {
        action: "query_explorer_contract",
        query_target: "explorer.contract",
        runtime_target: "/v1/chain/explorer/contract",
    },
    QueryActionSpec {
        action: "query_explorer_assets",
        query_target: "explorer.assets",
        runtime_target: "/v1/chain/explorer/assets",
    },
    QueryActionSpec {
        action: "query_explorer_mempool",
        query_target: "explorer.mempool",
        runtime_target: "/v1/chain/explorer/mempool",
    },
];

#[derive(Debug, Serialize)]
pub(super) struct GuiAgentCapabilitiesResponse {
    api_version: &'static str,
    actions: Vec<&'static str>,
    query_targets: Vec<GuiAgentQueryTargetCapability>,
}

#[derive(Debug, Serialize)]
struct GuiAgentQueryTargetCapability {
    id: &'static str,
    action: &'static str,
    runtime_target: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiAgentActionRequest {
    action: String,
    #[serde(default)]
    payload: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct GuiAgentActionResponse {
    ok: bool,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    state: StateSnapshot,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiAgentConfigPayload {
    config: LauncherConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiAgentQueryPayload {
    #[serde(default)]
    query_target: Option<String>,
    #[serde(default)]
    query: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiAgentNetworkTierPayload {
    tier: String,
    #[serde(default)]
    manifest: Option<String>,
}

pub(super) fn gui_agent_capabilities_response() -> GuiAgentCapabilitiesResponse {
    let query_targets = QUERY_ACTION_SPECS
        .iter()
        .map(|spec| GuiAgentQueryTargetCapability {
            id: spec.query_target,
            action: spec.action,
            runtime_target: spec.runtime_target,
        })
        .collect();
    GuiAgentCapabilitiesResponse {
        api_version: GUI_AGENT_API_VERSION,
        actions: GUI_AGENT_ACTIONS.to_vec(),
        query_targets,
    }
}

pub(super) fn execute_gui_agent_action(
    state: &mut ServiceState,
    body: &[u8],
    request_host: Option<&str>,
) -> GuiAgentActionResponse {
    let request: GuiAgentActionRequest = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(err) => {
            return action_error(
                state,
                "unknown",
                request_host,
                "invalid_request",
                format!("parse gui-agent action request JSON failed: {err}"),
                None,
            );
        }
    };

    let action = request.action.trim();
    if action.is_empty() {
        return action_error(
            state,
            "unknown",
            request_host,
            "invalid_request",
            "action must not be empty",
            None,
        );
    }

    match action {
        ACTION_START_GAME => {
            let config = match parse_action_config(request.payload.as_ref(), state, action) {
                Ok(config) => config,
                Err(err) => {
                    return action_error(state, action, request_host, "invalid_request", err, None)
                }
            };
            let outcome = start_process(state, config);
            poll_service_state(state);
            match outcome {
                Ok(()) => action_ok(state, action, request_host, None),
                Err(err) => action_error(
                    state,
                    action,
                    request_host,
                    classify_runtime_error_code(err.as_str()),
                    err,
                    None,
                ),
            }
        }
        ACTION_STOP_GAME => {
            if let Err(err) = ensure_empty_payload(request.payload.as_ref(), action) {
                return action_error(state, action, request_host, "invalid_request", err, None);
            }
            let outcome = stop_process(state);
            poll_service_state(state);
            match outcome {
                Ok(()) => action_ok(state, action, request_host, None),
                Err(err) => action_error(
                    state,
                    action,
                    request_host,
                    classify_runtime_error_code(err.as_str()),
                    err,
                    None,
                ),
            }
        }
        ACTION_START_CHAIN => {
            let config = match parse_action_config(request.payload.as_ref(), state, action) {
                Ok(config) => config,
                Err(err) => {
                    return action_error(state, action, request_host, "invalid_request", err, None)
                }
            };
            let outcome = start_chain_process(state, config);
            poll_service_state(state);
            let outcome = finalize_chain_start_outcome(state, outcome);
            match outcome {
                Ok(()) => action_ok(state, action, request_host, None),
                Err(err) => action_error(
                    state,
                    action,
                    request_host,
                    chain_error_code_for_state(state, err.as_str()),
                    err,
                    chain_error_data_for_state(state),
                ),
            }
        }
        ACTION_RECOVER_CHAIN => {
            if let Err(err) = ensure_empty_payload(request.payload.as_ref(), action) {
                return action_error(state, action, request_host, "invalid_request", err, None);
            }
            let Some(recovery) = state.chain_recovery.clone() else {
                return action_error(
                    state,
                    action,
                    request_host,
                    "action_failed",
                    "no stale execution world recovery is currently available",
                    None,
                );
            };
            if recovery.recovery_mode != "fresh_node_id" {
                return action_error(
                    state,
                    action,
                    request_host,
                    "action_failed",
                    format!(
                        "unsupported chain recovery mode: {}",
                        recovery.recovery_mode
                    ),
                    Some(to_json_value(&recovery)),
                );
            }
            let outcome = start_chain_process(state, recovery.suggested_config.clone());
            poll_service_state(state);
            let outcome = finalize_chain_start_outcome(state, outcome);
            match outcome {
                Ok(()) => action_ok(state, action, request_host, Some(to_json_value(&recovery))),
                Err(err) => action_error(
                    state,
                    action,
                    request_host,
                    chain_error_code_for_state(state, err.as_str()),
                    err,
                    chain_error_data_for_state(state).or_else(|| Some(to_json_value(&recovery))),
                ),
            }
        }
        ACTION_STOP_CHAIN => {
            if let Err(err) = ensure_empty_payload(request.payload.as_ref(), action) {
                return action_error(state, action, request_host, "invalid_request", err, None);
            }
            let outcome = stop_chain_process(state);
            poll_service_state(state);
            match outcome {
                Ok(()) => action_ok(state, action, request_host, None),
                Err(err) => action_error(
                    state,
                    action,
                    request_host,
                    classify_runtime_error_code(err.as_str()),
                    err,
                    None,
                ),
            }
        }
        ACTION_SET_CHAIN_NETWORK_TIER => {
            let payload = match parse_action_payload::<GuiAgentNetworkTierPayload>(
                request.payload.as_ref(),
                action,
            ) {
                Ok(payload) => payload,
                Err(err) => {
                    return action_error(state, action, request_host, "invalid_request", err, None)
                }
            };
            let mut config = state.config.clone();
            config.chain_network_tier = payload.tier;
            if let Some(manifest) = payload.manifest {
                config.chain_network_tier_manifest = manifest;
            } else {
                config.chain_network_tier_manifest.clear();
            }
            if let Err(err) = normalize_chain_network_tier_config(&mut config) {
                return action_error(state, action, request_host, "invalid_request", err, None);
            }
            state.config = config;
            state.append_log(format!(
                "chain network tier switched to {}",
                state.config.chain_network_tier
            ));
            state.mark_updated();
            poll_service_state(state);
            action_ok(
                state,
                action,
                request_host,
                Some(serde_json::json!({
                    "chain_network_tier": state.config.chain_network_tier,
                    "chain_network_tier_manifest": state.config.chain_network_tier_manifest,
                    "chain_running": state.chain_running.is_some()
                })),
            )
        }
        ACTION_SUBMIT_TRANSFER => {
            let submit_request = match parse_action_payload::<ChainTransferSubmitRequest>(
                request.payload.as_ref(),
                action,
            ) {
                Ok(payload) => payload,
                Err(err) => {
                    return action_error(state, action, request_host, "invalid_request", err, None)
                }
            };
            let response = submit_chain_transfer(state, &submit_request);
            poll_service_state(state);
            let data = to_json_value(&response);
            if response.ok {
                action_ok(state, action, request_host, Some(data))
            } else {
                action_error(
                    state,
                    action,
                    request_host,
                    response.error_code.as_deref().unwrap_or("action_failed"),
                    response
                        .error
                        .as_deref()
                        .unwrap_or("submit transfer failed"),
                    Some(data),
                )
            }
        }
        ACTION_SUBMIT_FEEDBACK => {
            let submit_request = match parse_action_payload::<ChainFeedbackSubmitRequest>(
                request.payload.as_ref(),
                action,
            ) {
                Ok(payload) => payload,
                Err(err) => {
                    return action_error(state, action, request_host, "invalid_request", err, None)
                }
            };
            let response = submit_chain_feedback(state, &submit_request);
            poll_service_state(state);
            let data = to_json_value(&response);
            if response.ok {
                action_ok(state, action, request_host, Some(data))
            } else {
                let error = response
                    .error
                    .as_deref()
                    .unwrap_or("submit feedback failed");
                action_error(
                    state,
                    action,
                    request_host,
                    classify_runtime_error_code(error),
                    error,
                    Some(data),
                )
            }
        }
        _ => {
            if let Some(spec) = query_action_spec(action) {
                execute_query_action(state, action, request.payload.as_ref(), request_host, spec)
            } else {
                action_error(
                    state,
                    action,
                    request_host,
                    "invalid_request",
                    format!("unsupported gui-agent action: {action}"),
                    None,
                )
            }
        }
    }
}

fn query_action_spec(action: &str) -> Option<QueryActionSpec> {
    QUERY_ACTION_SPECS
        .iter()
        .copied()
        .find(|spec| spec.action == action)
}

fn execute_query_action(
    state: &mut ServiceState,
    action: &str,
    payload: Option<&Value>,
    request_host: Option<&str>,
    spec: QueryActionSpec,
) -> GuiAgentActionResponse {
    let query_payload = match parse_query_payload(payload, action) {
        Ok(value) => value,
        Err(err) => return action_error(state, action, request_host, "invalid_request", err, None),
    };
    let runtime_target = match build_query_runtime_target(spec, &query_payload) {
        Ok(target) => target,
        Err(err) => return action_error(state, action, request_host, "invalid_request", err, None),
    };

    let data = query_chain_transfer_json(state, runtime_target.as_str());
    poll_service_state(state);
    let ok = data.get("ok").and_then(Value::as_bool).unwrap_or(true);
    if ok {
        action_ok(state, action, request_host, Some(data))
    } else {
        let error_code = data
            .get("error_code")
            .and_then(Value::as_str)
            .unwrap_or("action_failed")
            .to_string();
        let error = data
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("query action failed")
            .to_string();
        action_error(state, action, request_host, error_code, error, Some(data))
    }
}

fn parse_action_config(
    payload: Option<&Value>,
    state: &ServiceState,
    action: &str,
) -> Result<LauncherConfig, String> {
    let Some(payload) = payload else {
        return Ok(state.config.clone());
    };
    if payload.is_null() {
        return Ok(state.config.clone());
    }
    if payload
        .as_object()
        .is_some_and(|object| object.contains_key("config"))
    {
        let mut config_payload: GuiAgentConfigPayload = serde_json::from_value(payload.clone())
            .map_err(|err| format!("parse `{action}` payload failed: {err}"))?;
        normalize_chain_network_tier_config(&mut config_payload.config)?;
        return Ok(config_payload.config);
    }
    let mut config: LauncherConfig = serde_json::from_value(payload.clone())
        .map_err(|err| format!("parse `{action}` payload failed: {err}"))?;
    normalize_chain_network_tier_config(&mut config)?;
    Ok(config)
}

fn parse_action_payload<T: DeserializeOwned>(
    payload: Option<&Value>,
    action: &str,
) -> Result<T, String> {
    let Some(payload) = payload else {
        return Err(format!("`{action}` requires payload"));
    };
    serde_json::from_value(payload.clone())
        .map_err(|err| format!("parse `{action}` payload failed: {err}"))
}

fn ensure_empty_payload(payload: Option<&Value>, action: &str) -> Result<(), String> {
    match payload {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(map)) if map.is_empty() => Ok(()),
        Some(_) => Err(format!("`{action}` does not accept payload")),
    }
}

fn parse_query_payload(
    payload: Option<&Value>,
    action: &str,
) -> Result<GuiAgentQueryPayload, String> {
    match payload {
        None | Some(Value::Null) => Ok(GuiAgentQueryPayload::default()),
        Some(payload) => serde_json::from_value(payload.clone())
            .map_err(|err| format!("parse `{action}` payload failed: {err}")),
    }
}

fn build_query_runtime_target(
    spec: QueryActionSpec,
    payload: &GuiAgentQueryPayload,
) -> Result<String, String> {
    if let Some(query_target) = payload.query_target.as_deref() {
        if query_target != spec.query_target {
            return Err(format!(
                "payload.query_target `{query_target}` is invalid for action `{}`",
                spec.action
            ));
        }
    }

    let query = payload.query.as_deref().unwrap_or_default().trim();
    if query.is_empty() {
        return Ok(spec.runtime_target.to_string());
    }
    if query.contains('\r') || query.contains('\n') {
        return Err("payload.query must not contain CR/LF".to_string());
    }

    let query = query.trim_start_matches('?');
    if query.is_empty() {
        Ok(spec.runtime_target.to_string())
    } else {
        Ok(format!("{}?{}", spec.runtime_target, query))
    }
}

fn classify_runtime_error_code(error: &str) -> &'static str {
    let error_lower = error.to_ascii_lowercase();
    if error.contains("chain runtime is disabled") {
        "chain_disabled"
    } else if error.contains("proxy") {
        "proxy_error"
    } else if error_lower.contains("stale execution world")
        || error_lower.contains("latest state root mismatch")
    {
        "stale_execution_world"
    } else {
        "action_failed"
    }
}

fn to_json_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(|err| {
        serde_json::json!({
            "ok": false,
            "error_code": "serialize_error",
            "error": format!("serialize gui-agent data failed: {err}"),
        })
    })
}

fn action_ok(
    state: &ServiceState,
    action: &str,
    request_host: Option<&str>,
    data: Option<Value>,
) -> GuiAgentActionResponse {
    GuiAgentActionResponse {
        ok: true,
        action: action.to_string(),
        error_code: None,
        error: None,
        data,
        state: snapshot_from_state(state, request_host),
    }
}

fn action_error(
    state: &ServiceState,
    action: &str,
    request_host: Option<&str>,
    error_code: impl Into<String>,
    error: impl Into<String>,
    data: Option<Value>,
) -> GuiAgentActionResponse {
    GuiAgentActionResponse {
        ok: false,
        action: action.to_string(),
        error_code: Some(error_code.into()),
        error: Some(error.into()),
        data,
        state: snapshot_from_state(state, request_host),
    }
}
