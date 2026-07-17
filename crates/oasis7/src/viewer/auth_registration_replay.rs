use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::atomic_file::{platform_atomic_replace, sync_parent_directory};
use super::registration_replay_lock::RegistrationReplayProcessLock;

pub const HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV: &str =
    "OASIS7_HOSTED_REGISTRATION_REPLAY_LEDGER_PATH";
#[cfg(test)]
pub(crate) const HOSTED_REGISTRATION_REPLAY_CLAIM_BARRIER_DIR_ENV: &str =
    "OASIS7_HOSTED_REGISTRATION_REPLAY_CLAIM_BARRIER_DIR";
static HOSTED_REGISTRATION_REPLAY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn ensure_registration_grant_nonce_unused(nonce: &str) -> Result<(), String> {
    let path = registration_replay_ledger_path();
    let _guard = HOSTED_REGISTRATION_REPLAY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "registration grant replay ledger lock poisoned".to_string())?;
    let _process_lock = RegistrationReplayProcessLock::acquire(path.as_path())?;
    if read_registration_replay_ledger(path.as_path())?
        .claims
        .contains_key(nonce)
    {
        return Err("registration grant replay detected".to_string());
    }
    Ok(())
}

pub(crate) fn consume_registration_grant_nonce(nonce: &str) -> Result<(), String> {
    let path = registration_replay_ledger_path();
    let _guard = HOSTED_REGISTRATION_REPLAY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "registration grant replay ledger lock poisoned".to_string())?;
    let _process_lock = RegistrationReplayProcessLock::acquire(path.as_path())?;
    let mut ledger = read_registration_replay_ledger(path.as_path())?;
    if ledger
        .claims
        .insert(nonce.to_string(), RegistrationGrantClaim::Consumed)
        .is_some()
    {
        return Err("registration grant replay detected".to_string());
    }
    atomic_write_replay_ledger(path.as_path(), &ledger)
}

pub(crate) fn claim_registration_grant_nonce_for_recovery(
    nonce: &str,
    recovery_dir: &Path,
) -> Result<(), String> {
    let path = registration_replay_ledger_path();
    let owner = recovery_claim_owner(recovery_dir)?;
    #[cfg(test)]
    wait_at_registration_replay_claim_barrier(owner.as_str())?;
    let _guard = HOSTED_REGISTRATION_REPLAY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "registration grant replay ledger lock poisoned".to_string())?;
    let _process_lock = RegistrationReplayProcessLock::acquire(path.as_path())?;
    let mut ledger = read_registration_replay_ledger(path.as_path())?;
    match ledger.claims.get(nonce) {
        Some(RegistrationGrantClaim::Recovery { owner: existing }) if existing == &owner => {
            return Ok(());
        }
        Some(_) => return Err("registration grant replay detected".to_string()),
        None => {}
    }
    ledger.claims.insert(
        nonce.to_string(),
        RegistrationGrantClaim::Recovery { owner },
    );
    atomic_write_replay_ledger(path.as_path(), &ledger)
}

#[cfg(test)]
fn wait_at_registration_replay_claim_barrier(owner: &str) -> Result<(), String> {
    use std::time::{Duration, Instant};

    let Some(barrier_dir) =
        std::env::var_os(HOSTED_REGISTRATION_REPLAY_CLAIM_BARRIER_DIR_ENV).map(PathBuf::from)
    else {
        return Ok(());
    };
    fs::create_dir_all(&barrier_dir)
        .map_err(|err| format!("create registration replay test barrier failed: {err}"))?;
    fs::write(barrier_dir.join(format!("{owner}.ready")), b"ready")
        .map_err(|err| format!("signal registration replay test barrier failed: {err}"))?;
    let release = barrier_dir.join("release");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        if Instant::now() >= deadline {
            return Err("registration replay test barrier timed out".to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

pub fn preflight_hosted_registration_replay_ledger() -> Result<(), String> {
    let path = registration_replay_ledger_path();
    let _guard = HOSTED_REGISTRATION_REPLAY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "registration grant replay ledger lock poisoned".to_string())?;
    let _process_lock = RegistrationReplayProcessLock::acquire(path.as_path())?;
    let ledger = read_registration_replay_ledger(path.as_path())?;
    atomic_write_replay_ledger(path.as_path(), &ledger)
}

fn registration_replay_ledger_path() -> PathBuf {
    std::env::var_os(HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".oasis7-hosted-registration-replay.json"))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RegistrationReplayLedger {
    claims: BTreeMap<String, RegistrationGrantClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum RegistrationGrantClaim {
    Consumed,
    Recovery { owner: String },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RegistrationReplayLedgerWire {
    Legacy(BTreeSet<String>),
    Current(RegistrationReplayLedger),
}

fn read_registration_replay_ledger(path: &Path) -> Result<RegistrationReplayLedger, String> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes)
            .map_err(|err| format!("decode registration grant replay ledger failed: {err}"))?
        {
            RegistrationReplayLedgerWire::Legacy(consumed) => Ok(RegistrationReplayLedger {
                claims: consumed
                    .into_iter()
                    .map(|nonce| (nonce, RegistrationGrantClaim::Consumed))
                    .collect(),
            }),
            RegistrationReplayLedgerWire::Current(ledger) => Ok(ledger),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(RegistrationReplayLedger::default())
        }
        Err(err) => Err(format!(
            "read registration grant replay ledger failed: {err}"
        )),
    }
}

fn atomic_write_replay_ledger(
    path: &Path,
    ledger: &RegistrationReplayLedger,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|err| format!("create registration replay directory failed: {err}"))?;
    let temp = parent.join(format!(".registration-replay-{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(ledger)
        .map_err(|err| format!("encode registration replay ledger failed: {err}"))?;
    let mut temp_file = fs::File::create(&temp)
        .map_err(|err| format!("create registration replay ledger failed: {err}"))?;
    temp_file
        .write_all(bytes.as_slice())
        .map_err(|err| format!("write registration replay ledger failed: {err}"))?;
    temp_file
        .sync_all()
        .map_err(|err| format!("sync registration replay ledger failed: {err}"))?;
    platform_atomic_replace(&temp, path)
        .map_err(|err| format!("replace registration replay ledger failed: {err}"))?;
    sync_parent_directory(parent)
        .map_err(|err| format!("sync registration replay ledger directory failed: {err}"))
}

fn recovery_claim_owner(recovery_dir: &Path) -> Result<String, String> {
    let absolute = if recovery_dir.is_absolute() {
        recovery_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| format!("resolve recovery claim owner failed: {err}"))?
            .join(recovery_dir)
    };
    Ok(hex::encode(Sha256::digest(
        absolute.to_string_lossy().as_bytes(),
    )))
}
