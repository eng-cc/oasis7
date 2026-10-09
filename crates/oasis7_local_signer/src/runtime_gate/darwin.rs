// SAFETY: Darwin ABI calls are isolated here and each unsafe block documents
// its pointer, descriptor, or ownership preconditions locally.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use nix::fcntl::{OFlag, openat};
use nix::sys::stat::Mode;
use sha2::{Digest, Sha256};

use super::macho;
use super::{GateError, MAX_BOOTSTRAP_BYTES, RuntimeAttestation};

pub(super) const RUNTIME_PATH: &str = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9";
pub(super) const FRAMEWORK_PATH: &str =
    "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9";
const FRAMEWORK_DYLIB_PATH: &str =
    "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Python3";
const DYNLOAD_DIR: &str = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/lib/python3.9/lib-dynload";
const EXPECTED_RUNTIME_CDHASH: &str = "77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf";
const EXPECTED_FRAMEWORK_CDHASH: &str = "a43551195b8d2eefd9356d81c1098ffc9e0a8b47";
const APPLE_REQUIREMENT: &str = "anchor apple and identifier \"com.apple.python3\"";
const MAX_MACHO_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ATTR_BYTES: usize = 8192;
const ATTR_CMN_RETURNED_ATTRS: u32 = 0x8000_0000;
const ATTR_CMN_EXTENDED_SECURITY: u32 = 0x0040_0000;
const ATTR_VOL_CAPABILITIES: u32 = 0x0002_0000;
const VOL_CAP_INT_EXTENDED_SECURITY: u32 = 0x0000_0400;
const FSOPT_REPORT_FULLSIZE: libc::c_ulong = 0x0000_0004;
const MNT_LOCAL: u64 = 0x0000_1000;
const MNT_AUTOMOUNTED: u64 = 0x0040_0000;

#[repr(C)]
struct AttrList {
    bitmapcount: u16,
    reserved: u16,
    commonattr: u32,
    volattr: u32,
    dirattr: u32,
    fileattr: u32,
    forkattr: u32,
}

unsafe extern "C" {
    fn fgetattrlist(
        fd: libc::c_int,
        list: *mut AttrList,
        attributes: *mut c_void,
        attribute_size: libc::size_t,
        options: libc::c_ulong,
    ) -> libc::c_int;
    fn sysctlbyname(
        name: *const c_char,
        old_value: *mut c_void,
        old_length: *mut libc::size_t,
        new_value: *mut c_void,
        new_length: libc::size_t,
    ) -> libc::c_int;
}

#[derive(Debug)]
struct HeldPath {
    path: PathBuf,
    descriptors: Vec<File>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjectIdentity {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    nlink: u64,
    size: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}

impl ObjectIdentity {
    fn from(file: &File) -> Result<Self, GateError> {
        let metadata = file.metadata().map_err(|_| GateError::UnsafeMetadata)?;
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            nlink: metadata.nlink(),
            size: metadata.size(),
            mtime: (metadata.mtime(), metadata.mtime_nsec()),
            ctime: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

fn same_object_identity(expected: &ObjectIdentity, observed: &ObjectIdentity) -> bool {
    expected == observed
}

#[derive(Clone, Debug)]
pub(super) struct RuntimeFacts {
    pub(super) runtime_dev: String,
    pub(super) runtime_ino: String,
    pub(super) runtime_mode: String,
    pub(super) runtime_uid: String,
    pub(super) framework_dev: String,
    pub(super) framework_ino: String,
    pub(super) framework_mode: String,
    pub(super) framework_uid: String,
    pub(super) runtime_cdhash: String,
    pub(super) framework_cdhash: String,
    pub(super) runtime_arch: String,
    pub(super) os_build: String,
}

#[derive(Debug)]
pub(super) struct VerifiedRuntime {
    pub(super) attestation: RuntimeAttestation,
    pub(super) bootstrap: Vec<u8>,
    held_paths: Vec<HeldPath>,
}

impl VerifiedRuntime {
    pub(super) fn recheck(&self) -> Result<(), GateError> {
        for path in &self.held_paths {
            path.recheck()?;
        }
        Ok(())
    }

    pub(super) fn held_fds(&self) -> Vec<libc::c_int> {
        self.held_paths
            .iter()
            .flat_map(|path| path.descriptors.iter().map(AsRawFd::as_raw_fd))
            .collect()
    }
}

pub(super) fn verify_runtime(
    bootstrap_path: &Path,
    expected_bootstrap_sha256: &str,
) -> Result<VerifiedRuntime, GateError> {
    validate_digest(expected_bootstrap_sha256)?;
    let runtime = open_protected_path(Path::new(RUNTIME_PATH), NodeKind::Regular, true)?;
    let framework = open_protected_path(Path::new(FRAMEWORK_PATH), NodeKind::Directory, true)?;
    let framework_dylib =
        open_protected_path(Path::new(FRAMEWORK_DYLIB_PATH), NodeKind::Regular, true)?;
    let bootstrap = open_protected_path(bootstrap_path, NodeKind::Regular, false)?;
    let bootstrap_bytes = read_bootstrap(&bootstrap, expected_bootstrap_sha256)?;
    let runtime_bytes = read_bounded_file(runtime.last()?, MAX_MACHO_BYTES)?;
    let framework_bytes = read_bounded_file(framework_dylib.last()?, MAX_MACHO_BYTES)?;

    let extension_paths = [
        "_blake2.cpython-39-darwin.so",
        "_bz2.cpython-39-darwin.so",
        "_ctypes.cpython-39-darwin.so",
        "fcntl.cpython-39-darwin.so",
        "_hashlib.cpython-39-darwin.so",
        "_heapq.cpython-39-darwin.so",
        "_json.cpython-39-darwin.so",
        "_lzma.cpython-39-darwin.so",
        "_posixsubprocess.cpython-39-darwin.so",
        "_sha3.cpython-39-darwin.so",
        "_struct.cpython-39-darwin.so",
        "grp.cpython-39-darwin.so",
        "math.cpython-39-darwin.so",
        "select.cpython-39-darwin.so",
        "zlib.cpython-39-darwin.so",
    ];
    let mut extensions = Vec::with_capacity(extension_paths.len());
    for name in extension_paths {
        let path = Path::new(DYNLOAD_DIR).join(name);
        let held = open_protected_path(&path, NodeKind::Regular, true)?;
        let bytes = read_bounded_file(held.last()?, MAX_MACHO_BYTES)?;
        macho::verify_component(&bytes, name)?;
        extensions.push((held, bytes));
    }
    macho::verify_component(&runtime_bytes, "python3.9")?;
    macho::verify_component(&framework_bytes, "Python3")?;

    let runtime_cdhash = verify_apple_code(Path::new(RUNTIME_PATH), EXPECTED_RUNTIME_CDHASH)?;
    let framework_cdhash = verify_apple_code(Path::new(FRAMEWORK_PATH), EXPECTED_FRAMEWORK_CDHASH)?;

    runtime.recheck()?;
    framework.recheck()?;
    framework_dylib.recheck()?;
    bootstrap.recheck()?;
    for (extension, _) in &extensions {
        extension.recheck()?;
    }

    let runtime_id = ObjectIdentity::from(runtime.last()?)?;
    let framework_id = ObjectIdentity::from(framework.last()?)?;
    let facts = RuntimeFacts {
        runtime_dev: runtime_id.dev.to_string(),
        runtime_ino: runtime_id.ino.to_string(),
        runtime_mode: runtime_id.mode.to_string(),
        runtime_uid: runtime_id.uid.to_string(),
        framework_dev: framework_id.dev.to_string(),
        framework_ino: framework_id.ino.to_string(),
        framework_mode: framework_id.mode.to_string(),
        framework_uid: framework_id.uid.to_string(),
        runtime_cdhash,
        framework_cdhash,
        runtime_arch: process_architecture()?.to_owned(),
        os_build: os_build()?,
    };
    let attestation = RuntimeAttestation::from_facts(facts, expected_bootstrap_sha256);
    let mut held_paths = vec![runtime, framework, framework_dylib, bootstrap];
    held_paths.extend(extensions.into_iter().map(|(path, _bytes)| path));
    Ok(VerifiedRuntime {
        attestation,
        bootstrap: bootstrap_bytes,
        held_paths,
    })
}

impl HeldPath {
    fn last(&self) -> Result<&File, GateError> {
        self.descriptors.last().ok_or(GateError::UnsafePath)
    }

    fn recheck(&self) -> Result<(), GateError> {
        let fresh = open_protected_path(&self.path, NodeKind::Any, false)?;
        if fresh.descriptors.len() != self.descriptors.len() {
            return Err(GateError::PathChanged);
        }
        for (old, new) in self.descriptors.iter().zip(&fresh.descriptors) {
            validate_descriptor(old, NodeKind::Any, false)?;
            validate_descriptor(new, NodeKind::Any, false)?;
            if !same_object_identity(&ObjectIdentity::from(old)?, &ObjectIdentity::from(new)?) {
                return Err(GateError::PathChanged);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum NodeKind {
    Any,
    Directory,
    Regular,
}

fn path_components(path: &Path) -> Result<Vec<CString>, GateError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.first() != Some(&b'/') || bytes.contains(&0) || bytes.len() > 4096 {
        return Err(GateError::UnsafePath);
    }
    let mut components = Vec::new();
    for component in bytes[1..].split(|byte| *byte == b'/') {
        if component.is_empty() || component == b"." || component == b".." {
            return Err(GateError::UnsafePath);
        }
        components.push(CString::new(component).map_err(|_| GateError::UnsafePath)?);
    }
    if components.is_empty() {
        return Err(GateError::UnsafePath);
    }
    Ok(components)
}

fn open_protected_path(
    path: &Path,
    kind: NodeKind,
    executable: bool,
) -> Result<HeldPath, GateError> {
    let components = path_components(path)?;
    let root = File::open("/").map_err(|_| GateError::UnsafePath)?;
    validate_descriptor(&root, NodeKind::Directory, true)?;
    let mut descriptors = vec![root];
    for (index, component) in components.iter().enumerate() {
        let is_last = index + 1 == components.len();
        let expected = if is_last { kind } else { NodeKind::Directory };
        let flags = OFlag::O_RDONLY
            | OFlag::O_CLOEXEC
            | OFlag::O_NOFOLLOW
            | OFlag::O_NONBLOCK
            | if matches!(expected, NodeKind::Directory) {
                OFlag::O_DIRECTORY
            } else {
                OFlag::empty()
            };
        let descriptor = openat(
            descriptors.last().ok_or(GateError::UnsafePath)?.as_fd(),
            component.as_c_str(),
            flags,
            Mode::empty(),
        )
        .map_err(|_| GateError::UnsafePath)?;
        let file = File::from(descriptor);
        validate_descriptor(&file, expected, is_last && executable)?;
        descriptors.push(file);
    }
    Ok(HeldPath {
        path: path.to_path_buf(),
        descriptors,
    })
}

fn validate_descriptor(file: &File, kind: NodeKind, executable: bool) -> Result<(), GateError> {
    let metadata = file.metadata().map_err(|_| GateError::UnsafeMetadata)?;
    let is_dir = metadata.is_dir();
    let is_file = metadata.is_file();
    let valid_type = match kind {
        NodeKind::Any => is_dir || is_file,
        NodeKind::Directory => is_dir,
        NodeKind::Regular => is_file,
    };
    let mode = metadata.mode();
    if !valid_type
        || metadata.uid() != 0
        || !super::protected_mode_bits(mode, executable)
        || (is_file && metadata.nlink() != 1)
        || (is_dir && metadata.nlink() < 2)
    {
        return Err(GateError::UnsafeMetadata);
    }
    validate_filesystem(file)?;
    validate_acl(file)?;
    Ok(())
}

fn validate_filesystem(file: &File) -> Result<(), GateError> {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: statfs is a plain C output record and all-zero is a valid initializer.
        let mut info: libc::statfs = unsafe { std::mem::zeroed() };
        // SAFETY: `info` is a correctly sized writable Darwin `statfs`; fd is open.
        if unsafe { libc::fstatfs(file.as_raw_fd(), &mut info) } != 0 {
            return Err(GateError::FilesystemUnverifiable);
        }
        // SAFETY: fstatfs writes a NUL-terminated fixed-width filesystem name.
        let name = unsafe { CStr::from_ptr(info.f_fstypename.as_ptr()) };
        let flags = info.f_flags as u64;
        if name.to_bytes() != b"apfs" || flags & MNT_LOCAL == 0 || flags & MNT_AUTOMOUNTED != 0 {
            return Err(GateError::UnsupportedFilesystem);
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = file;
        Err(GateError::UnsupportedPlatform)
    }
}

fn query_attributes(file: &File, request: AttrList, capacity: usize) -> Result<Vec<u8>, GateError> {
    let mut buffer = vec![0u8; capacity];
    // SAFETY: request and buffer remain live, and fgetattrlist reads only the open fd.
    let result = unsafe {
        fgetattrlist(
            file.as_raw_fd(),
            (&request as *const AttrList).cast_mut(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            FSOPT_REPORT_FULLSIZE,
        )
    };
    if result != 0 {
        return Err(GateError::AclUnverifiable);
    }
    let returned = usize::try_from(u32::from_ne_bytes(
        buffer
            .get(..4)
            .ok_or(GateError::AclMalformed)?
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    ))
    .map_err(|_| GateError::AclMalformed)?;
    if returned < 4 || returned > capacity {
        return Err(GateError::AclMalformed);
    }
    buffer.truncate(returned);
    Ok(buffer)
}

fn validate_acl(file: &File) -> Result<(), GateError> {
    let volume_request = AttrList {
        bitmapcount: 5,
        reserved: 0,
        commonattr: 0,
        volattr: ATTR_CMN_RETURNED_ATTRS | ATTR_VOL_CAPABILITIES,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let volume = query_attributes(file, volume_request, 64)?;
    if volume.len() != 36 {
        return Err(GateError::AclMalformed);
    }
    let capabilities = read_u32_array(&volume, 4, 4)?;
    let valid = read_u32_array(&volume, 20, 4)?;
    if valid[1] & VOL_CAP_INT_EXTENDED_SECURITY == 0
        || capabilities[1] & VOL_CAP_INT_EXTENDED_SECURITY == 0
    {
        return Err(GateError::AclUnverifiable);
    }

    let security_request = AttrList {
        bitmapcount: 5,
        reserved: 0,
        commonattr: ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_EXTENDED_SECURITY,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let security = query_attributes(file, security_request, MAX_ATTR_BYTES)?;
    if security.len() < 32 {
        return Err(GateError::AclMalformed);
    }
    let returned = read_u32_array(&security, 4, 5)?;
    if returned[0] & ATTR_CMN_RETURNED_ATTRS == 0 || returned[1..].iter().any(|value| *value != 0) {
        return Err(GateError::AclMalformed);
    }
    let allowed = ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_EXTENDED_SECURITY;
    if returned[0] & !allowed != 0 {
        return Err(GateError::AclMalformed);
    }
    let reference = security.get(24..32).ok_or(GateError::AclMalformed)?;
    if returned[0] & ATTR_CMN_EXTENDED_SECURITY == 0 {
        if returned[0] != ATTR_CMN_RETURNED_ATTRS
            || security.len() != 32
            || reference.iter().any(|byte| *byte != 0)
        {
            return Err(GateError::AclMalformed);
        }
        return Ok(());
    }
    let offset = i32::from_ne_bytes(
        reference[..4]
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    );
    let length = usize::try_from(u32::from_ne_bytes(
        reference[4..]
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    ))
    .map_err(|_| GateError::AclMalformed)?;
    let start = 24usize
        .checked_add(usize::try_from(offset).map_err(|_| GateError::AclMalformed)?)
        .ok_or(GateError::AclMalformed)?;
    let end = start.checked_add(length).ok_or(GateError::AclMalformed)?;
    if offset != 8 || length != 44 || start != 32 || end != security.len() {
        return Err(GateError::AclMalformed);
    }
    let filesec = &security[start..end];
    let magic = u32::from_ne_bytes(
        filesec[..4]
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    );
    let entry_count = u32::from_ne_bytes(
        filesec[36..40]
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    );
    let acl_flags = u32::from_ne_bytes(
        filesec[40..44]
            .try_into()
            .map_err(|_| GateError::AclMalformed)?,
    );
    if magic != 0x012c_c16d || entry_count != u32::MAX || acl_flags != 0 {
        return Err(GateError::AclPresentOrMalformed);
    }
    Ok(())
}

fn read_u32_array(bytes: &[u8], offset: usize, count: usize) -> Result<Vec<u32>, GateError> {
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let start = offset
            .checked_add(index.checked_mul(4).ok_or(GateError::AclMalformed)?)
            .ok_or(GateError::AclMalformed)?;
        values.push(u32::from_ne_bytes(
            bytes
                .get(start..start + 4)
                .ok_or(GateError::AclMalformed)?
                .try_into()
                .map_err(|_| GateError::AclMalformed)?,
        ));
    }
    Ok(values)
}

fn read_bounded_file(file: &File, max: u64) -> Result<Vec<u8>, GateError> {
    let before = ObjectIdentity::from(file)?;
    if before.size == 0 || before.size > max {
        return Err(GateError::UnsafeMetadata);
    }
    let capacity = usize::try_from(before.size).map_err(|_| GateError::UnsafeMetadata)?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut reader = file.try_clone().map_err(|_| GateError::UnsafeMetadata)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|_| GateError::UnsafeMetadata)?;
    reader
        .take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| GateError::UnsafeMetadata)?;
    if bytes.is_empty() || bytes.len() as u64 > max || ObjectIdentity::from(file)? != before {
        return Err(GateError::PathChanged);
    }
    Ok(bytes)
}

fn read_bootstrap(path: &HeldPath, expected_sha256: &str) -> Result<Vec<u8>, GateError> {
    let bytes = read_bounded_file(path.last()?, MAX_BOOTSTRAP_BYTES as u64)?;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        return Err(GateError::BootstrapInvalid);
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    if digest != expected_sha256 {
        return Err(GateError::BootstrapDigestMismatch);
    }
    path.recheck()?;
    Ok(bytes)
}

#[cfg(test)]
pub(super) fn open_and_hash_probe(path: &Path) -> Result<Vec<u8>, GateError> {
    let held = open_protected_path(path, NodeKind::Regular, false)?;
    let bytes = read_bounded_file(held.last()?, MAX_BOOTSTRAP_BYTES as u64)?;
    held.recheck()?;
    Ok(bytes)
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn retained_name_recheck_detects_descriptor_identity_drift() {
        let identity = ObjectIdentity {
            dev: 1,
            ino: 2,
            uid: 0,
            gid: 0,
            mode: 0o100755,
            nlink: 1,
            size: 64,
            mtime: (10, 20),
            ctime: (30, 40),
        };
        assert!(same_object_identity(&identity, &identity));
        let mut replaced = identity.clone();
        replaced.ino += 1;
        assert!(!same_object_identity(&identity, &replaced));
        let mut mutated = identity.clone();
        mutated.mtime.1 += 1;
        assert!(!same_object_identity(&identity, &mutated));
    }
}

fn validate_digest(value: &str) -> Result<(), GateError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(GateError::InvalidDigest);
    }
    Ok(())
}

fn process_architecture() -> Result<&'static str, GateError> {
    match std::env::consts::ARCH {
        "aarch64" => Ok("arm64"),
        "x86_64" => Ok("x86_64"),
        _ => Err(GateError::UnsupportedArchitecture),
    }
}

fn os_build() -> Result<String, GateError> {
    let name = CString::new("kern.osversion").map_err(|_| GateError::SystemIdentityUnavailable)?;
    let mut length = 0usize;
    // SAFETY: first call asks the kernel for the required length only.
    if unsafe {
        sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || length == 0
        || length > 128
    {
        return Err(GateError::SystemIdentityUnavailable);
    }
    let mut value = vec![0u8; length];
    // SAFETY: value is writable for the kernel-reported bounded length.
    if unsafe {
        sysctlbyname(
            name.as_ptr(),
            value.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || length == 0
        || length > value.len()
    {
        return Err(GateError::SystemIdentityUnavailable);
    }
    value.truncate(length);
    if value.last() == Some(&0) {
        value.pop();
    }
    if value.is_empty()
        || !value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(byte))
    {
        return Err(GateError::SystemIdentityUnavailable);
    }
    String::from_utf8(value).map_err(|_| GateError::SystemIdentityUnavailable)
}

#[allow(unsafe_code)]
mod security {
    use super::{CString, GateError, Path, c_char, c_void};
    use std::os::unix::ffi::OsStrExt;

    type CfType = *const c_void;
    type SecCode = *mut c_void;
    type SecRequirement = *mut c_void;
    type SecFlags = u32;

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const K_SEC_CS_CHECK_ALL_ARCHITECTURES: SecFlags = 1 << 0;
    const K_SEC_CS_CHECK_NESTED_CODE: SecFlags = 1 << 3;
    const K_SEC_CS_STRICT_VALIDATE: SecFlags = 1 << 4;
    const K_SEC_CS_SINGLE_THREADED: SecFlags = 1 << 12;
    const K_SEC_CS_INTERNAL_INFORMATION: SecFlags = 1 << 0;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: CfType,
            bytes: *const u8,
            length: isize,
            is_directory: u8,
        ) -> *mut c_void;
        fn CFStringCreateWithCString(
            allocator: CfType,
            c_string: *const c_char,
            encoding: u32,
        ) -> *mut c_void;
        fn CFRelease(value: CfType);
        fn CFDictionaryGetValue(dictionary: CfType, key: CfType) -> CfType;
        fn CFDataGetLength(data: CfType) -> isize;
        fn CFDataGetBytePtr(data: CfType) -> *const u8;
    }

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecStaticCodeCreateWithPath(path: CfType, flags: SecFlags, code: *mut SecCode) -> i32;
        fn SecRequirementCreateWithString(
            text: CfType,
            flags: SecFlags,
            requirement: *mut SecRequirement,
        ) -> i32;
        fn SecStaticCodeCheckValidity(
            code: SecCode,
            flags: SecFlags,
            requirement: SecRequirement,
        ) -> i32;
        fn SecCodeCopySigningInformation(code: SecCode, flags: SecFlags, info: *mut CfType) -> i32;
        static kSecCodeInfoUnique: CfType;
    }

    struct OwnedCf(*mut c_void);

    impl Drop for OwnedCf {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: each OwnedCf is created from a retained CoreFoundation result.
                unsafe { CFRelease(self.0) };
            }
        }
    }

    fn create_code(path: &Path) -> Result<OwnedCf, GateError> {
        let bytes = path.as_os_str().as_bytes();
        let directory = u8::from(path.is_dir());
        // SAFETY: path bytes are retained during the call and contain no NUL.
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                std::ptr::null(),
                bytes.as_ptr(),
                isize::try_from(bytes.len()).map_err(|_| GateError::SignatureFailure)?,
                directory,
            )
        };
        if url.is_null() {
            return Err(GateError::SignatureFailure);
        }
        let url = OwnedCf(url);
        let mut code = std::ptr::null_mut();
        // SAFETY: url is a live file URL and code is a valid output pointer.
        if unsafe { SecStaticCodeCreateWithPath(url.0, 0, &mut code) } != 0 || code.is_null() {
            return Err(GateError::SignatureFailure);
        }
        Ok(OwnedCf(code))
    }

    fn create_requirement() -> Result<OwnedCf, GateError> {
        let text =
            CString::new(super::APPLE_REQUIREMENT).map_err(|_| GateError::SignatureFailure)?;
        // SAFETY: static requirement text is NUL-terminated for the call.
        let string = unsafe {
            CFStringCreateWithCString(std::ptr::null(), text.as_ptr(), K_CF_STRING_ENCODING_UTF8)
        };
        if string.is_null() {
            return Err(GateError::SignatureFailure);
        }
        let string = OwnedCf(string);
        let mut requirement = std::ptr::null_mut();
        // SAFETY: string is a valid CFString and requirement is an output pointer.
        if unsafe { SecRequirementCreateWithString(string.0, 0, &mut requirement) } != 0
            || requirement.is_null()
        {
            return Err(GateError::SignatureFailure);
        }
        Ok(OwnedCf(requirement))
    }

    pub(super) fn verify(path: &Path, expected_cdhash: &str) -> Result<String, GateError> {
        let code = create_code(path)?;
        let requirement = create_requirement()?;
        let flags = K_SEC_CS_CHECK_ALL_ARCHITECTURES
            | K_SEC_CS_CHECK_NESTED_CODE
            | K_SEC_CS_STRICT_VALIDATE
            | K_SEC_CS_SINGLE_THREADED;
        // SAFETY: code and requirement are live objects; this requests full code/resource validation.
        if unsafe { SecStaticCodeCheckValidity(code.0, flags, requirement.0) } != 0 {
            return Err(GateError::SignatureFailure);
        }
        let mut info: CfType = std::ptr::null();
        // SAFETY: signing information output is retained by Security on success.
        if unsafe {
            SecCodeCopySigningInformation(code.0, K_SEC_CS_INTERNAL_INFORMATION, &mut info)
        } != 0
            || info.is_null()
        {
            return Err(GateError::SignatureFailure);
        }
        let info = OwnedCf(info.cast_mut());
        // SAFETY: the static Security CFString key and retained dictionary are valid.
        let data = unsafe { CFDictionaryGetValue(info.0, kSecCodeInfoUnique) };
        if data.is_null() {
            return Err(GateError::SignatureFailure);
        }
        // SAFETY: returned CFData remains owned by info during these reads.
        let length = unsafe { CFDataGetLength(data) };
        let bytes = unsafe { CFDataGetBytePtr(data) };
        if length != 20 || bytes.is_null() {
            return Err(GateError::SignatureFailure);
        }
        // SAFETY: length was checked as 20 and CFData owns that many bytes.
        let cdhash = hex::encode(unsafe { std::slice::from_raw_parts(bytes, 20) });
        if cdhash != expected_cdhash {
            return Err(GateError::WrongCodeDirectoryHash);
        }
        Ok(cdhash)
    }
}

fn verify_apple_code(path: &Path, expected_cdhash: &str) -> Result<String, GateError> {
    security::verify(path, expected_cdhash)
}
