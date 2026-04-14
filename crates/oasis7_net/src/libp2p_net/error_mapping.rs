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
            ErrorResponse::from_code(DistributedErrorCode::ErrUnsupported, protocol.clone())
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
