use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use nix::fcntl::{Flock, FlockArg, OFlag, openat};
use nix::sys::stat::Mode;

use crate::error::SignerError;

pub(crate) struct CustodyLock {
    _file: Flock<File>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PublishOptions {
    pub(crate) owner_uid: u32,
    pub(crate) owner_gid: u32,
    pub(crate) mode: u32,
    pub(crate) replace: bool,
}

impl CustodyLock {
    pub(crate) fn acquire(
        path: &Path,
        owner_uid: u32,
        owner_gid: u32,
    ) -> Result<Self, SignerError> {
        let open_options = || {
            let mut options = OpenOptions::new();
            options.read(true).write(true);
            #[cfg(unix)]
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            options
        };
        #[cfg(unix)]
        let (file, created) = match open_options().create_new(true).open(path) {
            Ok(file) => (file, true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                (open_options().open(path)?, false)
            }
            Err(error) => return Err(SignerError::PersistenceFailed(error)),
        };
        #[cfg(not(unix))]
        return Err(SignerError::UnsupportedPlatformOrFs);

        let locked = match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(locked) => locked,
            Err((file, errno)) => {
                drop(file);
                let error: std::io::Error = errno.into();
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return Err(SignerError::LockBusy);
                }
                return Err(SignerError::PersistenceFailed(error));
            }
        };
        #[cfg(unix)]
        {
            use nix::unistd::{Gid, Uid, fchown};
            if created {
                fchown(
                    &*locked,
                    Some(Uid::from_raw(owner_uid)),
                    Some(Gid::from_raw(owner_gid)),
                )
                .map_err(|error| SignerError::PersistenceFailed(error.into()))?;
                locked.set_permissions(fs::Permissions::from_mode(0o600))?;
                locked.sync_all()?;
            }
            let metadata = locked.metadata()?;
            if !metadata.is_file()
                || metadata.uid() != owner_uid
                || metadata.gid() != owner_gid
                || metadata.nlink() != 1
                || metadata.permissions().mode() & 0o7777 != 0o600
            {
                return Err(SignerError::InstallationDrift);
            }
        }
        Ok(Self { _file: locked })
    }
}

pub(crate) fn read_regular(path: &Path, max_bytes: usize) -> Result<Vec<u8>, SignerError> {
    let file = open_regular_file(path, max_bytes)?;
    read_open_candidate(file, max_bytes)
}

/// Reads a bounded candidate from one validated file descriptor.
///
/// Every path component is opened relative to an already-open directory with
/// `O_NOFOLLOW`; metadata checks and the bounded read then use the same final
/// descriptor. The candidate must belong to `expected_owner_uid`, have no
/// group/other write bits, be singly linked, and remain within `max_bytes`.
pub fn read_candidate_bytes(
    path: &Path,
    expected_owner_uid: u32,
    max_bytes: usize,
) -> Result<Vec<u8>, SignerError> {
    let file = open_candidate_file(path, expected_owner_uid, max_bytes)?;
    read_open_candidate(file, max_bytes)
}

fn open_candidate_file(
    path: &Path,
    expected_owner_uid: u32,
    max_bytes: usize,
) -> Result<File, SignerError> {
    let file = open_regular_file(path, max_bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata()?;
        if metadata.uid() != expected_owner_uid {
            return Err(SignerError::AuthorizationDenied);
        }
        if metadata.permissions().mode() & 0o022 != 0 {
            return Err(SignerError::InvalidInput(
                "candidate must not be group- or other-writable".to_owned(),
            ));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (expected_owner_uid, max_bytes);
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    Ok(file)
}

fn open_regular_file(path: &Path, max_bytes: usize) -> Result<File, SignerError> {
    #[cfg(not(unix))]
    {
        let _ = (path, max_bytes);
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        use std::os::unix::fs::MetadataExt;

        let max_bytes = u64::try_from(max_bytes)
            .map_err(|_| SignerError::InvalidInput("candidate size limit is invalid".to_owned()))?;

        let mut components = Vec::new();
        if !path.is_absolute() {
            return Err(SignerError::InvalidInput(
                "candidate path must be absolute".to_owned(),
            ));
        }
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(part) => components.push(part.to_owned()),
                Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                    return Err(SignerError::InvalidInput(
                        "candidate path contains an unsafe component".to_owned(),
                    ));
                }
            }
        }
        let Some(filename) = components.pop() else {
            return Err(SignerError::InvalidInput(
                "candidate path must name a file".to_owned(),
            ));
        };

        let mut directory = File::open("/")?;
        for component in components {
            let descriptor = openat(
                directory.as_fd(),
                component.as_os_str(),
                OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
                Mode::empty(),
            )
            .map_err(map_openat_error)?;
            directory = File::from(descriptor);
            if !directory.metadata()?.is_dir() {
                return Err(SignerError::InstallationDrift);
            }
        }

        let descriptor = openat(
            directory.as_fd(),
            filename.as_os_str(),
            OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(map_openat_error)?;
        let file = File::from(descriptor);
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(SignerError::InvalidInput(
                "candidate must be a regular file".to_owned(),
            ));
        }
        if metadata.nlink() != 1 {
            return Err(SignerError::InvalidInput(
                "candidate must be singly linked".to_owned(),
            ));
        }
        if metadata.len() > max_bytes {
            return Err(SignerError::InvalidInput(
                "candidate exceeds the size limit".to_owned(),
            ));
        }
        Ok(file)
    }
}

fn map_openat_error(error: nix::errno::Errno) -> SignerError {
    match error {
        nix::errno::Errno::ELOOP | nix::errno::Errno::ENOTDIR | nix::errno::Errno::ENOENT => {
            SignerError::InstallationDrift
        }
        _ => SignerError::PersistenceFailed(error.into()),
    }
}

fn read_open_candidate(file: File, max_bytes: usize) -> Result<Vec<u8>, SignerError> {
    #[cfg(not(unix))]
    {
        let _ = (file, max_bytes);
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let metadata = file.metadata()?;
        let max_bytes_u64 = u64::try_from(max_bytes)
            .map_err(|_| SignerError::InvalidInput("candidate size limit is invalid".to_owned()))?;
        if !metadata.is_file() || metadata.len() > max_bytes_u64 || metadata.nlink() != 1 {
            return Err(SignerError::InvalidInput(
                "candidate metadata changed or exceeds the size limit".to_owned(),
            ));
        }
        let read_limit = max_bytes_u64.checked_add(1).ok_or_else(|| {
            SignerError::InvalidInput("candidate size limit is invalid".to_owned())
        })?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        (&file).take(read_limit).read_to_end(&mut bytes)?;
        let final_metadata = file.metadata()?;
        if bytes.len() > max_bytes
            || final_metadata.len() > max_bytes_u64
            || final_metadata.nlink() != 1
        {
            return Err(SignerError::InvalidInput(
                "candidate changed or exceeds the size limit".to_owned(),
            ));
        }
        Ok(bytes)
    }
}

pub(crate) fn write_private_new(path: &Path, bytes: &[u8]) -> Result<(), SignerError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(crate) fn atomic_publish(
    path: &Path,
    bytes: &[u8],
    publish_options: PublishOptions,
) -> Result<(), SignerError> {
    atomic_publish_with_hooks(path, bytes, publish_options, || Ok(()), || Ok(()))
}

pub(crate) fn atomic_publish_with_hooks<F, G>(
    path: &Path,
    bytes: &[u8],
    publish_options: PublishOptions,
    after_create_only_link: F,
    after_publish: G,
) -> Result<(), SignerError>
where
    F: FnOnce() -> Result<(), SignerError>,
    G: FnOnce() -> Result<(), SignerError>,
{
    #[cfg(not(unix))]
    {
        let _ = (
            path,
            bytes,
            publish_options,
            after_create_only_link,
            after_publish,
        );
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    #[cfg(unix)]
    {
        use nix::unistd::{Gid, Uid, fchown};

        let parent = path.parent().ok_or(SignerError::InstallationDrift)?;
        reject_symlink_components(parent)?;
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(SignerError::InstallationDrift)?;
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).map_err(|_| SignerError::UnsupportedPlatformOrFs)?;
        let staging = parent.join(format!(
            ".{filename}.stage-{}-{}",
            std::process::id(),
            hex::encode(random)
        ));

        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let mut file = options.open(&staging)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fchown(
            &file,
            Some(Uid::from_raw(publish_options.owner_uid)),
            Some(Gid::from_raw(publish_options.owner_gid)),
        )
        .map_err(|error| SignerError::PersistenceFailed(error.into()))?;
        file.set_permissions(fs::Permissions::from_mode(publish_options.mode))?;
        file.sync_all()?;
        drop(file);

        if publish_options.replace {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(SignerError::InstallationDrift);
                }
                Ok(metadata) if !metadata.is_file() || metadata.nlink() != 1 => {
                    return Err(SignerError::InstallationDrift);
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(SignerError::PersistenceFailed(error)),
            }
            fs::rename(&staging, path)?;
        } else {
            fs::hard_link(&staging, path)?;
            after_create_only_link()?;
            fs::remove_file(&staging)?;
        }
        after_publish()?;
        sync_dir(parent)
    }
}

pub(crate) fn sync_file_and_parent(path: &Path) -> Result<(), SignerError> {
    #[cfg(not(unix))]
    {
        let _ = path;
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    #[cfg(unix)]
    {
        let parent = path.parent().ok_or(SignerError::InstallationDrift)?;
        reject_symlink_components(parent)?;
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let file = options.open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(SignerError::InstallationDrift);
        }
        file.sync_all()?;
        sync_dir(parent)
    }
}

pub(crate) fn set_path_owner_mode(
    path: &Path,
    owner_uid: u32,
    owner_gid: u32,
    mode: u32,
) -> Result<(), SignerError> {
    #[cfg(not(unix))]
    {
        let _ = (path, owner_uid, owner_gid, mode);
        return Err(SignerError::UnsupportedPlatformOrFs);
    }
    #[cfg(unix)]
    {
        use nix::unistd::{Gid, Uid, fchown};

        let before = fs::symlink_metadata(path)?;
        if before.file_type().is_symlink() || (!before.is_dir() && before.nlink() != 1) {
            return Err(SignerError::InstallationDrift);
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | if before.is_dir() {
                    libc::O_DIRECTORY
                } else {
                    0
                },
        );
        let file = options.open(path)?;
        let opened = file.metadata()?;
        if opened.is_dir() != before.is_dir()
            || opened.dev() != before.dev()
            || opened.ino() != before.ino()
        {
            return Err(SignerError::InstallationDrift);
        }
        fchown(
            &file,
            Some(Uid::from_raw(owner_uid)),
            Some(Gid::from_raw(owner_gid)),
        )
        .map_err(|error| SignerError::PersistenceFailed(error.into()))?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        file.sync_all()?;
        let after = fs::symlink_metadata(path)?;
        if after.file_type().is_symlink()
            || after.dev() != opened.dev()
            || after.ino() != opened.ino()
        {
            return Err(SignerError::InstallationDrift);
        }
        Ok(())
    }
}

pub(crate) fn create_private_dir(path: &Path) -> Result<(), SignerError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700).create(path)?;
    }
    #[cfg(not(unix))]
    return Err(SignerError::UnsupportedPlatformOrFs);
    Ok(())
}

pub(crate) fn sync_dir(path: &Path) -> Result<(), SignerError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

pub(crate) fn reject_symlink(path: &Path) -> Result<(), SignerError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(SignerError::InstallationDrift);
    }
    Ok(())
}

pub(crate) fn reject_symlink_components(path: &Path) -> Result<(), SignerError> {
    if !path.is_absolute() {
        return Err(SignerError::InstallationDrift);
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => current.push(component.as_os_str()),
            Component::Normal(part) => {
                current.push(part);
                reject_symlink(&current)?;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(SignerError::InstallationDrift);
            }
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod candidate_tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_SCRATCH: AtomicUsize = AtomicUsize::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let id = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oasis7-local-signer-candidate-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create scratch directory");
            Self(fs::canonicalize(path).expect("canonical scratch path"))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn owner_uid(path: &Path) -> u32 {
        fs::metadata(path).expect("metadata").uid()
    }

    #[test]
    fn candidate_open_rejects_symlink_components_and_oversized_files() {
        let scratch = Scratch::new();
        let actual_dir = scratch.path().join("actual");
        fs::create_dir(&actual_dir).expect("create candidate directory");
        let actual = actual_dir.join("candidate.json");
        fs::write(&actual, b"candidate").expect("write candidate");
        let uid = owner_uid(&actual);

        let parent_link = scratch.path().join("parent-link");
        std::os::unix::fs::symlink(&actual_dir, &parent_link).expect("create parent symlink");
        assert!(read_candidate_bytes(&parent_link.join("candidate.json"), uid, 64).is_err());

        let final_link = scratch.path().join("final-link");
        std::os::unix::fs::symlink(&actual, &final_link).expect("create final symlink");
        assert!(read_candidate_bytes(&final_link, uid, 64).is_err());

        assert!(read_candidate_bytes(&actual, uid, 4).is_err());
    }

    #[test]
    fn candidate_open_checks_owner_and_group_or_other_write_bits() {
        let scratch = Scratch::new();
        let path = scratch.path().join("candidate.json");
        fs::write(&path, b"candidate").expect("write candidate");
        let uid = owner_uid(&path);
        let other_uid = if uid == 0 { 1 } else { 0 };

        assert!(read_candidate_bytes(&path, other_uid, 64).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o620))
            .expect("set group writable mode");
        assert!(read_candidate_bytes(&path, uid, 64).is_err());
    }

    #[test]
    fn candidate_bytes_are_read_from_the_validated_open_file_after_path_replacement() {
        let scratch = Scratch::new();
        let path = scratch.path().join("candidate.json");
        fs::write(&path, b"reviewed bytes").expect("write original candidate");
        let uid = owner_uid(&path);

        let opened = open_candidate_file(&path, uid, 64).expect("open candidate descriptor");
        let moved = scratch.path().join("opened-candidate.json");
        fs::rename(&path, &moved).expect("move opened candidate path");
        fs::write(&path, b"replacement bytes").expect("replace pathname");

        assert_eq!(
            read_open_candidate(opened, 64).expect("read validated descriptor"),
            b"reviewed bytes"
        );
    }

    #[test]
    fn candidate_open_rejects_fifo_without_blocking() {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let scratch = Scratch::new();
        let fifo = scratch.path().join("candidate.fifo");
        nix::unistd::mkfifo(&fifo, Mode::S_IRUSR | Mode::S_IWUSR).expect("create candidate fifo");
        let uid = owner_uid(scratch.path());
        let candidate = fifo.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            started_tx.send(()).expect("signal test start");
            result_tx
                .send(read_candidate_bytes(&candidate, uid, 64))
                .expect("send candidate result");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("candidate worker starts");
        let result = result_rx.recv_timeout(Duration::from_secs(1));
        let returned_promptly = result.is_ok();
        let final_result = if let Ok(result) = result {
            result
        } else {
            // Release a regressed blocking open so the test can fail cleanly.
            let writer = OpenOptions::new()
                .write(true)
                .open(&fifo)
                .expect("release blocking fifo reader");
            drop(writer);
            result_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("candidate open returns after fifo release")
        };
        worker.join().expect("candidate worker completes");

        assert!(returned_promptly, "candidate FIFO open blocked");
        assert!(matches!(final_result, Err(SignerError::InvalidInput(_))));
    }
}
