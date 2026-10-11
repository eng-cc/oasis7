use super::*;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{
    Dir, DirBuilder, DirBuilderExt, MetadataExt, OpenOptions, OpenOptionsExt, PermissionsExt,
};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use std::{
    io::{Read, Write},
    path::Component,
};

fn io_error(_: std::io::Error) -> String {
    "private goal filesystem operation failed".into()
}
fn sync(dir: &Dir) -> Result<(), String> {
    // cap-std directory handles are O_PATH on Linux, which cannot fsync.
    // Reopen the held directory itself, without resolving an ambient path,
    // as a readable directory descriptor. Setting only the custom DIRECTORY
    // flag avoids cap-primitives' dir_required -> O_PATH optimization.
    let mut options = OpenOptions::new();
    options
        .read(true)
        .follow(FollowSymlinks::No)
        .custom_flags(libc::O_DIRECTORY);
    dir.open_with(".", &options)
        .map_err(io_error)?
        .sync_all()
        .map_err(io_error)
}
fn private_file(metadata: &cap_std::fs::Metadata) -> Result<(), String> {
    if !metadata.is_file()
        || metadata.permissions().mode() & 0o7777 != 0o600
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.nlink() != 1
    {
        return Err("private goal file permissions or type invalid".into());
    }
    Ok(())
}
fn private_directory(dir: &Dir) -> Result<(), String> {
    let metadata = dir.dir_metadata().map_err(io_error)?;
    if metadata.permissions().mode() & 0o7777 != 0o700
        || metadata.uid() != unsafe { libc::geteuid() }
    {
        return Err("private goal directory must be owned by operator with mode 0700".into());
    }
    Ok(())
}
pub(super) fn open_root(root: &Path) -> Result<Dir, String> {
    if !root.is_absolute() {
        return Err("private goal root must be an absolute operator-private path".into());
    }
    let components = root.components().collect::<Vec<_>>();
    if components.len() < 2
        || components
            .iter()
            .skip(1)
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("invalid private goal root path".into());
    }
    let mut parent = Dir::open_ambient_dir("/", cap_std::ambient_authority()).map_err(io_error)?;
    for component in &components[1..components.len() - 1] {
        parent = parent
            .open_dir_nofollow(component.as_os_str())
            .map_err(io_error)?;
    }
    let name = components
        .last()
        .expect("root has final component")
        .as_os_str();
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    match parent.create_dir_with(name, &builder) {
        Ok(()) => sync(&parent)?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(io_error(e)),
    }
    let directory = parent.open_dir_nofollow(name).map_err(io_error)?;
    private_directory(&directory)?;
    sync(&directory)?;
    Ok(directory)
}
fn aad(binding: &PrivateGoalBindingV1, reference: &PrivateGoalRef) -> Result<Vec<u8>, String> {
    serde_cbor::to_vec(&("oasis7/private-goal-store/v1", reference.as_str(), binding))
        .map_err(|_| "private goal AAD encoding failed".into())
}
fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| "private goal randomness unavailable")?;
    Ok(bytes)
}
pub(super) fn put(
    dir: &Dir,
    key: &[u8; 32],
    binding: &PrivateGoalBindingV1,
    plaintext: &[u8],
) -> Result<PrivateGoalRef, String> {
    put_with_reference(dir, key, binding, plaintext, || {
        Ok(PrivateGoalRef(hex::encode(random::<32>()?)))
    })
}

pub(super) fn put_with_reference(
    dir: &Dir,
    key: &[u8; 32],
    binding: &PrivateGoalBindingV1,
    plaintext: &[u8],
    mut next_reference: impl FnMut() -> Result<PrivateGoalRef, String>,
) -> Result<PrivateGoalRef, String> {
    private_directory(dir)?;
    verify_key(dir, key)?;
    for _ in 0..8 {
        let reference = next_reference()?;
        let nonce = random::<24>()?;
        let ciphertext = XChaCha20Poly1305::new(key.into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad(binding, &reference)?,
                },
            )
            .map_err(|_| "private goal encryption failed")?;
        let mut envelope = Vec::with_capacity(32 + ciphertext.len());
        envelope.extend_from_slice(MAGIC);
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&ciphertext);
        if envelope.len() > MAX_PRIVATE_GOAL_FILE_BYTES {
            return Err("private goal file exceeds byte limit".into());
        }
        if publish_immutable(dir, reference.as_str(), &envelope)? {
            return Ok(reference);
        }
    }
    Err("private goal reference collision retry budget exhausted".into())
}
pub(super) fn get(
    dir: &Dir,
    key: &[u8; 32],
    reference: &PrivateGoalRef,
    binding: &PrivateGoalBindingV1,
) -> Result<Zeroizing<Vec<u8>>, String> {
    private_directory(dir)?;
    verify_key(dir, key)?;
    let bytes = read_private(dir, reference.as_str(), MAX_PRIVATE_GOAL_FILE_BYTES)?;
    if bytes.len() > MAX_PRIVATE_GOAL_FILE_BYTES || bytes.len() < 48 || &bytes[..8] != MAGIC {
        return Err("invalid private goal encrypted envelope".into());
    }
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&bytes[8..32]),
            Payload {
                msg: &bytes[32..],
                aad: &aad(binding, reference)?,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| "private goal authentication failed".into())
}

const KEY_MARKER: &str = ".key-verifier-v1";
const KEY_MAGIC: &[u8; 8] = b"O7KEY001";
const KEY_AAD: &[u8] = b"oasis7/private-goal-key-verifier/v1";
const KEY_BODY: &[u8] = b"oasis7 private goal namespace key v1";

fn read_private(dir: &Dir, name: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .follow(FollowSymlinks::No)
        .custom_flags(libc::O_NONBLOCK);
    let file = dir.open_with(name, &options).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    private_file(&metadata)?;
    if metadata.len() > limit as u64 {
        return Err("private goal file exceeds byte limit".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err("private goal file exceeds byte limit".into());
    }
    Ok(bytes)
}

// Shared durable, immutable publication. A collision never replaces the winner.
fn publish_immutable(dir: &Dir, name: &str, bytes: &[u8]) -> Result<bool, String> {
    for _ in 0..8 {
        let temp = format!(".{}.tmp", hex::encode(random::<32>()?));
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .mode(0o600)
            .follow(FollowSymlinks::No);
        let mut file = match dir.open_with(&temp, &options) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(io_error(e)),
        };
        if let Err(error) = file
            .metadata()
            .map_err(io_error)
            .and_then(|metadata| private_file(&metadata))
        {
            let _ = dir.remove_file(&temp);
            return Err(error);
        }
        let publication = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            dir.hard_link(&temp, dir, name)
        })();
        let cleanup = dir.remove_file(&temp);
        match publication {
            Ok(()) => {
                cleanup.map_err(io_error)?;
                sync(dir)?;
                return Ok(true);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup.map_err(io_error)?;
                sync(dir)?;
                return Ok(false);
            }
            Err(e) => {
                let _ = cleanup;
                return Err(io_error(e));
            }
        }
    }
    Err("private goal temporary collision retry budget exhausted".into())
}

fn verify_key(dir: &Dir, key: &[u8; 32]) -> Result<(), String> {
    let bytes = read_private(dir, KEY_MARKER, 256)?;
    if bytes.len() < 48 || &bytes[..8] != KEY_MAGIC {
        return Err("invalid private goal key verifier".into());
    }
    let plaintext = XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&bytes[8..32]),
            Payload {
                msg: &bytes[32..],
                aad: KEY_AAD,
            },
        )
        .map_err(|_| "private goal namespace key mismatch or corruption")?;
    if plaintext.len() != KEY_BODY.len() + 32 || !plaintext.starts_with(KEY_BODY) {
        return Err("invalid private goal key verifier body".into());
    }
    Ok(())
}

fn verify_key_durable(dir: &Dir, key: &[u8; 32]) -> Result<(), String> {
    verify_key(dir, key)?;
    // A racing initializer may have linked and removed its temporary without
    // yet syncing the directory. Every successful open completes publication.
    sync(dir)
}

/// A marker-less nonempty namespace is never repaired or re-keyed. This includes
/// interrupted initialization temporaries: operator inspection is required.
pub(super) fn initialize_key(dir: &Dir, key: &[u8; 32]) -> Result<(), String> {
    match dir.symlink_metadata(KEY_MARKER) {
        Ok(_) => return verify_key_durable(dir, key),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(io_error(e)),
    }
    if dir.entries().map_err(io_error)?.next().is_some() {
        // A concurrent initializer may have published while we inspected.
        // Verify it if present; never adopt any other nonempty namespace.
        return verify_key_durable(dir, key);
    }
    let nonce = random::<24>()?;
    let mut challenge = KEY_BODY.to_vec();
    challenge.extend_from_slice(&random::<32>()?);
    let encrypted = XChaCha20Poly1305::new(key.into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &challenge,
                aad: KEY_AAD,
            },
        )
        .map_err(|_| "private goal key initialization failed")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(KEY_MAGIC);
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&encrypted);
    publish_immutable(dir, KEY_MARKER, &bytes)?;
    // The collision winner is authoritative, including its fsynced key marker.
    verify_key_durable(dir, key)
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::*;

    #[test]
    fn private_goal_directory_sync_uses_held_directory_after_rename() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-private-goal-sync-{}",
            hex::encode(random::<16>().unwrap())
        ));
        let moved = root.with_extension("moved");
        std::fs::create_dir(&root).unwrap();
        let directory = Dir::open_ambient_dir(&root, cap_std::ambient_authority()).unwrap();
        std::fs::rename(&root, &moved).unwrap();
        // Linux cap-std opened an O_PATH handle. The old into_std_file +
        // sync_all implementation fails EBADF here; capability-relative reopen
        // must sync the same directory even after its ambient path moved.
        sync(&directory).expect("fsync held Linux directory through a readable descriptor");
        std::fs::remove_dir(&moved).unwrap();
    }
}
