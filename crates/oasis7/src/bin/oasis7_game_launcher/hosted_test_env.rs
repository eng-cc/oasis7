//! Shared environment fixture for hosted issuer readers and writers in this binary.
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, MutexGuard};

const ISSUER: &str = oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV;
const LOGIN: &str = "OASIS7_HOSTED_TEST_LOGIN_ENABLED";
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct SavedEnvironment([(&'static str, Option<OsString>); 2]);

impl SavedEnvironment {
    fn capture() -> Self {
        Self([
            (ISSUER, std::env::var_os(ISSUER)),
            (LOGIN, std::env::var_os(LOGIN)),
        ])
    }
}

impl Drop for SavedEnvironment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            set_value(key, value.as_deref());
        }
    }
}

fn set_value(key: &str, value: Option<&OsStr>) {
    // SAFETY: hosted fixtures coordinate these keys through ENV_LOCK and keep
    // their HTTP workers stopped before releasing the fixture.
    unsafe {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
}

pub(crate) struct HostedTestEnvironment {
    // Field drop order restores values before releasing the shared lock.
    _saved: SavedEnvironment,
    _lock: MutexGuard<'static, ()>,
}

impl HostedTestEnvironment {
    pub(crate) fn acquire() -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self {
            _saved: SavedEnvironment::capture(),
            _lock: lock,
        }
    }

    pub(crate) fn set_issuer(&self, value: Option<&OsStr>) {
        set_value(ISSUER, value);
    }

    pub(crate) fn set_login_enabled(&self, enabled: bool) {
        set_value(LOGIN, enabled.then_some(OsStr::new("1")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc;

    #[test]
    fn hosted_env_restores_absent_and_present_values() {
        let env = HostedTestEnvironment::acquire();
        for prior in [None, Some(OsStr::new("prior issuer"))] {
            env.set_issuer(prior);
            env.set_login_enabled(false);
            {
                let _saved = SavedEnvironment::capture();
                env.set_issuer(Some(OsStr::new("replacement")));
                env.set_login_enabled(true);
            }
            assert_eq!(std::env::var_os(ISSUER).as_deref(), prior);
            assert_eq!(std::env::var_os(LOGIN), None);
        }
        env.set_login_enabled(true);
        {
            let _saved = SavedEnvironment::capture();
            env.set_login_enabled(false);
        }
        assert_eq!(std::env::var_os(LOGIN), Some(OsString::from("1")));
    }

    #[cfg(unix)]
    #[test]
    fn hosted_env_restores_non_unicode_value() {
        use std::os::unix::ffi::OsStringExt;
        let env = HostedTestEnvironment::acquire();
        let prior = OsString::from_vec(vec![0xff, b'x']);
        env.set_issuer(Some(&prior));
        {
            let _saved = SavedEnvironment::capture();
            env.set_issuer(None);
        }
        assert_eq!(std::env::var_os(ISSUER), Some(prior));
    }

    #[test]
    fn hosted_env_restores_after_unwind_and_recovers_poisoned_lock() {
        // Hold the outer lock while checking unwind restoration so another
        // fixture cannot race the assertions. Then separately poison this lock.
        {
            let env = HostedTestEnvironment::acquire();
            env.set_issuer(Some(OsStr::new("before unwind")));
            env.set_login_enabled(false);
            assert!(
                catch_unwind(AssertUnwindSafe(|| {
                    let _saved = SavedEnvironment::capture();
                    env.set_issuer(None);
                    env.set_login_enabled(true);
                    panic!("fixture panic");
                }))
                .is_err()
            );
            assert_eq!(
                std::env::var_os(ISSUER),
                Some(OsString::from("before unwind"))
            );
            assert_eq!(std::env::var_os(LOGIN), None);
        }
        let mut prior = None;
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let env = HostedTestEnvironment::acquire();
                prior = Some((std::env::var_os(ISSUER), std::env::var_os(LOGIN)));
                env.set_issuer(Some(OsStr::new("changed before panic")));
                env.set_login_enabled(true);
                panic!("poison fixture lock");
            }))
            .is_err()
        );
        let env = HostedTestEnvironment::acquire();
        let (issuer, login) = prior.unwrap();
        assert_eq!(std::env::var_os(ISSUER), issuer);
        assert_eq!(std::env::var_os(LOGIN), login);
        env.set_issuer(Some(OsStr::new("usable after poison")));
        assert_eq!(
            std::env::var_os(ISSUER),
            Some(OsString::from("usable after poison"))
        );
    }

    #[test]
    fn hosted_env_contenders_cannot_replace_the_current_issuer() {
        let env = HostedTestEnvironment::acquire();
        let owner_key = hex::encode([71_u8; 32]);
        let contender_key = hex::encode([81_u8; 32]);
        let prior = std::env::var_os(ISSUER);
        env.set_issuer(Some(OsStr::new(&owner_key)));
        let (attempted_tx, attempted_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let contender = std::thread::spawn(move || {
            assert!(matches!(
                ENV_LOCK.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            attempted_tx.send(()).unwrap();
            let env = HostedTestEnvironment::acquire();
            env.set_issuer(Some(OsStr::new(&contender_key)));
            let public = oasis7::viewer::derive_hosted_registration_issuer_public_key(
                &std::env::var(ISSUER).unwrap(),
            )
            .unwrap();
            assert_eq!(
                public,
                hex::encode(
                    ed25519_dalek::SigningKey::from_bytes(&[81; 32])
                        .verifying_key()
                        .to_bytes()
                )
            );
            acquired_tx.send(()).unwrap();
        });
        attempted_rx.recv().unwrap();
        assert!(matches!(
            acquired_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert_eq!(std::env::var(ISSUER).unwrap(), owner_key);
        drop(env);
        acquired_rx.recv().unwrap();
        contender.join().unwrap();
        let _env = HostedTestEnvironment::acquire();
        assert_eq!(std::env::var_os(ISSUER), prior);
    }
}
