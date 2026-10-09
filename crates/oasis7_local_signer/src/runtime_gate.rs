//! Independently approved pre-Python gate for the fixed CLT Python runtime.

// SAFETY: each FFI block below has a local SAFETY explanation; keep unsafe use
// contained to this runtime gate module rather than enabling it crate-wide.
#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(not(target_os = "macos"))]
mod darwin {
    use std::path::Path;

    use super::{GateError, RuntimeAttestation};

    pub(super) const RUNTIME_PATH: &str = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9";
    pub(super) const FRAMEWORK_PATH: &str =
        "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9";

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

    pub(super) fn verify_runtime(
        _bootstrap_path: &Path,
        _expected_bootstrap_sha256: &str,
    ) -> Result<VerifiedRuntime, GateError> {
        Err(GateError::UnsupportedPlatform)
    }

    #[derive(Debug)]
    pub(super) struct VerifiedRuntime {
        pub(super) attestation: RuntimeAttestation,
        pub(super) bootstrap: Vec<u8>,
    }

    impl VerifiedRuntime {
        pub(super) fn recheck(&self) -> Result<(), GateError> {
            Err(GateError::UnsupportedPlatform)
        }

        pub(super) fn held_fds(&self) -> Vec<libc::c_int> {
            Vec::new()
        }
    }
}
mod macho;

use std::collections::BTreeMap;
use std::ffi::{CString, OsString};
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

const FD3: libc::c_int = 3;
const MAX_ATTESTATION_BYTES: usize = 4096;
pub(super) const MAX_BOOTSTRAP_BYTES: usize = 64 * 1024;
const CLEAN_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GateError {
    InvalidArguments,
    InvalidDigest,
    BootstrapInvalid,
    BootstrapDigestMismatch,
    UnsafeEnvironment,
    #[cfg(not(target_os = "macos"))]
    UnsupportedPlatform,
    UnsupportedArchitecture,
    UnsafePath,
    UnsafeMetadata,
    FilesystemUnverifiable,
    UnsupportedFilesystem,
    AclUnverifiable,
    AclMalformed,
    AclPresentOrMalformed,
    PathChanged,
    SystemIdentityUnavailable,
    SignatureFailure,
    WrongCodeDirectoryHash,
    MalformedMachO,
    UnsafeMachOPath,
    UnexpectedMachOArchitectures,
    UnexpectedMachODependency,
    UnexpectedDependencyComponent,
    AttestationInvalid,
    PipeFailure,
    ExecFailure,
}

impl fmt::Display for GateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidArguments => "unsupported runtime-gate arguments",
            Self::InvalidDigest => "invalid independently approved bootstrap digest",
            Self::BootstrapInvalid => "protected bootstrap source is invalid",
            Self::BootstrapDigestMismatch => "bootstrap digest mismatch",
            Self::UnsafeEnvironment => "runtime gate requires the fixed clean environment",
            #[cfg(not(target_os = "macos"))]
            Self::UnsupportedPlatform => "runtime gate is supported only on macOS",
            Self::UnsupportedArchitecture => "runtime gate architecture is unsupported",
            Self::UnsafePath => "protected path cannot be admitted",
            Self::UnsafeMetadata => "protected descriptor metadata cannot be admitted",
            Self::FilesystemUnverifiable => "filesystem provenance is unavailable",
            Self::UnsupportedFilesystem => "filesystem is not supported by the runtime gate",
            Self::AclUnverifiable => "descriptor ACL evidence is unavailable",
            Self::AclMalformed => "descriptor ACL evidence is malformed",
            Self::AclPresentOrMalformed => "descriptor ACL is present or malformed",
            Self::PathChanged => "protected path identity changed during verification",
            Self::SystemIdentityUnavailable => "operating-system identity is unavailable",
            Self::SignatureFailure => "Apple code signature or resource validation failed",
            Self::WrongCodeDirectoryHash => "Apple code-directory hash is not approved",
            Self::MalformedMachO => "Mach-O load commands are malformed",
            Self::UnsafeMachOPath => "Mach-O image contains a dynamic path override",
            Self::UnexpectedMachOArchitectures => "Mach-O architecture set is not approved",
            Self::UnexpectedMachODependency => "Mach-O dependency set is not approved",
            Self::UnexpectedDependencyComponent => "Mach-O component is outside the fixed closure",
            Self::AttestationInvalid => "runtime attestation cannot be serialized safely",
            Self::PipeFailure => "runtime attestation pipe setup failed",
            Self::ExecFailure => "direct CLT Python exec failed",
        })
    }
}

impl std::error::Error for GateError {}

fn protected_mode_bits(mode: u32, executable: bool) -> bool {
    mode & 0o7022 == 0 && (!executable || mode & 0o100 != 0)
}

#[derive(Debug)]
struct RuntimeAttestation {
    fields: BTreeMap<String, String>,
}

impl RuntimeAttestation {
    #[cfg(target_os = "macos")]
    fn from_facts(facts: darwin::RuntimeFacts, bootstrap_sha256: &str) -> Self {
        let fields = BTreeMap::from([
            ("apple_anchor".to_owned(), "apple".to_owned()),
            ("bootstrap_sha256".to_owned(), bootstrap_sha256.to_owned()),
            (
                "dependency_policy_id".to_owned(),
                "clt-python39-apple-dyld-v1".to_owned(),
            ),
            ("framework_cdhash".to_owned(), facts.framework_cdhash),
            ("framework_dev".to_owned(), facts.framework_dev),
            ("framework_ino".to_owned(), facts.framework_ino),
            ("framework_mode".to_owned(), facts.framework_mode),
            (
                "framework_path".to_owned(),
                darwin::FRAMEWORK_PATH.to_owned(),
            ),
            ("framework_resource_seal".to_owned(), "valid".to_owned()),
            ("framework_uid".to_owned(), facts.framework_uid),
            ("os_build".to_owned(), facts.os_build),
            ("policy_id".to_owned(), "clt-python3.9-v1".to_owned()),
            ("requirement_id".to_owned(), "com.apple.python3".to_owned()),
            ("runtime_arch".to_owned(), facts.runtime_arch),
            ("runtime_cdhash".to_owned(), facts.runtime_cdhash),
            ("runtime_dev".to_owned(), facts.runtime_dev),
            ("runtime_ino".to_owned(), facts.runtime_ino),
            ("runtime_mode".to_owned(), facts.runtime_mode),
            ("runtime_path".to_owned(), darwin::RUNTIME_PATH.to_owned()),
            ("runtime_uid".to_owned(), facts.runtime_uid),
            ("runtime_version".to_owned(), "3.9".to_owned()),
            (
                "schema_version".to_owned(),
                "oasis7.local-signer.runtime-attestation.v1".to_owned(),
            ),
            (
                "system_volume_trust".to_owned(),
                "current-booted-apple-os".to_owned(),
            ),
        ]);
        Self { fields }
    }

    fn bytes(&self) -> Result<Vec<u8>, GateError> {
        let expected_keys = [
            "apple_anchor",
            "bootstrap_sha256",
            "dependency_policy_id",
            "framework_cdhash",
            "framework_dev",
            "framework_ino",
            "framework_mode",
            "framework_path",
            "framework_resource_seal",
            "framework_uid",
            "os_build",
            "policy_id",
            "requirement_id",
            "runtime_arch",
            "runtime_cdhash",
            "runtime_dev",
            "runtime_ino",
            "runtime_mode",
            "runtime_path",
            "runtime_uid",
            "runtime_version",
            "schema_version",
            "system_volume_trust",
        ];
        if self.fields.len() != expected_keys.len()
            || self.fields.keys().map(String::as_str).collect::<Vec<_>>() != expected_keys
            || self
                .fields
                .values()
                .any(|value| !value.is_ascii() || value.bytes().any(|byte| byte.is_ascii_control()))
        {
            return Err(GateError::AttestationInvalid);
        }
        for name in [
            "runtime_dev",
            "runtime_ino",
            "runtime_uid",
            "runtime_mode",
            "framework_dev",
            "framework_ino",
            "framework_uid",
            "framework_mode",
        ] {
            if !canonical_decimal(self.fields.get(name).ok_or(GateError::AttestationInvalid)?) {
                return Err(GateError::AttestationInvalid);
            }
        }
        for name in ["runtime_cdhash", "framework_cdhash"] {
            if !lower_hex(
                self.fields.get(name).ok_or(GateError::AttestationInvalid)?,
                40,
            ) {
                return Err(GateError::AttestationInvalid);
            }
        }
        if !lower_hex(
            self.fields
                .get("bootstrap_sha256")
                .ok_or(GateError::AttestationInvalid)?,
            64,
        ) {
            return Err(GateError::AttestationInvalid);
        }
        for name in [
            "runtime_dev",
            "runtime_ino",
            "framework_dev",
            "framework_ino",
        ] {
            if self.fields[name]
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0)
                .is_none()
            {
                return Err(GateError::AttestationInvalid);
            }
        }
        if self.fields["runtime_uid"] != "0"
            || self.fields["framework_uid"] != "0"
            || self.fields["runtime_arch"] != "arm64" && self.fields["runtime_arch"] != "x86_64"
            || self.fields["runtime_mode"]
                .parse::<u32>()
                .ok()
                .is_none_or(|mode| {
                    !protected_mode_bits(mode, true)
                        || mode & (libc::S_IFMT as u32) != libc::S_IFREG as u32
                })
            || self.fields["framework_mode"]
                .parse::<u32>()
                .ok()
                .is_none_or(|mode| {
                    !protected_mode_bits(mode, true)
                        || mode & (libc::S_IFMT as u32) != libc::S_IFDIR as u32
                })
            || self.fields["os_build"].is_empty()
            || self.fields["os_build"].len() > 64
            || !self.fields["os_build"]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
        {
            return Err(GateError::AttestationInvalid);
        }
        let mut bytes =
            serde_json::to_vec(&self.fields).map_err(|_| GateError::AttestationInvalid)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_ATTESTATION_BYTES {
            return Err(GateError::AttestationInvalid);
        }
        Ok(bytes)
    }
}

#[derive(Debug)]
struct GateArguments {
    bootstrap_path: PathBuf,
    expected_bootstrap_sha256: String,
    operation_and_args: Vec<OsString>,
}

#[derive(Debug)]
struct PreparedLaunch {
    runtime: darwin::VerifiedRuntime,
    operation_and_args: Vec<OsString>,
}

fn parse_arguments(args: &[OsString]) -> Result<GateArguments, GateError> {
    if args.len() < 6
        || args[0] != "--bootstrap-source"
        || args[2] != "--expected-bootstrap-sha256"
        || args[4] != "--"
    {
        return Err(GateError::InvalidArguments);
    }
    let bootstrap_path = PathBuf::from(&args[1]);
    let digest = args[3].to_str().ok_or(GateError::InvalidDigest)?.to_owned();
    if !bootstrap_path.is_absolute()
        || args[5] != "plan" && args[5] != "apply"
        || args[5..]
            .iter()
            .any(|argument| argument.as_bytes().contains(&0))
    {
        return Err(GateError::InvalidArguments);
    }
    validate_sha256(&digest)?;
    Ok(GateArguments {
        bootstrap_path,
        expected_bootstrap_sha256: digest,
        operation_and_args: args[5..].to_vec(),
    })
}

fn validate_environment<I>(environment: I) -> Result<(), GateError>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut values = BTreeMap::new();
    for (name, value) in environment {
        let name = name
            .into_string()
            .map_err(|_| GateError::UnsafeEnvironment)?;
        let value = value
            .into_string()
            .map_err(|_| GateError::UnsafeEnvironment)?;
        if values.insert(name, value).is_some() {
            return Err(GateError::UnsafeEnvironment);
        }
    }
    if values.len() != 3
        || values.get("PATH").map(String::as_str) != Some(CLEAN_PATH)
        || values.get("LANG").map(String::as_str) != Some("C")
        || values.get("LC_ALL").map(String::as_str) != Some("C")
    {
        return Err(GateError::UnsafeEnvironment);
    }
    Ok(())
}

fn prepare_launch(arguments: GateArguments) -> Result<PreparedLaunch, GateError> {
    let runtime = darwin::verify_runtime(
        &arguments.bootstrap_path,
        &arguments.expected_bootstrap_sha256,
    )?;
    Ok(PreparedLaunch {
        runtime,
        operation_and_args: arguments.operation_and_args,
    })
}

fn run_after_verification<T>(
    verify: impl FnOnce() -> Result<T, GateError>,
    execute: impl FnOnce(T) -> Result<(), GateError>,
) -> Result<(), GateError> {
    execute(verify()?)
}

fn run_pre_exec_checks(
    cleanup: impl FnOnce() -> Result<(), GateError>,
    recheck: impl FnOnce() -> Result<(), GateError>,
    exec: impl FnOnce() -> Result<(), GateError>,
) -> Result<(), GateError> {
    cleanup()?;
    recheck()?;
    exec()
}

fn descriptor_is_preserved(descriptor: libc::c_int, preserved: &[libc::c_int]) -> bool {
    descriptor == FD3 || preserved.contains(&descriptor)
}

fn encode_exec_strings(
    bootstrap: &[u8],
    operation_and_args: &[OsString],
) -> Result<Vec<CString>, GateError> {
    let bootstrap = std::str::from_utf8(bootstrap).map_err(|_| GateError::BootstrapInvalid)?;
    let mut strings = vec![
        CString::new(darwin::RUNTIME_PATH).map_err(|_| GateError::ExecFailure)?,
        CString::new("-I").expect("static argv token"),
        CString::new("-S").expect("static argv token"),
        CString::new("-B").expect("static argv token"),
        CString::new("-c").expect("static argv token"),
        CString::new(bootstrap).map_err(|_| GateError::BootstrapInvalid)?,
    ];
    for argument in operation_and_args {
        strings.push(CString::new(argument.as_bytes()).map_err(|_| GateError::InvalidArguments)?);
    }
    Ok(strings)
}

fn launch(prepared: PreparedLaunch) -> Result<(), GateError> {
    let attestation = prepared.runtime.attestation.bytes()?;
    let args = encode_exec_strings(&prepared.runtime.bootstrap, &prepared.operation_and_args)?;
    let env = [
        CString::new(format!("PATH={CLEAN_PATH}")).map_err(|_| GateError::ExecFailure)?,
        CString::new("LANG=C").expect("static environment entry"),
        CString::new("LC_ALL=C").expect("static environment entry"),
    ];
    let argv = args
        .iter()
        .map(|argument| argument.as_ptr())
        .chain([std::ptr::null()])
        .collect::<Vec<_>>();
    let envp = env
        .iter()
        .map(|entry| entry.as_ptr())
        .chain([std::ptr::null()])
        .collect::<Vec<_>>();
    let path = CString::new(darwin::RUNTIME_PATH).map_err(|_| GateError::ExecFailure)?;
    // FD 3 was reserved before any protected descriptors were opened.
    // SAFETY: FD 3 is the gate's CLOEXEC reservation pipe reader.
    if unsafe { libc::close(FD3) } != 0 {
        return Err(GateError::PipeFailure);
    }
    let read_fd = create_attestation_pipe(&attestation)?;
    install_attestation_fd(read_fd)?;
    run_pre_exec_checks(
        || close_unexpected_descriptors(&prepared.runtime.held_fds()),
        || prepared.runtime.recheck(),
        || {
            // SAFETY: path, argv and envp are NUL-terminated, live through the call, and argv/envp end in null pointers.
            unsafe { libc::execve(path.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
            Err(GateError::ExecFailure)
        },
    )
}

fn create_attestation_pipe(record: &[u8]) -> Result<libc::c_int, GateError> {
    if record.is_empty() || record.len() > MAX_ATTESTATION_BYTES {
        return Err(GateError::AttestationInvalid);
    }
    let mut descriptors = [-1; 2];
    // SAFETY: descriptors is a writable two-element output array.
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
        return Err(GateError::PipeFailure);
    }
    let (reader, writer) = (descriptors[0], descriptors[1]);
    for descriptor in descriptors {
        // SAFETY: each descriptor is live and F_GETFD/F_SETFD operate on it.
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            // SAFETY: both descriptors were returned by pipe and have not been closed.
            unsafe {
                libc::close(reader);
                libc::close(writer);
            }
            return Err(GateError::PipeFailure);
        }
    }
    let mut written = 0usize;
    while written < record.len() {
        // SAFETY: slice bounds are valid and writer is a live pipe descriptor.
        let result = unsafe {
            libc::write(
                writer,
                record[written..].as_ptr().cast(),
                record.len() - written,
            )
        };
        if result < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            // SAFETY: both descriptors were returned by pipe and have not been closed.
            unsafe {
                libc::close(reader);
                libc::close(writer);
            }
            return Err(GateError::PipeFailure);
        }
        let count = usize::try_from(result).map_err(|_| GateError::PipeFailure)?;
        if count == 0 {
            // SAFETY: both descriptors were returned by pipe and have not been closed.
            unsafe {
                libc::close(reader);
                libc::close(writer);
            }
            return Err(GateError::PipeFailure);
        }
        written = written.checked_add(count).ok_or(GateError::PipeFailure)?;
    }
    // SAFETY: writer was returned by pipe and the complete record was written.
    if unsafe { libc::close(writer) } != 0 {
        // SAFETY: reader is still open and was returned by pipe.
        unsafe {
            libc::close(reader);
        }
        return Err(GateError::PipeFailure);
    }
    Ok(reader)
}

fn install_attestation_fd(read_fd: libc::c_int) -> Result<(), GateError> {
    // SAFETY: dup2 either installs the live pipe reader at FD 3 or fails.
    if read_fd != FD3 && unsafe { libc::dup2(read_fd, FD3) } < 0 {
        // SAFETY: read_fd is the live descriptor returned by create_attestation_pipe.
        unsafe {
            libc::close(read_fd);
        }
        return Err(GateError::PipeFailure);
    }
    if read_fd != FD3 {
        // SAFETY: duplicated descriptor is live; close original reader.
        unsafe {
            libc::close(read_fd);
        }
    }
    // SAFETY: FD 3 is open after the successful dup2 or was already the pipe reader.
    let flags = unsafe { libc::fcntl(FD3, libc::F_GETFD) };
    if flags < 0 || unsafe { libc::fcntl(FD3, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } < 0 {
        // SAFETY: FD 3 is the pipe reader installed above.
        unsafe {
            libc::close(FD3);
        }
        return Err(GateError::PipeFailure);
    }
    // SAFETY: zeroed is a valid initial representation for Darwin's plain stat output struct.
    let mut status: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: status is writable and FD 3 is open.
    if unsafe { libc::fstat(FD3, &mut status) } != 0
        || status.st_mode & libc::S_IFMT != libc::S_IFIFO
        || status.st_uid != unsafe { libc::geteuid() }
    {
        // SAFETY: FD 3 is the pipe reader installed above.
        unsafe {
            libc::close(FD3);
        }
        return Err(GateError::PipeFailure);
    }
    // SAFETY: FD 3 is open and F_GETFL is a read-only query.
    let access = unsafe { libc::fcntl(FD3, libc::F_GETFL) } & libc::O_ACCMODE;
    if access != libc::O_RDONLY {
        // SAFETY: FD 3 is the pipe reader installed above.
        unsafe {
            libc::close(FD3);
        }
        return Err(GateError::PipeFailure);
    }
    Ok(())
}

fn close_unexpected_descriptors(preserved: &[libc::c_int]) -> Result<(), GateError> {
    // SAFETY: getdtablesize does not mutate process state and yields the descriptor bound.
    let limit = unsafe { libc::getdtablesize() };
    if limit < 4 {
        return Err(GateError::PipeFailure);
    }
    for descriptor in 4..limit {
        if !descriptor_is_preserved(descriptor, preserved) {
            // SAFETY: closing an unexpected or already-closed descriptor is harmless.
            unsafe {
                libc::close(descriptor);
            }
        }
    }
    Ok(())
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_decimal(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn validate_sha256(value: &str) -> Result<(), GateError> {
    if lower_hex(value, 64) {
        Ok(())
    } else {
        Err(GateError::InvalidDigest)
    }
}

/// Runs the native gate and directly replaces it with the fixed CLT Python process.
pub fn run_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    validate_environment(std::env::vars_os())?;
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let arguments = parse_arguments(&args)?;
    reserve_attestation_descriptor()?;
    run_after_verification(|| prepare_launch(arguments), launch).map_err(Into::into)
}

fn reserve_attestation_descriptor() -> Result<(), GateError> {
    close_all_descriptors_from(FD3)?;
    let mut descriptors = [-1; 2];
    // SAFETY: descriptors is a writable two-element output array.
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
        return Err(GateError::PipeFailure);
    }
    if descriptors[0] != FD3 {
        // SAFETY: both values are live pipe descriptors.
        unsafe {
            libc::close(descriptors[0]);
            libc::close(descriptors[1]);
        }
        return Err(GateError::PipeFailure);
    }
    for descriptor in descriptors {
        // SAFETY: each descriptor is live and the descriptor flags are queried/updated.
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            // SAFETY: both values are live pipe descriptors (one may have been duplicated).
            unsafe {
                libc::close(FD3);
                libc::close(descriptors[1]);
            }
            return Err(GateError::PipeFailure);
        }
    }
    // SAFETY: the writer was returned by pipe and the reservation reader remains at FD 3.
    if unsafe { libc::close(descriptors[1]) } != 0 {
        // SAFETY: FD 3 is the reservation reader.
        unsafe {
            libc::close(FD3);
        }
        return Err(GateError::PipeFailure);
    }
    Ok(())
}

fn close_all_descriptors_from(first: libc::c_int) -> Result<(), GateError> {
    // SAFETY: getdtablesize does not mutate process state and yields the descriptor bound.
    let limit = unsafe { libc::getdtablesize() };
    if limit <= first {
        return Err(GateError::PipeFailure);
    }
    for descriptor in first..limit {
        // SAFETY: gate startup intentionally closes inherited descriptors at or above `first`.
        unsafe {
            libc::close(descriptor);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn argument_contract_has_one_external_digest_and_only_plan_or_apply() {
        let valid = args(&[
            "--bootstrap-source",
            "/private/var/db/approved/bootstrap.py",
            "--expected-bootstrap-sha256",
            &"a".repeat(64),
            "--",
            "plan",
            "--release-dir",
            "/private/var/db/approved/release",
        ]);
        let parsed = parse_arguments(&valid).expect("valid fixed gate arguments");
        assert_eq!(parsed.operation_and_args[0], "plan");
        assert!(
            parse_arguments(&args(&[
                "--bootstrap-source",
                "/private/var/db/bootstrap.py",
                "--expected-bootstrap-sha256",
                &"A".repeat(64),
                "--",
                "plan",
            ]))
            .is_err()
        );
        assert!(
            parse_arguments(&args(&[
                "--bootstrap-source",
                "/private/var/db/bootstrap.py",
                "--expected-bootstrap-sha256",
                &"a".repeat(64),
                "--",
                "diagnose",
            ]))
            .is_err()
        );
    }

    #[test]
    fn clean_environment_is_exact_and_rejects_dyld_or_python_overrides() {
        let allowed = [
            (OsString::from("PATH"), OsString::from(CLEAN_PATH)),
            (OsString::from("LANG"), OsString::from("C")),
            (OsString::from("LC_ALL"), OsString::from("C")),
        ];
        assert!(validate_environment(allowed.clone()).is_ok());
        assert!(
            validate_environment(allowed.into_iter().chain([(
                OsString::from("DYLD_INSERT_LIBRARIES"),
                OsString::from("/tmp/x")
            )]))
            .is_err()
        );
    }

    #[test]
    fn clean_environment_rejects_duplicate_names_before_map_collection() {
        let exact = [
            (OsString::from("PATH"), OsString::from(CLEAN_PATH)),
            (OsString::from("LANG"), OsString::from("C")),
            (OsString::from("LC_ALL"), OsString::from("C")),
        ];
        for (duplicate_name, duplicate_value) in &exact {
            let iterator = exact
                .iter()
                .cloned()
                .chain([(duplicate_name.clone(), duplicate_value.clone())]);
            assert!(
                validate_environment(iterator).is_err(),
                "duplicate {:?} must reject before collection",
                duplicate_name
            );
        }
    }

    #[test]
    fn python_exec_arguments_keep_fixed_isolated_no_site_no_bytecode_flags() {
        let operation = args(&["apply", "--plan", "/private/var/db/approved/plan.json"]);
        let strings = encode_exec_strings(b"trusted bootstrap", &operation)
            .expect("fixed direct Python arguments");
        let values = strings
            .iter()
            .map(|argument| argument.to_str().expect("ASCII argument"))
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            [
                darwin::RUNTIME_PATH,
                "-I",
                "-S",
                "-B",
                "-c",
                "trusted bootstrap",
                "apply",
                "--plan",
                "/private/var/db/approved/plan.json",
            ]
        );
        assert!(encode_exec_strings(b"invalid\0bootstrap", &operation).is_err());
    }

    #[test]
    fn schema_is_compact_sorted_ascii_and_has_exactly_23_fields() {
        let fields = BTreeMap::from([
            ("apple_anchor".to_owned(), "apple".to_owned()),
            ("bootstrap_sha256".to_owned(), "a".repeat(64)),
            (
                "dependency_policy_id".to_owned(),
                "clt-python39-apple-dyld-v1".to_owned(),
            ),
            ("framework_cdhash".to_owned(), "a".repeat(40)),
            ("framework_dev".to_owned(), "1".to_owned()),
            ("framework_ino".to_owned(), "2".to_owned()),
            ("framework_mode".to_owned(), "16877".to_owned()),
            (
                "framework_path".to_owned(),
                darwin::FRAMEWORK_PATH.to_owned(),
            ),
            ("framework_resource_seal".to_owned(), "valid".to_owned()),
            ("framework_uid".to_owned(), "0".to_owned()),
            ("os_build".to_owned(), "25E1".to_owned()),
            ("policy_id".to_owned(), "clt-python3.9-v1".to_owned()),
            ("requirement_id".to_owned(), "com.apple.python3".to_owned()),
            ("runtime_arch".to_owned(), "arm64".to_owned()),
            ("runtime_cdhash".to_owned(), "b".repeat(40)),
            ("runtime_dev".to_owned(), "1".to_owned()),
            ("runtime_ino".to_owned(), "3".to_owned()),
            ("runtime_mode".to_owned(), "33261".to_owned()),
            ("runtime_path".to_owned(), darwin::RUNTIME_PATH.to_owned()),
            ("runtime_uid".to_owned(), "0".to_owned()),
            ("runtime_version".to_owned(), "3.9".to_owned()),
            (
                "schema_version".to_owned(),
                "oasis7.local-signer.runtime-attestation.v1".to_owned(),
            ),
            (
                "system_volume_trust".to_owned(),
                "current-booted-apple-os".to_owned(),
            ),
        ]);
        let attestation = RuntimeAttestation {
            fields: fields.clone(),
        };
        let bytes = attestation.bytes().expect("exact attestation schema");
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(bytes[..bytes.len() - 1].is_ascii());
        assert!(bytes.windows(2).all(|window| window != b"\n\n"));
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");
        assert_eq!(value.as_object().expect("object").len(), 23);
        let json = &bytes[..bytes.len() - 1];
        assert!(!json.iter().any(u8::is_ascii_whitespace));
        let keys = value
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));

        for (field, mode) in [
            ("runtime_mode", 0o104755),
            ("runtime_mode", 0o102755),
            ("framework_mode", 0o41755),
        ] {
            let mut invalid = fields.clone();
            invalid.insert(field.to_owned(), mode.to_string());
            assert!(
                (RuntimeAttestation { fields: invalid }).bytes().is_err(),
                "special permissions must reject for {field}"
            );
        }
    }

    #[test]
    fn injected_verification_failures_never_reach_exec() {
        let failures = [
            ("metadata", GateError::UnsafeMetadata),
            ("fstatfs", GateError::UnsupportedFilesystem),
            ("acl", GateError::AclPresentOrMalformed),
            ("signature", GateError::SignatureFailure),
            ("resource seal", GateError::SignatureFailure),
            ("dependency", GateError::UnexpectedMachODependency),
        ];
        for (stage, failure) in failures {
            let mut exec_calls = 0;
            let result = run_after_verification::<()>(
                || Err(failure),
                |()| {
                    exec_calls += 1;
                    Ok(())
                },
            );
            assert_eq!(result, Err(failure), "{stage} failure must be preserved");
            assert_eq!(exec_calls, 0, "{stage} failure must not reach exec");
        }
    }

    #[test]
    fn preserved_descriptor_cleanup_and_name_recheck_gate_exec_in_order() {
        use std::cell::RefCell;

        let calls = RefCell::new(Vec::new());
        let result = run_pre_exec_checks(
            || {
                calls.borrow_mut().push("preserve verified descriptors");
                Ok(())
            },
            || {
                calls.borrow_mut().push("recheck descriptor names");
                Ok(())
            },
            || {
                calls.borrow_mut().push("exec");
                Ok(())
            },
        );
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls.into_inner(),
            [
                "preserve verified descriptors",
                "recheck descriptor names",
                "exec"
            ]
        );

        let exec_calls = RefCell::new(0);
        let result = run_pre_exec_checks(
            || Ok(()),
            || Err(GateError::PathChanged),
            || {
                *exec_calls.borrow_mut() += 1;
                Ok(())
            },
        );
        assert_eq!(result, Err(GateError::PathChanged));
        assert_eq!(*exec_calls.borrow(), 0, "changed names must not reach exec");

        let exec_calls = RefCell::new(0);
        let result = run_pre_exec_checks(
            || Err(GateError::PipeFailure),
            || Ok(()),
            || {
                *exec_calls.borrow_mut() += 1;
                Ok(())
            },
        );
        assert_eq!(result, Err(GateError::PipeFailure));
        assert_eq!(
            *exec_calls.borrow(),
            0,
            "descriptor preservation failure must not reach exec"
        );
    }

    #[test]
    fn unexpected_descriptor_cleanup_preserves_only_attested_descriptors_and_fd3() {
        let retained = [5, 8, 13];
        assert!(descriptor_is_preserved(FD3, &retained));
        for descriptor in retained {
            assert!(descriptor_is_preserved(descriptor, &retained));
        }
        assert!(!descriptor_is_preserved(4, &retained));
        assert!(!descriptor_is_preserved(9, &retained));
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod darwin_probe {
    use super::*;
    use sha2::Digest;
    use std::path::Path;

    #[test]
    fn production_gate_admits_the_installed_clt_runtime_without_exec_or_mutation() {
        let path = Path::new(
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/lib/python3.9/ctypes/util.py",
        );
        let bytes =
            darwin::open_and_hash_probe(path).expect("read-only root-protected probe input");
        let digest = hex::encode(sha2::Sha256::digest(&bytes));
        let runtime = darwin::verify_runtime(path, &digest).expect("fixed CLT runtime admission");
        assert_eq!(runtime.bootstrap, bytes);
        assert_eq!(
            runtime
                .attestation
                .fields
                .get("runtime_cdhash")
                .map(String::as_str),
            Some("77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf")
        );
        assert_eq!(
            runtime
                .attestation
                .fields
                .get("framework_cdhash")
                .map(String::as_str),
            Some("a43551195b8d2eefd9356d81c1098ffc9e0a8b47")
        );
        runtime
            .recheck()
            .expect("retained descriptor and path identity");
    }
}
