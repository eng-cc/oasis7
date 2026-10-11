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
pub struct Directory {
    file: File,
}
impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(ContextError::new("UNSAFE_PATH", "root must be absolute"));
        }
        let resolved = std::fs::canonicalize(path)?;
        let path = resolved.as_path();
        let fd = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(last());
        }
        let mut d = Self {
            file: unsafe { File::from_raw_fd(fd) },
        };
        for c in path.components() {
            match c {
                std::path::Component::RootDir => (),
                std::path::Component::Normal(s) => {
                    let s = s
                        .to_str()
                        .ok_or_else(|| ContextError::new("UNSAFE_PATH", "path must be UTF-8"))?;
                    d = d.child(s)?
                }
                _ => return Err(ContextError::new("UNSAFE_PATH", "invalid root path")),
            }
        }
        Ok(d)
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
        self.child(n)
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
        let dup = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if dup < 0 {
            return Err(last());
        }
        let dir = unsafe { libc::fdopendir(dup) };
        if dir.is_null() {
            unsafe { libc::close(dup) };
            return Err(last());
        }
        let mut names = Vec::new();
        loop {
            let e = unsafe { libc::readdir(dir) };
            if e.is_null() {
                break;
            }
            let b = unsafe { std::ffi::CStr::from_ptr((*e).d_name.as_ptr()) };
            let s = b
                .to_str()
                .map_err(|_| ContextError::new("UNSAFE_PATH", "invalid filename"));
            match s {
                Ok(s) if s != "." && s != ".." => {
                    if names.len() >= 10002 {
                        unsafe { libc::closedir(dir) };
                        return Err(ContextError::new(
                            "LIMIT_EXCEEDED",
                            "directory object count exceeds scan limit",
                        ));
                    }
                    names.push(s.into())
                }
                Ok(_) => (),
                Err(e) => {
                    unsafe { libc::closedir(dir) };
                    return Err(e);
                }
            }
        }
        unsafe { libc::closedir(dir) };
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
        let fd = unsafe {
            libc::openat(
                local.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600,
            )
        };
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
