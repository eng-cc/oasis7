//! Password-encrypted transport for private key and archive bytes.
//! Header parameters are fixed before any KDF work; caller owns passphrase erasure.
use crate::error::SignerError;
use argon2::{Algorithm, Argon2, Block, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{AeadInPlace, KeyInit},
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const MAX_ENVELOPE_BYTES: usize = 32 * 1024 * 1024;
// Reserve JSON/header and base64 expansion space before allocating or deriving.
pub const MAX_BACKUP_ENVELOPE_BYTES: usize = 2 * 1024 * 1024 * 1024;
fn envelope_limit(kind: &str) -> usize {
    if kind == "backup" {
        MAX_BACKUP_ENVELOPE_BYTES
    } else {
        MAX_ENVELOPE_BYTES
    }
}
fn plaintext_limit(kind: &str) -> usize {
    (envelope_limit(kind) - 4096) / 4 * 3 - 16
}
#[cfg(test)]
const MAX_PLAINTEXT_BYTES: usize = (MAX_ENVELOPE_BYTES - 4096) / 4 * 3 - 16;
const MEMORY_KIB: u32 = 65536;
const ITERATIONS: u32 = 3;
const LANES: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    version: String,
    kind: String,
    cipher: String,
    kdf: String,
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
    salt: String,
    nonce: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    header: Header,
    ciphertext: String,
}
fn invalid() -> SignerError {
    SignerError::CryptoOrBindingInvalid
}
fn validate_inputs(kind: &str, passphrase: &[u8]) -> Result<(), SignerError> {
    if kind.is_empty()
        || kind.len() > 64
        || !kind
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || !(12..=1024).contains(&passphrase.len())
    {
        return Err(SignerError::InvalidInput(
            "invalid envelope kind or passphrase length".into(),
        ));
    }
    Ok(())
}
fn derive(passphrase: &[u8], salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, SignerError> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, LANES, Some(32)).map_err(|_| invalid())?;
    let mut key = Zeroizing::new([0u8; 32]);
    let mut memory = Zeroizing::new(vec![Block::default(); params.block_count()]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into_with_memory(passphrase, salt, &mut *key, &mut *memory)
        .map_err(|_| invalid())?;
    Ok(key)
}
/// Encrypt bytes with a random 128-bit salt and 192-bit nonce. No plaintext is serialized.
pub fn seal(kind: &str, plaintext: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, SignerError> {
    validate_inputs(kind, passphrase)?;
    if plaintext.len() > plaintext_limit(kind) {
        return Err(invalid());
    }
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut salt).map_err(|_| invalid())?;
    getrandom::fill(&mut nonce).map_err(|_| invalid())?;
    let header = Header {
        version: "oasis7.encrypted_envelope.v1".into(),
        kind: kind.into(),
        cipher: "XChaCha20Poly1305".into(),
        kdf: "Argon2id-v19".into(),
        memory_kib: MEMORY_KIB,
        iterations: ITERATIONS,
        lanes: LANES,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
    };
    let aad = serde_json::to_vec(&header).map_err(|_| invalid())?;
    let key = derive(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&*key).map_err(|_| invalid())?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(plaintext.len() + 16));
    bytes.extend_from_slice(plaintext);
    cipher
        .encrypt_in_place(XNonce::from_slice(&nonce), &aad, &mut *bytes)
        .map_err(|_| invalid())?;
    let output = serde_json::to_vec(&Envelope {
        header,
        ciphertext: STANDARD.encode(&*bytes),
    })
    .map_err(|_| invalid())?;
    if output.len() > envelope_limit(kind) {
        return Err(invalid());
    }
    Ok(output)
}
/// Authenticate the complete header and ciphertext before returning zeroizing plaintext.
pub fn open(
    expected_kind: &str,
    envelope: &[u8],
    passphrase: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SignerError> {
    validate_inputs(expected_kind, passphrase)?;
    if envelope.len() > envelope_limit(expected_kind) {
        return Err(invalid());
    }
    let parsed: Envelope = serde_json::from_slice(envelope).map_err(|_| invalid())?;
    let h = &parsed.header;
    if h.version != "oasis7.encrypted_envelope.v1"
        || h.kind != expected_kind
        || h.cipher != "XChaCha20Poly1305"
        || h.kdf != "Argon2id-v19"
        || h.memory_kib != MEMORY_KIB
        || h.iterations != ITERATIONS
        || h.lanes != LANES
    {
        return Err(invalid());
    }
    let salt: [u8; 16] = STANDARD
        .decode(&h.salt)
        .map_err(|_| invalid())?
        .try_into()
        .map_err(|_| invalid())?;
    let nonce: [u8; 24] = STANDARD
        .decode(&h.nonce)
        .map_err(|_| invalid())?
        .try_into()
        .map_err(|_| invalid())?;
    let mut bytes = Zeroizing::new(STANDARD.decode(&parsed.ciphertext).map_err(|_| invalid())?);
    if bytes.len() < 16 || bytes.len() > plaintext_limit(expected_kind) + 16 {
        return Err(invalid());
    }
    let aad = serde_json::to_vec(h).map_err(|_| invalid())?;
    let key = derive(passphrase, &salt)?;
    XChaCha20Poly1305::new_from_slice(&*key)
        .map_err(|_| invalid())?
        .decrypt_in_place(XNonce::from_slice(&nonce), &aad, &mut *bytes)
        .map_err(|_| invalid())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PASS: &[u8] = b"test passphrase with enough length";
    #[test]
    fn roundtrip_randomized_and_wrong_password() {
        let a = seal("key", b"secret", PASS).unwrap();
        let b = seal("key", b"secret", PASS).unwrap();
        assert_ne!(a, b);
        assert_eq!(&**open("key", &a, PASS).unwrap(), b"secret");
        assert!(open("key", &a, b"different password").is_err());
        assert!(open("archive", &a, PASS).is_err());
    }
    #[test]
    fn tamper_and_closed_header() {
        let original = seal("archive", b"private archive", PASS).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        value["header"]["memory_kib"] = 4294967295u32.into();
        assert!(open("archive", &serde_json::to_vec(&value).unwrap(), PASS).is_err());
        value = serde_json::from_slice(&original).unwrap();
        value["header"]["extra"] = true.into();
        assert!(open("archive", &serde_json::to_vec(&value).unwrap(), PASS).is_err());
        value = serde_json::from_slice(&original).unwrap();
        let ciphertext = value["ciphertext"].as_str().unwrap();
        let mut bytes = STANDARD.decode(ciphertext).unwrap();
        bytes[0] ^= 1;
        value["ciphertext"] = STANDARD.encode(bytes).into();
        assert!(open("archive", &serde_json::to_vec(&value).unwrap(), PASS).is_err());
        value = serde_json::from_slice(&original).unwrap();
        value["header"]["kind"] = "key".into();
        assert!(open("key", &serde_json::to_vec(&value).unwrap(), PASS).is_err());
    }
    #[test]
    fn backup_archive_exceeds_key_envelope_limit_without_widening_key_import() {
        let plaintext = Zeroizing::new(vec![7; 25 * 1024 * 1024]);
        assert!(seal("key", &plaintext, PASS).is_err());
        let envelope = seal("backup", &plaintext, PASS).unwrap();
        assert!(envelope.len() > MAX_ENVELOPE_BYTES);
        assert!(open("key", &envelope, PASS).is_err());
        assert_eq!(*open("backup", &envelope, PASS).unwrap(), *plaintext);
    }
    #[test]
    fn input_bounds_before_crypto() {
        assert!(seal("key", b"data", b"short").is_err());
        assert!(seal("key", b"data", &[1; 1025]).is_err());
        assert!(seal("bad kind", b"data", PASS).is_err());
        assert!(open("key", &vec![0; MAX_ENVELOPE_BYTES + 1], PASS).is_err());
        assert!(seal("key", &vec![0; MAX_PLAINTEXT_BYTES + 1], PASS).is_err());
    }
}
