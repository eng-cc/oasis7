//! Volatile, one-use secret channel. No Serialize or Debug implementation.
use crate::{Error, Result};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
struct Secret {
    window: String,
    operation: String,
    purpose: String,
    deadline: Instant,
    bytes: Zeroizing<Vec<u8>>,
}
#[derive(Default)]
pub struct SecretStore {
    inner: Mutex<HashMap<String, Secret>>,
}
impl SecretStore {
    pub fn insert(
        &self,
        window: &str,
        operation: &str,
        purpose: &str,
        bytes: Vec<u8>,
    ) -> Result<String> {
        let bytes = Zeroizing::new(bytes);
        if window.is_empty()
            || operation.is_empty()
            || operation.len() > 256
            || purpose.is_empty()
            || purpose.len() > 128
            || bytes.is_empty()
            || bytes.len() > 4096
        {
            return Err(Error::Invalid(
                "bounded secret and explicit window/operation/purpose required".into(),
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("secret channel unavailable".into()))?;
        inner.retain(|_, secret| secret.deadline > Instant::now());
        if inner.len() >= 32 {
            return Err(Error::Invalid("secret channel capacity reached".into()));
        }
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy)
            .map_err(|_| Error::Invalid("secure secret identity unavailable".into()))?;
        let token = entropy
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        inner.insert(
            token.clone(),
            Secret {
                window: window.into(),
                operation: operation.into(),
                purpose: purpose.into(),
                deadline: Instant::now() + Duration::from_secs(120),
                bytes,
            },
        );
        Ok(token)
    }
    pub fn take(
        &self,
        token: &str,
        window: &str,
        operation: &str,
        purpose: &str,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("secret channel unavailable".into()))?;
        let secret = inner
            .get(token)
            .ok_or_else(|| Error::Invalid("secret handle missing or consumed".into()))?;
        if secret.window != window || secret.operation != operation || secret.purpose != purpose {
            return Err(Error::Invalid(
                "secret handle does not belong to this window/operation/purpose".into(),
            ));
        }
        let secret = inner.remove(token).unwrap();
        if secret.deadline <= Instant::now() {
            return Err(Error::Invalid("secret handle expired".into()));
        }
        Ok(secret.bytes)
    }
    pub fn cancel(&self, window: &str, operation: &str) -> Result<()> {
        self.inner
            .lock()
            .map_err(|_| Error::Invalid("secret channel unavailable".into()))?
            .retain(|_, secret| secret.window != window || secret.operation != operation);
        Ok(())
    }
    pub fn clear(&self) -> Result<()> {
        self.inner
            .lock()
            .map_err(|_| Error::Invalid("secret channel unavailable".into()))?
            .clear();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_are_scoped_once_and_cancelled() {
        let store = SecretStore::default();
        let token = store
            .insert("main", "operation", "backup", b"secret".to_vec())
            .unwrap();
        assert!(store.take(&token, "other", "operation", "backup").is_err());
        assert!(store.take(&token, "main", "other", "backup").is_err());
        assert!(store.take(&token, "main", "operation", "restore").is_err());
        assert_eq!(
            &*store.take(&token, "main", "operation", "backup").unwrap(),
            b"secret"
        );
        assert!(store.take(&token, "main", "operation", "backup").is_err());
        let token = store
            .insert("main", "operation", "backup", b"secret".to_vec())
            .unwrap();
        store.cancel("main", "operation").unwrap();
        assert!(store.take(&token, "main", "operation", "backup").is_err());
    }
    #[test]
    fn expired_secret_is_destroyed_without_consumption() {
        let store = SecretStore::default();
        let token = store
            .insert("main", "operation", "backup", b"secret".to_vec())
            .unwrap();
        store
            .inner
            .lock()
            .unwrap()
            .get_mut(&token)
            .unwrap()
            .deadline = Instant::now() - Duration::from_secs(1);
        assert!(store.take(&token, "main", "operation", "backup").is_err());
        assert!(store.inner.lock().unwrap().is_empty());
    }
}
