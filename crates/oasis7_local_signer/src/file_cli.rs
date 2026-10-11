//! Local caller payload preparation. No installation, sudo or private store access.
use crate::{
    error::SignerError,
    identity::{parse_sha256_hex, sha256_hex},
    local_fs::{Directory, read_candidate_bytes},
    protocol::SignContext,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};
fn invalid() -> SignerError {
    SignerError::InvalidInput(
        "file-payload requires absolute paths and exact digest-bound arguments".into(),
    )
}
pub fn dispatch(args: &[String]) -> Option<Result<Value, SignerError>> {
    if args.first()?.as_str() != "file-payload" {
        return None;
    }
    Some(prepare(args))
}
fn prepare(args: &[String]) -> Result<Value, SignerError> {
    let allowed = [
        "--context",
        "--expected-context-sha256",
        "--file",
        "--expected-file-sha256",
        "--output",
    ];
    let mut o = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let k = args[i].as_str();
        let v = args.get(i + 1).ok_or_else(invalid)?.as_str();
        if !allowed.contains(&k) || o.insert(k, v).is_some() {
            return Err(invalid());
        }
        i += 2;
    }
    if o.len() != allowed.len() {
        return Err(invalid());
    }
    for k in ["--context", "--file", "--output"] {
        crate::types::validate_absolute_path(o[k]).map_err(|_| invalid())?;
    }
    for k in ["--expected-context-sha256", "--expected-file-sha256"] {
        parse_sha256_hex(o[k])?;
    }
    let uid = crate::installation::current_uid();
    if crate::installation::current_euid() != uid {
        return Err(SignerError::AuthorizationDenied);
    }
    let bytes = read_candidate_bytes(Path::new(o["--context"]), uid, 65536)?;
    if sha256_hex(&bytes) != o["--expected-context-sha256"] {
        return Err(SignerError::CryptoOrBindingInvalid);
    }
    let context: SignContext = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    crate::local_fs::reject_symlink_components(Path::new(o["--file"]))?;
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(o["--file"])?;
    let before = f.metadata()?;
    if !before.is_file() || before.nlink() != 1 {
        return Err(invalid());
    }
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        size = size.checked_add(n as u64).ok_or_else(invalid)?;
        hasher.update(&buffer[..n]);
    }
    let after = f.metadata()?;
    if (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) || size != before.len()
    {
        return Err(SignerError::IdConflict);
    }
    let hash = hex::encode(hasher.finalize());
    if hash != o["--expected-file-sha256"] {
        return Err(SignerError::CryptoOrBindingInvalid);
    }
    let payload = crate::detached::signing_payload_from_digest(&context, &hash, size)?;
    let output = Path::new(o["--output"]);
    let parent = Directory::open(output.parent().ok_or_else(invalid)?, false)?;
    let metadata = parent.metadata()?;
    if metadata.uid() != uid || metadata.permissions().mode() & 0o7777 != 0o700 {
        return Err(SignerError::AuthorizationDenied);
    }
    parent.write_new(
        output
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(invalid)?,
        &payload,
    )?;
    Ok(
        json!({"status":"PAYLOAD_PREPARED","output":output,"file_sha256":hash,"file_size_bytes":size,"payload_sha256":sha256_hex(&payload),"purpose":"file_ed25519_v1"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caller_builder_emits_exact_envelope_and_never_overwrites() {
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).unwrap();
        let root = std::env::current_dir()
            .unwrap()
            .join(format!(".file-payload-test-{}", hex::encode(random)));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let context = SignContext {
            deployment_id: "deployment-01".into(),
            network_id: "network-01".into(),
            task_uid: "task-01".into(),
            source_head_oid: "aa".repeat(20),
            protocol_context: crate::protocol::RollbackProtocolContext {
                authority_id: "authority-01".into(),
                rollback_ticket: "file-01".into(),
                receipt_id: "receipt-01".into(),
                nonce: "nonce-01".into(),
            },
        };
        let bytes = serde_json::to_vec(&context).unwrap();
        std::fs::write(root.join("context.json"), &bytes).unwrap();
        std::fs::write(root.join("file.bin"), b"file contents").unwrap();
        let args = vec![
            "file-payload".into(),
            "--context".into(),
            root.join("context.json").to_str().unwrap().into(),
            "--expected-context-sha256".into(),
            sha256_hex(&bytes),
            "--file".into(),
            root.join("file.bin").to_str().unwrap().into(),
            "--expected-file-sha256".into(),
            sha256_hex(b"file contents"),
            "--output".into(),
            root.join("payload.bin").to_str().unwrap().into(),
        ];
        assert!(prepare(&args).is_ok());
        let payload = std::fs::read(root.join("payload.bin")).unwrap();
        assert_eq!(
            payload,
            crate::detached::signing_payload(&context, b"file contents").unwrap()
        );
        assert_eq!(
            std::fs::metadata(root.join("payload.bin"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        assert!(prepare(&args).is_err());
        assert_eq!(std::fs::read(root.join("payload.bin")).unwrap(), payload);
        let mut wrong = args;
        wrong[9] = "bb".repeat(32);
        assert!(prepare(&wrong).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
