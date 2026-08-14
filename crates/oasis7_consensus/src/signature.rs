use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2::Sha256;

use super::distributed::{ActionEnvelope, WorldHeadAnnounce};
use super::ed25519_signer_policy::parse_ed25519_public_key_bytes;
use super::error::WorldError;

pub const ED25519_SIGNATURE_V1_PREFIX: &str = "ed25519:v1:";

#[derive(Debug, Clone)]
pub struct HmacSha256Signer {
    key: Vec<u8>,
}

impl HmacSha256Signer {
    pub fn new(key: impl Into<Vec<u8>>) -> Result<Self, WorldError> {
        let key = key.into();
        if key.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "hmac signer key cannot be empty".to_string(),
            });
        }
        Ok(Self { key })
    }

    pub fn sign_action(&self, action: &ActionEnvelope) -> Result<String, WorldError> {
        let mut signable = action.clone();
        signable.signature.clear();
        self.sign_value(&signable)
    }

    pub fn verify_action(&self, action: &ActionEnvelope) -> Result<(), WorldError> {
        if action.signature.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "action signature missing for {}:{}",
                    action.world_id, action.action_id
                ),
            });
        }

        let mut signable = action.clone();
        let signature_hex = signable.signature.clone();
        signable.signature.clear();
        self.verify_value(&signable, &signature_hex)
    }

    pub fn sign_head(&self, head: &WorldHeadAnnounce) -> Result<String, WorldError> {
        let mut signable = head.clone();
        signable.signature.clear();
        self.sign_value(&signable)
    }

    pub fn verify_head(&self, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        if head.signature.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "head signature missing for {}@{}",
                    head.world_id, head.height
                ),
            });
        }

        let mut signable = head.clone();
        let signature_hex = signable.signature.clone();
        signable.signature.clear();
        self.verify_value(&signable, &signature_hex)
    }

    fn sign_value<T: Serialize>(&self, value: &T) -> Result<String, WorldError> {
        let bytes = to_canonical_cbor(value)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| {
            WorldError::DistributedValidationFailed {
                reason: "failed to initialize hmac signer".to_string(),
            }
        })?;
        mac.update(&bytes);
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    fn verify_value<T: Serialize>(&self, value: &T, signature_hex: &str) -> Result<(), WorldError> {
        let bytes = to_canonical_cbor(value)?;
        let signature =
            hex::decode(signature_hex).map_err(|_| WorldError::DistributedValidationFailed {
                reason: "signature is not valid hex".to_string(),
            })?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| {
            WorldError::DistributedValidationFailed {
                reason: "failed to initialize hmac verifier".to_string(),
            }
        })?;
        mac.update(&bytes);
        mac.verify_slice(&signature)
            .map_err(|_| WorldError::DistributedValidationFailed {
                reason: "signature verification failed".to_string(),
            })
    }
}

#[derive(Debug, Clone)]
pub struct Ed25519SignatureSigner {
    signing_key: SigningKey,
    public_key_hex: String,
}

impl Ed25519SignatureSigner {
    pub fn new(private_key_hex: &str, public_key_hex: &str) -> Result<Self, WorldError> {
        let private_key = decode_hex_array::<32>(private_key_hex, "ed25519 private key")?;
        let signing_key = SigningKey::from_bytes(&private_key);
        let expected_public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        if expected_public_key_hex != public_key_hex {
            return Err(WorldError::DistributedValidationFailed {
                reason: "ed25519 public key does not match private key".to_string(),
            });
        }
        Ok(Self {
            signing_key,
            public_key_hex: public_key_hex.to_string(),
        })
    }

    pub fn public_key_hex(&self) -> &str {
        self.public_key_hex.as_str()
    }

    pub fn sign_action(&self, action: &ActionEnvelope) -> Result<String, WorldError> {
        let mut signable = action.clone();
        signable.signature.clear();
        self.sign_value(&signable)
    }

    pub fn verify_action(action: &ActionEnvelope) -> Result<String, WorldError> {
        if action.signature.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "action signature missing for {}:{}",
                    action.world_id, action.action_id
                ),
            });
        }

        let mut signable = action.clone();
        let signature = signable.signature.clone();
        signable.signature.clear();
        verify_signature_value(&signable, &signature)
    }

    pub fn sign_head(&self, head: &WorldHeadAnnounce) -> Result<String, WorldError> {
        let mut signable = head.clone();
        signable.signature.clear();
        self.sign_value(&signable)
    }

    pub fn verify_head(head: &WorldHeadAnnounce) -> Result<String, WorldError> {
        if head.signature.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "head signature missing for {}@{}",
                    head.world_id, head.height
                ),
            });
        }

        let mut signable = head.clone();
        let signature = signable.signature.clone();
        signable.signature.clear();
        verify_signature_value(&signable, &signature)
    }

    fn sign_value<T: Serialize>(&self, value: &T) -> Result<String, WorldError> {
        let payload = to_canonical_cbor(value)?;
        let signature: Signature = self.signing_key.sign(payload.as_slice());
        Ok(format!(
            "{ED25519_SIGNATURE_V1_PREFIX}{}:{}",
            self.public_key_hex,
            hex::encode(signature.to_bytes())
        ))
    }
}

fn verify_signature_value<T: Serialize>(value: &T, signature: &str) -> Result<String, WorldError> {
    let payload = to_canonical_cbor(value)?;
    let (public_key_hex, signature_hex) = parse_signature(signature)?;
    let public_key_bytes = parse_ed25519_public_key_bytes(public_key_hex, "ed25519 public key")?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "ed25519 signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| {
        WorldError::DistributedValidationFailed {
            reason: "ed25519 public key is invalid".to_string(),
        }
    })?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(payload.as_slice(), &signature)
        .map_err(|_| WorldError::DistributedValidationFailed {
            reason: "signature verification failed".to_string(),
        })?;
    Ok(hex::encode(public_key_bytes))
}

fn parse_signature(signature: &str) -> Result<(&str, &str), WorldError> {
    if !signature.starts_with(ED25519_SIGNATURE_V1_PREFIX) {
        return Err(WorldError::DistributedValidationFailed {
            reason: "signature must use ed25519:v1 format".to_string(),
        });
    }
    let encoded = &signature[ED25519_SIGNATURE_V1_PREFIX.len()..];
    let (public_key_hex, signature_hex) =
        encoded
            .split_once(':')
            .ok_or_else(|| WorldError::DistributedValidationFailed {
                reason: "signature must include signer public key and signature hex".to_string(),
            })?;
    if public_key_hex.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "signature signer public key cannot be empty".to_string(),
        });
    }
    if signature_hex.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "signature hex cannot be empty".to_string(),
        });
    }
    Ok((public_key_hex, signature_hex))
}

fn decode_hex_array<const N: usize>(input: &str, field: &str) -> Result<[u8; N], WorldError> {
    let bytes = hex::decode(input).map_err(|_| WorldError::DistributedValidationFailed {
        reason: format!("{field} must be valid hex"),
    })?;
    bytes
        .try_into()
        .map_err(|_| WorldError::DistributedValidationFailed {
            reason: format!("{field} must be {N}-byte hex"),
        })
}

fn to_canonical_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, WorldError> {
    super::util::to_canonical_cbor(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_action() -> ActionEnvelope {
        ActionEnvelope {
            world_id: "w1".to_string(),
            action_id: "a1".to_string(),
            actor_id: "agent-1".to_string(),
            action_kind: "move".to_string(),
            payload_cbor: vec![1, 2],
            payload_hash: "hash".to_string(),
            nonce: 7,
            timestamp_ms: 42,
            intent_batch_hash: String::new(),
            idempotency_key: String::new(),
            zone_id: String::new(),
            signature: String::new(),
        }
    }

    fn demo_head() -> WorldHeadAnnounce {
        WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 3,
            block_hash: "b3".to_string(),
            state_root: "s3".to_string(),
            timestamp_ms: 99,
            signature: String::new(),
        }
    }

    fn ed25519_signer() -> Ed25519SignatureSigner {
        let private_key_hex = hex::encode([7_u8; 32]);
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        Ed25519SignatureSigner::new(private_key_hex.as_str(), public_key_hex.as_str())
            .expect("ed25519 signer")
    }

    #[test]
    fn hmac_sign_and_verify_action_roundtrip() {
        let signer = HmacSha256Signer::new(b"demo-key".to_vec()).expect("signer");
        let mut action = demo_action();
        action.signature = signer.sign_action(&action).expect("sign");
        signer.verify_action(&action).expect("verify");
    }

    #[test]
    fn hmac_sign_and_verify_head_roundtrip() {
        let signer = HmacSha256Signer::new(b"demo-key".to_vec()).expect("signer");
        let mut head = demo_head();
        head.signature = signer.sign_head(&head).expect("sign");
        signer.verify_head(&head).expect("verify");
    }

    #[test]
    fn hmac_verify_rejects_tampered_action() {
        let signer = HmacSha256Signer::new(b"demo-key".to_vec()).expect("signer");
        let mut action = demo_action();
        action.signature = signer.sign_action(&action).expect("sign");
        action.payload_hash = "tampered".to_string();

        let result = signer.verify_action(&action);
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    #[test]
    fn ed25519_sign_and_verify_action_roundtrip() {
        let signer = ed25519_signer();
        let mut action = demo_action();
        action.signature = signer.sign_action(&action).expect("sign");
        let signer_public_key =
            Ed25519SignatureSigner::verify_action(&action).expect("verify action");
        assert_eq!(signer_public_key, signer.public_key_hex());
    }

    #[test]
    fn ed25519_sign_and_verify_head_roundtrip() {
        let signer = ed25519_signer();
        let mut head = demo_head();
        head.signature = signer.sign_head(&head).expect("sign");
        let signer_public_key = Ed25519SignatureSigner::verify_head(&head).expect("verify head");
        assert_eq!(signer_public_key, signer.public_key_hex());
    }

    #[test]
    fn ed25519_verify_rejects_tampered_action() {
        let signer = ed25519_signer();
        let mut action = demo_action();
        action.signature = signer.sign_action(&action).expect("sign");
        action.payload_hash = "tampered".to_string();
        let result = Ed25519SignatureSigner::verify_action(&action);
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    #[test]
    fn ed25519_verify_rejects_non_ed25519_signature_format() {
        let mut action = demo_action();
        action.signature = "deadbeef".to_string();
        let result = Ed25519SignatureSigner::verify_action(&action);
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    #[test]
    fn ed25519_verify_action_normalizes_uppercase_signer_public_key() {
        let signer = ed25519_signer();
        let mut action = demo_action();
        action.signature = signer.sign_action(&action).expect("sign");
        let encoded = action
            .signature
            .strip_prefix(ED25519_SIGNATURE_V1_PREFIX)
            .expect("ed25519 signature prefix");
        let (public_key_hex, signature_hex) = encoded
            .split_once(':')
            .expect("ed25519 signer and signature hex");
        action.signature = format!(
            "{ED25519_SIGNATURE_V1_PREFIX}{}:{signature_hex}",
            public_key_hex.to_uppercase()
        );

        let signer_public_key = Ed25519SignatureSigner::verify_action(&action).expect("verify");
        assert_eq!(signer_public_key, signer.public_key_hex());
    }
}
