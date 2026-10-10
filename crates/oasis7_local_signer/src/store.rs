#[cfg(test)]
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::authorization::{AuthorizedSign, authorize_sign};
use crate::control::{MAINTENANCE_FILE, MAINTENANCE_SCHEMA, MaintenanceRecord};
use crate::error::SignerError;
use crate::identity::{canonical_json, sha256_hex};
use crate::installation::{current_euid, load_fixed_installation_config, trusted_sudo_caller};
use crate::local_fs::{
    CustodyLock, create_private_dir, read_regular, reject_symlink_components, sync_dir,
    write_private_new,
};
use crate::protocol::{
    DoctorResponse, ExplicitNull, InspectResponse, IpcRequest, IpcResponse, RESULT_SCHEMA,
    SignResult, validate_id,
};
use crate::types::{BatchGrant, InstallationConfig, Policy};

pub(crate) const POLICY_FILE: &str = "control/policy.json";
pub(crate) const GRANTS_DIR: &str = "control/grants";
pub(crate) const REVOKED_GRANTS_DIR: &str = "control/revoked-grants";
pub(crate) const RECORDS_DIR: &str = "state/records";
pub(crate) const LOCK_FILE: &str = "state/custody.lock";
const MAX_CONTROL_BYTES: usize = 1024 * 1024;
const MAX_RESULT_FILE_BYTES: usize = 1024 * 1024;
const MAX_RECORDS_TO_SCAN: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreFaultPoint {
    AfterReservationSync,
    AfterResultStagingSync,
    BeforeResultRename,
    AfterResultRename,
    BeforeReplayParentSync,
}

#[derive(Debug)]
pub struct SignerStore {
    pub(crate) installation: InstallationConfig,
    pub(crate) root: PathBuf,
    pub(crate) caller_uid: u32,
    pub(crate) signer_uid: u32,
    pub(crate) control_owner_uid: u32,
    pub(crate) control_group_gid: u32,
    #[cfg(test)]
    test_fault: Cell<Option<StoreFaultPoint>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reservation {
    schema_version: String,
    installation_id: String,
    deployment_id: String,
    release_id: String,
    worker_sha256: String,
    policy_revision: String,
    caller_uid: u32,
    signer_uid: u32,
    grant_id: String,
    operation_key: String,
    request_key: String,
    request_id: String,
    fingerprint: String,
    purpose: String,
    signer_id: String,
    provider_id: ExplicitNull<String>,
    public_key_sha256: String,
    payload_sha256: String,
    context: crate::protocol::SignContext,
    not_before_ms: u64,
    expires_at_ms: u64,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditRecord {
    schema_version: String,
    recorded_at: u64,
    caller_uid: u32,
    signer_uid: u32,
    installation_id: String,
    deployment_id: String,
    release_id: String,
    worker_sha256: String,
    policy_revision: String,
    grant_id: String,
    operation_key: String,
    request_key: String,
    request_id: String,
    purpose: String,
    signer_id: String,
    public_key_sha256: String,
    payload_sha256: String,
    response_sha256: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultManifest {
    schema_version: String,
    request_key: String,
    fingerprint: String,
    response_sha256: String,
    audit_sha256: String,
}

impl SignerStore {
    pub fn open_worker() -> Result<Self, SignerError> {
        let installation = load_fixed_installation_config()?;
        let caller_uid = trusted_sudo_caller(&installation)?;
        let signer_uid = current_euid();
        let executable = std::env::current_exe().map_err(SignerError::PersistenceFailed)?;
        if executable != Path::new(&installation.worker_executable) {
            return Err(SignerError::InstallationDrift);
        }
        let root = PathBuf::from(&installation.store_dir);
        validate_store_layout(&root, installation.signer_uid, installation.signer_gid)?;
        let control_group_gid = installation.signer_gid;
        Ok(Self {
            installation,
            root,
            caller_uid,
            signer_uid,
            control_owner_uid: 0,
            control_group_gid,
            #[cfg(test)]
            test_fault: Cell::new(None),
        })
    }

    pub fn handle_request(&self, request: IpcRequest) -> Result<IpcResponse, SignerError> {
        self.handle_request_at(request, unix_time_ms()?)
    }

    fn handle_request_at(
        &self,
        request: IpcRequest,
        now_ms: u64,
    ) -> Result<IpcResponse, SignerError> {
        request.validate()?;
        let installation_id = match &request {
            IpcRequest::Doctor {
                installation_id, ..
            }
            | IpcRequest::Inspect {
                installation_id, ..
            }
            | IpcRequest::Sign {
                installation_id, ..
            } => installation_id,
        };
        if installation_id != &self.installation.installation_id
            || self.installation.caller(self.caller_uid).is_none()
        {
            return Err(SignerError::AuthorizationDenied);
        }
        match request {
            IpcRequest::Doctor { .. } => self.doctor(),
            IpcRequest::Inspect {
                request_id,
                purpose,
                provider_id,
                ..
            } => self.inspect(&request_id, &purpose, provider_id.0.as_deref(), now_ms),
            request @ IpcRequest::Sign { .. } => self.sign(request, now_ms),
        }
    }

    fn doctor(&self) -> Result<IpcResponse, SignerError> {
        let policy_path = self.root.join(POLICY_FILE);
        let (ready, status_code) = match self.check_maintenance() {
            Err(error) => (false, error.code()),
            Ok(()) => match read_control::<Policy>(
                &policy_path,
                self.control_owner_uid,
                self.control_group_gid,
            ) {
                Ok(policy) if policy.validate(&self.installation).is_ok() => {
                    let ready = !policy.enabled_purposes.is_empty();
                    (ready, if ready { "READY" } else { "DISABLED" })
                }
                Ok(_) => (false, "INSTALLATION_DRIFT"),
                Err(error) => (false, error.code()),
            },
        };
        Ok(IpcResponse::Doctor(DoctorResponse {
            schema_version: "oasis7.local_signer_doctor.v1".to_owned(),
            installation_id: self.installation.installation_id.clone(),
            ready,
            status_code: status_code.to_owned(),
        }))
    }

    fn inspect(
        &self,
        request_id: &str,
        purpose: &str,
        provider_id: Option<&str>,
        now_ms: u64,
    ) -> Result<IpcResponse, SignerError> {
        validate_id(request_id).map_err(|error| SignerError::InvalidInput(error.to_string()))?;
        validate_id(purpose).map_err(|error| SignerError::InvalidInput(error.to_string()))?;
        let _lock = self.acquire_lock()?;
        self.check_maintenance()?;
        let request_key = crate::identity::request_key(
            &self.installation.installation_id,
            purpose,
            self.caller_uid,
            provider_id,
            request_id,
        )?;
        let record_dir = Path::new(RECORDS_DIR);
        let record_path = self.root.join(record_dir).join(&request_key);
        if !path_exists(&record_path)? {
            return Ok(IpcResponse::Inspect(InspectResponse {
                schema_version: "oasis7.local_signer_inspect.v1".to_owned(),
                request_id: request_id.to_owned(),
                operation_key: ExplicitNull(None),
                status: "missing".to_owned(),
                payload_sha256: ExplicitNull(None),
                audit_digest: ExplicitNull(None),
                error_code: ExplicitNull(None),
            }));
        }
        let reservation: Reservation = read_json(&record_path.join("reservation.json"))?;
        if reservation.caller_uid != self.caller_uid
            || reservation.request_id != request_id
            || reservation.purpose != purpose
            || reservation.request_key != request_key
        {
            return Err(SignerError::IdConflict);
        }
        self.recheck_reservation_authorization(&reservation, now_ms)?;
        let result_dir = record_path.join("result");
        let (status, audit_digest) = if path_exists(&result_dir)? {
            let (_, audit, _) = read_committed_result(&record_path, &reservation)?;
            ("committed", Some(audit))
        } else {
            ("reserved", None)
        };
        Ok(IpcResponse::Inspect(InspectResponse {
            schema_version: "oasis7.local_signer_inspect.v1".to_owned(),
            request_id: request_id.to_owned(),
            operation_key: ExplicitNull(Some(reservation.operation_key)),
            status: status.to_owned(),
            payload_sha256: ExplicitNull(Some(reservation.payload_sha256)),
            audit_digest: ExplicitNull(audit_digest),
            error_code: ExplicitNull(None),
        }))
    }

    fn sign(&self, request: IpcRequest, now_ms: u64) -> Result<IpcResponse, SignerError> {
        self.check_maintenance()?;
        let _lock = self.acquire_lock()?;
        self.check_maintenance()?;
        let authorized = self.authorize(&request, now_ms)?;
        let record_dir = self.root.join(RECORDS_DIR).join(&authorized.request_key);
        let reservation = if path_exists(&record_dir)? {
            if !record_dir.is_dir() {
                return Err(SignerError::RecoveryRequired);
            }
            if has_result_staging(&record_dir)? {
                return Err(SignerError::RecoveryRequired);
            }
            let existing: Reservation = read_json(&record_dir.join("reservation.json"))?;
            if existing.fingerprint != authorized.fingerprint
                || existing.request_key != authorized.request_key
                || existing.operation_key != authorized.operation_key
                || existing.grant_id != authorized.grant_id
            {
                return Err(SignerError::IdConflict);
            }
            if path_exists(&record_dir.join("result"))? {
                let (response, _, _) = read_committed_result(&record_dir, &existing)?;
                verify_response_signature(
                    &response,
                    &authorized.public_key_sha256,
                    &authorized.payload,
                )?;
                self.maybe_inject_fault(StoreFaultPoint::BeforeReplayParentSync)?;
                sync_dir(&record_dir)?;
                return Ok(IpcResponse::Sign(response));
            }
            existing
        } else {
            self.check_budget(&authorized)?;
            create_private_dir(&record_dir)?;
            sync_dir(&self.root.join(RECORDS_DIR))?;
            let reservation = self.make_reservation(&authorized);
            let bytes = canonical_json(&reservation)?;
            write_private_new(&record_dir.join("reservation.json"), &bytes)?;
            sync_dir(&record_dir)?;
            self.maybe_inject_fault(StoreFaultPoint::AfterReservationSync)?;
            reservation
        };

        if reservation.status != "reserved" {
            return Err(SignerError::RecoveryRequired);
        }
        let (signing_key, public_key) = self.read_signing_key(&authorized)?;
        let signature = signing_key.sign(&authorized.payload);
        public_key
            .verify(&authorized.payload, &signature)
            .map_err(|_| SignerError::CryptoOrBindingInvalid)?;

        let response_without_audit = make_sign_result(
            &authorized,
            &public_key.to_bytes(),
            &signature.to_bytes(),
            None,
        );
        let response_sha256 = sha256_hex(&canonical_json(&response_without_audit)?);
        let audit = self.make_audit(&authorized, response_sha256, unix_time_ms()?);
        let audit_bytes = canonical_json(&audit)?;
        let audit_digest = sha256_hex(&audit_bytes);
        let response = make_sign_result(
            &authorized,
            &public_key.to_bytes(),
            &signature.to_bytes(),
            Some(audit_digest.clone()),
        );
        let response_bytes = canonical_json(&response)?;
        let manifest = ResultManifest {
            schema_version: "oasis7.local_signer_result_manifest.v1".to_owned(),
            request_key: authorized.request_key.clone(),
            fingerprint: authorized.fingerprint.clone(),
            response_sha256: sha256_hex(&response_bytes),
            audit_sha256: audit_digest,
        };
        self.recheck_authorization_before_commit(&request, now_ms)?;
        self.commit_result(&record_dir, &response_bytes, &audit_bytes, &manifest)?;
        let committed = read_committed_result(&record_dir, &reservation)?.0;
        verify_response_signature(
            &committed,
            &authorized.public_key_sha256,
            &authorized.payload,
        )?;
        Ok(IpcResponse::Sign(committed))
    }

    fn authorize(&self, request: &IpcRequest, now_ms: u64) -> Result<AuthorizedSign, SignerError> {
        let IpcRequest::Sign {
            grant_id,
            signer_id,
            ..
        } = request
        else {
            return Err(SignerError::InvalidInput(
                "expected sign command".to_owned(),
            ));
        };
        let grant_id = grant_id
            .0
            .as_deref()
            .ok_or(SignerError::AuthorizationDenied)?;
        validate_id(grant_id).map_err(|_| SignerError::AuthorizationDenied)?;
        if path_exists(
            &self
                .root
                .join(REVOKED_GRANTS_DIR)
                .join(format!("{grant_id}.json")),
        )? {
            return Err(SignerError::AuthorizationDenied);
        }
        let policy: Policy = read_control(
            &self.root.join(POLICY_FILE),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        let grant: BatchGrant = read_control(
            &self.root.join(GRANTS_DIR).join(format!("{grant_id}.json")),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        let authorized = authorize_sign(
            &self.installation,
            &policy,
            &grant,
            self.caller_uid,
            request,
            now_ms,
        )?;
        if authorized.signer_id != *signer_id {
            return Err(SignerError::AuthorizationDenied);
        }
        crate::key_management::require_active_key(
            &self.root,
            self.control_owner_uid,
            self.control_group_gid,
            &self.installation.installation_id,
            &authorized.signer_id,
            &authorized.purpose,
        )?;
        Ok(authorized)
    }

    fn recheck_reservation_authorization(
        &self,
        reservation: &Reservation,
        now_ms: u64,
    ) -> Result<(), SignerError> {
        crate::key_management::require_active_key(
            &self.root,
            self.control_owner_uid,
            self.control_group_gid,
            &self.installation.installation_id,
            &reservation.signer_id,
            &reservation.purpose,
        )?;
        let grant_id = &reservation.grant_id;
        if path_exists(
            &self
                .root
                .join(REVOKED_GRANTS_DIR)
                .join(format!("{grant_id}.json")),
        )? {
            return Err(SignerError::AuthorizationDenied);
        }
        let policy: Policy = read_control(
            &self.root.join(POLICY_FILE),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        let grant: BatchGrant = read_control(
            &self.root.join(GRANTS_DIR).join(format!("{grant_id}.json")),
            self.control_owner_uid,
            self.control_group_gid,
        )?;
        policy
            .validate(&self.installation)
            .map_err(|_| SignerError::AuthorizationDenied)?;
        grant
            .validate(&self.installation, &policy)
            .map_err(|_| SignerError::AuthorizationDenied)?;
        let key = policy
            .key_binding(&reservation.purpose, &reservation.signer_id)
            .ok_or(SignerError::AuthorizationDenied)?;
        let mut items = grant.items.iter().filter(|item| {
            item.operation_key == reservation.operation_key
                && item.payload_sha256 == reservation.payload_sha256
                && item.public_key_sha256 == reservation.public_key_sha256
                && item.protocol_bindings == reservation.context.protocol_context
        });
        let item = items.next().ok_or(SignerError::AuthorizationDenied)?;
        if items.next().is_some()
            || grant.grant_id != reservation.grant_id
            || grant.caller_uid != self.caller_uid
            || grant.policy_revision != policy.policy_revision
            || reservation.policy_revision != policy.policy_revision
            || !policy
                .enabled_purposes
                .iter()
                .any(|value| value == &reservation.purpose)
            || !key
                .protocol_authorities
                .contains(&reservation.context.protocol_context.authority_id)
            || !key.deployment_ids.contains(&reservation.deployment_id)
            || !key.network_ids.contains(&reservation.context.network_id)
            || now_ms < grant.not_before.max(reservation.not_before_ms)
            || now_ms >= grant.expires_at.min(reservation.expires_at_ms)
            || grant.task_uid != reservation.context.task_uid
            || grant.source_head_oid != reservation.context.source_head_oid
            || grant.network_id != reservation.context.network_id
        {
            return Err(SignerError::AuthorizationDenied);
        }
        if item.signer_id != reservation.signer_id || item.purpose != reservation.purpose {
            return Err(SignerError::AuthorizationDenied);
        }
        if item.provider_id != reservation.provider_id || reservation.provider_id.0.is_some() {
            return Err(SignerError::AuthorizationDenied);
        }
        Ok(())
    }

    fn recheck_authorization_before_commit(
        &self,
        request: &IpcRequest,
        initial_now_ms: u64,
    ) -> Result<(), SignerError> {
        let now_ms = unix_time_ms()?.max(initial_now_ms);
        self.authorize(request, now_ms).map(|_| ())
    }

    fn check_budget(&self, authorized: &AuthorizedSign) -> Result<(), SignerError> {
        let records_path = self.root.join(RECORDS_DIR);
        let mut count = 0_u32;
        for entry in fs::read_dir(&records_path)? {
            let entry = entry?;
            if count as usize >= MAX_RECORDS_TO_SCAN {
                return Err(SignerError::RecoveryRequired);
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(SignerError::RecoveryRequired);
            }
            let reservation: Reservation = read_json(&path.join("reservation.json"))?;
            if reservation.operation_key == authorized.operation_key {
                count = count.saturating_add(1);
            }
        }
        if count >= authorized.max_distinct_requests {
            return Err(SignerError::BudgetExhausted);
        }
        Ok(())
    }

    fn make_reservation(&self, authorized: &AuthorizedSign) -> Reservation {
        Reservation {
            schema_version: "oasis7.local_signer_reservation.v1".to_owned(),
            installation_id: authorized.installation_id.clone(),
            deployment_id: authorized.deployment_id.clone(),
            release_id: self.installation.release_id.clone(),
            worker_sha256: self.installation.worker_sha256.clone(),
            policy_revision: authorized.policy_revision.clone(),
            caller_uid: authorized.caller_uid,
            signer_uid: self.signer_uid,
            grant_id: authorized.grant_id.clone(),
            operation_key: authorized.operation_key.clone(),
            request_key: authorized.request_key.clone(),
            request_id: authorized.request_id.clone(),
            fingerprint: authorized.fingerprint.clone(),
            purpose: authorized.purpose.clone(),
            signer_id: authorized.signer_id.clone(),
            provider_id: ExplicitNull(None),
            public_key_sha256: authorized.public_key_sha256.clone(),
            payload_sha256: authorized.payload_sha256.clone(),
            context: authorized.context.clone(),
            not_before_ms: authorized.not_before_ms,
            expires_at_ms: authorized.expires_at_ms,
            status: "reserved".to_owned(),
        }
    }

    fn make_audit(
        &self,
        authorized: &AuthorizedSign,
        response_sha256: String,
        recorded_at: u64,
    ) -> AuditRecord {
        AuditRecord {
            schema_version: "oasis7.local_signer_audit.v1".to_owned(),
            recorded_at,
            caller_uid: authorized.caller_uid,
            signer_uid: self.signer_uid,
            installation_id: authorized.installation_id.clone(),
            deployment_id: authorized.deployment_id.clone(),
            release_id: self.installation.release_id.clone(),
            worker_sha256: self.installation.worker_sha256.clone(),
            policy_revision: authorized.policy_revision.clone(),
            grant_id: authorized.grant_id.clone(),
            operation_key: authorized.operation_key.clone(),
            request_key: authorized.request_key.clone(),
            request_id: authorized.request_id.clone(),
            purpose: authorized.purpose.clone(),
            signer_id: authorized.signer_id.clone(),
            public_key_sha256: authorized.public_key_sha256.clone(),
            payload_sha256: authorized.payload_sha256.clone(),
            response_sha256,
            status: "committed".to_owned(),
        }
    }

    fn read_signing_key(
        &self,
        authorized: &AuthorizedSign,
    ) -> Result<(SigningKey, VerifyingKey), SignerError> {
        crate::key_management::require_active_key(
            &self.root,
            self.control_owner_uid,
            self.control_group_gid,
            &self.installation.installation_id,
            &authorized.signer_id,
            &authorized.purpose,
        )?;
        let key_dir = self.root.join("keys").join(&authorized.signer_id);
        reject_symlink_components(&key_dir)?;
        validate_owned_directory(
            &key_dir,
            self.signer_uid,
            self.installation.signer_gid,
            0o700,
        )
        .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
        let seed_bytes = Zeroizing::new(read_owned_key_file(
            &key_dir.join("seed.bin"),
            self.signer_uid,
            self.installation.signer_gid,
        )?);
        let seed: Zeroizing<[u8; 32]> = Zeroizing::new(
            seed_bytes
                .as_slice()
                .try_into()
                .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?,
        );
        let public_bytes = read_owned_key_file(
            &key_dir.join("public.bin"),
            self.signer_uid,
            self.installation.signer_gid,
        )?;
        let public_raw: [u8; 32] = public_bytes
            .as_slice()
            .try_into()
            .map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
        if sha256_hex(&public_raw) != authorized.public_key_sha256 {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        if verifying_key.to_bytes() != public_raw {
            return Err(SignerError::CryptoOrBindingInvalid);
        }
        Ok((signing_key, verifying_key))
    }

    fn commit_result(
        &self,
        record_dir: &Path,
        response_bytes: &[u8],
        audit_bytes: &[u8],
        manifest: &ResultManifest,
    ) -> Result<(), SignerError> {
        let staging = record_dir.join(format!(
            ".result-staging-{}-{}",
            std::process::id(),
            sha256_hex(response_bytes)[..12].to_owned()
        ));
        create_private_dir(&staging)?;
        write_private_new(&staging.join("response.json"), response_bytes)?;
        write_private_new(&staging.join("audit.json"), audit_bytes)?;
        let manifest_bytes = canonical_json(manifest)?;
        write_private_new(&staging.join("manifest.json"), &manifest_bytes)?;
        sync_dir(&staging)?;
        self.maybe_inject_fault(StoreFaultPoint::AfterResultStagingSync)?;
        let result = record_dir.join("result");
        if path_exists(&result)? {
            return Err(SignerError::RecoveryRequired);
        }
        self.maybe_inject_fault(StoreFaultPoint::BeforeResultRename)?;
        fs::rename(&staging, &result)?;
        self.maybe_inject_fault(StoreFaultPoint::AfterResultRename)?;
        sync_dir(record_dir)
    }

    fn maybe_inject_fault(&self, point: StoreFaultPoint) -> Result<(), SignerError> {
        #[cfg(test)]
        if self.test_fault.get() == Some(point) {
            self.test_fault.set(None);
            return Err(SignerError::PersistenceFailed(std::io::Error::other(
                "injected persistence fault",
            )));
        }

        #[cfg(not(test))]
        let _ = point;

        Ok(())
    }

    fn acquire_lock(&self) -> Result<CustodyLock, SignerError> {
        CustodyLock::acquire(
            &self.root.join(LOCK_FILE),
            self.signer_uid,
            self.installation.signer_gid,
        )
    }

    fn check_maintenance(&self) -> Result<(), SignerError> {
        let path = self.root.join(MAINTENANCE_FILE);
        if !path_exists(&path)? {
            return Ok(());
        }
        let record: MaintenanceRecord =
            read_control(&path, self.control_owner_uid, self.control_group_gid)?;
        if record.schema_version != MAINTENANCE_SCHEMA
            || record.installation_id != self.installation.installation_id
            || record.phase != "committed"
            || crate::identity::parse_sha256_hex(&record.operation_key).is_err()
            || crate::identity::parse_sha256_hex(&record.target_sha256).is_err()
            || record
                .base_sha256
                .as_ref()
                .is_some_and(|digest| crate::identity::parse_sha256_hex(digest).is_err())
            || validate_id(&record.transaction_id).is_err()
        {
            return Err(SignerError::RecoveryRequired);
        }
        let target = self
            .maintenance_target_path(&record.target_path)
            .ok_or(SignerError::RecoveryRequired)?;
        if record.target_path.starts_with("keys/") {
            let signer_id = Path::new(&record.target_path)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(SignerError::RecoveryRequired)?;
            validate_id(signer_id).map_err(|_| SignerError::RecoveryRequired)?;
            validate_owned_directory(
                &target,
                self.signer_uid,
                self.installation.signer_gid,
                0o700,
            )
            .map_err(|_| SignerError::RecoveryRequired)?;
            let public_key = read_owned_key_file(
                &target.join("public.bin"),
                self.signer_uid,
                self.installation.signer_gid,
            )
            .map_err(|_| SignerError::RecoveryRequired)?;
            if sha256_hex(&public_key) != record.target_sha256 {
                return Err(SignerError::RecoveryRequired);
            }
        } else {
            let bytes = read_control_bytes(&target, self.control_owner_uid, self.control_group_gid)
                .map_err(|_| SignerError::RecoveryRequired)?;
            if sha256_hex(&bytes) != record.target_sha256 {
                return Err(SignerError::RecoveryRequired);
            }
        }
        Ok(())
    }

    fn maintenance_target_path(&self, relative: &str) -> Option<PathBuf> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return None;
        }
        let components: Vec<_> = path
            .components()
            .filter_map(|component| match component {
                std::path::Component::Normal(value) => value.to_str(),
                _ => None,
            })
            .collect();
        let permitted = match components.as_slice() {
            [
                "control",
                "policy.json" | "key-catalog.json" | "restore.json",
            ] => true,
            ["control", "grants" | "revoked-grants", file] => file
                .strip_suffix(".json")
                .is_some_and(|id| validate_id(id).is_ok()),
            ["keys", signer_id] => validate_id(signer_id).is_ok(),
            _ => false,
        };
        permitted.then(|| self.root.join(path))
    }
}

fn make_sign_result(
    authorized: &AuthorizedSign,
    public_key: &[u8; 32],
    signature: &[u8; 64],
    audit_digest: Option<String>,
) -> SignResult {
    SignResult {
        schema_version: RESULT_SCHEMA.to_owned(),
        request_id: authorized.request_id.clone(),
        operation_key: ExplicitNull(Some(authorized.operation_key.clone())),
        status: "committed".to_owned(),
        public_key_base64: ExplicitNull(Some(
            base64::engine::general_purpose::STANDARD.encode(public_key),
        )),
        signature_base64: ExplicitNull(Some(
            base64::engine::general_purpose::STANDARD.encode(signature),
        )),
        provider_attestation: ExplicitNull(None),
        audit_digest: ExplicitNull(audit_digest),
        error_code: ExplicitNull(None),
    }
}

fn verify_response_signature(
    response: &SignResult,
    expected_public_key_sha256: &str,
    payload: &[u8],
) -> Result<(), SignerError> {
    if response.status != "committed" || response.error_code.0.is_some() {
        return Err(SignerError::RecoveryRequired);
    }
    let public_bytes = response
        .public_key_base64
        .0
        .as_deref()
        .ok_or(SignerError::RecoveryRequired)?;
    let signature_bytes = response
        .signature_base64
        .0
        .as_deref()
        .ok_or(SignerError::RecoveryRequired)?;
    let public_raw = base64::engine::general_purpose::STANDARD
        .decode(public_bytes)
        .map_err(|_| SignerError::RecoveryRequired)?;
    let signature_raw = base64::engine::general_purpose::STANDARD
        .decode(signature_bytes)
        .map_err(|_| SignerError::RecoveryRequired)?;
    let public: [u8; 32] = public_raw
        .as_slice()
        .try_into()
        .map_err(|_| SignerError::RecoveryRequired)?;
    if sha256_hex(&public) != expected_public_key_sha256 {
        return Err(SignerError::RecoveryRequired);
    }
    let signature =
        Signature::from_slice(&signature_raw).map_err(|_| SignerError::RecoveryRequired)?;
    let verifying = VerifyingKey::from_bytes(&public).map_err(|_| SignerError::RecoveryRequired)?;
    verifying
        .verify(payload, &signature)
        .map_err(|_| SignerError::RecoveryRequired)
}

fn read_committed_result(
    record_dir: &Path,
    reservation: &Reservation,
) -> Result<(SignResult, String, String), SignerError> {
    let result_dir = record_dir.join("result");
    reject_symlink_components(&result_dir)?;
    let response_bytes = read_regular(&result_dir.join("response.json"), MAX_RESULT_FILE_BYTES)?;
    let audit_bytes = read_regular(&result_dir.join("audit.json"), MAX_RESULT_FILE_BYTES)?;
    let manifest_bytes = read_regular(&result_dir.join("manifest.json"), MAX_RESULT_FILE_BYTES)?;
    validate_committed_bytes(reservation, &response_bytes, &audit_bytes, &manifest_bytes)
}

fn validate_committed_bytes(
    reservation: &Reservation,
    response_bytes: &[u8],
    audit_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<(SignResult, String, String), SignerError> {
    let response: SignResult =
        serde_json::from_slice(&response_bytes).map_err(|_| SignerError::RecoveryRequired)?;
    let audit: AuditRecord =
        serde_json::from_slice(&audit_bytes).map_err(|_| SignerError::RecoveryRequired)?;
    let manifest: ResultManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| SignerError::RecoveryRequired)?;
    if canonical_json(&response)? != response_bytes
        || canonical_json(&audit)? != audit_bytes
        || canonical_json(&manifest)? != manifest_bytes
    {
        return Err(SignerError::RecoveryRequired);
    }
    let response_sha = sha256_hex(&response_bytes);
    let audit_sha = sha256_hex(&audit_bytes);
    if manifest.schema_version != "oasis7.local_signer_result_manifest.v1"
        || manifest.request_key != reservation.request_key
        || manifest.fingerprint != reservation.fingerprint
        || manifest.response_sha256 != response_sha
        || manifest.audit_sha256 != audit_sha
        || audit.schema_version != "oasis7.local_signer_audit.v1"
        || audit.recorded_at == 0
        || audit.status != "committed"
        || audit.caller_uid != reservation.caller_uid
        || audit.signer_uid != reservation.signer_uid
        || audit.installation_id != reservation.installation_id
        || audit.deployment_id != reservation.deployment_id
        || audit.release_id != reservation.release_id
        || audit.worker_sha256 != reservation.worker_sha256
        || audit.policy_revision != reservation.policy_revision
        || audit.grant_id != reservation.grant_id
        || audit.request_key != reservation.request_key
        || audit.request_id != reservation.request_id
        || audit.operation_key != reservation.operation_key
        || audit.purpose != reservation.purpose
        || audit.signer_id != reservation.signer_id
        || audit.public_key_sha256 != reservation.public_key_sha256
        || audit.payload_sha256 != reservation.payload_sha256
        || audit.response_sha256 != audit_response_hash(&response)?
        || response.schema_version != RESULT_SCHEMA
        || response.request_id != reservation.request_id
        || response.operation_key.0.as_deref() != Some(reservation.operation_key.as_str())
        || response.audit_digest.0.as_deref() != Some(audit_sha.as_str())
        || response.status != "committed"
    {
        return Err(SignerError::RecoveryRequired);
    }
    Ok((response, audit_sha, response_sha))
}

pub(crate) fn validate_snapshot_record(
    request_key: &str,
    reservation_bytes: &[u8],
    result: Option<(&[u8], &[u8], &[u8])>,
    installation: &InstallationConfig,
    grants: &std::collections::BTreeMap<String, BatchGrant>,
) -> Result<(), SignerError> {
    let r: Reservation =
        serde_json::from_slice(reservation_bytes).map_err(|_| SignerError::RecoveryRequired)?;
    if canonical_json(&r)? != reservation_bytes
        || r.schema_version != "oasis7.local_signer_reservation.v1"
        || r.installation_id != installation.installation_id
        || r.deployment_id != installation.deployment_id
        || r.signer_uid != installation.signer_uid
        || installation.caller(r.caller_uid).is_none()
        || r.request_key != request_key
        || r.status != "reserved"
        || r.not_before_ms >= r.expires_at_ms
        || !crate::key_management::supported_purpose(&r.purpose)
    {
        return Err(SignerError::RecoveryRequired);
    }
    for id in [
        &r.release_id,
        &r.policy_revision,
        &r.grant_id,
        &r.request_id,
        &r.signer_id,
    ] {
        validate_id(id).map_err(|_| SignerError::RecoveryRequired)?;
    }
    for hash in [
        &r.worker_sha256,
        &r.operation_key,
        &r.request_key,
        &r.fingerprint,
        &r.public_key_sha256,
        &r.payload_sha256,
    ] {
        crate::types::validate_sha256(hash).map_err(|_| SignerError::RecoveryRequired)?;
    }
    if crate::identity::request_key(
        &r.installation_id,
        &r.purpose,
        r.caller_uid,
        r.provider_id.0.as_deref(),
        &r.request_id,
    )? != r.request_key
    {
        return Err(SignerError::RecoveryRequired);
    }
    let g = grants
        .get(&r.grant_id)
        .ok_or(SignerError::RecoveryRequired)?;
    if g.caller_uid != r.caller_uid
        || g.policy_revision != r.policy_revision
        || g.not_before > r.not_before_ms
        || g.expires_at < r.expires_at_ms
        || g.network_id != r.context.network_id
        || g.task_uid != r.context.task_uid
        || g.source_head_oid != r.context.source_head_oid
        || r.context.deployment_id != r.deployment_id
        || !g.items.iter().any(|item| {
            item.operation_key == r.operation_key
                && item.payload_sha256 == r.payload_sha256
                && item.purpose == r.purpose
                && item.signer_id == r.signer_id
                && item.public_key_sha256 == r.public_key_sha256
                && item.provider_id == r.provider_id
                && item.protocol_bindings == r.context.protocol_context
        })
    {
        return Err(SignerError::RecoveryRequired);
    }
    if let Some((response, audit, manifest)) = result {
        let (response, _, _) = validate_committed_bytes(&r, response, audit, manifest)?;
        let public = base64::engine::general_purpose::STANDARD
            .decode(
                response
                    .public_key_base64
                    .0
                    .as_deref()
                    .ok_or(SignerError::RecoveryRequired)?,
            )
            .map_err(|_| SignerError::RecoveryRequired)?;
        if public.len() != 32
            || sha256_hex(&public) != r.public_key_sha256
            || response
                .signature_base64
                .0
                .as_deref()
                .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                .is_none_or(|b| b.len() != 64)
        {
            return Err(SignerError::RecoveryRequired);
        }
    }
    Ok(())
}

fn audit_response_hash(response: &SignResult) -> Result<String, SignerError> {
    let mut unsigned = response.clone();
    unsigned.audit_digest = ExplicitNull(None);
    Ok(sha256_hex(&canonical_json(&unsigned)?))
}

pub(crate) fn read_control<T: for<'de> Deserialize<'de> + Serialize>(
    path: &Path,
    owner_uid: u32,
    owner_gid: u32,
) -> Result<T, SignerError> {
    let bytes = read_control_bytes(path, owner_uid, owner_gid)?;
    let value: T = serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
    if canonical_json(&value)? != bytes {
        return Err(SignerError::RecoveryRequired);
    }
    Ok(value)
}

pub(crate) fn read_control_bytes(
    path: &Path,
    owner_uid: u32,
    owner_gid: u32,
) -> Result<Vec<u8>, SignerError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            SignerError::AuthorizationDenied
        } else {
            SignerError::PersistenceFailed(error)
        }
    })?;
    if !metadata.is_file()
        || metadata.uid() != owner_uid
        || metadata.gid() != owner_gid
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o777 != 0o640
    {
        return Err(SignerError::InstallationDrift);
    }
    read_regular(path, MAX_CONTROL_BYTES).map_err(|error| match error {
        SignerError::PersistenceFailed(ref io_error)
            if io_error.kind() == std::io::ErrorKind::NotFound =>
        {
            SignerError::AuthorizationDenied
        }
        other => other,
    })
}

fn read_json<T: for<'de> Deserialize<'de> + Serialize>(path: &Path) -> Result<T, SignerError> {
    let bytes = read_regular(path, MAX_RESULT_FILE_BYTES)?;
    let value: T = serde_json::from_slice(&bytes).map_err(|_| SignerError::RecoveryRequired)?;
    if canonical_json(&value)? != bytes {
        return Err(SignerError::RecoveryRequired);
    }
    Ok(value)
}

fn path_exists(path: &Path) -> Result<bool, SignerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SignerError::RecoveryRequired),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(SignerError::PersistenceFailed(error)),
    }
}

fn has_result_staging(record_dir: &Path) -> Result<bool, SignerError> {
    for entry in fs::read_dir(record_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(".result-staging-") {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn validate_owned_directory(
    path: &Path,
    owner_uid: u32,
    owner_gid: u32,
    mode: u32,
) -> Result<(), SignerError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
        || metadata.gid() != owner_gid
        || metadata.permissions().mode() & 0o7777 != mode
    {
        return Err(SignerError::InstallationDrift);
    }
    Ok(())
}

pub(crate) fn read_owned_key_file(
    path: &Path,
    owner_uid: u32,
    owner_gid: u32,
) -> Result<Vec<u8>, SignerError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SignerError::KeyOrAuthorityUnavailable)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
        || metadata.gid() != owner_gid
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o7777 != 0o600
    {
        return Err(SignerError::KeyOrAuthorityUnavailable);
    }
    read_regular(path, 32).map_err(|_| SignerError::KeyOrAuthorityUnavailable)
}

pub(crate) fn validate_store_layout(
    root: &Path,
    signer_uid: u32,
    signer_gid: u32,
) -> Result<(), SignerError> {
    reject_symlink_components(root)?;
    for (relative, owner_uid, owner_gid, mode) in [
        ("", 0, 0, 0o711),
        ("control", 0, signer_gid, 0o750),
        ("control/grants", 0, signer_gid, 0o750),
        ("control/revoked-grants", 0, signer_gid, 0o750),
        ("keys", signer_uid, signer_gid, 0o700),
        ("state", signer_uid, signer_gid, 0o700),
        (RECORDS_DIR, signer_uid, signer_gid, 0o700),
        ("work", 0, 0, 0o711),
        ("backup-staging", signer_uid, signer_gid, 0o700),
    ] {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != owner_uid
            || metadata.gid() != owner_gid
            || metadata.permissions().mode() & 0o7777 != mode
        {
            return Err(SignerError::InstallationDrift);
        }
    }
    Ok(())
}

fn unix_time_ms() -> Result<u64, SignerError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SignerError::CryptoOrBindingInvalid)?;
    u64::try_from(duration.as_millis()).map_err(|_| SignerError::CryptoOrBindingInvalid)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
