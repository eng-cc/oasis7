use std::io::{Read, Write};

use crate::error::SignerError;
use crate::protocol::{
    DoctorResponse, ExplicitNull, InspectResponse, IpcRequest, IpcResponse, SignResult,
    read_request_frame, write_response_frame,
};
use crate::store::SignerStore;

/// Process exactly one framed request and emit exactly one framed response.
///
/// This entry point is kept generic so protocol behavior can be tested without
/// installing the worker or accessing host keys.
pub fn serve_one<R: Read, W: Write>(input: &mut R, output: &mut W) -> i32 {
    let request = match read_request_frame(input) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{}", error);
            return 2;
        }
    };

    let result = SignerStore::open_worker().and_then(|store| store.handle_request(request.clone()));
    let (response, exit_code) = match result {
        Ok(response) => (response, 0),
        Err(error) => (error_response(&request, &error), error.exit_code()),
    };
    match write_response_frame(output, &response) {
        Ok(()) => exit_code,
        Err(error) => {
            eprintln!("{}", error);
            7
        }
    }
}

pub fn error_response(request: &IpcRequest, error: &SignerError) -> IpcResponse {
    match request {
        IpcRequest::Doctor {
            installation_id, ..
        } => IpcResponse::Doctor(DoctorResponse {
            schema_version: "oasis7.local_signer_doctor.v1".to_owned(),
            installation_id: installation_id.clone(),
            ready: false,
            status_code: error.code().to_owned(),
        }),
        IpcRequest::Inspect { request_id, .. } => IpcResponse::Inspect(InspectResponse {
            schema_version: "oasis7.local_signer_inspect.v1".to_owned(),
            request_id: request_id.clone(),
            operation_key: ExplicitNull(None),
            status: "error".to_owned(),
            payload_sha256: ExplicitNull(None),
            audit_digest: ExplicitNull(None),
            error_code: ExplicitNull(Some(error.code().to_owned())),
        }),
        IpcRequest::Sign { request_id, .. } => IpcResponse::Sign(SignResult {
            schema_version: crate::protocol::RESULT_SCHEMA.to_owned(),
            request_id: request_id.clone(),
            operation_key: ExplicitNull(None),
            status: "error".to_owned(),
            public_key_base64: ExplicitNull(None),
            signature_base64: ExplicitNull(None),
            provider_attestation: ExplicitNull(None),
            audit_digest: ExplicitNull(None),
            error_code: ExplicitNull(Some(error.code().to_owned())),
        }),
    }
}

/// Executable entry point used by the fixed, no-argument worker binary.
pub fn run_worker() -> i32 {
    if std::env::args_os().len() != 1 {
        eprintln!("worker does not accept command-line arguments");
        return 2;
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_one(&mut stdin.lock(), &mut stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{IPC_SCHEMA, RollbackProtocolContext, SignContext};

    fn sign_request() -> IpcRequest {
        IpcRequest::Sign {
            schema_version: IPC_SCHEMA.to_owned(),
            installation_id: "install-01".to_owned(),
            request_id: "request-01".to_owned(),
            purpose: "rollback_strict_audit_v1".to_owned(),
            provider_id: ExplicitNull(None),
            signer_id: "signer-01".to_owned(),
            grant_id: ExplicitNull(Some("grant-01".to_owned())),
            context: SignContext {
                deployment_id: "deployment-01".to_owned(),
                network_id: "network-01".to_owned(),
                task_uid: "task-01".to_owned(),
                source_head_oid: "0123456789abcdef0123456789abcdef01234567".to_owned(),
                protocol_context: RollbackProtocolContext {
                    authority_id: "authority-01".to_owned(),
                    rollback_ticket: "ticket-01".to_owned(),
                    receipt_id: "receipt-01".to_owned(),
                    nonce: "nonce-01".to_owned(),
                },
            },
            payload_base64: "eA==".to_owned(),
        }
    }

    #[test]
    fn failed_sign_response_has_explicit_null_secret_and_success_fields() {
        let response = error_response(&sign_request(), &SignerError::AuthorizationDenied);
        let IpcResponse::Sign(result) = response else {
            panic!("expected sign response");
        };
        assert_eq!(result.status, "error");
        assert_eq!(result.error_code.0.as_deref(), Some("AUTHORIZATION_DENIED"));
        assert!(result.operation_key.0.is_none());
        assert!(result.public_key_base64.0.is_none());
        assert!(result.signature_base64.0.is_none());
        assert!(result.provider_attestation.0.is_none());
        assert!(result.audit_digest.0.is_none());
    }
}
