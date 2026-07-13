use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::world_error::WorldError;

use crate::NodeError;
use crate::network_bridge::{
    REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX, REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX,
};

pub(crate) fn replication_network_error_kind_label(code: DistributedErrorCode) -> &'static str {
    match code {
        DistributedErrorCode::ErrNotFound => "not_found",
        DistributedErrorCode::ErrUnsupported => "unsupported",
        DistributedErrorCode::ErrTimeout => "timeout",
        DistributedErrorCode::ErrNotAvailable => "not_available",
        DistributedErrorCode::ErrBusy => "busy",
        DistributedErrorCode::ErrRateLimited => "rate_limited",
        DistributedErrorCode::ErrOverloaded => "overloaded",
        DistributedErrorCode::ErrBadRequest => "bad_request",
        DistributedErrorCode::ErrUnauthorized => "unauthorized",
        DistributedErrorCode::ErrStateMismatch => "state_mismatch",
        DistributedErrorCode::ErrInvalidHash => "invalid_hash",
    }
}

pub(crate) fn replication_network_error_is_availability_gap(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.starts_with(REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX)
}

pub(crate) fn replication_network_error_is_route_unavailable(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.starts_with(REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX)
}

pub(crate) fn replication_network_error_mentions_protocol(err: &NodeError, protocol: &str) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains(protocol)
}

pub(crate) fn replication_network_error_is_not_found(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    network_request_reason_has_kind(reason, "not_found")
        || (reason.contains("NetworkRequestFailed") && reason.contains("ErrNotFound"))
}

pub(crate) fn replication_network_error_is_unsupported_protocol(
    err: &NodeError,
    protocol: &str,
) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    (network_request_reason_has_kind(reason, "unsupported")
        && network_request_reason_detail_mentions_protocol(reason, protocol))
        || (reason.contains("NetworkRequestFailed")
            && reason.contains("ErrUnsupported")
            && reason.contains(protocol))
}

pub(crate) fn replication_network_error_is_protocol_unavailable(
    err: &NodeError,
    protocol: &str,
) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    (reason.starts_with(REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX) && reason.contains(protocol))
        || reason.contains("NetworkProtocolUnavailable")
            && (reason.contains("handler missing") || reason.contains(protocol))
}

pub(crate) fn replication_network_error_is_timeout_protocol(
    err: &NodeError,
    protocol: &str,
) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains(protocol)
        && (network_request_reason_has_kind(reason, "timeout")
            || reason.contains("request failed: Timeout")
            || reason.contains("timed out"))
}

pub(crate) fn replication_network_error_is_rate_limited_protocol(
    err: &NodeError,
    protocol: &str,
) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    (network_request_reason_has_kind(reason, "rate_limited")
        && network_request_reason_has_protocol(reason, protocol))
        || (reason.contains("NetworkRequestFailed")
            && reason.contains("ErrRateLimited")
            && reason.contains(protocol))
}

pub(crate) fn replication_network_error_should_keep_timeout_over_provider_gap(
    current: Option<&NodeError>,
    candidate: &NodeError,
    protocol: &str,
) -> bool {
    let Some(current) = current else {
        return false;
    };
    replication_network_error_is_timeout_protocol(current, protocol)
        && (replication_network_error_is_protocol_unavailable(candidate, protocol)
            || replication_network_error_is_availability_gap(candidate))
}

#[cfg(feature = "libp2p")]
pub(crate) fn network_world_error_is_retryable_connection_gap(err: &WorldError) -> bool {
    oasis7_net::world_error_is_retryable_connection_gap(err)
}

#[cfg(not(feature = "libp2p"))]
pub(crate) fn network_world_error_is_retryable_connection_gap(_err: &WorldError) -> bool {
    false
}

#[cfg(feature = "libp2p")]
pub(crate) fn network_world_error_is_publish_failure(err: &WorldError) -> bool {
    oasis7_net::world_error_is_publish_failure(err)
}

#[cfg(not(feature = "libp2p"))]
pub(crate) fn network_world_error_is_publish_failure(_err: &WorldError) -> bool {
    false
}

fn network_request_reason_has_kind(reason: &str, kind: &str) -> bool {
    reason
        .split_whitespace()
        .any(|field| field.strip_prefix("kind=") == Some(kind))
}

fn network_request_reason_detail_mentions_protocol(reason: &str, protocol: &str) -> bool {
    reason
        .split_once(" detail=")
        .is_some_and(|(_, detail)| detail.contains(protocol))
}

fn network_request_reason_has_protocol(reason: &str, protocol: &str) -> bool {
    reason
        .split_whitespace()
        .any(|field| field.strip_prefix("protocol=") == Some(protocol))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_protocol_matches_libp2p_command_timeout_shape() {
        let protocol = "/aw/node/replication/fetch-commit/1.0.0";
        let err = NodeError::Replication {
            reason: format!(
                "libp2p command request_to_peer protocol={protocol} timed out after 1500ms"
            ),
        };

        assert!(replication_network_error_is_timeout_protocol(
            &err, protocol
        ));
    }

    #[test]
    fn rate_limited_protocol_matches_structured_response_shape() {
        let protocol = "/aw/node/replication/fetch-blob/1.0.0";
        let err = NodeError::Replication {
            reason: format!(
                "replication network request failed: kind=rate_limited protocol={protocol} detail=fetch-blob response budget exhausted; retry after window reset"
            ),
        };

        assert!(replication_network_error_is_rate_limited_protocol(
            &err, protocol
        ));
    }
}
