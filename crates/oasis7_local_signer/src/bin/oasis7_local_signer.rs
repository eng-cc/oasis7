use std::io::{self, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use oasis7_local_signer::error::SignerError;
use oasis7_local_signer::identity::sha256_hex;
use oasis7_local_signer::installation::{
    current_euid, current_uid, load_fixed_installation_config, validate_fixed_sudo,
    worker_sudo_args,
};
use oasis7_local_signer::job::JobDirectory;
use oasis7_local_signer::protocol::{IPC_SCHEMA, IpcRequest, IpcResponse, read_response_frame};
use oasis7_local_signer::types::InstallationConfig;

const WORKER_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_STDERR_BYTES: usize = 64 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: operation failed", error.code);
        std::process::exit(error.exit_code);
    }
}

struct CliError {
    code: &'static str,
    exit_code: i32,
}

impl CliError {
    const fn new(code: &'static str, exit_code: i32) -> Self {
        Self { code, exit_code }
    }

    fn signer(error: SignerError) -> Self {
        Self::new(error.code(), error.exit_code())
    }

    const fn protocol() -> Self {
        Self::new("IPC_PROTOCOL_ERROR", 8)
    }

    const fn timeout() -> Self {
        Self::new("WORKER_TIMEOUT", 13)
    }
}

struct WorkerOutcome {
    response: IpcResponse,
    exit_status: ExitStatus,
}

fn run() -> Result<(), CliError> {
    let all_args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(result) = oasis7_local_signer::file_cli::dispatch(&all_args) {
        return print_json(&result.map_err(CliError::signer)?);
    }
    if all_args.as_slice() == ["--help"] {
        return print_json(
            &serde_json::json!({"usage":["doctor", "prepare|submit|inspect --job-id ID", "file-payload --context ABSOLUTE --expected-context-sha256 SHA256 --file ABSOLUTE --expected-file-sha256 SHA256 --output ABSOLUTE"], "file_payload":"No worker or installation access. Output is create-only in caller-owned 0700 directory; verify signatures over the exact envelope, not raw file bytes."}),
        );
    }
    let mut args = all_args.into_iter();
    let command = args
        .next()
        .ok_or_else(|| CliError::new("INVALID_INPUT", 2))?;
    let job_id = match (command.as_str(), args.next(), args.next()) {
        ("doctor", None, None) => None,
        ("prepare" | "submit" | "inspect", Some(flag), Some(value))
            if flag == "--job-id" && args.next().is_none() =>
        {
            oasis7_local_signer::protocol::validate_id(&value)
                .map_err(|_| CliError::new("INVALID_INPUT", 2))?;
            Some(value)
        }
        _ => return Err(CliError::new("INVALID_INPUT", 2)),
    };

    let installation = load_fixed_installation_config().map_err(CliError::signer)?;
    let caller_uid = require_bound_caller(&installation)?;

    match command.as_str() {
        "doctor" => {
            let request = IpcRequest::Doctor {
                schema_version: IPC_SCHEMA.to_owned(),
                installation_id: installation.installation_id.clone(),
            };
            let outcome = invoke_worker(&installation, request)?;
            print_response(&outcome.response)?;
            match &outcome.response {
                IpcResponse::Doctor(response)
                    if response.ready && outcome.exit_status.success() =>
                {
                    Ok(())
                }
                IpcResponse::Doctor(response) => {
                    Err(code_error(&response.status_code, outcome.exit_status))
                }
                _ => Err(CliError::protocol()),
            }
        }
        "prepare" => {
            let job_id = job_id
                .as_deref()
                .ok_or_else(|| CliError::new("INVALID_INPUT", 2))?;
            let job_dir = JobDirectory::open(&installation, caller_uid, job_id, true)
                .map_err(CliError::signer)?;
            prepare_or_report_ready(&installation, &job_dir, job_id)
        }
        "submit" => {
            let job_id = job_id
                .as_deref()
                .ok_or_else(|| CliError::new("INVALID_INPUT", 2))?;
            let job_dir = JobDirectory::open(&installation, caller_uid, job_id, false)
                .map_err(CliError::signer)?;
            require_job_material(&job_dir)?;
            if !job_dir.present("request.json").map_err(CliError::signer)?
                || !job_dir
                    .present("request.payload.sha256")
                    .map_err(CliError::signer)?
            {
                return Err(CliError::new("INVALID_INPUT", 2));
            }
            let prepared = job_dir.prepare().map_err(CliError::signer)?;
            let outcome = invoke_worker(&installation, prepared.ipc_request())?;
            print_response(&outcome.response)?;
            match &outcome.response {
                IpcResponse::Sign(result)
                    if result.status == "committed"
                        && result.error_code.0.is_none()
                        && outcome.exit_status.success() =>
                {
                    Ok(())
                }
                IpcResponse::Sign(result) => Err(code_error(
                    result
                        .error_code
                        .0
                        .as_deref()
                        .unwrap_or("CRYPTO_OR_BINDING_INVALID"),
                    outcome.exit_status,
                )),
                _ => Err(CliError::protocol()),
            }
        }
        "inspect" => {
            let job_id = job_id
                .as_deref()
                .ok_or_else(|| CliError::new("INVALID_INPUT", 2))?;
            let job_dir = JobDirectory::open(&installation, caller_uid, job_id, false)
                .map_err(CliError::signer)?;
            if !job_dir.present("request.json").map_err(CliError::signer)? {
                return Err(CliError::new("RECOVERY_REQUIRED", 10));
            }
            let request = job_dir.read_request().map_err(CliError::signer)?;
            let ipc_request = IpcRequest::Inspect {
                schema_version: IPC_SCHEMA.to_owned(),
                installation_id: installation.installation_id.clone(),
                request_id: request.request_id,
                purpose: request.purpose,
                provider_id: request.provider_id,
            };
            let outcome = invoke_worker(&installation, ipc_request)?;
            print_response(&outcome.response)?;
            if matches!(outcome.response, IpcResponse::Inspect(_)) && outcome.exit_status.success()
            {
                Ok(())
            } else {
                Err(match &outcome.response {
                    IpcResponse::Inspect(response) => code_error(
                        response
                            .error_code
                            .0
                            .as_deref()
                            .unwrap_or("CRYPTO_OR_BINDING_INVALID"),
                        outcome.exit_status,
                    ),
                    _ => CliError::protocol(),
                })
            }
        }
        _ => Err(CliError::new("INVALID_INPUT", 2)),
    }
}

fn require_bound_caller(installation: &InstallationConfig) -> Result<u32, CliError> {
    let caller_uid = current_uid();
    if current_euid() != caller_uid || installation.caller(caller_uid).is_none() {
        return Err(CliError::new("AUTHORIZATION_DENIED", 3));
    }
    Ok(caller_uid)
}

fn prepare_or_report_ready(
    installation: &InstallationConfig,
    job_dir: &JobDirectory,
    job_id: &str,
) -> Result<(), CliError> {
    let input_present = job_dir.present("input.json").map_err(CliError::signer)?;
    let payload_present = job_dir.present("payload.bin").map_err(CliError::signer)?;
    if !input_present && !payload_present {
        if job_dir.present("request.json").map_err(CliError::signer)?
            || job_dir
                .present("request.payload.sha256")
                .map_err(CliError::signer)?
        {
            return Err(CliError::new("RECOVERY_REQUIRED", 10));
        }
        let output = serde_json::json!({
            "schema_version": "oasis7.local_signer_prepare_result.v1",
            "status": "JOB_DIRECTORY_READY",
            "job_id": job_id,
            "work_dir": job_dir.path(),
            "installation_id": installation.installation_id,
            "job_directory_exists": true,
            "signing_enabled": false
        });
        return print_json(&output);
    }
    if !input_present || !payload_present {
        return Err(CliError::new("INVALID_INPUT", 2));
    }

    let prepared = job_dir.prepare().map_err(CliError::signer)?;
    let output = serde_json::json!({
        "schema_version": "oasis7.local_signer_prepare_result.v1",
        "status": "PREPARED",
        "job_id": job_id,
        "request_id": prepared.request.request_id,
        "payload_sha256": sha256_hex(&prepared.payload),
        "installation_id": installation.installation_id,
        "request_metadata_persisted": true,
        "signing_enabled": false
    });
    print_json(&output)
}

fn require_job_material(job_dir: &JobDirectory) -> Result<(), CliError> {
    if !job_dir.present("input.json").map_err(CliError::signer)?
        || !job_dir.present("payload.bin").map_err(CliError::signer)?
    {
        return Err(CliError::new("INVALID_INPUT", 2));
    }
    Ok(())
}

fn invoke_worker(
    installation: &InstallationConfig,
    request: IpcRequest,
) -> Result<WorkerOutcome, CliError> {
    let sudo = validate_fixed_sudo().map_err(CliError::signer)?;
    let frame = oasis7_local_signer::protocol::encode_request_frame(&request)
        .map_err(|_| CliError::protocol())?;
    let mut child = Command::new(sudo)
        .args(worker_sudo_args(installation))
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| CliError::new("UNSUPPORTED_PLATFORM_OR_FS", 9))?;

    let stdin = child.stdin.take().ok_or_else(CliError::protocol)?;
    let stdout = child.stdout.take().ok_or_else(CliError::protocol)?;
    let stderr = child.stderr.take().ok_or_else(CliError::protocol)?;
    let writer = thread::spawn(move || write_frame(stdin, &frame));
    let reader = thread::spawn(move || {
        let mut stdout = stdout;
        read_response_frame(&mut stdout).map_err(|_| ())
    });
    let stderr_reader = thread::spawn(move || drain_bounded(stderr, MAX_STDERR_BYTES));

    let (exit_status, timed_out) = wait_bounded(&mut child)?;
    if timed_out {
        // The killed child closes all three pipes. Drain/join them without
        // allowing their expected EOF/broken-pipe results to mask the timeout.
        let _ = writer.join();
        let _ = reader.join();
        let _ = stderr_reader.join();
        return Err(CliError::timeout());
    }
    let write_result = writer.join().map_err(|_| CliError::protocol())?;
    let response_result = reader.join().map_err(|_| CliError::protocol())?;
    let _bounded_stderr = stderr_reader.join().map_err(|_| CliError::protocol())?;
    write_result.map_err(|_| CliError::protocol())?;
    let response = response_result.map_err(|_| {
        if !exit_status.success() {
            CliError::new("WORKER_FAILED", known_worker_exit(&exit_status))
        } else {
            CliError::protocol()
        }
    })?;
    Ok(WorkerOutcome {
        response,
        exit_status,
    })
}

fn write_frame(mut stdin: impl Write, frame: &[u8]) -> io::Result<()> {
    stdin.write_all(frame)?;
    stdin.flush()
}

fn drain_bounded(mut stream: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Ok(kept);
        }
        let remaining = limit.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
    }
}

fn wait_bounded(child: &mut Child) -> Result<(ExitStatus, bool), CliError> {
    let deadline = Instant::now() + WORKER_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|_| CliError::protocol())? {
            return Ok((status, false));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().map_err(|_| CliError::protocol())?;
            return Ok((status, true));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn print_response(response: &IpcResponse) -> Result<(), CliError> {
    print_json(response)
}

fn print_json(value: &impl serde::Serialize) -> Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, value).map_err(|_| CliError::protocol())?;
    stdout.write_all(b"\n").map_err(|_| CliError::protocol())?;
    stdout.flush().map_err(|_| CliError::protocol())
}

fn code_error(code: &str, status: ExitStatus) -> CliError {
    let exit_code = match code {
        "INVALID_INPUT" => 2,
        "AUTHORIZATION_DENIED" => 3,
        "KEY_OR_AUTHORITY_UNAVAILABLE" => 4,
        "LOCK_BUSY" => 5,
        "ID_CONFLICT" => 6,
        "PERSISTENCE_FAILED" => 7,
        "CRYPTO_OR_BINDING_INVALID" => 8,
        "UNSUPPORTED_PLATFORM_OR_FS" => 9,
        "RECOVERY_REQUIRED" => 10,
        "INSTALLATION_DRIFT" => 11,
        "BUDGET_EXHAUSTED" => 12,
        _ if !status.success() => known_worker_exit(&status),
        _ => 8,
    };
    CliError::new(
        match code {
            "INVALID_INPUT" => "INVALID_INPUT",
            "AUTHORIZATION_DENIED" => "AUTHORIZATION_DENIED",
            "KEY_OR_AUTHORITY_UNAVAILABLE" => "KEY_OR_AUTHORITY_UNAVAILABLE",
            "LOCK_BUSY" => "LOCK_BUSY",
            "ID_CONFLICT" => "ID_CONFLICT",
            "PERSISTENCE_FAILED" => "PERSISTENCE_FAILED",
            "CRYPTO_OR_BINDING_INVALID" => "CRYPTO_OR_BINDING_INVALID",
            "UNSUPPORTED_PLATFORM_OR_FS" => "UNSUPPORTED_PLATFORM_OR_FS",
            "RECOVERY_REQUIRED" => "RECOVERY_REQUIRED",
            "INSTALLATION_DRIFT" => "INSTALLATION_DRIFT",
            "BUDGET_EXHAUSTED" => "BUDGET_EXHAUSTED",
            _ => "WORKER_FAILED",
        },
        exit_code,
    )
}

fn known_worker_exit(status: &ExitStatus) -> i32 {
    status
        .code()
        .filter(|code| (2..=12).contains(code))
        .unwrap_or(8)
}
