//! Root-only management commands; encrypted material never appears in JSON output.
use crate::{
    admin::AdminStore,
    backup::RestorePlan,
    error::SignerError,
    identity::{parse_sha256_hex, sha256_hex},
    local_fs::Directory,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{IsTerminal, Read},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};
use zeroize::Zeroizing;
const MAX_FILE: usize = 32 * 1024 * 1024;
const COMMANDS: &[&str] = &[
    "key-list",
    "key-show",
    "key-new",
    "key-update",
    "key-state",
    "key-rotate",
    "key-export",
    "key-import",
    "backup-create",
    "restore-plan",
    "restore-apply",
    "key-audit",
    "control-show",
    "grant-list",
    "grant-show",
    "signing-audit",
    "management-recover",
];
fn invalid() -> SignerError {
    SignerError::InvalidInput("management command arguments are invalid".into())
}
struct Options {
    values: BTreeMap<String, String>,
    flags: BTreeSet<String>,
}
fn parse(args: &[String]) -> Result<Options, SignerError> {
    let command = args.first().ok_or_else(invalid)?.as_str();
    let (required, optional, flags): (&[&str], &[&str], &[&str]) = match command {
        "key-list" | "key-audit" | "control-show" | "grant-list" | "signing-audit" => {
            (&[], &[], &[])
        }
        "key-show" => (&["--signer-id"], &[], &[]),
        "grant-show" => (&["--grant-id"], &[], &[]),
        "management-recover" => (
            &["--expected-operation-key", "--expected-target-sha256"],
            &[],
            &[],
        ),
        "key-new" => (
            &["--signer-id", "--purpose"],
            &["--name"],
            &["--exportable"],
        ),
        "key-update" => (
            &["--signer-id", "--name", "--description", "--tags"],
            &[],
            &[],
        ),
        "key-state" => (&["--signer-id", "--state"], &[], &["--confirm-delete"]),
        "key-rotate" => (&["--signer-id", "--new-signer-id"], &[], &[]),
        "key-export" => (&["--signer-id", "--output"], &[], &["--passphrase-stdin"]),
        "key-import" => (
            &["--signer-id", "--input", "--expected-sha256"],
            &[],
            &["--passphrase-stdin", "--exportable"],
        ),
        "backup-create" => (
            &["--output"],
            &[],
            &["--passphrase-stdin", "--include-nonexportable"],
        ),
        "restore-plan" => (
            &["--input", "--expected-sha256", "--output"],
            &[],
            &["--passphrase-stdin"],
        ),
        "restore-apply" => (
            &[
                "--input",
                "--expected-sha256",
                "--plan",
                "--expected-plan-sha256",
            ],
            &[],
            &["--passphrase-stdin"],
        ),
        _ => return Err(invalid()),
    };
    let mut o = Options {
        values: BTreeMap::new(),
        flags: BTreeSet::new(),
    };
    let mut i = 1;
    while i < args.len() {
        let key = &args[i];
        if flags.contains(&key.as_str()) {
            if !o.flags.insert(key.clone()) {
                return Err(invalid());
            }
            i += 1;
        } else {
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                return Err(invalid());
            }
            let value = args.get(i + 1).ok_or_else(invalid)?;
            if o.values.insert(key.clone(), value.clone()).is_some() {
                return Err(invalid());
            }
            i += 2;
        }
    }
    if required.iter().any(|k| !o.values.contains_key(*k)) {
        return Err(invalid());
    }
    for (k, v) in &o.values {
        if k.ends_with("signer-id") || k == "--grant-id" {
            crate::protocol::validate_id(v)?;
        }
        if k.contains("sha256") || k == "--expected-operation-key" {
            parse_sha256_hex(v)?;
        }
        if matches!(k.as_str(), "--input" | "--output" | "--plan") {
            crate::types::validate_absolute_path(v).map_err(|_| invalid())?;
        }
    }
    if flags.contains(&"--passphrase-stdin") && !o.flags.contains("--passphrase-stdin") {
        return Err(invalid());
    }
    if command == "key-state"
        && !matches!(
            o.values["--state"].as_str(),
            "active" | "inactive" | "archived" | "deleted"
        )
    {
        return Err(invalid());
    }
    if command == "key-state"
        && (o.values["--state"] == "deleted")
        && !o.flags.contains("--confirm-delete")
    {
        return Err(invalid());
    }
    Ok(o)
}
fn password() -> Result<Zeroizing<Vec<u8>>, SignerError> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Err(SignerError::InvalidInput("passphrase input must be non-terminal stdin; never pass secrets as arguments or environment variables".into()));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    stdin.lock().take(1026).read_to_end(&mut bytes)?;
    if bytes.len() > 1025 || !bytes.ends_with(b"\n") {
        return Err(invalid());
    }
    bytes.pop();
    if bytes.contains(&b'\n') || bytes.contains(&b'\r') || bytes.is_empty() || bytes.len() > 1024 {
        return Err(invalid());
    }
    Ok(bytes)
}
fn file_path(path: &str) -> Result<(Directory, String), SignerError> {
    let p = Path::new(path);
    let parent = p.parent().ok_or_else(invalid)?;
    let name = p.file_name().and_then(|n| n.to_str()).ok_or_else(invalid)?;
    Ok((Directory::open(parent, true)?, name.into()))
}
fn read_input(path: &str, digest: &str) -> Result<Vec<u8>, SignerError> {
    let (dir, name) = file_path(path)?;
    let bytes = dir.read(&name, MAX_FILE)?;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o7777 != 0o600
        || sha256_hex(&bytes) != digest
    {
        return Err(invalid());
    }
    Ok(bytes)
}
fn write_output(path: &str, bytes: &[u8]) -> Result<Value, SignerError> {
    let (dir, name) = file_path(path)?;
    dir.write_new(&name, bytes)?;
    Ok(json!({"output":path,"sha256":sha256_hex(bytes),"protected_output":true}))
}
pub fn dispatch(args: &[String]) -> Option<Result<Value, SignerError>> {
    let command = args.first()?.as_str();
    if command == "--help" || command == "help" {
        return Some(if args.len() == 1 {
            Ok(json!({"commands":COMMANDS,"usage":[
"key-list | key-audit | control-show | grant-list | signing-audit",
"key-show --signer-id ID | grant-show --grant-id ID",
"key-new --signer-id ID --purpose PURPOSE [--name NAME] [--exportable]",
"key-update --signer-id ID --name NAME --description TEXT --tags comma,separated",
"key-state --signer-id ID --state active|inactive|archived|deleted [--confirm-delete]",
"key-rotate --signer-id OLD --new-signer-id NEW",
"key-export --signer-id ID --output ABSOLUTE --passphrase-stdin",
"key-import --signer-id ID --input ABSOLUTE --expected-sha256 SHA256 --passphrase-stdin [--exportable]",
"backup-create --output ABSOLUTE --passphrase-stdin [--include-nonexportable]",
"restore-plan --input ABSOLUTE --expected-sha256 SHA256 --output ABSOLUTE --passphrase-stdin",
"restore-apply --input ABSOLUTE --expected-sha256 SHA256 --plan ABSOLUTE --expected-plan-sha256 SHA256 --passphrase-stdin",
"management-recover --expected-operation-key SHA256 --expected-target-sha256 SHA256"
],"passphrase":"Required --passphrase-stdin reads one non-terminal line; 12 to 1024 bytes. No argument/environment passwords.","files":"Absolute protected root-owned paths, create-only 0600 outputs; encrypted inputs root-owned 0600 single-link and expected SHA256.","delete":"key-state deleted requires --confirm-delete; public history is retained.","host_installation":"Use approved packaged installer; management commands do not install releases."}))
        } else {
            Err(invalid())
        });
    }
    if !COMMANDS.contains(&command) {
        return None;
    }
    Some(execute(args))
}
fn execute(args: &[String]) -> Result<Value, SignerError> {
    let o = parse(args)?;
    let admin = AdminStore::open_fixed_root()?;
    let v = &o.values;
    let command = args[0].as_str();
    let details = match command {
        "control-show" => admin.control_inventory()?,
        "grant-list" => admin.control_inventory()?["grants"].clone(),
        "grant-show" => {
            let inventory = admin.control_inventory()?;
            inventory["grants"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .find(|entry| entry["grant"]["grant_id"].as_str() == Some(v["--grant-id"].as_str()))
                .cloned()
                .ok_or(SignerError::AuthorizationDenied)?
        }
        "signing-audit" => json!(admin.signing_audit()?),
        "management-recover" => admin.recover_management(
            &v["--expected-operation-key"],
            &v["--expected-target-sha256"],
        )?,
        "key-list" => json!(admin.key_list()?),
        "key-show" => json!(admin.key_get(&v["--signer-id"])?),
        "key-audit" => json!(admin.key_events()?),
        "key-new" => json!(
            admin.key_create_managed(
                &v["--signer-id"],
                &v["--purpose"],
                v.get("--name")
                    .map_or(v["--signer-id"].as_str(), String::as_str),
                o.flags.contains("--exportable")
            )?
        ),
        "key-update" => json!(admin.key_update(
            &v["--signer-id"],
            &v["--name"],
            &v["--description"],
            if v["--tags"].is_empty() {
                vec![]
            } else {
                v["--tags"].split(',').map(str::to_owned).collect()
            }
        )?),
        "key-state" => json!(admin.key_set_state(&v["--signer-id"], &v["--state"])?),
        "key-rotate" => json!(admin.key_rotate(&v["--signer-id"], &v["--new-signer-id"])?),
        "key-export" => {
            let password = password()?;
            write_output(
                &v["--output"],
                &admin.key_export(&v["--signer-id"], &password)?,
            )?
        }
        "key-import" => {
            let encrypted = read_input(&v["--input"], &v["--expected-sha256"])?;
            let password = password()?;
            json!(admin.key_import(
                &v["--signer-id"],
                &encrypted,
                &password,
                o.flags.contains("--exportable")
            )?)
        }
        "backup-create" => {
            let password = password()?;
            write_output(
                &v["--output"],
                &admin.backup_encrypted(&password, o.flags.contains("--include-nonexportable"))?,
            )?
        }
        "restore-plan" => {
            let encrypted = read_input(&v["--input"], &v["--expected-sha256"])?;
            let password = password()?;
            let plan = admin.restore_plan(&encrypted, &password)?;
            let bytes = serde_json::to_vec(&plan).map_err(|_| invalid())?;
            let output = write_output(&v["--output"], &bytes)?;
            json!({"plan":plan,"file":output})
        }
        "restore-apply" => {
            let encrypted = read_input(&v["--input"], &v["--expected-sha256"])?;
            let bytes = read_input(&v["--plan"], &v["--expected-plan-sha256"])?;
            let plan: RestorePlan = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
            let password = password()?;
            admin.restore_commit(&encrypted, &password, &plan)?;
            json!({"restored":true,"signing_enabled":false})
        }
        _ => return Err(invalid()),
    };
    Ok(
        json!({"schema_version":"oasis7.local_signer_admin_result.v1","status":"SUCCEEDED","action":command,"details":details}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn strict_management_parser() {
        assert!(
            parse(&args(&[
                "key-new",
                "--signer-id",
                "key-01",
                "--purpose",
                "file_ed25519_v1"
            ]))
            .is_ok()
        );
        assert!(
            parse(&args(&[
                "key-state",
                "--signer-id",
                "key-01",
                "--state",
                "deleted"
            ]))
            .is_err()
        );
        assert!(
            parse(&args(&[
                "key-export",
                "--signer-id",
                "key-01",
                "--output",
                "/root/out"
            ]))
            .is_err()
        );
        assert!(parse(&args(&["key-list", "--password", "secret"])).is_err());
        assert!(parse(&args(&["key-list", "--exportable"])).is_err());
        assert!(
            parse(&args(&[
                "key-state",
                "--signer-id",
                "key-01",
                "--state",
                "inactive"
            ]))
            .is_ok()
        );
        assert!(
            parse(&args(&[
                "key-state",
                "--signer-id",
                "key-01",
                "--state",
                "disabled"
            ]))
            .is_err()
        );
        assert!(
            parse(&args(&[
                "key-show",
                "--signer-id",
                "key-01",
                "--signer-id",
                "key-02"
            ]))
            .is_err()
        );
    }
    #[test]
    fn help_does_not_open_host() {
        assert!(dispatch(&args(&["--help"])).unwrap().is_ok());
        assert!(dispatch(&args(&["unknown"])).is_none());
    }
}
