use crate::error::WorldError;
use oasis7_proto::distributed::{DistributedErrorCode, ErrorResponse};

pub(super) fn error_response_from_world_error(err: &WorldError) -> ErrorResponse {
    match err {
        WorldError::NetworkRequestFailed {
            code,
            message,
            retryable,
        } => ErrorResponse {
            code: *code,
            message: message.clone(),
            retryable: *retryable,
        },
        WorldError::NetworkProtocolUnavailable { protocol } => {
            ErrorResponse::from_code(protocol_unavailable_code(protocol), protocol.clone())
        }
        WorldError::DistributedValidationFailed { reason } => {
            ErrorResponse::from_code(DistributedErrorCode::ErrBadRequest, reason.clone())
        }
        WorldError::BlobNotFound { content_hash } => ErrorResponse::from_code(
            DistributedErrorCode::ErrNotFound,
            format!("blob not found: {content_hash}"),
        ),
        WorldError::BlobHashMismatch { expected, actual } => ErrorResponse::from_code(
            DistributedErrorCode::ErrStateMismatch,
            format!("blob hash mismatch expected={expected} actual={actual}"),
        ),
        WorldError::BlobHashInvalid { content_hash } => ErrorResponse::from_code(
            DistributedErrorCode::ErrInvalidHash,
            format!("blob hash invalid: {content_hash}"),
        ),
        WorldError::SignatureKeyInvalid => ErrorResponse::from_code(
            DistributedErrorCode::ErrUnauthorized,
            "invalid signature key",
        ),
        WorldError::Io(message) => {
            ErrorResponse::from_code(DistributedErrorCode::ErrNotAvailable, message.clone())
        }
        WorldError::Serde(message) => {
            ErrorResponse::from_code(DistributedErrorCode::ErrBadRequest, message.clone())
        }
    }
}

fn protocol_unavailable_code(protocol: &str) -> DistributedErrorCode {
    if protocol.contains("handler missing") || protocol.starts_with('/') {
        DistributedErrorCode::ErrUnsupported
    } else if protocol_unavailable_is_bad_request(protocol) {
        DistributedErrorCode::ErrBadRequest
    } else if protocol_unavailable_is_retryable_gap(protocol) {
        DistributedErrorCode::ErrNotAvailable
    } else {
        DistributedErrorCode::ErrUnsupported
    }
}

fn protocol_unavailable_is_bad_request(protocol: &str) -> bool {
    protocol.contains("must be utf-8") || protocol.contains("must be valid")
}

fn protocol_unavailable_is_retryable_gap(protocol: &str) -> bool {
    protocol == "libp2p"
        || protocol.starts_with("transport dial failed: ")
        || protocol.contains("is not connected for protocol")
        || protocol.contains("no connected peers for protocol")
        || protocol.contains("no admissible connected peers for protocol")
        || protocol.contains("no connected providers for protocol")
        || protocol.contains("no healthy provider for protocol")
        || protocol.contains("no healthy connected providers for protocol")
        || protocol.contains("request failed: ConnectionClosed")
        || protocol.contains("request failed: DialFailure")
        || protocol.contains("request failed: Timeout")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_protocol_unavailable_handler_missing_maps_to_unsupported() {
        let response = error_response_from_world_error(&WorldError::NetworkProtocolUnavailable {
            protocol: "/aw/node/replication/ping".to_string(),
        });
        assert_eq!(response.code, DistributedErrorCode::ErrUnsupported);
        assert!(!response.retryable);
    }

    #[test]
    fn network_protocol_unavailable_availability_gap_maps_to_not_available() {
        let response = error_response_from_world_error(&WorldError::NetworkProtocolUnavailable {
            protocol: "no connected peers for protocol /aw/node/replication/ping".to_string(),
        });
        assert_eq!(response.code, DistributedErrorCode::ErrNotAvailable);
        assert!(response.retryable);
    }

    #[test]
    fn network_protocol_unavailable_no_admissible_peers_maps_to_not_available() {
        let response = error_response_from_world_error(&WorldError::NetworkProtocolUnavailable {
            protocol:
                "libp2p-replication no admissible connected peers for protocol /aw/node/replication/ping"
                    .to_string(),
        });
        assert_eq!(response.code, DistributedErrorCode::ErrNotAvailable);
        assert!(response.retryable);
    }

    #[test]
    fn network_protocol_unavailable_no_healthy_provider_maps_to_not_available() {
        let response = error_response_from_world_error(&WorldError::NetworkProtocolUnavailable {
            protocol:
                "libp2p-replication no healthy connected providers for protocol /aw/node/replication/ping"
                    .to_string(),
        });
        assert_eq!(response.code, DistributedErrorCode::ErrNotAvailable);
        assert!(response.retryable);
    }

    #[test]
    fn network_protocol_unavailable_bad_request_maps_to_bad_request() {
        let response = error_response_from_world_error(&WorldError::NetworkProtocolUnavailable {
            protocol: "cached peer record payload must be utf-8".to_string(),
        });
        assert_eq!(response.code, DistributedErrorCode::ErrBadRequest);
        assert!(!response.retryable);
    }
}
