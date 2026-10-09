use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufWriter;
use std::io::Write;
use std::net::TcpStream;

pub(super) const AGENCY_CONTROL_REQUEST_TYPE: &str = "agency_control_request";
pub(super) const AGENCY_CONTROL_RESPONSE_TYPE: &str = "agency_control_response";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum AgencyControlOperation {
    Inspect,
    CorrectMemory {
        correction_id: String,
        target_memory_id: String,
        expected_revision: u64,
        replacement_summary: String,
    },
    InstallDelegation {
        grant_id: String,
        object_id: String,
        action_kinds: Vec<String>,
        period_id: String,
        valid_from_tick: u64,
        valid_until_tick: u64,
        limit_units: u64,
    },
    RevokeDelegation {
        grant_id: String,
        expected_revision: u64,
    },
    OverridePendingIntent {
        control_id: String,
        intent_id: String,
        request_digest: String,
        grant_id: String,
        expected_grant_revision: u64,
    },
    InterruptPendingIntent {
        control_id: String,
        intent_id: String,
        request_digest: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        grant_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_grant_revision: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgencyControlRequest {
    #[serde(rename = "type")]
    pub(super) frame_type: String,
    pub(super) request_id: String,
    pub(super) player_id: String,
    pub(super) public_key: String,
    pub(super) agent_id: String,
    pub(super) world_id: String,
    pub(super) reorg_epoch: u64,
    pub(super) command: AgencyControlOperation,
    pub(super) auth: Option<crate::viewer::protocol::PlayerAuthProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgencyControlError {
    pub(super) code: String,
    pub(super) message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgencyControlResponse {
    #[serde(rename = "type")]
    pub(super) frame_type: String,
    pub(super) request_id: String,
    pub(super) status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<AgencyControlError>,
}

impl AgencyControlResponse {
    pub(super) fn success(request_id: impl Into<String>, data: Value) -> Self {
        Self {
            frame_type: AGENCY_CONTROL_RESPONSE_TYPE.to_string(),
            request_id: request_id.into(),
            status: "ok".to_string(),
            data: Some(data),
            error: None,
        }
    }

    pub(super) fn error(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            frame_type: AGENCY_CONTROL_RESPONSE_TYPE.to_string(),
            request_id: request_id.into(),
            status: "error".to_string(),
            data: None,
            error: Some(AgencyControlError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

pub(super) enum ParsedAgencyControlFrame {
    NotAgencyControl,
    Request(AgencyControlRequest),
    Invalid(AgencyControlResponse),
}

pub(super) fn parse_agency_control_frame(raw: &str) -> ParsedAgencyControlFrame {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return ParsedAgencyControlFrame::NotAgencyControl;
    };
    if value.get("type").and_then(Value::as_str) != Some(AGENCY_CONTROL_REQUEST_TYPE) {
        return ParsedAgencyControlFrame::NotAgencyControl;
    }
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    match serde_json::from_value::<AgencyControlRequest>(value) {
        Ok(request) => ParsedAgencyControlFrame::Request(request),
        Err(error) => ParsedAgencyControlFrame::Invalid(AgencyControlResponse::error(
            request_id,
            "invalid_request",
            error.to_string(),
        )),
    }
}

pub(super) fn write_agency_control_response(
    writer: &mut BufWriter<TcpStream>,
    response: &AgencyControlResponse,
) -> std::io::Result<()> {
    serde_json::to_writer(&mut *writer, response).map_err(std::io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
