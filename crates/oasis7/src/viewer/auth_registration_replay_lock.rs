use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::atomic_file::sync_parent_directory;

const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(5);
const INCOMPLETE_LOCK_GRACE: Duration = Duration::from_secs(1);
static LOCK_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize)]
struct RegistrationReplayLockOwner {
    pid: u32,
    token: String,
}

pub(super) struct RegistrationReplayProcessLock {
    lock_dir: PathBuf,
    token: String,
}

impl RegistrationReplayProcessLock {
    pub(super) fn acquire(ledger_path: &Path) -> Result<Self, String> {
        let lock_dir = replay_lock_dir(ledger_path);
        let parent = lock_dir.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|err| {
            format!("create registration replay ledger lock parent failed: {err}")
        })?;
        let deadline = Instant::now() + LOCK_WAIT_TIMEOUT;
        loop {
            match fs::create_dir(&lock_dir) {
                Ok(()) => {
                    let token = new_lock_token();
                    if let Err(error) = write_lock_owner(&lock_dir, token.as_str()) {
                        let _ = fs::remove_dir_all(&lock_dir);
                        return Err(error);
                    }
                    sync_parent_directory(parent).map_err(|err| {
                        format!("sync registration replay ledger lock parent failed: {err}")
                    })?;
                    return Ok(Self { lock_dir, token });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if reclaim_stale_lock(&lock_dir)? {
                        continue;
                    }
                    if Instant::now() >= deadline {
                        return Err(
                            "registration replay ledger lock is held by a live process".to_string()
                        );
                    }
                    std::thread::sleep(LOCK_RETRY_INTERVAL);
                }
                Err(err) => {
                    return Err(format!(
                        "create registration replay ledger process lock failed: {err}"
                    ));
                }
            }
        }
    }
}

impl Drop for RegistrationReplayProcessLock {
    fn drop(&mut self) {
        let owner_path = self.lock_dir.join("owner.json");
        let owns_lock = fs::read(owner_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<RegistrationReplayLockOwner>(&bytes).ok())
            .is_some_and(|owner| owner.token == self.token);
        if owns_lock {
            let parent = self.lock_dir.parent().unwrap_or_else(|| Path::new("."));
            if fs::remove_dir_all(&self.lock_dir).is_ok() {
                let _ = sync_parent_directory(parent);
            }
        }
    }
}

fn replay_lock_dir(ledger_path: &Path) -> PathBuf {
    let mut name: OsString = ledger_path.as_os_str().to_owned();
    name.push(".lock");
    PathBuf::from(name)
}

fn write_lock_owner(lock_dir: &Path, token: &str) -> Result<(), String> {
    let owner = RegistrationReplayLockOwner {
        pid: std::process::id(),
        token: token.to_string(),
    };
    let bytes = serde_json::to_vec(&owner)
        .map_err(|err| format!("encode registration replay ledger lock owner failed: {err}"))?;
    let path = lock_dir.join("owner.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| format!("create registration replay ledger lock owner failed: {err}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|err| format!("persist registration replay ledger lock owner failed: {err}"))
}

fn reclaim_stale_lock(lock_dir: &Path) -> Result<bool, String> {
    let owner_path = lock_dir.join("owner.json");
    let owner = match fs::read(&owner_path) {
        Ok(bytes) => serde_json::from_slice::<RegistrationReplayLockOwner>(&bytes).ok(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(format!(
                "read registration replay ledger lock owner failed: {err}"
            ));
        }
    };
    let stale = match owner {
        Some(owner) => !process_is_alive(owner.pid),
        None => lock_age(lock_dir).is_some_and(|age| age >= INCOMPLETE_LOCK_GRACE),
    };
    if !stale {
        return Ok(false);
    }
    let quarantine = lock_dir.with_extension(format!(
        "stale-{}-{}",
        std::process::id(),
        LOCK_TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    match fs::rename(lock_dir, &quarantine) {
        Ok(()) => {
            fs::remove_dir_all(&quarantine).map_err(|err| {
                format!("remove stale registration replay ledger lock failed: {err}")
            })?;
            Ok(true)
        }
        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::AlreadyExists
            ) =>
        {
            Ok(true)
        }
        Err(err) => Err(format!(
            "quarantine stale registration replay ledger lock failed: {err}"
        )),
    }
}

fn lock_age(lock_dir: &Path) -> Option<Duration> {
    let modified = fs::metadata(lock_dir).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}

fn new_lock_token() -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = LOCK_TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{time}-{counter}", std::process::id())
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    i32::try_from(pid).is_ok_and(|pid| unsafe { kill(pid, 0) == 0 })
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn OpenProcess(
            desired_access: u32,
            inherit_handle: i32,
            process_id: u32,
        ) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    unsafe { CloseHandle(handle) };
    true
}

#[cfg(not(any(unix, windows)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_process_lock_is_reclaimed() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-registration-stale-lock-{}-{}",
            std::process::id(),
            new_lock_token()
        ));
        let ledger = root.join("ledger.json");
        let lock_dir = replay_lock_dir(&ledger);
        fs::create_dir_all(&lock_dir).expect("create stale lock");
        let stale = RegistrationReplayLockOwner {
            pid: u32::MAX,
            token: "dead-owner".to_string(),
        };
        fs::write(
            lock_dir.join("owner.json"),
            serde_json::to_vec(&stale).expect("encode stale owner"),
        )
        .expect("write stale owner");

        let guard = RegistrationReplayProcessLock::acquire(&ledger)
            .expect("dead process lock must be reclaimed");
        drop(guard);
        assert!(!lock_dir.exists());
        let _ = fs::remove_dir_all(root);
    }
}
