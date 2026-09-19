use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION: u8 = 1;
pub const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX: &str = "octransferauth:v1:";
pub const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V2_PREFIX: &str = "octransferauth:v2:";
pub const TRANSFER_TX_VERSION_V2: u8 = 2;
pub const TRANSFER_TX_TYPE_ASSET_TRANSFER: &str = "asset_transfer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenTransfer {
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: u64,
    pub nonce: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_version: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fee: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_payload_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<String>,
}

pub type MainTokenTransferRequest = MainTokenTransfer;

impl MainTokenTransfer {
    /// Construct the current v2 asset-transfer request used by the launcher.
    pub fn new(
        from_account_id: impl Into<String>,
        to_account_id: impl Into<String>,
        amount: u64,
        nonce: u64,
    ) -> Self {
        Self {
            from_account_id: from_account_id.into(),
            to_account_id: to_account_id.into(),
            amount,
            nonce,
            asset_id: Some("main_token".to_string()),
            memo: None,
            chain_id: None,
            network_id: None,
            tx_version: Some(TRANSFER_TX_VERSION_V2),
            tx_type: Some(TRANSFER_TX_TYPE_ASSET_TRANSFER.to_string()),
            valid_until_unix_ms: None,
            max_fee: None,
            fee_asset_id: None,
            application_payload_hash: None,
            client_request_id: None,
        }
    }

    pub fn signature_prefix(&self) -> &'static str {
        if self.uses_v2_context() {
            MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V2_PREFIX
        } else {
            MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX
        }
    }

    fn uses_v2_context(&self) -> bool {
        self.chain_id.is_some()
            || self.network_id.is_some()
            || self.tx_version.is_some()
            || self.tx_type.is_some()
            || self.valid_until_unix_ms.is_some()
            || self
                .asset_id
                .as_deref()
                .is_some_and(|value| value != "main_token")
            || self.memo.is_some()
    }
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

pub type SignedMainTokenTransfer = MainTokenActionAuthProof;

/// The wire request shape consumed by the existing chain transfer endpoint.
/// Authentication fields intentionally stay alongside the transfer fields;
/// this is not the nested runtime consensus envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedMainTokenTransferRequest {
    #[serde(flatten)]
    pub transfer: MainTokenTransfer,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedMainTokenActionAuth {
    pub account_id: String,
    pub signer_public_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientSigningError {
    InvalidRequest(String),
    InvalidSignature(String),
    AccountMismatch(String),
}

impl std::fmt::Display for ClientSigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(message)
            | Self::InvalidSignature(message)
            | Self::AccountMismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ClientSigningError {}

#[derive(Debug, Serialize)]
struct MainTokenTransferSigningEnvelope<'a> {
    version: u8,
    operation: &'static str,
    account_id: &'a str,
    public_key: &'a str,
    action: MainTokenTransferAction<'a>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
enum MainTokenTransferAction<'a> {
    TransferMainToken(MainTokenTransferSigningData<'a>),
}

#[derive(Debug, Serialize)]
struct MainTokenTransferSigningData<'a> {
    from_account_id: &'a str,
    to_account_id: &'a str,
    amount: u64,
    nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memo: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chain_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_version: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_until_unix_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_fee: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_asset_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    application_payload_hash: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_request_id: Option<&'a str>,
}

pub fn build_main_token_transfer_signing_payload(
    transfer: &MainTokenTransfer,
    account_id: &str,
    signer_public_key_hex: &str,
) -> Result<Vec<u8>, ClientSigningError> {
    let account_id = normalize_required(account_id, "main token auth account_id")?;
    let public_key = normalize_public_key(signer_public_key_hex)?;
    let envelope = MainTokenTransferSigningEnvelope {
        version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
        operation: "transfer_main_token",
        account_id: account_id.as_str(),
        public_key: public_key.as_str(),
        action: MainTokenTransferAction::TransferMainToken(MainTokenTransferSigningData {
            from_account_id: transfer.from_account_id.as_str(),
            to_account_id: transfer.to_account_id.as_str(),
            amount: transfer.amount,
            nonce: transfer.nonce,
            asset_id: transfer.asset_id.as_deref(),
            memo: transfer.memo.as_deref(),
            chain_id: transfer.chain_id.as_deref(),
            network_id: transfer.network_id.as_deref(),
            tx_version: transfer.tx_version,
            tx_type: transfer.tx_type.as_deref(),
            valid_until_unix_ms: transfer.valid_until_unix_ms,
            max_fee: transfer.max_fee,
            fee_asset_id: transfer.fee_asset_id.as_deref(),
            application_payload_hash: transfer.application_payload_hash.as_deref(),
            client_request_id: transfer.client_request_id.as_deref(),
        }),
    };
    serde_json::to_vec(&envelope).map_err(|err| {
        ClientSigningError::InvalidRequest(format!(
            "encode main token auth signing payload failed: {err}"
        ))
    })
}

pub fn build_transfer_signing_payload(
    transfer: &MainTokenTransfer,
    account_id: &str,
    signer_public_key_hex: &str,
) -> Result<Vec<u8>, ClientSigningError> {
    build_main_token_transfer_signing_payload(transfer, account_id, signer_public_key_hex)
}

pub fn main_token_transfer_signing_payload(
    transfer: &MainTokenTransfer,
    account_id: &str,
    signer_public_key_hex: &str,
) -> Result<Vec<u8>, ClientSigningError> {
    build_main_token_transfer_signing_payload(transfer, account_id, signer_public_key_hex)
}

pub fn sign_main_token_transfer(
    transfer: &MainTokenTransfer,
    account_id: &str,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<MainTokenActionAuthProof, ClientSigningError> {
    let account_id = normalize_required(account_id, "main token auth account_id")?;
    let public_key = normalize_public_key(signer_public_key_hex)?;
    if transfer.from_account_id.trim() != account_id {
        return Err(ClientSigningError::AccountMismatch(format!(
            "main token auth account_id does not match transfer from_account_id: expected={} actual={account_id}",
            transfer.from_account_id.trim()
        )));
    }
    let expected_account = format!("oc:pk:{public_key}");
    if account_id != expected_account {
        return Err(ClientSigningError::AccountMismatch(format!(
            "main token auth account_id does not match signer public key: expected={expected_account} actual={account_id}"
        )));
    }
    let private_key =
        decode_hex_array::<32>(signer_private_key_hex, "main token auth signer private key")?;
    let signing_key = SigningKey::from_bytes(&private_key);
    let expected_public_key = hex::encode(signing_key.verifying_key().to_bytes());
    if expected_public_key != public_key {
        return Err(ClientSigningError::InvalidRequest(format!(
            "main token auth signer public key does not match private key: expected={expected_public_key} actual={public_key}"
        )));
    }
    let payload = build_main_token_transfer_signing_payload(
        transfer,
        account_id.as_str(),
        public_key.as_str(),
    )?;
    let signature: Signature = signing_key.sign(payload.as_slice());
    Ok(MainTokenActionAuthProof {
        scheme: MainTokenActionAuthScheme::Ed25519,
        account_id,
        public_key: Some(public_key),
        signature: Some(format!(
            "{}{}",
            transfer.signature_prefix(),
            hex::encode(signature.to_bytes())
        )),
        threshold: None,
        participant_signatures: Vec::new(),
    })
}

pub fn sign_main_token_transfer_request(
    transfer: &MainTokenTransferRequest,
    account_id: &str,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<MainTokenActionAuthProof, ClientSigningError> {
    sign_main_token_transfer(
        transfer,
        account_id,
        signer_public_key_hex,
        signer_private_key_hex,
    )
}

pub fn build_signed_main_token_transfer_request(
    transfer: &MainTokenTransfer,
    proof: &MainTokenActionAuthProof,
) -> Result<SignedMainTokenTransferRequest, ClientSigningError> {
    if proof.scheme != MainTokenActionAuthScheme::Ed25519 {
        return Err(ClientSigningError::InvalidRequest(
            "main token transfer request requires ed25519 proof".into(),
        ));
    }
    let public_key = normalize_public_key(proof.public_key.as_deref().ok_or_else(|| {
        ClientSigningError::InvalidRequest("main token auth public_key is required".into())
    })?)?;
    let signature = proof.signature.clone().ok_or_else(|| {
        ClientSigningError::InvalidRequest("main token auth signature is required".into())
    })?;
    Ok(SignedMainTokenTransferRequest {
        transfer: transfer.clone(),
        public_key,
        signature,
    })
}

pub fn sign_transfer(
    transfer: &MainTokenTransfer,
    account_id: &str,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<MainTokenActionAuthProof, ClientSigningError> {
    sign_main_token_transfer(
        transfer,
        account_id,
        signer_public_key_hex,
        signer_private_key_hex,
    )
}

pub fn verify_main_token_transfer_signature(
    transfer: &MainTokenTransfer,
    proof: &MainTokenActionAuthProof,
) -> Result<VerifiedMainTokenActionAuth, ClientSigningError> {
    if proof.scheme != MainTokenActionAuthScheme::Ed25519 {
        return Err(ClientSigningError::InvalidRequest(
            "main token transfer verification requires ed25519 proof".into(),
        ));
    }
    let account_id = normalize_required(proof.account_id.as_str(), "main token auth account_id")?;
    let public_key = normalize_public_key(proof.public_key.as_deref().ok_or_else(|| {
        ClientSigningError::InvalidRequest("main token auth public_key is required".into())
    })?)?;
    if transfer.from_account_id.trim() != account_id {
        return Err(ClientSigningError::AccountMismatch(format!(
            "main token auth account_id does not match transfer from_account_id: expected={} actual={account_id}",
            transfer.from_account_id.trim()
        )));
    }
    let expected_account = format!("oc:pk:{public_key}");
    if account_id != expected_account {
        return Err(ClientSigningError::AccountMismatch(format!(
            "main token auth account_id does not match signer public key: expected={expected_account} actual={account_id}"
        )));
    }
    let signature = proof.signature.as_deref().ok_or_else(|| {
        ClientSigningError::InvalidRequest("main token auth signature is required".into())
    })?;
    let signature_hex = signature
        .strip_prefix(transfer.signature_prefix())
        .ok_or_else(|| {
            ClientSigningError::InvalidSignature(format!(
                "main token auth signature is not {}",
                transfer.signature_prefix()
            ))
        })?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "main token auth signature")?;
    let verifying_key_bytes =
        decode_hex_array::<32>(public_key.as_str(), "main token auth public key")?;
    let verifying_key = VerifyingKey::from_bytes(&verifying_key_bytes).map_err(|err| {
        ClientSigningError::InvalidRequest(format!(
            "parse main token auth public key failed: {err}"
        ))
    })?;
    let payload = build_main_token_transfer_signing_payload(
        transfer,
        account_id.as_str(),
        public_key.as_str(),
    )?;
    verifying_key
        .verify(payload.as_slice(), &Signature::from_bytes(&signature_bytes))
        .map_err(|err| {
            ClientSigningError::InvalidSignature(format!(
                "verify main token auth signature failed: {err}"
            ))
        })?;
    Ok(VerifiedMainTokenActionAuth {
        account_id,
        signer_public_keys: vec![public_key],
    })
}

fn normalize_required(raw: &str, label: &str) -> Result<String, ClientSigningError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ClientSigningError::InvalidRequest(format!(
            "{label} is empty"
        )));
    }
    Ok(value.to_string())
}

fn normalize_public_key(raw: &str) -> Result<String, ClientSigningError> {
    let bytes = decode_hex_array::<32>(raw.trim(), "main token auth signer public key")?;
    Ok(hex::encode(bytes))
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], ClientSigningError> {
    let bytes = hex::decode(raw).map_err(|err| {
        ClientSigningError::InvalidRequest(format!("decode {label} failed: {err}"))
    })?;
    if bytes.len() != N {
        return Err(ClientSigningError::InvalidRequest(format!(
            "{label} length mismatch: expected {N} bytes, got {}",
            bytes.len()
        )));
    }
    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(bytes.as_slice());
    Ok(fixed)
}
