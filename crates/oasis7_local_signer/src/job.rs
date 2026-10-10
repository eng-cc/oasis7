use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::SignerError;
use crate::identity::sha256_hex;
use crate::local_fs::{Directory, read_regular, reject_symlink_components};
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
    Ok(Path::new(&caller.work_dir).join(job_id))
}

pub struct JobDirectory {
    directory: Directory,
    path: PathBuf,
}

impl JobDirectory {
    pub fn open(
        installation: &InstallationConfig,
        caller_uid: u32,
        job_id: &str,
        create: bool,
    ) -> Result<Self, SignerError> {
        let path = resolve_work_dir(installation, caller_uid, job_id)?;
        let caller = installation
            .caller(caller_uid)
            .ok_or(SignerError::AuthorizationDenied)?;
        let root = Directory::open(Path::new(&caller.work_dir), false)?;
        let metadata = root.metadata()?;
        validate_caller_metadata(&metadata, caller_uid)?;
        if metadata.dev() != caller.work_device_id || metadata.ino() != caller.work_inode {
            return Err(SignerError::InstallationDrift);
        }
        let directory = root.child(job_id, create)?;
        validate_caller_metadata(&directory.metadata()?, caller_uid)?;
        Ok(Self { directory, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn present(&self, name: &str) -> Result<bool, SignerError> {
        self.directory.exists(name)
    }
    pub fn prepare(&self) -> Result<PreparedJob, SignerError> {
        prepare_open_job(&self.directory)
    }
    pub fn read_request(&self) -> Result<JobRequest, SignerError> {
        let bytes = self.directory.read("request.json", MAX_METADATA_BYTES)?;
        let request: JobRequest =
            serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
        validate_job_request(&request)?;
        Ok(request)
    }
}

pub fn prepare_job_dir(
    installation: &InstallationConfig,
    caller_uid: u32,
    job_id: &str,
) -> Result<PathBuf, SignerError> {
    if crate::installation::current_uid() != caller_uid {
        return Err(SignerError::AuthorizationDenied);
    }
    Ok(JobDirectory::open(installation, caller_uid, job_id, true)?.path)
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
    Ok(JobDirectory::open(installation, caller_uid, job_id, false)?.path)
}

pub fn prepare_job(work_dir: &Path) -> Result<PreparedJob, SignerError> {
    reject_symlink_components(work_dir)?;
    prepare_open_job(&Directory::open(work_dir, false)?)
}

fn prepare_open_job(directory: &Directory) -> Result<PreparedJob, SignerError> {
    let input_bytes = directory.read("input.json", MAX_METADATA_BYTES)?;
    let payload = directory.read("payload.bin", MAX_ROLLBACK_PAYLOAD_BYTES)?;
    let input: JobInput = serde_json::from_slice(&input_bytes)
        .map_err(|_| SignerError::InvalidInput("job input schema is invalid".to_owned()))?;
    validate_job_input(&input)?;
    let payload_hash = sha256_hex(&payload);
    let request = match (
        directory.exists("request.json")?,
        directory.exists("request.payload.sha256")?,
    ) {
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
            directory.write_new("request.payload.sha256", payload_hash.as_bytes())?;
            directory.write_new("request.json", &request_bytes)?;
            directory.sync()?;
            request
        }
        (true, true) => {
            let request_bytes = directory.read("request.json", MAX_METADATA_BYTES)?;
            let request: JobRequest = serde_json::from_slice(&request_bytes)
                .map_err(|_| SignerError::RecoveryRequired)?;
            validate_job_request(&request)?;
            let previous_payload_hash = directory.read("request.payload.sha256", 64)?;
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
    if !matches!(
        input.purpose.as_str(),
        "rollback_strict_audit_v1" | "file_ed25519_v1"
    ) || input.provider_id.0.is_some()
        || input.grant_id.0.is_none()
    {
        return Err(SignerError::InvalidInput(
            "job must bind a supported purpose, null provider, and a grant".to_owned(),
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

fn validate_caller_metadata(metadata: &fs::Metadata, caller_uid: u32) -> Result<(), SignerError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != caller_uid
            || metadata.permissions().mode() & 0o7777 != 0o700
        {
            return Err(SignerError::InstallationDrift);
        }
    }
    #[cfg(not(unix))]
    return Err(SignerError::UnsupportedPlatformOrFs);
    Ok(())
}
