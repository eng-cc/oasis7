//! Signer compatibility adapter for the shared encrypted transport.
use crate::error::SignerError;
use oasis7_secure_envelope::EnvelopeError;
use zeroize::Zeroizing;

pub use oasis7_secure_envelope::{MAX_BACKUP_ENVELOPE_BYTES, MAX_ENVELOPE_BYTES};
fn map_error(error: EnvelopeError) -> SignerError {
    match error {
        EnvelopeError::InvalidInput => {
            SignerError::InvalidInput("invalid envelope kind or passphrase length".into())
        }
        EnvelopeError::InvalidEnvelope => SignerError::CryptoOrBindingInvalid,
    }
}
pub fn seal(kind: &str, plaintext: &[u8], passphrase: &[u8]) -> Result<Vec<u8>, SignerError> {
    oasis7_secure_envelope::seal(kind, plaintext, passphrase).map_err(map_error)
}
pub fn open(
    kind: &str,
    envelope: &[u8],
    passphrase: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SignerError> {
    oasis7_secure_envelope::open(kind, envelope, passphrase).map_err(map_error)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapter_preserves_error_categories_and_shared_interoperability() {
        let pass = b"synthetic adapter passphrase";
        assert!(matches!(
            seal("key", b"data", b"short"),
            Err(SignerError::InvalidInput(_))
        ));
        assert!(matches!(
            open("key", b"invalid", pass),
            Err(SignerError::CryptoOrBindingInvalid)
        ));
        let shared = oasis7_secure_envelope::seal("key", b"synthetic seed", pass).unwrap();
        assert_eq!(&**open("key", &shared, pass).unwrap(), b"synthetic seed");
        let adapter = seal("archive", b"synthetic archive", pass).unwrap();
        assert_eq!(
            &**oasis7_secure_envelope::open("archive", &adapter, pass).unwrap(),
            b"synthetic archive"
        );
    }
}
