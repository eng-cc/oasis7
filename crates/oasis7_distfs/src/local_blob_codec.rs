use std::io::{Read, Write};

use sha2::{Digest, Sha256};

use crate::{HashAlgorithm, WorldError, validate_hash};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobReadReceipt {
    pub decoded_bytes: usize,
    pub content_hash: String,
}

/// Decode and verify a raw or O7CBLOB1 blob from an already authorized reader.
/// Output is provisional until this function succeeds; callers must discard it
/// on error. No paths, caches, pins, or files are opened or written by this codec.
pub fn decode_local_blob_into<R: Read, W: Write>(
    algorithm: HashAlgorithm,
    expected_hash: &str,
    mut input: R,
    mut output: W,
    max_decoded_bytes: usize,
) -> Result<BlobReadReceipt, WorldError> {
    validate_hash(expected_hash)?;
    let mut prefix = Vec::with_capacity(80);
    input.by_ref().take(80).read_to_end(&mut prefix)?;
    let compressed = prefix.starts_with(b"O7CBLOB1");
    let expected_len = if compressed {
        if prefix.len() != 80 || &prefix[16..80] != expected_hash.as_bytes() {
            return Err(invalid("invalid compressed blob header"));
        }
        let length = u64::from_le_bytes(prefix[8..16].try_into().expect("fixed header"));
        if length > max_decoded_bytes as u64 {
            return Err(invalid(
                "ResourceLimited: decoded blob length exceeds budget",
            ));
        }
        Some(length)
    } else {
        None
    };
    let raw = std::io::Cursor::new(prefix).chain(input);
    let mut reader: Box<dyn Read + '_> = if compressed {
        // Drop the consumed header; the remainder is the zstd stream.
        let mut raw = raw;
        std::io::copy(&mut raw.by_ref().take(80), &mut std::io::sink())?;
        Box::new(zstd::stream::read::Decoder::new(raw)?)
    } else {
        Box::new(raw)
    };
    let mut blake3 = blake3::Hasher::new();
    let mut sha256 = Sha256::new();
    let mut count = 0usize;
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let length = reader.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        count = count
            .checked_add(length)
            .ok_or_else(|| invalid("ResourceLimited: blob size overflow"))?;
        if count > max_decoded_bytes {
            return Err(invalid("ResourceLimited: decoded blob exceeds budget"));
        }
        blake3.update(&buffer[..length]);
        sha256.update(&buffer[..length]);
        output.write_all(&buffer[..length])?;
    }
    if expected_len.is_some_and(|length| length != count as u64) {
        return Err(invalid("compressed blob decoded length mismatch"));
    }
    let actual = match algorithm {
        HashAlgorithm::Blake3 => blake3.finalize().to_hex().to_string(),
        HashAlgorithm::Sha256 => hex::encode(sha256.finalize()),
    };
    if actual != expected_hash {
        return Err(WorldError::BlobHashMismatch {
            expected: expected_hash.to_string(),
            actual,
        });
    }
    Ok(BlobReadReceipt {
        decoded_bytes: count,
        content_hash: actual,
    })
}

fn invalid(reason: &str) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifies_raw_and_compressed_and_enforces_exact_budget() {
        let bytes = vec![b'a'; 4096];
        let hash = crate::blake3_hex(&bytes);
        let mut encoded = b"O7CBLOB1".to_vec();
        encoded.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        encoded.extend_from_slice(hash.as_bytes());
        encoded.extend(zstd::stream::encode_all(bytes.as_slice(), 3).unwrap());
        for input in [bytes.clone(), encoded] {
            let mut output = Vec::new();
            assert_eq!(
                decode_local_blob_into(
                    HashAlgorithm::Blake3,
                    &hash,
                    input.as_slice(),
                    &mut output,
                    bytes.len()
                )
                .unwrap()
                .decoded_bytes,
                bytes.len()
            );
            assert_eq!(output, bytes);
            assert!(
                decode_local_blob_into(
                    HashAlgorithm::Blake3,
                    &hash,
                    input.as_slice(),
                    std::io::sink(),
                    bytes.len() - 1
                )
                .is_err()
            );
        }
    }
    #[test]
    fn rejects_truncation_and_wrong_hash() {
        let hash = crate::blake3_hex(b"hello");
        assert!(
            decode_local_blob_into(
                HashAlgorithm::Blake3,
                &hash,
                &b"hell"[..],
                std::io::sink(),
                10
            )
            .is_err()
        );
        assert!(
            decode_local_blob_into(
                HashAlgorithm::Blake3,
                &hash,
                &b"O7CBLOB1"[..],
                std::io::sink(),
                10
            )
            .is_err()
        );
    }
}
