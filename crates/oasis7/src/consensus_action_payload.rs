use crate::runtime;
use crate::simulator::{Action as SimulatorAction, ActionSubmitter};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

const CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION: u8 = 1;
const MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION: u8 = 1;
const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX: &str = "octransferauth:v1:";
const MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX: &str = "occlaimauth:v1:";
const MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX: &str = "ocgenesisauth:v1:";
const MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX: &str = "octreasuryauth:v1:";
const MAIN_TOKEN_RESTRICTED_CLAIM_LIVEOPS_POOL_TOP_UP_AUTH_SIGNATURE_V1_PREFIX: &str =
    "ocrestrictedclaimliveopspoolauth:v1:";
const MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX: &str =
    "ocrestrictedgrantadminauth:v1:";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusActionPayloadEnvelope {
    pub version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<ConsensusActionAuthEnvelope>,
    pub body: ConsensusActionPayloadBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ConsensusActionAuthEnvelope {
    MainTokenAction(MainTokenActionAuthProof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainTokenActionAuthScheme {
    Ed25519,
    ThresholdEd25519,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenActionParticipantSignature {
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenActionAuthProof {
    pub scheme: MainTokenActionAuthScheme,
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participant_signatures: Vec<MainTokenActionParticipantSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedMainTokenActionAuth {
    pub account_id: String,
    pub signer_public_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainTokenActionAuthError {
    InvalidRequest(String),
    InvalidSignature(String),
    AccountMismatch(String),
    UnsupportedAction(String),
}

impl std::fmt::Display for MainTokenActionAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(message)
            | Self::InvalidSignature(message)
            | Self::AccountMismatch(message)
            | Self::UnsupportedAction(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for MainTokenActionAuthError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ConsensusActionPayloadBody {
    RuntimeAction {
        action: runtime::Action,
    },
    SimulatorAction {
        action: SimulatorAction,
        #[serde(default)]
        submitter: ActionSubmitter,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct MainTokenActionSigningEnvelope<'a> {
    version: u8,
    operation: &'static str,
    account_id: &'a str,
    public_key: &'a str,
    action: MainTokenActionSigningPayload<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "data")]
enum MainTokenActionSigningPayload<'a> {
    TransferMainToken(TransferMainTokenSigningData<'a>),
    ClaimMainTokenVesting(ClaimMainTokenVestingSigningData<'a>),
    InitializeMainTokenGenesis(InitializeMainTokenGenesisSigningData<'a>),
    DistributeMainTokenTreasury(DistributeMainTokenTreasurySigningData<'a>),
    TopUpRestrictedStarterClaimLiveopsPool(TopUpRestrictedStarterClaimLiveopsPoolSigningData<'a>),
    UpdateRestrictedStarterClaimAdminRegistry(
        UpdateRestrictedStarterClaimAdminRegistrySigningData<'a>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct TransferMainTokenSigningData<'a> {
    from_account_id: &'a str,
    to_account_id: &'a str,
    amount: u64,
    nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ClaimMainTokenVestingSigningData<'a> {
    bucket_id: &'a str,
    beneficiary: &'a str,
    nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct InitializeMainTokenGenesisSigningData<'a> {
    allocations: &'a [runtime::MainTokenGenesisAllocationPlan],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct DistributeMainTokenTreasurySigningData<'a> {
    proposal_id: runtime::ProposalId,
    distribution_id: &'a str,
    bucket_id: &'a str,
    distributions: &'a [runtime::MainTokenTreasuryDistribution],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct TopUpRestrictedStarterClaimLiveopsPoolSigningData<'a> {
    controller_account_id: &'a str,
    top_up_id: &'a str,
    amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct UpdateRestrictedStarterClaimAdminRegistrySigningData<'a> {
    controller_account_id: &'a str,
    next_admin_account_ids: &'a [String],
}

impl ConsensusActionPayloadEnvelope {
    pub fn from_runtime_action(action: runtime::Action) -> Self {
        Self {
            version: CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION,
            auth: None,
            body: ConsensusActionPayloadBody::RuntimeAction { action },
        }
    }

    pub fn from_runtime_action_with_auth(
        action: runtime::Action,
        auth: ConsensusActionAuthEnvelope,
    ) -> Self {
        Self {
            version: CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION,
            auth: Some(auth),
            body: ConsensusActionPayloadBody::RuntimeAction { action },
        }
    }

    pub fn from_simulator_action(action: SimulatorAction, submitter: ActionSubmitter) -> Self {
        Self {
            version: CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION,
            auth: None,
            body: ConsensusActionPayloadBody::SimulatorAction { action, submitter },
        }
    }
}

pub fn main_token_action_auth_required(action: &runtime::Action) -> bool {
    matches!(
        action,
        runtime::Action::TransferMainToken { .. }
            | runtime::Action::ClaimMainTokenVesting { .. }
            | runtime::Action::InitializeMainTokenGenesis { .. }
            | runtime::Action::DistributeMainTokenTreasury { .. }
            | runtime::Action::TopUpRestrictedStarterClaimLiveopsPool { .. }
            | runtime::Action::UpdateRestrictedStarterClaimAdminRegistry { .. }
    )
}

pub fn sign_main_token_runtime_action_auth(
    action: &runtime::Action,
    account_id: &str,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<MainTokenActionAuthProof, MainTokenActionAuthError> {
    ensure_main_token_action_supported(action)?;
    let account_id = normalize_required_field(account_id, "main token auth account_id")?;
    let public_key =
        normalize_public_key_field(signer_public_key_hex, "main token auth signer public key")?;
    let signing_key =
        signing_key_from_hex(signer_private_key_hex, "main token auth signer private key")?;
    verify_keypair_match(
        &signing_key,
        public_key.as_str(),
        "main token auth signer public key",
    )?;
    let payload =
        build_main_token_action_signing_payload(action, account_id.as_str(), public_key.as_str())?;
    let signature: Signature = signing_key.sign(payload.as_slice());
    Ok(MainTokenActionAuthProof {
        scheme: MainTokenActionAuthScheme::Ed25519,
        account_id,
        public_key: Some(public_key),
        signature: Some(format!(
            "{}{}",
            main_token_action_signature_prefix(action)?,
            hex::encode(signature.to_bytes())
        )),
        threshold: None,
        participant_signatures: Vec::new(),
    })
}

pub fn sign_threshold_main_token_runtime_action_auth(
    action: &runtime::Action,
    account_id: &str,
    threshold: u16,
    signer_keypairs: &[(&str, &str)],
) -> Result<MainTokenActionAuthProof, MainTokenActionAuthError> {
    ensure_main_token_action_supported(action)?;
    if threshold == 0 {
        return Err(MainTokenActionAuthError::InvalidRequest(
            "main token auth threshold must be > 0".to_string(),
        ));
    }
    let account_id = normalize_required_field(account_id, "main token auth account_id")?;
    if signer_keypairs.len() < threshold as usize {
        return Err(MainTokenActionAuthError::InvalidRequest(format!(
            "main token auth participant count below threshold: participants={} threshold={threshold}",
            signer_keypairs.len()
        )));
    }
    let mut participant_signatures = Vec::with_capacity(signer_keypairs.len());
    for (public_key_hex, private_key_hex) in signer_keypairs {
        let public_key =
            normalize_public_key_field(public_key_hex, "main token auth signer public key")?;
        let signing_key =
            signing_key_from_hex(private_key_hex, "main token auth signer private key")?;
        verify_keypair_match(
            &signing_key,
            public_key.as_str(),
            "main token auth signer public key",
        )?;
        let payload = build_main_token_action_signing_payload(
            action,
            account_id.as_str(),
            public_key.as_str(),
        )?;
        let signature: Signature = signing_key.sign(payload.as_slice());
        participant_signatures.push(MainTokenActionParticipantSignature {
            public_key,
            signature: format!(
                "{}{}",
                main_token_action_signature_prefix(action)?,
                hex::encode(signature.to_bytes())
            ),
        });
    }
    Ok(MainTokenActionAuthProof {
        scheme: MainTokenActionAuthScheme::ThresholdEd25519,
        account_id,
        public_key: None,
        signature: None,
        threshold: Some(threshold),
        participant_signatures,
    })
}

pub fn verify_main_token_runtime_action_auth(
    action: &runtime::Action,
    proof: &MainTokenActionAuthProof,
) -> Result<VerifiedMainTokenActionAuth, MainTokenActionAuthError> {
    ensure_main_token_action_supported(action)?;
    let account_id =
        normalize_required_field(proof.account_id.as_str(), "main token auth account_id")?;
    match proof.scheme {
        MainTokenActionAuthScheme::Ed25519 => {
            let public_key = normalize_public_key_field(
                proof.public_key.as_deref().ok_or_else(|| {
                    MainTokenActionAuthError::InvalidRequest(
                        "main token auth public_key is required for ed25519".to_string(),
                    )
                })?,
                "main token auth public key",
            )?;
            let payload = build_main_token_action_signing_payload(
                action,
                account_id.as_str(),
                public_key.as_str(),
            )?;
            validate_main_token_action_account_binding(
                action,
                account_id.as_str(),
                Some(public_key.as_str()),
            )?;
            verify_main_token_action_signature(
                action,
                public_key.as_str(),
                proof.signature.as_deref().ok_or_else(|| {
                    MainTokenActionAuthError::InvalidRequest(
                        "main token auth signature is required for ed25519".to_string(),
                    )
                })?,
                payload.as_slice(),
            )?;
            Ok(VerifiedMainTokenActionAuth {
                account_id,
                signer_public_keys: vec![public_key],
            })
        }
        MainTokenActionAuthScheme::ThresholdEd25519 => {
            let threshold = proof.threshold.ok_or_else(|| {
                MainTokenActionAuthError::InvalidRequest(
                    "main token auth threshold is required for threshold_ed25519".to_string(),
                )
            })?;
            if threshold == 0 {
                return Err(MainTokenActionAuthError::InvalidRequest(
                    "main token auth threshold must be > 0".to_string(),
                ));
            }
            validate_main_token_action_account_binding(action, account_id.as_str(), None)?;
            if proof.participant_signatures.len() < threshold as usize {
                return Err(MainTokenActionAuthError::InvalidRequest(format!(
                    "main token auth participant count below threshold: participants={} threshold={threshold}",
                    proof.participant_signatures.len()
                )));
            }
            let mut signer_public_keys = Vec::with_capacity(proof.participant_signatures.len());
            let mut seen = std::collections::BTreeSet::new();
            for participant in &proof.participant_signatures {
                let public_key = normalize_public_key_field(
                    participant.public_key.as_str(),
                    "main token auth participant public key",
                )?;
                if !seen.insert(public_key.clone()) {
                    return Err(MainTokenActionAuthError::InvalidRequest(format!(
                        "duplicate main token auth participant public key: {public_key}"
                    )));
                }
                let payload = build_main_token_action_signing_payload(
                    action,
                    account_id.as_str(),
                    public_key.as_str(),
                )?;
                verify_main_token_action_signature(
                    action,
                    public_key.as_str(),
                    participant.signature.as_str(),
                    payload.as_slice(),
                )?;
                signer_public_keys.push(public_key);
            }
            Ok(VerifiedMainTokenActionAuth {
                account_id,
                signer_public_keys,
            })
        }
    }
}

pub fn encode_consensus_action_payload(
    envelope: &ConsensusActionPayloadEnvelope,
) -> Result<Vec<u8>, String> {
    serde_cbor::to_vec(envelope)
        .map_err(|err| format!("encode consensus action payload envelope failed: {err}"))
}

pub fn decode_consensus_action_payload_envelope(
    payload_cbor: &[u8],
) -> Result<ConsensusActionPayloadEnvelope, String> {
    match serde_cbor::from_slice::<ConsensusActionPayloadEnvelope>(payload_cbor) {
        Ok(envelope) => {
            if envelope.version != CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION {
                return Err(format!(
                    "unsupported consensus payload envelope version {}",
                    envelope.version
                ));
            }
            Ok(envelope)
        }
        Err(envelope_err) => match serde_cbor::from_slice::<runtime::Action>(payload_cbor) {
            Ok(action) => Ok(ConsensusActionPayloadEnvelope::from_runtime_action(action)),
            Err(runtime_err) => Err(format!(
                "decode consensus payload envelope failed ({envelope_err}); runtime fallback failed ({runtime_err})"
            )),
        },
    }
}

pub fn decode_consensus_action_payload(
    payload_cbor: &[u8],
) -> Result<ConsensusActionPayloadBody, String> {
    decode_consensus_action_payload_envelope(payload_cbor).map(|envelope| envelope.body)
}

fn build_main_token_action_signing_payload(
    action: &runtime::Action,
    account_id: &str,
    public_key: &str,
) -> Result<Vec<u8>, MainTokenActionAuthError> {
    let envelope = MainTokenActionSigningEnvelope {
        version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
        operation: main_token_action_operation(action)?,
        account_id,
        public_key,
        action: build_main_token_action_signing_action(action)?,
    };
    serde_json::to_vec(&envelope).map_err(|err| {
        MainTokenActionAuthError::InvalidRequest(format!(
            "encode main token auth signing payload failed: {err}"
        ))
    })
}

fn build_main_token_action_signing_action(
    action: &runtime::Action,
) -> Result<MainTokenActionSigningPayload<'_>, MainTokenActionAuthError> {
    match action {
        runtime::Action::TransferMainToken {
            from_account_id,
            to_account_id,
            amount,
            nonce,
        } => Ok(MainTokenActionSigningPayload::TransferMainToken(
            TransferMainTokenSigningData {
                from_account_id: from_account_id.as_str(),
                to_account_id: to_account_id.as_str(),
                amount: *amount,
                nonce: *nonce,
            },
        )),
        runtime::Action::ClaimMainTokenVesting {
            bucket_id,
            beneficiary,
            nonce,
        } => Ok(MainTokenActionSigningPayload::ClaimMainTokenVesting(
            ClaimMainTokenVestingSigningData {
                bucket_id: bucket_id.as_str(),
                beneficiary: beneficiary.as_str(),
                nonce: *nonce,
            },
        )),
        runtime::Action::InitializeMainTokenGenesis { allocations } => {
            Ok(MainTokenActionSigningPayload::InitializeMainTokenGenesis(
                InitializeMainTokenGenesisSigningData {
                    allocations: allocations.as_slice(),
                },
            ))
        }
        runtime::Action::DistributeMainTokenTreasury {
            proposal_id,
            distribution_id,
            bucket_id,
            distributions,
        } => Ok(MainTokenActionSigningPayload::DistributeMainTokenTreasury(
            DistributeMainTokenTreasurySigningData {
                proposal_id: *proposal_id,
                distribution_id: distribution_id.as_str(),
                bucket_id: bucket_id.as_str(),
                distributions: distributions.as_slice(),
            },
        )),
        runtime::Action::TopUpRestrictedStarterClaimLiveopsPool {
            controller_account_id,
            top_up_id,
            amount,
        } => Ok(
            MainTokenActionSigningPayload::TopUpRestrictedStarterClaimLiveopsPool(
                TopUpRestrictedStarterClaimLiveopsPoolSigningData {
                    controller_account_id: controller_account_id.as_str(),
                    top_up_id: top_up_id.as_str(),
                    amount: *amount,
                },
            ),
        ),
        runtime::Action::UpdateRestrictedStarterClaimAdminRegistry {
            controller_account_id,
            next_admin_account_ids,
        } => Ok(
            MainTokenActionSigningPayload::UpdateRestrictedStarterClaimAdminRegistry(
                UpdateRestrictedStarterClaimAdminRegistrySigningData {
                    controller_account_id: controller_account_id.as_str(),
                    next_admin_account_ids: next_admin_account_ids.as_slice(),
                },
            ),
        ),
        _ => Err(MainTokenActionAuthError::UnsupportedAction(format!(
            "main token auth is not supported for action {action:?}"
        ))),
    }
}

fn validate_main_token_action_account_binding(
    action: &runtime::Action,
    account_id: &str,
    public_key: Option<&str>,
) -> Result<(), MainTokenActionAuthError> {
    match action {
        runtime::Action::TransferMainToken {
            from_account_id, ..
        } => {
            if account_id != from_account_id.trim() {
                return Err(MainTokenActionAuthError::AccountMismatch(format!(
                    "main token auth account_id does not match transfer from_account_id: expected={} actual={account_id}",
                    from_account_id.trim()
                )));
            }
            let public_key = public_key.ok_or_else(|| {
                MainTokenActionAuthError::InvalidRequest(
                    "main token auth public key is required for transfer binding".to_string(),
                )
            })?;
            let expected = runtime::main_token_account_id_from_node_public_key(public_key);
            if account_id != expected {
                return Err(MainTokenActionAuthError::AccountMismatch(format!(
                    "main token auth account_id does not match signer public key: expected={expected} actual={account_id}"
                )));
            }
        }
        runtime::Action::ClaimMainTokenVesting { beneficiary, .. } => {
            if account_id != beneficiary.trim() {
                return Err(MainTokenActionAuthError::AccountMismatch(format!(
                    "main token auth account_id does not match claim beneficiary: expected={} actual={account_id}",
                    beneficiary.trim()
                )));
            }
            if beneficiary.trim().starts_with("oc:pk:") {
                let public_key = public_key.ok_or_else(|| {
                    MainTokenActionAuthError::InvalidRequest(
                        "main token auth public key is required for oc:pk claim binding"
                            .to_string(),
                    )
                })?;
                let expected = runtime::main_token_account_id_from_node_public_key(public_key);
                if account_id != expected {
                    return Err(MainTokenActionAuthError::AccountMismatch(format!(
                        "main token auth account_id does not match signer public key: expected={expected} actual={account_id}"
                    )));
                }
            }
        }
        runtime::Action::InitializeMainTokenGenesis { .. }
        | runtime::Action::DistributeMainTokenTreasury { .. } => {}
        runtime::Action::TopUpRestrictedStarterClaimLiveopsPool {
            controller_account_id,
            ..
        } => {
            if account_id != controller_account_id.trim() {
                return Err(MainTokenActionAuthError::AccountMismatch(format!(
                    "main token auth account_id does not match restricted claim liveops pool top-up controller_account_id: expected={} actual={account_id}",
                    controller_account_id.trim()
                )));
            }
        }
        runtime::Action::UpdateRestrictedStarterClaimAdminRegistry {
            controller_account_id,
            ..
        } => {
            if account_id != controller_account_id.trim() {
                return Err(MainTokenActionAuthError::AccountMismatch(format!(
                    "main token auth account_id does not match restricted grant admin registry controller_account_id: expected={} actual={account_id}",
                    controller_account_id.trim()
                )));
            }
        }
        _ => {
            return Err(MainTokenActionAuthError::UnsupportedAction(format!(
                "main token auth is not supported for action {action:?}"
            )));
        }
    }
    Ok(())
}

fn verify_main_token_action_signature(
    action: &runtime::Action,
    public_key_hex: &str,
    signature: &str,
    signing_payload: &[u8],
) -> Result<(), MainTokenActionAuthError> {
    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "main token auth public key")?;
    let prefix = main_token_action_signature_prefix(action)?;
    let signature_hex = signature.strip_prefix(prefix).ok_or_else(|| {
        MainTokenActionAuthError::InvalidSignature(format!(
            "main token auth signature is not {prefix}"
        ))
    })?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "main token auth signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|err| {
        MainTokenActionAuthError::InvalidRequest(format!(
            "parse main token auth public key failed: {err}"
        ))
    })?;
    verifying_key
        .verify(signing_payload, &Signature::from_bytes(&signature_bytes))
        .map_err(|err| {
            MainTokenActionAuthError::InvalidSignature(format!(
                "verify main token auth signature failed: {err}"
            ))
        })
}

fn main_token_action_operation(
    action: &runtime::Action,
) -> Result<&'static str, MainTokenActionAuthError> {
    match action {
        runtime::Action::TransferMainToken { .. } => Ok("transfer_main_token"),
        runtime::Action::ClaimMainTokenVesting { .. } => Ok("claim_main_token_vesting"),
        runtime::Action::InitializeMainTokenGenesis { .. } => Ok("initialize_main_token_genesis"),
        runtime::Action::DistributeMainTokenTreasury { .. } => Ok("distribute_main_token_treasury"),
        runtime::Action::TopUpRestrictedStarterClaimLiveopsPool { .. } => {
            Ok("top_up_restricted_starter_claim_liveops_pool")
        }
        runtime::Action::UpdateRestrictedStarterClaimAdminRegistry { .. } => {
            Ok("update_restricted_starter_claim_admin_registry")
        }
        _ => Err(MainTokenActionAuthError::UnsupportedAction(format!(
            "main token auth is not supported for action {action:?}"
        ))),
    }
}

fn main_token_action_signature_prefix(
    action: &runtime::Action,
) -> Result<&'static str, MainTokenActionAuthError> {
    match action {
        runtime::Action::TransferMainToken { .. } => {
            Ok(MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX)
        }
        runtime::Action::ClaimMainTokenVesting { .. } => {
            Ok(MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX)
        }
        runtime::Action::InitializeMainTokenGenesis { .. } => {
            Ok(MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX)
        }
        runtime::Action::DistributeMainTokenTreasury { .. } => {
            Ok(MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX)
        }
        runtime::Action::TopUpRestrictedStarterClaimLiveopsPool { .. } => {
            Ok(MAIN_TOKEN_RESTRICTED_CLAIM_LIVEOPS_POOL_TOP_UP_AUTH_SIGNATURE_V1_PREFIX)
        }
        runtime::Action::UpdateRestrictedStarterClaimAdminRegistry { .. } => {
            Ok(MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX)
        }
        _ => Err(MainTokenActionAuthError::UnsupportedAction(format!(
            "main token auth is not supported for action {action:?}"
        ))),
    }
}

fn ensure_main_token_action_supported(
    action: &runtime::Action,
) -> Result<(), MainTokenActionAuthError> {
    main_token_action_operation(action).map(|_| ())
}

fn normalize_required_field(raw: &str, label: &str) -> Result<String, MainTokenActionAuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(MainTokenActionAuthError::InvalidRequest(format!(
            "{label} is empty"
        )));
    }
    Ok(value.to_string())
}

fn normalize_public_key_field(raw: &str, label: &str) -> Result<String, MainTokenActionAuthError> {
    let normalized = normalize_required_field(raw, label)?;
    let bytes = decode_hex_array::<32>(normalized.as_str(), label)?;
    Ok(hex::encode(bytes))
}

fn signing_key_from_hex(
    private_key_hex: &str,
    label: &str,
) -> Result<SigningKey, MainTokenActionAuthError> {
    let private_key_bytes = decode_hex_array::<32>(private_key_hex, label)?;
    Ok(SigningKey::from_bytes(&private_key_bytes))
}

fn verify_keypair_match(
    signing_key: &SigningKey,
    public_key_hex: &str,
    label: &str,
) -> Result<(), MainTokenActionAuthError> {
    let expected_public_key = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_key != public_key_hex {
        return Err(MainTokenActionAuthError::InvalidRequest(format!(
            "{label} does not match private key: expected={expected_public_key} actual={public_key_hex}"
        )));
    }
    Ok(())
}

fn decode_hex_array<const N: usize>(
    raw: &str,
    label: &str,
) -> Result<[u8; N], MainTokenActionAuthError> {
    let bytes = hex::decode(raw).map_err(|err| {
        MainTokenActionAuthError::InvalidRequest(format!("decode {label} failed: {err}"))
    })?;
    if bytes.len() != N {
        return Err(MainTokenActionAuthError::InvalidRequest(format!(
            "{label} length mismatch: expected {N} bytes, got {}",
            bytes.len()
        )));
    }
    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(bytes.as_slice());
    Ok(fixed)
}
