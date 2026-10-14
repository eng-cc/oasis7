//! Bounded HTTP transport for the five WorldService operations.
use super::{
    authority,
    projection::WorldServiceProjection,
    verified_view::{ReadEvidencePolicy, VerifiedWorldView},
    wire::*,
};
use oasis7_client_api::world_service::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;
#[path = "client_cooldown.rs"]
mod cooldown;
#[path = "client_formal.rs"]
mod formal;
#[path = "client_formal_changes.rs"]
mod formal_changes;
#[path = "client_formal_operations.rs"]
mod formal_operations;
pub use cooldown::WorldServiceQueryState;
pub use formal::FormalReadTrustConfigV1;
#[cfg(test)]
#[path = "client_cooldown_tests.rs"]
mod cooldown_tests;

#[derive(Clone)]
pub struct WorldServiceClientConfig {
    pub formal_read_trust: Option<FormalReadTrustConfigV1>,
    pub endpoint: String,
    pub trusted_service_public_key: String,
    pub expected_world: WorldIdentity,
    pub scope_id: String,
    pub read_private_key_hex: String,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}

#[derive(Clone)]
pub struct WorldServiceAgentSignerConfig {
    pub private_key_hex: String,
    pub delegation_generation: u64,
}
impl WorldServiceAgentSignerConfig {
    pub fn from_env() -> Result<Option<Self>, String> {
        Self::from_values(
            std::env::var("OASIS7_WORLD_SERVICE_AGENT_PRIVATE_KEY").ok(),
            std::env::var("OASIS7_WORLD_SERVICE_AGENT_DELEGATION_GENERATION").ok(),
        )
    }
    fn from_values(
        key: Option<String>,
        generation: Option<String>,
    ) -> Result<Option<Self>, String> {
        match (key, generation) {
            (None, None) => Ok(None),
            (Some(private_key_hex), Some(generation)) => {
                authority::sign_read_request("agent-configuration", (), &private_key_hex)?;
                let delegation_generation = generation
                    .parse::<u64>()
                    .map_err(|_| "invalid Agent delegation generation".to_string())?;
                if delegation_generation == 0 {
                    return Err("Agent delegation generation must be nonzero".into());
                }
                Ok(Some(Self {
                    private_key_hex,
                    delegation_generation,
                }))
            }
            _ => Err(
                "Agent private key and delegation generation must be configured together".into(),
            ),
        }
    }
}
impl std::fmt::Debug for WorldServiceAgentSignerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldServiceAgentSignerConfig")
            .field("delegation_generation", &self.delegation_generation)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for WorldServiceClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldServiceClientConfig")
            .field("endpoint", &self.endpoint)
            .field("expected_world", &self.expected_world)
            .field("scope_id", &self.scope_id)
            .finish_non_exhaustive()
    }
}
impl WorldServiceClientConfig {
    pub(crate) fn read_authority_identity(&self) -> Result<String, String> {
        authority::request_digest(
            "local-read-authority",
            &(
                &self.endpoint,
                &self.trusted_service_public_key,
                &self.expected_world,
                &self.scope_id,
                &self.read_private_key_hex,
            ),
        )
    }

    pub fn from_env() -> Result<Option<Self>, String> {
        let Ok(endpoint) = std::env::var("OASIS7_WORLD_SERVICE_ENDPOINT") else {
            return Ok(None);
        };
        let required = |name: &str| std::env::var(name).map_err(|_| format!("missing {name}"));
        let formal_read_trust = match std::env::var("OASIS7_WORLD_SERVICE_FORMAL_TRUST_FILE") {
            Ok(path) => {
                use std::io::Read;
                let file = std::fs::File::open(&path)
                    .map_err(|error| format!("formal trust file: {error}"))?;
                let mut bytes = Vec::new();
                file.take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| format!("formal trust file: {error}"))?;
                if bytes.len() > 4 * 1024 * 1024 {
                    return Err("formal trust file exceeds bound".into());
                }
                let trust: FormalReadTrustConfigV1 = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("formal trust file: {error}"))?;
                trust.verified_anchor()?;
                Some(trust)
            }
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => return Err(format!("formal trust file: {error}")),
        };
        let max_response_bytes = if formal_read_trust.is_some() {
            16 * 1024 * 1024
        } else {
            1_048_576
        };
        Ok(Some(Self {
            endpoint,
            trusted_service_public_key: required("OASIS7_WORLD_SERVICE_PUBLIC_KEY")?,
            expected_world: WorldIdentity {
                world_id: required("OASIS7_WORLD_SERVICE_WORLD_ID")?,
                genesis_digest: required("OASIS7_WORLD_SERVICE_GENESIS_DIGEST")?,
            },
            scope_id: required("OASIS7_WORLD_SERVICE_SCOPE")?,
            read_private_key_hex: required("OASIS7_WORLD_SERVICE_READ_PRIVATE_KEY")?,
            timeout: Duration::from_millis(1500),
            max_response_bytes,
            formal_read_trust,
        }))
    }
}

#[derive(Debug)]
pub enum WorldServiceClientError {
    Configuration(String),
    Transport(String),
    Rejected { status: u16, message: String },
    Assurance(String),
    Cooldown { status: u16, retry_after: Duration },
}
impl std::fmt::Display for WorldServiceClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WorldServiceClientError {}

pub trait WorldServicePort {
    fn describe(&self) -> Result<DescribeWorldResponse, WorldServiceClientError>;
    fn submit(
        &self,
        request: SubmitIntentRequest<WorldServicePayloadV1>,
    ) -> Result<SubmitObservation<Value>, WorldServiceClientError>;
    fn lookup(
        &self,
        request: LookupIntentRequest,
        original: WorldServicePayloadV1,
    ) -> Result<IntentResponse<Value>, WorldServiceClientError>;
    fn read_view(
        &self,
        request: ReadWorldViewRequest,
    ) -> Result<VerifiedWorldView, WorldServiceClientError>;
    fn read_changes(
        &self,
        request: ReadWorldChangesRequest,
    ) -> Result<ReadWorldChangesResponse<Value>, WorldServiceClientError>;
}

#[derive(Clone)]
pub struct RemoteWorldServiceClient {
    formal_read: Option<std::sync::Arc<std::sync::Mutex<super::formal_read::FormalReadVerifier>>>,
    formal_trust_identity: Option<String>,
    evidence_policy: ReadEvidencePolicy,
    config: WorldServiceClientConfig,
    http: reqwest::blocking::Client,
    query_state: WorldServiceQueryState,
}
impl RemoteWorldServiceClient {
    pub fn new(config: WorldServiceClientConfig) -> Result<Self, WorldServiceClientError> {
        if let Some(formal) = config.formal_read_trust.clone() {
            let anchor = formal
                .verified_anchor()
                .map_err(WorldServiceClientError::Configuration)?;
            return Self::new_formal(config, formal.initial_trust, anchor);
        }
        Self::new_with_evidence_policy(config, ReadEvidencePolicy::Ordinary)
    }

    /// The controlled prerequisite policy must be deliberately configured by
    /// the caller. It permits one exact signed engineering scope only.
    pub fn new_with_evidence_policy(
        config: WorldServiceClientConfig,
        evidence_policy: ReadEvidencePolicy,
    ) -> Result<Self, WorldServiceClientError> {
        if config.formal_read_trust.is_some() {
            if evidence_policy != ReadEvidencePolicy::Ordinary {
                return Err(WorldServiceClientError::Configuration(
                    "formal trust and engineering evidence policy cannot be combined".into(),
                ));
            }
            return Self::new(config);
        }
        Self::new_base_with_evidence_policy(config, evidence_policy)
    }
    fn new_base_with_evidence_policy(
        config: WorldServiceClientConfig,
        evidence_policy: ReadEvidencePolicy,
    ) -> Result<Self, WorldServiceClientError> {
        config
            .expected_world
            .validate()
            .map_err(|e| WorldServiceClientError::Configuration(e.to_string()))?;
        config
            .expected_world
            .validate()
            .map_err(|e| WorldServiceClientError::Configuration(e.to_string()))?;
        let endpoint = reqwest::Url::parse(&config.endpoint)
            .map_err(|e| WorldServiceClientError::Configuration(e.to_string()))?;
        if !["http", "https"].contains(&endpoint.scheme())
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || config.scope_id.trim().is_empty()
            || config.timeout.is_zero()
            || config.max_response_bytes == 0
        {
            return Err(WorldServiceClientError::Configuration(
                "invalid service connection".into(),
            ));
        }
        // Decode and validate configured trust/signer before any request.
        let proof = authority::sign_read_request(DESCRIBE_PATH, (), &config.read_private_key_hex)
            .map_err(WorldServiceClientError::Configuration)?;
        authority::verify_read_request(DESCRIBE_PATH, &proof)
            .map_err(WorldServiceClientError::Configuration)?;
        if hex::decode(&config.trusted_service_public_key).map_or(true, |key| key.len() != 32) {
            return Err(WorldServiceClientError::Configuration(
                "invalid service trust key".into(),
            ));
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| WorldServiceClientError::Configuration(e.to_string()))?;
        Ok(Self {
            formal_read: None,
            formal_trust_identity: None,
            evidence_policy,
            config,
            http,
            query_state: WorldServiceQueryState::default(),
        })
    }
    /// Formal membership is read from the independently proved public projection;
    /// engineering membership retains its explicit signed diagnostic capability.
    pub fn read_membership(
        &self,
        request: ReadMembershipRequest,
    ) -> Result<super::membership::AuthenticatedMembershipObservation, WorldServiceClientError>
    {
        if self.formal_read.is_none() {
            request
                .validate()
                .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        }
        if request.world != self.config.expected_world || self.config.scope_id != "public" {
            return Err(WorldServiceClientError::Assurance(
                "membership requires configured public world scope".into(),
            ));
        }
        if self.formal_read.is_some() {
            let view = self.read_view(ReadWorldViewRequest {
                observation_nonce: None,
                contract_version: request.contract_version,
                formal_anchor: None,
                world: request.world.clone(),
                scope_id: "public".into(),
                min_commit: request.min_commit.clone(),
                fixed_commit: request.fixed_commit.clone(),
                deadline_unix_ms: None,
            })?;
            return super::membership::AuthenticatedMembershipObservation::from_formal_view(
                &view, &request,
            )
            .map_err(WorldServiceClientError::Assurance);
        }
        let description = self.describe()?;
        if !description
            .capabilities
            .contains(&Capability::GovernanceMembership)
        {
            return Err(WorldServiceClientError::Assurance(
                "authenticated governance membership capability is unsupported".into(),
            ));
        }
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| {
            WorldServiceClientError::Configuration(
                "membership observation randomness is unavailable".into(),
            )
        })?;
        let envelope = MembershipReadEnvelope {
            query: request.clone(),
            observation_nonce: hex::encode(nonce),
        };
        let response: ReadMembershipResponse = self.signed_call(MEMBERSHIP_PATH, envelope)?;
        if response.execution_evidence_scope != description.execution_evidence_scope {
            return Err(WorldServiceClientError::Assurance(
                "membership scope differs from service description".into(),
            ));
        }
        super::membership::AuthenticatedMembershipObservation::from_signed_transport(
            response,
            &request,
            self.evidence_policy,
        )
        .map_err(WorldServiceClientError::Assurance)
    }
    pub fn with_query_state(mut self, state: WorldServiceQueryState) -> Self {
        self.query_state = state;
        self
    }
    fn check_evidence_scope(&self, scope: Option<&str>) -> Result<(), WorldServiceClientError> {
        if self.evidence_policy.accepts(scope) {
            Ok(())
        } else {
            Err(WorldServiceClientError::Assurance(
                "service evidence scope is not allowed by explicit client policy".into(),
            ))
        }
    }
    pub fn read_authority_identity(&self) -> Result<String, String> {
        let identity = self.config.read_authority_identity()?;
        if let Some(formal) = &self.formal_trust_identity {
            return authority::request_digest("local-formal-read-authority", &(identity, formal));
        }
        if self.evidence_policy == ReadEvidencePolicy::Ordinary {
            Ok(identity)
        } else {
            authority::request_digest(
                "local-read-evidence-policy",
                &(identity, self.evidence_policy),
            )
        }
    }

    pub fn config(&self) -> &WorldServiceClientConfig {
        &self.config
    }

    /// Relay a device-signed read unchanged. The configured local reader key is
    /// never used here; the service checks the device's current scope authority.
    pub fn read_actor_view(
        &self,
        signed: SignedReadRequest<ReadWorldViewRequest>,
    ) -> Result<VerifiedWorldView, WorldServiceClientError> {
        self.check_signed_formal_anchor(&signed.request)?;
        (if self.formal_read.is_some() {
            signed.request.validate_formal_shape()
        } else {
            signed.request.validate()
        })
        .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        if signed.request.world != self.config.expected_world
            || signed.request.scope_id != self.config.scope_id
        {
            return Err(WorldServiceClientError::Assurance(
                "actor read scope/world differs from configuration".into(),
            ));
        }
        authority::verify_read_request(VIEW_PATH, &signed)
            .map_err(WorldServiceClientError::Assurance)?;
        let response: ReadWorldViewResponse<WorldServiceProjection> =
            self.call(VIEW_PATH, &signed)?;
        self.finish_view(response, &signed.request, &signed.subject_public_key)
    }

    fn call<Q: Serialize, R: Serialize + DeserializeOwned>(
        &self,
        path: &str,
        request: &Q,
    ) -> Result<R, WorldServiceClientError> {
        if path != SUBMIT_PATH {
            self.query_state.check()?;
        }
        let digest =
            authority::request_digest(path, request).map_err(WorldServiceClientError::Assurance)?;
        let mut response = self
            .http
            .post(format!(
                "{}{}",
                self.config.endpoint.trim_end_matches('/'),
                path
            ))
            .json(request)
            .send()
            .map_err(|error| {
                let mut message = error.to_string();
                let mut source = std::error::Error::source(&error);
                for _ in 0..4 {
                    let Some(cause) = source else {
                        break;
                    };
                    message.push_str(": ");
                    message.push_str(&cause.to_string());
                    source = cause.source();
                }
                WorldServiceClientError::Transport(message)
            })?;
        let status = response.status().as_u16();
        if let Some(retry_after) = self.query_state.observe(
            status,
            response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
        )? {
            return Err(WorldServiceClientError::Cooldown {
                status,
                retry_after,
            });
        }
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take(self.config.max_response_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| WorldServiceClientError::Transport(e.to_string()))?;
        if bytes.len() > self.config.max_response_bytes {
            return Err(WorldServiceClientError::Assurance(
                "service response exceeds bound".into(),
            ));
        }
        if status != 200 {
            let message = serde_json::from_slice::<SignedServiceResponse<Value>>(&bytes)
                .ok()
                .filter(|reply| {
                    authority::verify_service_response(
                        path,
                        &digest,
                        reply,
                        &self.config.trusted_service_public_key,
                    )
                    .is_ok()
                })
                .and_then(|reply| {
                    reply
                        .payload
                        .get("message")
                        .and_then(Value::as_str)
                        .map(|message| message.chars().take(1024).collect::<String>())
                })
                .unwrap_or_else(|| "service rejected request".into());
            return Err(WorldServiceClientError::Rejected { status, message });
        }
        let signed: SignedServiceResponse<R> = serde_json::from_slice(&bytes)
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        authority::verify_service_response(
            path,
            &digest,
            &signed,
            &self.config.trusted_service_public_key,
        )
        .map_err(WorldServiceClientError::Assurance)?;
        Ok(signed.payload)
    }
    /// Forward the original browser proof without replacing its signer.
    pub fn read_owner_view(
        &self,
        signed: SignedReadRequest<ReadWorldViewRequest>,
    ) -> Result<VerifiedWorldView, WorldServiceClientError> {
        if self.evidence_policy != ReadEvidencePolicy::Ordinary {
            return Err(WorldServiceClientError::Assurance(
                "owner read helper is unavailable for engineering evidence policy".into(),
            ));
        }
        self.check_signed_formal_anchor(&signed.request)?;
        authority::verify_read_request(VIEW_PATH, &signed)
            .map_err(WorldServiceClientError::Assurance)?;
        if signed.request.world != self.config.expected_world {
            return Err(WorldServiceClientError::Assurance(
                "owner read world mismatch".into(),
            ));
        }
        let response = self.call(VIEW_PATH, &signed)?;
        self.finish_view(response, &signed.request, &signed.subject_public_key)
    }
    fn signed_call<Q: Serialize, R: Serialize + DeserializeOwned>(
        &self,
        path: &str,
        request: Q,
    ) -> Result<R, WorldServiceClientError> {
        let signed = authority::sign_read_request(path, request, &self.config.read_private_key_hex)
            .map_err(WorldServiceClientError::Assurance)?;
        self.call(path, &signed)
    }
}

impl WorldServicePort for RemoteWorldServiceClient {
    fn describe(&self) -> Result<DescribeWorldResponse, WorldServiceClientError> {
        let response: DescribeWorldResponse = self.signed_call(
            DESCRIBE_PATH,
            DescribeWorldRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                expected_world: self.config.expected_world.clone(),
                trust_config_ref: self.config.trusted_service_public_key.clone(),
            },
        )?;
        self.check_evidence_scope(response.execution_evidence_scope.as_deref())?;
        if self.formal_read.is_some() {
            if response.formal_profile.as_ref() != Some(&FormalReadProfileV1::controlled()) {
                return Err(WorldServiceClientError::Assurance(
                    "configured formal service did not advertise the pinned formal profile".into(),
                ));
            }
        } else if response.formal_profile.is_some() {
            return Err(WorldServiceClientError::Assurance(
                "formal profile requires configured external proof trust".into(),
            ));
        }
        response
            .validate(&self.config.expected_world)
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        Ok(response)
    }
    fn submit(
        &self,
        request: SubmitIntentRequest<WorldServicePayloadV1>,
    ) -> Result<SubmitObservation<Value>, WorldServiceClientError> {
        if self.formal_read.is_some() {
            return self.submit_formal(request);
        }
        self.require_unproved_operation_profile(None)?;
        if request.correlation.key.world != self.config.expected_world {
            return Err(WorldServiceClientError::Assurance(
                "intent world differs from configuration".into(),
            ));
        }
        request
            .validate()
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        let correlation = request.correlation.clone();
        match self.call::<_, IntentResponse<Value>>(SUBMIT_PATH, &request) {
            Ok(response) => {
                if response.formal_profile.is_some() {
                    return Ok(SubmitObservation::OutcomeUnknown(correlation));
                }
                self.check_evidence_scope(response.execution_evidence_scope.as_deref())?;
                response
                    .validate(&correlation)
                    .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
                Ok(SubmitObservation::Response(Box::new(response)))
            }
            // A lost/malformed/untrusted response cannot establish rejection.
            Err(
                WorldServiceClientError::Transport(_)
                | WorldServiceClientError::Assurance(_)
                | WorldServiceClientError::Cooldown { .. },
            ) => Ok(SubmitObservation::OutcomeUnknown(correlation)),
            Err(error) => Err(error),
        }
    }
    fn lookup(
        &self,
        request: LookupIntentRequest,
        original: WorldServicePayloadV1,
    ) -> Result<IntentResponse<Value>, WorldServiceClientError> {
        if self.formal_read.is_some() {
            return self.lookup_formal(request, original);
        }
        self.require_unproved_operation_profile(None)?;
        request
            .validate()
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        let expected = super::derive_correlation(self.config.expected_world.clone(), &original)
            .map_err(WorldServiceClientError::Assurance)?;
        if expected.key != request.key {
            return Err(WorldServiceClientError::Assurance(
                "lookup key differs from signed payload".into(),
            ));
        }
        let response: IntentResponse<Value> =
            self.call(LOOKUP_PATH, &AuthenticatedLookup { request, original })?;
        self.check_evidence_scope(response.execution_evidence_scope.as_deref())?;
        self.require_unproved_operation_profile(response.formal_profile.as_ref())?;
        response
            .validate(&expected)
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        Ok(response)
    }
    fn read_view(
        &self,
        request: ReadWorldViewRequest,
    ) -> Result<VerifiedWorldView, WorldServiceClientError> {
        let request = self.prepare_read_view_request(request)?;
        (if self.formal_read.is_some() {
            request.validate_formal_shape()
        } else {
            request.validate()
        })
        .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        if request.world != self.config.expected_world || request.scope_id != self.config.scope_id {
            return Err(WorldServiceClientError::Assurance(
                "read scope/world differs from configuration".into(),
            ));
        }
        let signed = authority::sign_read_request(
            VIEW_PATH,
            request.clone(),
            &self.config.read_private_key_hex,
        )
        .map_err(WorldServiceClientError::Assurance)?;
        let response: ReadWorldViewResponse<WorldServiceProjection> =
            self.call(VIEW_PATH, &signed)?;
        let identity = self
            .read_authority_identity()
            .map_err(WorldServiceClientError::Assurance)?;
        self.finish_view(response, &request, &signed.subject_public_key)
            .map(|view| view.bind_read_authority(identity))
    }
    fn read_changes(
        &self,
        request: ReadWorldChangesRequest,
    ) -> Result<ReadWorldChangesResponse<Value>, WorldServiceClientError> {
        self.require_unproved_operation_profile(None)?;
        request
            .validate()
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        if request.cursor.commit.world != self.config.expected_world
            || request.cursor.scope_id != self.config.scope_id
        {
            return Err(WorldServiceClientError::Assurance(
                "changes scope/world differs from configuration".into(),
            ));
        }
        let response: ReadWorldChangesResponse<Value> =
            self.signed_call(CHANGES_PATH, request.clone())?;
        self.check_evidence_scope(response.execution_evidence_scope.as_deref())?;
        self.require_unproved_operation_profile(response.formal_profile.as_ref())?;
        response
            .validate(&request)
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        let encoded = serde_json::to_vec(&response)
            .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?;
        if encoded.len() as u64 > request.max_bytes {
            return Err(WorldServiceClientError::Assurance(
                "changes byte limit exceeded".into(),
            ));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn config(endpoint: String) -> WorldServiceClientConfig {
        let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        WorldServiceClientConfig {
            formal_read_trust: None,
            endpoint,
            trusted_service_public_key: hex::encode(key.verifying_key().to_bytes()),
            expected_world: WorldIdentity {
                world_id: "world".into(),
                genesis_digest: "genesis".into(),
            },
            scope_id: "agent:a".into(),
            read_private_key_hex: hex::encode([7; 32]),
            timeout: Duration::from_millis(500),
            max_response_bytes: 4096,
        }
    }
    fn serve(
        transform: impl FnOnce(&mut SignedServiceResponse<Value>) + Send + 'static,
    ) -> (String, thread::JoinHandle<()>) {
        serve_payload(VIEW_PATH, serde_json::json!({"ok":true}), transform)
    }
    fn serve_payload(
        path: &'static str,
        payload: Value,
        transform: impl FnOnce(&mut SignedServiceResponse<Value>) + Send + 'static,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let (head_end, length) = loop {
                let mut buf = [0; 1024];
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
                if let Some(at) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&request[..at]).to_lowercase();
                    let length: usize = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap();
                    if request.len() >= at + 4 + length {
                        break (at + 4, length);
                    }
                }
            };
            let signed: SignedReadRequest<Value> =
                serde_json::from_slice(&request[head_end..head_end + length]).unwrap();
            authority::verify_read_request(path, &signed).unwrap();
            let digest = authority::request_digest(path, &signed).unwrap();
            let mut response =
                authority::sign_service_response(path, digest, payload, &hex::encode([9; 32]))
                    .unwrap();
            transform(&mut response);
            let body = serde_json::to_vec(&response).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        (endpoint, task)
    }
    #[test]
    fn engineering_owner_read_rejects_before_transport_preserving_original_proof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let config = config(format!("http://{}", listener.local_addr().unwrap()));
        let signed = authority::sign_read_request(
            VIEW_PATH,
            ReadWorldViewRequest {
                observation_nonce: None,
                formal_anchor: None,
                contract_version: 1,
                world: config.expected_world.clone(),
                scope_id: config.scope_id.clone(),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            },
            &config.read_private_key_hex,
        )
        .unwrap();
        let original = serde_json::to_vec(&signed).unwrap();
        let client = RemoteWorldServiceClient::new_with_evidence_policy(
            config,
            ReadEvidencePolicy::AllowControlledPrerequisite,
        )
        .unwrap();
        for _ in 0..2 {
            assert!(matches!(
                client.read_owner_view(signed.clone()),
                Err(WorldServiceClientError::Assurance(_))
            ));
            assert_eq!(serde_json::to_vec(&signed).unwrap(), original);
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }

    #[test]
    fn signed_http_scope_rejects_unknown_downgrade_and_unsigned_tamper() {
        let payload = serde_json::json!({
            "contract_version":1,"world":{"world_id":"world","genesis_digest":"genesis"},
            "binding":{"provider_world_id":"world","branch_id":"main","finality_ref":"fixture",
                "reorg_generation":0,"governing_manifest_ref":"fixture","authority_generation":1,"permission_generation":1},
            "capabilities":[],"availability":{"readable":false,"writable":false,"recovering":true,
                "reason":null,"retry_after_ms":null},"current_view":null
        });
        for scope in [None, Some("unknown_scope")] {
            let mut changed = payload.clone();
            if let Some(scope) = scope {
                changed["execution_evidence_scope"] = serde_json::json!(scope);
            }
            let (endpoint, task) = serve_payload(DESCRIBE_PATH, changed, |_| {});
            let client = RemoteWorldServiceClient::new_with_evidence_policy(
                config(endpoint),
                ReadEvidencePolicy::AllowControlledPrerequisite,
            )
            .unwrap();
            assert!(matches!(
                client.describe(),
                Err(WorldServiceClientError::Assurance(_))
            ));
            task.join().unwrap();
        }
        let mut signed = payload;
        signed["execution_evidence_scope"] = serde_json::json!(
            crate::world_service::verified_view::CONTROLLED_PREREQUISITE_READ_SCOPE
        );
        let (endpoint, task) = serve_payload(DESCRIBE_PATH, signed, |response| {
            response
                .payload
                .as_object_mut()
                .unwrap()
                .remove("execution_evidence_scope");
        });
        let client = RemoteWorldServiceClient::new(config(endpoint)).unwrap();
        assert!(matches!(
            client.describe(),
            Err(WorldServiceClientError::Assurance(_))
        ));
        task.join().unwrap();
    }
    #[test]
    fn response_is_bound_to_trusted_key_request_and_operation() {
        let (endpoint, task) = serve(|_| {});
        let client = RemoteWorldServiceClient::new(config(endpoint)).unwrap();
        let result: Value = client
            .signed_call(VIEW_PATH, serde_json::json!({"scope":"agent:a"}))
            .unwrap();
        assert_eq!(result["ok"], true);
        task.join().unwrap();
        let (endpoint, task) = serve(|response| response.payload = serde_json::json!({"ok":false}));
        let client = RemoteWorldServiceClient::new(config(endpoint)).unwrap();
        assert!(matches!(
            client.signed_call::<_, Value>(VIEW_PATH, serde_json::json!({"scope":"agent:a"})),
            Err(WorldServiceClientError::Assurance(_))
        ));
        task.join().unwrap();
    }
    #[test]
    fn lost_submission_response_is_unknown_without_retry_or_local_effect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = RemoteWorldServiceClient::new(config(format!(
            "http://{}",
            listener.local_addr().unwrap()
        )))
        .unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let _ = stream.read(&mut buffer);
        });
        let correlation = RequestCorrelation {
            key: RequestKey {
                world: client.config.expected_world.clone(),
                verified_subject: "subject".into(),
                operation_domain: "gameplay".into(),
                nonce_scope: "player_auth".into(),
                request_id_or_nonce: "12".into(),
            },
            payload_digest: "digest".into(),
        };
        let result = client
            .submit(SubmitIntentRequest {
                contract_version: 1,
                correlation: correlation.clone(),
                deadline_unix_ms: None,
                signed_payload: WorldServicePayloadV1::GameplayJson(vec![1]),
            })
            .unwrap();
        assert!(
            matches!(result, SubmitObservation::OutcomeUnknown(observed) if observed == correlation)
        );
        worker.join().unwrap();
    }
    #[test]
    fn credentials_are_redacted_from_connection_debug() {
        let config = config("http://localhost:1".into());
        assert!(!format!("{config:?}").contains(&config.read_private_key_hex));
    }
    #[test]
    fn agent_signer_requires_explicit_complete_valid_delegation_configuration() {
        assert!(
            WorldServiceAgentSignerConfig::from_values(None, None)
                .unwrap()
                .is_none()
        );
        assert!(
            WorldServiceAgentSignerConfig::from_values(Some(hex::encode([8; 32])), None).is_err()
        );
        assert!(WorldServiceAgentSignerConfig::from_values(None, Some("1".into())).is_err());
        assert!(
            WorldServiceAgentSignerConfig::from_values(Some("invalid".into()), Some("1".into()))
                .is_err()
        );
        assert!(
            WorldServiceAgentSignerConfig::from_values(
                Some(hex::encode([8; 32])),
                Some("0".into())
            )
            .is_err()
        );
        let signer = WorldServiceAgentSignerConfig::from_values(
            Some(hex::encode([8; 32])),
            Some("2".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(signer.delegation_generation, 2);
        assert!(!format!("{signer:?}").contains(&signer.private_key_hex));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "client/private_goal_client.rs"]
mod private_goal_client;

#[cfg(not(target_arch = "wasm32"))]
#[path = "client/private_task_context_client.rs"]
mod private_task_context_client;
