//! Working-tree reads must never follow a document or ancestor symlink.
use std::path::{Component, Path};

pub(super) fn read(root: &Path, relative: &str, remaining: usize) -> Result<String, &'static str> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("path_unavailable");
    }
    let file = open(root, path)?;
    super::coverage::read_open_document(file, remaining)
}

#[cfg(unix)]
fn reason(error: std::io::Error) -> &'static str {
    if error.raw_os_error() == Some(libc::ELOOP) {
        return "symlink";
    }
    match error.kind() {
        std::io::ErrorKind::NotFound => "deleted",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "path_unavailable",
    }
}

#[cfg(unix)]
fn open(root: &Path, path: &Path) -> Result<std::fs::File, &'static str> {
    open_checked(root, path, |_| {})
}

#[cfg(unix)]
fn open_checked(
    root: &Path,
    path: &Path,
    mut after_metadata: impl FnMut(&Path),
) -> Result<std::fs::File, &'static str> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    let mut directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)
        .map_err(reason)?;
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        let name =
            std::ffi::CString::new(part.as_os_str().as_bytes()).map_err(|_| "path_unavailable")?;
        let is_directory = parts.peek().is_some();
        // lstat relative to the held directory: classify links without opening their targets.
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        let status = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if status != 0 {
            return Err(reason(std::io::Error::last_os_error()));
        }
        let metadata = unsafe { metadata.assume_init() };
        if metadata.st_mode & libc::S_IFMT == libc::S_IFLNK {
            return Err("symlink");
        }
        after_metadata(Path::new(part.as_os_str()));
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if is_directory { libc::O_DIRECTORY } else { 0 };
        // Each ancestor is held by descriptor, so renaming/replacing it cannot redirect this read.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(reason(std::io::Error::last_os_error()));
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        if !is_directory {
            return Ok(file);
        }
        directory = file;
    }
    Err("path_unavailable")
}

#[cfg(not(unix))]
fn open(root: &Path, path: &Path) -> Result<std::fs::File, &'static str> {
    // The supported desktop host is Unix. Fail closed elsewhere until an equivalent
    // handle-relative reader is available; a check followed by open is raceable.
    let _ = (root, path);
    Err("unsupported_confined_read")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn replacement_between_metadata_and_open_cannot_follow_leaf_or_ancestor() {
        for ancestor in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            fs::create_dir(root.join("doc")).unwrap();
            fs::create_dir(root.join("private")).unwrap();
            fs::write(root.join("doc/prd.md"), "Original").unwrap();
            fs::write(root.join("private/prd.md"), "Secret").unwrap();
            let result = open_checked(root, Path::new("doc/prd.md"), |part| {
                if ancestor && part == Path::new("doc") {
                    fs::rename(root.join("doc"), root.join("old-doc")).unwrap();
                    symlink(root.join("private"), root.join("doc")).unwrap();
                } else if !ancestor && part == Path::new("prd.md") {
                    fs::remove_file(root.join("doc/prd.md")).unwrap();
                    symlink(root.join("private/prd.md"), root.join("doc/prd.md")).unwrap();
                }
            });
            assert!(result.is_err());
        }
    }

    #[test]
    fn held_ancestor_is_not_redirected_when_its_path_is_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("doc")).unwrap();
        fs::create_dir(root.join("private")).unwrap();
        fs::write(root.join("doc/prd.md"), "Original").unwrap();
        fs::write(root.join("private/prd.md"), "Secret").unwrap();
        let file = open_checked(root, Path::new("doc/prd.md"), |part| {
            if part == Path::new("prd.md") {
                fs::rename(root.join("doc"), root.join("old-doc")).unwrap();
                symlink(root.join("private"), root.join("doc")).unwrap();
            }
        })
        .unwrap();
        assert_eq!(
            super::super::coverage::read_open_document(file, 1024).unwrap(),
            "Original"
        );
        assert_eq!(read(root, "doc/prd.md", 1024), Err("symlink"));
    }
}
