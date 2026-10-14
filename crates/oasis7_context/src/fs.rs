//! All private-store operations stay relative to held directory descriptors.
// SAFETY: every descriptor is checked before ownership, C names outlive calls, and
// directory streams own independently opened descriptors and close exactly once.
#![allow(unsafe_code)]
use crate::error::{ContextError, Result};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    os::unix::fs::MetadataExt,
    path::Path,
    time::{Duration, Instant},
};
fn name(s: &str) -> Result<CString> {
    if s.is_empty() || s == "." || s == ".." || s.contains('/') {
        return Err(ContextError::new("UNSAFE_PATH", "invalid relative path"));
    }
    CString::new(s).map_err(|_| ContextError::new("UNSAFE_PATH", "invalid path"))
}
fn last() -> ContextError {
    let e = std::io::Error::last_os_error();
    if matches!(e.raw_os_error(), Some(libc::ELOOP)) {
        ContextError::new("UNSAFE_PATH", "symbolic link is not allowed")
    } else if matches!(e.raw_os_error(), Some(libc::ENOSYS) | Some(libc::ENOTSUP)) {
        ContextError::new(
            "PLATFORM_UNSUPPORTED",
            "required filesystem operation is unsupported",
        )
    } else {
        e.into()
    }
}
fn checked_file(fd: i32) -> Result<File> {
    if fd < 0 {
        return Err(last());
    }
    let f = unsafe { File::from_raw_fd(fd) };
    let m = f.metadata()?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(ContextError::new(
            "UNSAFE_PATH",
            "expected single-link regular file",
        ));
    }
    Ok(f)
}
// fdopendir transfers ownership; this guard closes on EOF and every error path.
struct DirectoryStream<F: FnOnce(libc::c_int)> {
    pointer: *mut libc::DIR,
    on_close: Option<F>,
}
impl<F: FnOnce(libc::c_int)> Drop for DirectoryStream<F> {
    fn drop(&mut self) {
        let status = unsafe { libc::closedir(self.pointer) };
        if let Some(observer) = self.on_close.take() {
            observer(status);
        }
    }
}
#[cfg(target_os = "macos")]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__error() }
}
#[cfg(target_os = "linux")]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}
fn set_errno(value: libc::c_int) {
    unsafe { *errno_pointer() = value };
}
fn errno() -> libc::c_int {
    unsafe { *errno_pointer() }
}
pub struct Directory {
    file: File,
}
impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with(path, || {})
    }
    fn open_with(path: &Path, before_leaf: impl FnOnce()) -> Result<Self> {
        if !path.is_absolute() {
            return Err(ContextError::new("UNSAFE_PATH", "root must be absolute"));
        }
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::RootDir | std::path::Component::CurDir => (),
                std::path::Component::Normal(s) => components.push(s),
                _ => return Err(ContextError::new("UNSAFE_PATH", "invalid root path")),
            }
        }
        let mut parent = std::path::PathBuf::from("/");
        for component in components.iter().take(components.len().saturating_sub(1)) {
            parent.push(component);
        }
        // Resolve ancestor aliases such as macOS /var, but never resolve the
        // caller-selected final component. Open that leaf once with O_NOFOLLOW.
        let resolved = std::fs::canonicalize(parent)?;
        let fd = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(last());
        }
        let mut directory = Self {
            file: unsafe { File::from_raw_fd(fd) },
        };
        for component in resolved.components() {
            if let std::path::Component::Normal(s) = component {
                directory = directory.child(
                    s.to_str()
                        .ok_or_else(|| ContextError::new("UNSAFE_PATH", "path must be UTF-8"))?,
                )?;
            }
        }
        before_leaf();
        if let Some(leaf) = components.last() {
            directory = directory.child(
                leaf.to_str()
                    .ok_or_else(|| ContextError::new("UNSAFE_PATH", "path must be UTF-8"))?,
            )?;
        }
        Ok(directory)
    }
    pub fn child(&self, n: &str) -> Result<Self> {
        let n = name(n)?;
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                n.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(ContextError::new(
                "UNSAFE_PATH",
                "cannot open store directory",
            ));
        }
        Ok(Self {
            file: unsafe { File::from_raw_fd(fd) },
        })
    }
    pub fn create_dir(&self, n: &str) -> Result<Self> {
        let c = name(n)?;
        let r = unsafe { libc::mkdirat(self.file.as_raw_fd(), c.as_ptr(), 0o700) };
        if r < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(last());
        }
        self.child(n)
    }
    pub fn publish_new_directory(&self, stage: &str, target: &str) -> Result<()> {
        self.publish(stage, target, false)?;
        self.sync().map_err(|_| {
            let mut e = ContextError::new(
                "PERSISTENCE_UNCERTAIN",
                "published directory could not be synchronized",
            );
            e.effect = "applied".into();
            e
        })
    }
    pub fn create_dir_new(&self, n: &str) -> Result<Self> {
        let c = name(n)?;
        if unsafe { libc::mkdirat(self.file.as_raw_fd(), c.as_ptr(), 0o700) } < 0 {
            return Err(last());
        }
        self.child(n).map_err(|mut e| {
            e.effect = "applied".into();
            e
        })
    }
    /// Observe a single entry relative to this held parent without following links.
    pub fn contains(&self, n: &str) -> Result<bool> {
        let n = name(n)?;
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.file.as_raw_fd(),
                n.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == 0
        {
            return Ok(true);
        }
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            return Ok(false);
        }
        Err(last())
    }
    pub fn remove_dir(&self, n: &str) -> Result<()> {
        let n = name(n)?;
        if unsafe { libc::unlinkat(self.file.as_raw_fd(), n.as_ptr(), libc::AT_REMOVEDIR) } < 0 {
            return Err(last());
        }
        Ok(())
    }
    pub fn matches_path(&self, path: &Path) -> Result<bool> {
        let other = Self::open(path)?;
        let a = self.file.metadata()?;
        let b = other.file.metadata()?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    /// Compare held ancestor identities without resolving mutable pathnames.
    pub fn is_within(&self, ancestor: &Self) -> Result<bool> {
        let expected = ancestor.identity()?;
        let mut current = Self {
            file: self.file.try_clone()?,
        };
        loop {
            let identity = current.identity()?;
            if identity == expected {
                return Ok(true);
            }
            let fd = unsafe {
                libc::openat(
                    current.file.as_raw_fd(),
                    c"..".as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(last());
            }
            let parent = Self {
                file: unsafe { File::from_raw_fd(fd) },
            };
            if parent.identity()? == identity {
                return Ok(false);
            }
            current = parent;
        }
    }
    /// Identity of the held directory, independent of later pathname replacement.
    pub fn identity(&self) -> Result<(u64, u64)> {
        let metadata = self.file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }
    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            file: self.file.try_clone()?,
        })
    }
    /// Pin a child's working directory to this held descriptor, never its mutable pathname.
    pub fn pin_command_cwd(&self, command: &mut std::process::Command) -> Result<()> {
        use std::os::unix::process::CommandExt;
        let directory = self.file.try_clone()?;
        // SAFETY: the closure owns the duplicated descriptor through spawn. After fork it
        // performs only fchdir and errno retrieval; no allocation or lock acquisition.
        unsafe {
            command.pre_exec(move || {
                if libc::fchdir(directory.as_raw_fd()) != 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        Ok(())
    }
    pub fn file_private_permissions(&self, n: &str) -> Result<bool> {
        let c = name(n)?;
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        Ok(checked_file(fd)?.metadata()?.mode() & 0o077 == 0)
    }
    pub fn private_permissions(&self) -> Result<bool> {
        Ok(self.file.metadata()?.mode() & 0o077 == 0)
    }
    pub fn sync(&self) -> Result<()> {
        self.file.sync_all().map_err(Into::into)
    }
    pub fn read(&self, n: &str, max: usize) -> Result<Vec<u8>> {
        let n = name(n)?;
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                n.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            return Err(ContextError::new("ENTRY_NOT_FOUND", "file not found"));
        }
        let mut f = checked_file(fd)?;
        let before = f.metadata()?;
        if before.len() > max as u64 {
            return Err(ContextError::new("LIMIT_EXCEEDED", "file exceeds limit"));
        }
        let mut bytes = Vec::with_capacity(before.len() as usize);
        Read::by_ref(&mut f)
            .take(max as u64 + 1)
            .read_to_end(&mut bytes)?;
        let after = f.metadata()?;
        if bytes.len() > max {
            return Err(ContextError::new("LIMIT_EXCEEDED", "file exceeds limit"));
        }
        if !after.is_file() || after.nlink() != 1 {
            return Err(ContextError::new(
                "UNSAFE_PATH",
                "file link identity changed",
            ));
        }
        if before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
        {
            return Err(ContextError::new(
                "UNSTABLE_ENTRY",
                "file changed while reading",
            ));
        }
        Ok(bytes)
    }
    pub fn names(&self) -> Result<Vec<String>> {
        self.names_with(|dir| unsafe { libc::readdir(dir) }, |_| {})
    }
    fn names_with(
        &self,
        mut read_next: impl FnMut(*mut libc::DIR) -> *mut libc::dirent,
        on_close: impl FnOnce(libc::c_int),
    ) -> Result<Vec<String>> {
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(last());
        }
        let pointer = unsafe { libc::fdopendir(fd) };
        if pointer.is_null() {
            let error = last();
            unsafe { libc::close(fd) };
            return Err(error);
        }
        let stream = DirectoryStream {
            pointer,
            on_close: Some(on_close),
        };
        let mut names = Vec::new();
        loop {
            // POSIX readdir distinguishes EOF from failure only via errno.
            set_errno(0);
            let entry = read_next(stream.pointer);
            if entry.is_null() {
                if errno() != 0 {
                    return Err(last());
                }
                break;
            }
            let bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            let value = bytes
                .to_str()
                .map_err(|_| ContextError::new("UNSAFE_PATH", "invalid filename"))?;
            if value != "." && value != ".." {
                if names.len() >= 10002 {
                    return Err(ContextError::new(
                        "LIMIT_EXCEEDED",
                        "directory object count exceeds scan limit",
                    ));
                }
                names.push(value.into());
            }
        }
        names.sort();
        Ok(names)
    }
    pub fn remove_file(&self, n: &str) -> Result<()> {
        let n = name(n)?;
        if unsafe { libc::unlinkat(self.file.as_raw_fd(), n.as_ptr(), 0) } < 0 {
            return Err(last());
        }
        Ok(())
    }
    pub fn write_atomic(&self, n: &str, bytes: &[u8], replace: bool) -> Result<()> {
        self.write_atomic_with(n, bytes, replace, |_| Ok(()))
    }
    // A private callback makes interruption boundaries testable without environmental backdoors.
    fn write_atomic_with(
        &self,
        n: &str,
        bytes: &[u8],
        replace: bool,
        mut checkpoint: impl FnMut(u8) -> Result<()>,
    ) -> Result<()> {
        let temp = format!(".context-stage-{}", random_id());
        let c = name(&temp)?;
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        let result = (|| {
            let mut f = checked_file(fd)?;
            f.write_all(bytes)?;
            checkpoint(0)?;
            f.sync_all()?;
            checkpoint(1)?;
            self.publish(&temp, n, replace)?;
            checkpoint(2).and_then(|_| self.sync()).map_err(|_| {
                let mut e = ContextError::new(
                    "PERSISTENCE_UNCERTAIN",
                    "published file could not be synchronized",
                );
                e.effect = "applied".into();
                e
            })
        })();
        if result.is_err() {
            let _ = self.remove_file(&temp);
        }
        result
    }
    fn publish(&self, from: &str, to: &str, replace: bool) -> Result<()> {
        let a = name(from)?;
        let b = name(to)?;
        let fd = self.file.as_raw_fd();
        let r = if replace {
            unsafe { libc::renameat(fd, a.as_ptr(), fd, b.as_ptr()) }
        } else {
            rename_no_replace(fd, &a, fd, &b)
        };
        if r < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EEXIST) {
                return Err(ContextError::new("ALREADY_EXISTS", "target already exists"));
            }
            return Err(last());
        }
        Ok(())
    }
    pub fn lock(&self, timeout_ms: u64) -> Result<WriteLock> {
        if timeout_ms > 30000 {
            return Err(ContextError::new(
                "INVALID_INPUT",
                "lock timeout exceeds limit",
            ));
        }
        let local = self.create_dir(".context-local")?;
        let c = name("write.lock")?;
        // Separate atomic creation from opening the persistent lock inode.
        // Concurrent non-exclusive O_CREAT opens can return ENOENT on macOS.
        let mut fd = unsafe {
            libc::openat(
                local.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EEXIST) {
            fd = unsafe {
                libc::openat(
                    local.file.as_raw_fd(),
                    c.as_ptr(),
                    libc::O_RDWR | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
        }
        let f = checked_file(fd)?;
        let start = Instant::now();
        loop {
            if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(WriteLock(f));
            }
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::EWOULDBLOCK) {
                return Err(e.into());
            }
            if start.elapsed() >= Duration::from_millis(timeout_ms) {
                return Err(ContextError::new("LOCK_BUSY", "store write lock is busy"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
pub struct WriteLock(File);
impl Drop for WriteLock {
    fn drop(&mut self) {
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}
pub fn random_id() -> String {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("OS randomness unavailable");
    hex::encode(bytes)
}
#[cfg(target_os = "linux")]
fn rename_no_replace(a: i32, x: &CString, b: i32, y: &CString) -> i32 {
    unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            a,
            x.as_ptr(),
            b,
            y.as_ptr(),
            libc::RENAME_NOREPLACE,
        ) as i32
    }
}
#[cfg(target_os = "macos")]
fn rename_no_replace(a: i32, x: &CString, b: i32, y: &CString) -> i32 {
    unsafe { libc::renameatx_np(a, x.as_ptr(), b, y.as_ptr(), libc::RENAME_EXCL) }
}
pub fn publish_new_directory(parent: &Path, stage: &str, target: &str) -> Result<()> {
    let d = Directory::open(parent)?;
    d.publish(stage, target, false)?;
    d.sync().map_err(|_| {
        let mut e = ContextError::new(
            "PERSISTENCE_UNCERTAIN",
            "published directory could not be synchronized",
        );
        e.effect = "applied".into();
        e
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_reuses_one_inode_and_preserves_contention() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        let first = dir.lock(0).unwrap();
        let path = temp.path().join(".context-local/write.lock");
        let inode = std::fs::metadata(&path).unwrap().ino();
        assert_eq!(dir.lock(0).err().unwrap().code, "LOCK_BUSY");
        drop(first);
        let _second = dir.lock(0).unwrap();
        assert_eq!(std::fs::metadata(path).unwrap().ino(), inode);
    }
    #[test]
    fn existing_lock_rejects_symlinks_and_hardlinks() {
        for hardlink in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let dir = Directory::open(temp.path()).unwrap();
            dir.create_dir(".context-local").unwrap();
            let target = temp.path().join("target");
            std::fs::write(&target, b"unchanged").unwrap();
            let lock = temp.path().join(".context-local/write.lock");
            if hardlink {
                std::fs::hard_link(&target, lock).unwrap();
            } else {
                std::os::unix::fs::symlink(&target, lock).unwrap();
            }
            assert_eq!(dir.lock(0).err().unwrap().code, "UNSAFE_PATH");
            assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
        }
    }
    #[test]
    fn enumeration_distinguishes_eof_and_error_and_always_closes_stream() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        dir.write_atomic("fixture", b"fixture", false).unwrap();
        let closed = std::cell::Cell::new(0);
        let observe = |status| {
            assert_eq!(status, 0);
            closed.set(closed.get() + 1);
        };
        let mut emitted = false;
        let error = dir
            .names_with(
                |stream| {
                    if emitted {
                        set_errno(libc::EIO);
                        return std::ptr::null_mut();
                    }
                    loop {
                        let entry = unsafe { libc::readdir(stream) };
                        assert!(!entry.is_null());
                        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
                        if name.to_bytes() == b"fixture" {
                            emitted = true;
                            return entry;
                        }
                    }
                },
                observe,
            )
            .unwrap_err();
        assert_eq!(error.code, "IO_ERROR");
        assert!(emitted);
        assert_eq!(closed.get(), 1);
        set_errno(libc::EIO);
        let names = dir
            .names_with(
                |_| {
                    assert_eq!(errno(), 0);
                    std::ptr::null_mut()
                },
                observe,
            )
            .unwrap();
        assert!(names.is_empty());
        assert_eq!(closed.get(), 2);
        assert_eq!(dir.names().unwrap(), vec!["fixture"]);
    }
    #[test]
    fn final_root_symlink_is_rejected_even_with_normalization_suffixes() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let actual = temp.path().join("actual");
        std::fs::create_dir(&actual).unwrap();
        let alias = temp.path().join("alias");
        symlink(&actual, &alias).unwrap();
        for path in [
            alias.clone(),
            std::path::PathBuf::from(format!("{}/", alias.display())),
            alias.join("."),
            alias.join(".."),
        ] {
            assert_eq!(Directory::open(&path).err().unwrap().code, "UNSAFE_PATH");
        }
        // An ancestor alias remains valid, including macOS system aliases.
        std::fs::create_dir(actual.join("nested")).unwrap();
        assert!(Directory::open(&alias.join("nested")).is_ok());
    }
    #[test]
    fn leaf_replacement_cannot_redirect_held_root_open() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let other = temp.path().join("other");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&other).unwrap();
        let err = Directory::open_with(&root, || {
            std::fs::rename(&root, temp.path().join("original")).unwrap();
            symlink(&other, &root).unwrap();
        })
        .err()
        .unwrap();
        assert_eq!(err.code, "UNSAFE_PATH");
        assert!(Directory::open(&temp.path().join("original")).is_ok());
    }
    #[test]
    fn interruption_before_publication_preserves_original() {
        for boundary in [0, 1] {
            let temp = tempfile::tempdir().unwrap();
            let dir = Directory::open(temp.path()).unwrap();
            dir.write_atomic("entry.md", b"old complete", false)
                .unwrap();
            let err = dir
                .write_atomic_with("entry.md", b"new complete", true, |phase| {
                    if phase == boundary {
                        Err(ContextError::new("IO_ERROR", "injected interruption"))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            assert_eq!(err.effect, "none");
            assert_eq!(dir.read("entry.md", 100).unwrap(), b"old complete");
            assert_eq!(dir.names().unwrap(), vec!["entry.md"]);
        }
    }
    #[test]
    fn interruption_after_publication_reports_applied() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        dir.write_atomic("entry.md", b"old complete", false)
            .unwrap();
        let err = dir
            .write_atomic_with("entry.md", b"new complete", true, |phase| {
                if phase == 2 {
                    Err(ContextError::new(
                        "IO_ERROR",
                        "injected directory sync failure",
                    ))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(err.code, "PERSISTENCE_UNCERTAIN");
        assert_eq!(err.effect, "applied");
        assert_eq!(dir.read("entry.md", 100).unwrap(), b"new complete");
    }
    #[test]
    fn create_publication_never_overwrites_existing_file_or_directory() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        dir.write_atomic("entry.md", b"original", false).unwrap();
        assert_eq!(
            dir.write_atomic("entry.md", b"other", false)
                .unwrap_err()
                .code,
            "ALREADY_EXISTS"
        );
        assert_eq!(dir.read("entry.md", 100).unwrap(), b"original");
        dir.create_dir_new("target").unwrap();
        dir.create_dir_new("stage").unwrap();
        assert_eq!(
            dir.publish_new_directory("stage", "target")
                .unwrap_err()
                .code,
            "ALREADY_EXISTS"
        );
        assert!(dir.child("stage").is_ok());
        assert!(dir.child("target").is_ok());
    }
}
