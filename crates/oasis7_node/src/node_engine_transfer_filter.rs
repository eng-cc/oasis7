use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{NodeConsensusAction, NodeError};

pub(super) fn should_drop_transfer_action_before_proposal(
    action: &NodeConsensusAction,
    now_ms: i64,
) -> Result<bool, NodeError> {
    let Some(action_json) = decode_pending_runtime_action_json(action.payload_cbor.as_slice())
        .map_err(|err| NodeError::Consensus {
            reason: format!(
                "decode pending consensus action failed before proposal action_id={}: {err}",
                action.action_id
            ),
        })?
    else {
        return Ok(false);
    };
    let Some("TransferMainToken") = action_json.get("type").and_then(JsonValue::as_str) else {
        return Ok(false);
    };
    let Some(data) = action_json.get("data").and_then(JsonValue::as_object) else {
        return Ok(false);
    };
    if data
        .get("asset_id")
        .and_then(JsonValue::as_str)
        .is_some_and(|value| value != "main_token")
    {
        return Ok(true);
    }
    if data
        .get("valid_until_unix_ms")
        .and_then(JsonValue::as_i64)
        .is_some_and(|value| value < now_ms)
    {
        return Ok(true);
    }
    Ok(false)
}

fn decode_pending_runtime_action_json(payload_cbor: &[u8]) -> Result<Option<JsonValue>, String> {
    let envelope =
        match serde_cbor::from_slice::<PendingConsensusActionPayloadEnvelope>(payload_cbor) {
            Ok(envelope) => envelope,
            Err(_) => PendingConsensusActionPayloadEnvelope {
                version: 1,
                auth: None,
                body: PendingConsensusActionPayloadBody::RuntimeAction {
                    action: serde_cbor::from_slice::<JsonValue>(payload_cbor)
                        .map_err(|err| format!("legacy runtime action decode failed: {err}"))?,
                },
            },
        };
    match envelope.body {
        PendingConsensusActionPayloadBody::RuntimeAction { action } => Ok(Some(action)),
        PendingConsensusActionPayloadBody::SimulatorAction { .. } => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PendingConsensusActionPayloadEnvelope {
    version: u8,
    #[serde(default)]
    auth: Option<JsonValue>,
    body: PendingConsensusActionPayloadBody,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum PendingConsensusActionPayloadBody {
    RuntimeAction {
        action: JsonValue,
    },
    SimulatorAction {
        action: JsonValue,
        #[serde(default)]
        submitter: JsonValue,
    },
}
