use super::{GitError, error};
use crate::fs::Directory;
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

fn changed() -> GitError {
    error(
        "HISTORY_IDENTITY_CHANGED",
        "Git repository metadata identity changed",
    )
}
fn unavailable() -> GitError {
    error(
        "HISTORY_UNAVAILABLE",
        "Git repository metadata is unavailable or unsafe",
    )
}
fn identity(path: &Path) -> Result<(u64, u64), GitError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| changed())?;
    if metadata.file_type().is_symlink() {
        return Err(changed());
    }
    Ok((metadata.dev(), metadata.ino()))
}
struct FrozenFile {
    identity: (u64, u64),
    bytes: Vec<u8>,
}
fn frozen_file(directory: &Directory, path: &Path, name: &str) -> Result<FrozenFile, GitError> {
    let before = identity(path)?;
    let bytes = directory.read(name, 8192).map_err(|_| unavailable())?;
    if identity(path)? != before {
        return Err(changed());
    }
    Ok(FrozenFile {
        identity: before,
        bytes,
    })
}
fn resolve(base: &Path, bytes: &[u8]) -> Result<PathBuf, GitError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| unavailable())?
        .trim_end_matches(['\r', '\n']);
    if text.is_empty() || text.contains(['\r', '\n', '\0']) {
        return Err(unavailable());
    }
    let path = Path::new(text);
    fs::canonicalize(if path.is_absolute() {
        path.into()
    } else {
        base.join(path)
    })
    .map_err(|_| unavailable())
}
pub(super) struct RepositoryIdentity {
    worktree: Directory,
    git: Option<(PathBuf, Directory)>,
    common: Option<(PathBuf, Directory)>,
    gitfile: Option<FrozenFile>,
    commondir: Option<FrozenFile>,
}
impl RepositoryIdentity {
    pub(super) fn capture(root: &Path, held: Option<&Directory>) -> Result<Self, GitError> {
        let worktree = match held {
            Some(dir) => dir.try_clone(),
            None => Directory::open(root),
        }
        .map_err(|_| unavailable())?;
        if !worktree.matches_path(root).map_err(|_| changed())? {
            return Err(changed());
        }
        let entry = root.join(".git");
        let metadata = match fs::symlink_metadata(&entry) {
            Ok(metadata) => Some(metadata),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(unavailable()),
        };
        let mut result = Self {
            worktree,
            git: None,
            common: None,
            gitfile: None,
            commondir: None,
        };
        let Some(metadata) = metadata else {
            return Err(unavailable());
        };
        let git_path = if metadata.is_dir() && !metadata.file_type().is_symlink() {
            entry.clone()
        } else if metadata.is_file() && !metadata.file_type().is_symlink() {
            let file = frozen_file(&result.worktree, &entry, ".git")?;
            let bytes = file
                .bytes
                .strip_prefix(b"gitdir: ")
                .ok_or_else(unavailable)?;
            let path = resolve(root, bytes)?;
            result.gitfile = Some(file);
            path
        } else {
            return Err(unavailable());
        };
        let expected = identity(&git_path)?;
        let git = Directory::open(&git_path).map_err(|_| unavailable())?;
        if git.identity().map_err(|_| changed())? != expected {
            return Err(changed());
        }
        let common_path = match fs::symlink_metadata(git_path.join("commondir")) {
            Ok(_) => {
                let file = frozen_file(&git, &git_path.join("commondir"), "commondir")?;
                let path = resolve(&git_path, &file.bytes)?;
                result.commondir = Some(file);
                path
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => git_path.clone(),
            Err(_) => return Err(unavailable()),
        };
        let expected_common = identity(&common_path)?;
        let common = Directory::open(&common_path).map_err(|_| unavailable())?;
        if common.identity().map_err(|_| changed())? != expected_common {
            return Err(changed());
        }
        result.git = Some((git_path, git));
        result.common = Some((common_path, common));
        result.verify(root)?;
        Ok(result)
    }
    pub(super) fn verify(&self, root: &Path) -> Result<(), GitError> {
        if !self.worktree.matches_path(root).map_err(|_| changed())? {
            return Err(changed());
        }
        let Some((git_path, git)) = &self.git else {
            return match fs::symlink_metadata(root.join(".git")) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(changed()),
            };
        };
        if !git.matches_path(git_path).map_err(|_| changed())? {
            return Err(changed());
        }
        if let Some(file) = &self.gitfile {
            let bytes = file.bytes.strip_prefix(b"gitdir: ").ok_or_else(changed)?;
            if resolve(root, bytes).map_err(|_| changed())? != *git_path {
                return Err(changed());
            }
            if identity(&root.join(".git"))? != file.identity
                || self.worktree.read(".git", 8192).map_err(|_| changed())? != file.bytes
            {
                return Err(changed());
            }
        }
        if let Some((path, common)) = &self.common {
            if !common.matches_path(path).map_err(|_| changed())? {
                return Err(changed());
            }
        }
        if let Some(file) = &self.commondir {
            if resolve(git_path, &file.bytes).map_err(|_| changed())?
                != self.common.as_ref().ok_or_else(changed)?.0
            {
                return Err(changed());
            }
            if identity(&git_path.join("commondir"))? != file.identity
                || git.read("commondir", 8192).map_err(|_| changed())? != file.bytes
            {
                return Err(changed());
            }
        } else {
            match fs::symlink_metadata(git_path.join("commondir")) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                _ => return Err(changed()),
            }
        }
        Ok(())
    }
    pub(super) fn pin(&self, command: &mut Command) -> Result<(), GitError> {
        self.worktree
            .pin_command_cwd(command)
            .map_err(|_| unavailable())?;
        Ok(())
    }
}
