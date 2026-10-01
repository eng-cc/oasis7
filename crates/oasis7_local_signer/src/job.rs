use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::SignerError;
use crate::identity::sha256_hex;
use crate::local_fs::{
    create_private_dir, read_regular, reject_symlink_components, write_private_new,
};
use crate::protocol::{
    ExplicitNull, IpcRequest, MAX_METADATA_BYTES, MAX_ROLLBACK_PAYLOAD_BYTES, SignContext,
    validate_id,
};
use crate::types::InstallationConfig;

pub const JOB_INPUT_SCHEMA: &str = "oasis7.local_signer_job_input.v1";
pub const JOB_SCHEMA: &str = "oasis7.local_signer_job.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobInput {
    pub schema_version: String,
    pub installation_id: String,
    pub purpose: String,
    pub provider_id: ExplicitNull<String>,
    pub signer_id: String,
    pub grant_id: ExplicitNull<String>,
    pub context: SignContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRequest {
    pub schema_version: String,
    pub installation_id: String,
    pub request_id: String,
    pub purpose: String,
    pub provider_id: ExplicitNull<String>,
    pub signer_id: String,
    pub grant_id: ExplicitNull<String>,
    pub context: SignContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedJob {
    pub request: JobRequest,
    pub payload: Vec<u8>,
}

impl PreparedJob {
    pub fn ipc_request(&self) -> IpcRequest {
        IpcRequest::Sign {
            schema_version: crate::protocol::IPC_SCHEMA.to_owned(),
            installation_id: self.request.installation_id.clone(),
            request_id: self.request.request_id.clone(),
            purpose: self.request.purpose.clone(),
            provider_id: self.request.provider_id.clone(),
            signer_id: self.request.signer_id.clone(),
            grant_id: self.request.grant_id.clone(),
            context: self.request.context.clone(),
            payload_base64: base64::engine::general_purpose::STANDARD.encode(&self.payload),
        }
    }
}

pub fn resolve_work_dir(
    installation: &InstallationConfig,
    caller_uid: u32,
    job_id: &str,
) -> Result<PathBuf, SignerError> {
    installation.validate().map_err(SignerError::InvalidInput)?;
    validate_id(job_id).map_err(|error| SignerError::InvalidInput(error.to_string()))?;
    let caller = installation
        .caller(caller_uid)
        .ok_or(SignerError::AuthorizationDenied)?;
    Ok(Path::new(&installation.store_dir)
        .join("work")
        .join(&caller.work_subdir)
        .join(job_id))
}

pub fn prepare_job_dir(
    installation: &InstallationConfig,
    caller_uid: u32,
    job_id: &str,
) -> Result<PathBuf, SignerError> {
    if crate::installation::current_uid() != caller_uid {
        return Err(SignerError::AuthorizationDenied);
    }
    let job_dir = resolve_work_dir(installation, caller_uid, job_id)?;
    let caller_root = job_dir.parent().ok_or(SignerError::InstallationDrift)?;
    reject_symlink_components(caller_root)?;
    validate_caller_work_root(caller_root, caller_uid)?;
    if !work_dir_exists(&job_dir) {
        create_private_dir(&job_dir)?;
        crate::local_fs::sync_dir(caller_root)?;
    }
    validate_caller_work_root(&job_dir, caller_uid)?;
    Ok(job_dir)
}

/// Resolve a caller-owned job directory without creating it.
///
/// Every component is checked for symlinks and the fixed caller work root and
/// requested job directory must be private directories owned by `caller_uid`.
pub fn resolve_existing_work_dir(
    installation: &InstallationConfig,
    caller_uid: u32,
    job_id: &str,
) -> Result<PathBuf, SignerError> {
    let job_dir = resolve_work_dir(installation, caller_uid, job_id)?;
    let caller_root = job_dir.parent().ok_or(SignerError::InstallationDrift)?;
    reject_symlink_components(&job_dir)?;
    validate_caller_work_root(caller_root, caller_uid)?;
    validate_caller_work_root(&job_dir, caller_uid)?;
    Ok(job_dir)
}

pub fn prepare_job(work_dir: &Path) -> Result<PreparedJob, SignerError> {
    reject_symlink_components(work_dir)?;
    let input_path = work_dir.join("input.json");
    let payload_path = work_dir.join("payload.bin");
    let request_path = work_dir.join("request.json");
    let payload_binding_path = work_dir.join("request.payload.sha256");
    let input_bytes = read_regular(&input_path, MAX_METADATA_BYTES)?;
    let payload = read_regular(&payload_path, MAX_ROLLBACK_PAYLOAD_BYTES)?;
    let input: JobInput = serde_json::from_slice(&input_bytes)
        .map_err(|_| SignerError::InvalidInput("job input schema is invalid".to_owned()))?;
    validate_job_input(&input)?;
    let payload_hash = sha256_hex(&payload);
    let request = match (request_path.exists(), payload_binding_path.exists()) {
        (false, false) => {
            let request = JobRequest {
                schema_version: JOB_SCHEMA.to_owned(),
                installation_id: input.installation_id.clone(),
                request_id: new_request_id()?,
                purpose: input.purpose.clone(),
                provider_id: input.provider_id.clone(),
                signer_id: input.signer_id.clone(),
                grant_id: input.grant_id.clone(),
                context: input.context.clone(),
            };
            let request_bytes = serde_json::to_vec(&request)
                .map_err(|_| SignerError::InvalidInput("job request is invalid".to_owned()))?;
            write_private_new(&payload_binding_path, payload_hash.as_bytes())?;
            write_private_new(&request_path, &request_bytes)?;
            crate::local_fs::sync_dir(work_dir)?;
            request
        }
        (true, true) => {
            let request_bytes = read_regular(&request_path, MAX_METADATA_BYTES)?;
            let request: JobRequest = serde_json::from_slice(&request_bytes)
                .map_err(|_| SignerError::RecoveryRequired)?;
            validate_job_request(&request)?;
            let previous_payload_hash = read_regular(&payload_binding_path, 64)?;
            if previous_payload_hash != payload_hash.as_bytes()
                || !request_matches(&input, &request)
            {
                return Err(SignerError::IdConflict);
            }
            request
        }
        _ => return Err(SignerError::RecoveryRequired),
    };
    Ok(PreparedJob { request, payload })
}

fn validate_job_input(input: &JobInput) -> Result<(), SignerError> {
    if input.schema_version != JOB_INPUT_SCHEMA {
        return Err(SignerError::InvalidInput(
            "unsupported job input schema_version".to_owned(),
        ));
    }
    validate_id(&input.installation_id)
        .and_then(|()| validate_id(&input.purpose))
        .and_then(|()| validate_id(&input.signer_id))
        .map_err(|error| SignerError::InvalidInput(error.to_string()))?;
    if input.purpose != "rollback_strict_audit_v1"
        || input.provider_id.0.is_some()
        || input.grant_id.0.is_none()
    {
        return Err(SignerError::InvalidInput(
            "M0 job must bind rollback purpose, null provider, and a grant".to_owned(),
        ));
    }
    validate_id(input.grant_id.0.as_deref().unwrap_or_default())
        .map_err(|error| SignerError::InvalidInput(error.to_string()))?;
    let ipc = PreparedJob {
        request: JobRequest {
            schema_version: JOB_SCHEMA.to_owned(),
            installation_id: input.installation_id.clone(),
            request_id: "request-placeholder".to_owned(),
            purpose: input.purpose.clone(),
            provider_id: input.provider_id.clone(),
            signer_id: input.signer_id.clone(),
            grant_id: input.grant_id.clone(),
            context: input.context.clone(),
        },
        payload: vec![1],
    }
    .ipc_request();
    ipc.validate()?;
    Ok(())
}

fn validate_job_request(request: &JobRequest) -> Result<(), SignerError> {
    if request.schema_version != JOB_SCHEMA {
        return Err(SignerError::RecoveryRequired);
    }
    validate_id(&request.request_id).map_err(|_| SignerError::RecoveryRequired)?;
    Ok(())
}

fn request_matches(input: &JobInput, request: &JobRequest) -> bool {
    request.schema_version == JOB_SCHEMA
        && request.installation_id == input.installation_id
        && request.purpose == input.purpose
        && request.provider_id == input.provider_id
        && request.signer_id == input.signer_id
        && request.grant_id == input.grant_id
        && request.context == input.context
}

fn new_request_id() -> Result<String, SignerError> {
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
    Ok(hex::encode(random))
}

pub fn verify_prepared_job_payload(
    prepared: &PreparedJob,
    current_payload: &[u8],
) -> Result<(), SignerError> {
    if prepared.payload != current_payload {
        return Err(SignerError::IdConflict);
    }
    Ok(())
}

pub fn job_request_matches_input(input: &JobInput, request: &JobRequest) -> bool {
    request_matches(input, request)
}

pub fn cleanup_incomplete_job_binding(work_dir: &Path) -> Result<(), SignerError> {
    let request_path = work_dir.join("request.json");
    let payload_binding_path = work_dir.join("request.payload.sha256");
    if request_path.exists() != payload_binding_path.exists() {
        return Err(SignerError::RecoveryRequired);
    }
    Ok(())
}

pub fn request_json_path(work_dir: &Path) -> PathBuf {
    work_dir.join("request.json")
}

pub fn read_existing_request(work_dir: &Path) -> Result<JobRequest, SignerError> {
    let bytes = read_regular(&work_dir.join("request.json"), MAX_METADATA_BYTES)?;
    let request: JobRequest =
        serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
    validate_job_request(&request)?;
    Ok(request)
}

pub fn work_dir_exists(work_dir: &Path) -> bool {
    fs::symlink_metadata(work_dir)
        .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn validate_caller_work_root(path: &Path, caller_uid: u32) -> Result<(), SignerError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| SignerError::InstallationDrift)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != caller_uid
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(SignerError::InstallationDrift);
        }
    }
    #[cfg(not(unix))]
    return Err(SignerError::UnsupportedPlatformOrFs);
    Ok(())
}
