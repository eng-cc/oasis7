use std::io::{Read, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crate::error::SignerError;
use crate::protocol::{
    DoctorResponse, ExplicitNull, InspectResponse, IpcRequest, IpcResponse, SignResult,
    read_request_frame, write_response_frame,
};
use crate::store::SignerStore;

// Leave time before the caller's existing 20-second process-wait deadline
// for sudo and the parent to observe worker exit and drain its pipes.
const WORKER_DEADLINE: Duration = Duration::from_secs(18);

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
    run_worker_with_deadline(WORKER_DEADLINE)
}

fn run_worker_with_deadline(timeout: Duration) -> i32 {
    let (completed_tx, completed_rx) = mpsc::sync_channel(1);
    let watchdog = thread::spawn(move || match completed_rx.recv_timeout(timeout) {
        Ok(()) => {}
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
            eprintln!("worker request exceeded its execution deadline");
            // The request thread may be blocked in an attacker-controlled
            // stdin read. Terminating the process releases that read and all
            // signer-UID locks instead of leaving a detached worker behind.
            std::process::exit(12);
        }
    });
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let exit_code = serve_one(&mut stdin.lock(), &mut stdout.lock());
    let _ = completed_tx.send(());
    if watchdog.join().is_err() {
        return 12;
    }
    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{IPC_SCHEMA, RollbackProtocolContext, SignContext};
    use std::process::{Command, Stdio};
    use std::time::Instant;

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

    #[test]
    fn deadline_child_entry() {
        if std::env::var_os("OASIS7_SIGNER_WORKER_DEADLINE_CHILD").is_none() {
            return;
        }
        let _ = run_worker_with_deadline(Duration::from_millis(250));
        panic!("worker deadline must terminate the child process");
    }

    #[test]
    fn direct_worker_process_exits_for_open_incomplete_frames() {
        for (case, bytes) in [
            ("no-input", &[][..]),
            ("partial-header", &[0, 0][..]),
            ("partial-body", &[0, 0, 0, 8, b'{', b'}'][..]),
        ] {
            let mut child = Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "worker::tests::deadline_child_entry",
                    "--nocapture",
                ])
                .env("OASIS7_SIGNER_WORKER_DEADLINE_CHILD", case)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn direct worker test process");
            let mut stdin = child.stdin.take().expect("child stdin pipe");
            stdin
                .write_all(bytes)
                .expect("write incomplete frame prefix");
            // Keep stdin open: EOF must not be what releases the worker.
            let started = Instant::now();
            let status = loop {
                if let Some(status) = child.try_wait().expect("poll child process") {
                    break status;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(3),
                    "{case} worker process did not honor its deadline"
                );
                thread::sleep(Duration::from_millis(10));
            };
            drop(stdin);
            assert_eq!(status.code(), Some(12), "{case} process exit status");
        }
    }
}
