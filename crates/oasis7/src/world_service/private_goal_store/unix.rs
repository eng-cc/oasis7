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
    dir.try_clone()
        .map_err(io_error)?
        .into_std_file()
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
            file.write_all(&envelope)?;
            file.sync_all()?;
            // link is atomic and never overwrites an existing immutable record.
            dir.hard_link(&temp, dir, reference.as_str())
        })();
        let cleanup = dir.remove_file(&temp);
        match publication {
            Ok(()) => {
                cleanup.map_err(io_error)?;
                sync(dir)?;
                return Ok(reference);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup.map_err(io_error)?;
                sync(dir)?;
            }
            Err(e) => {
                let _ = cleanup;
                return Err(io_error(e));
            }
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
    let mut options = OpenOptions::new();
    options
        .read(true)
        .follow(FollowSymlinks::No)
        .custom_flags(libc::O_NONBLOCK);
    let file = dir
        .open_with(reference.as_str(), &options)
        .map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    private_file(&metadata)?;
    if metadata.len() > MAX_PRIVATE_GOAL_FILE_BYTES as u64 {
        return Err("private goal file exceeds byte limit".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PRIVATE_GOAL_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
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
