use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use oasis7_local_signer::admin::AdminStore;
use oasis7_local_signer::error::SignerError;
use oasis7_local_signer::installation::load_fixed_installation_config;
use oasis7_local_signer::protocol::validate_id;
use oasis7_local_signer::read_candidate_bytes;
use oasis7_local_signer::types::InstallationConfig;

const RESULT_SCHEMA: &str = "oasis7.local_signer_admin_result.v1";
const MAX_CANDIDATE_BYTES: usize = 1024 * 1024;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match execute(&args) {
        Ok(result) => {
            println!("{result}");
            if result["status"] == "BLOCKED" {
                let exit = result["exit_code"].as_i64().unwrap_or(9) as i32;
                std::process::exit(exit);
            }
        }
        Err(error) => {
            println!("{}", blocked(error.code, error.reason, error.exit_code));
            std::process::exit(error.exit_code);
        }
    }
}

#[derive(Debug)]
struct CliError {
    code: &'static str,
    reason: &'static str,
    exit_code: i32,
}

impl CliError {
    const fn new(code: &'static str, reason: &'static str, exit_code: i32) -> Self {
        Self {
            code,
            reason,
            exit_code,
        }
    }

    fn signer(error: SignerError) -> Self {
        Self::new(
            error.code(),
            "administrative operation failed closed; inspect the code and host state before retrying",
            error.exit_code(),
        )
    }

    const fn invalid() -> Self {
        Self::new(
            "INVALID_INPUT",
            "admin command or arguments do not match the fixed CLI schema",
            2,
        )
    }

    const fn unavailable(reason: &'static str) -> Self {
        Self::new("UNSUPPORTED_PLATFORM_OR_FS", reason, 9)
    }

    const fn recovery(reason: &'static str) -> Self {
        Self::new("RECOVERY_REQUIRED", reason, 10)
    }
}

enum Command {
    InstallDryRun,
    InstallApply,
    KeyCreate { signer_id: String, purpose: String },
    KeyPublic { signer_id: String, format: String },
    InstallPolicy { candidate: PathBuf, digest: String },
    InstallGrant { candidate: PathBuf, digest: String },
    RevokeGrant { grant_id: String },
    SetPurpose { purpose: String, enabled: bool },
    BackupPrepare,
    RestorePrepare,
    RestoreCommit,
}

fn execute(args: &[String]) -> Result<serde_json::Value, CliError> {
    if let Some(result) = oasis7_local_signer::manager_cli::dispatch(args) {
        return result.map_err(CliError::signer);
    }
    let command = parse_command(args)?;
    match command {
        Command::InstallDryRun => Err(CliError::unavailable(
            "use the independently approved packaged install-release.py plan/apply entrypoint",
        )),
        Command::InstallApply => Err(CliError::unavailable(
            "use the independently approved packaged install-release.py plan/apply entrypoint; no host changes were made",
        )),
        Command::BackupPrepare => Err(CliError::recovery(
            "backup is not implemented; no snapshot was created",
        )),
        Command::RestorePrepare => Err(CliError::recovery(
            "restore planning is not implemented; no snapshot was read or restored",
        )),
        Command::RestoreCommit => Err(CliError::recovery(
            "restore commit is not implemented; no state was changed",
        )),
        action => {
            // Open the fixed installation before reading any candidate path or
            // invoking a mutator. AdminStore enforces euid=0 and the V2 layout.
            let admin = AdminStore::open_fixed_root().map_err(CliError::signer)?;
            match action {
                Command::KeyCreate { signer_id, purpose } => {
                    let public = admin
                        .create_key(&signer_id, &purpose)
                        .map_err(CliError::signer)?;
                    Ok(succeeded(
                        "key-created",
                        "created",
                        serde_json::json!({
                            "signer_id": signer_id,
                            "purpose": purpose,
                            "public_key_hex": hex::encode(public),
                            "secret_exported": false
                        }),
                    ))
                }
                Command::KeyPublic { signer_id, format } => {
                    let public = admin.key_public(&signer_id).map_err(CliError::signer)?;
                    let public_text = match format.as_str() {
                        "hex" => hex::encode(public),
                        "raw" => {
                            return Err(CliError::new(
                                "INVALID_INPUT",
                                "raw public-key output is not supported by the JSON admin interface; use --format hex",
                                2,
                            ));
                        }
                        _ => return Err(CliError::invalid()),
                    };
                    Ok(succeeded(
                        "key-public",
                        "read-only",
                        serde_json::json!({
                            "signer_id": signer_id,
                            "format": "hex",
                            "public_key": public_text,
                            "secret_exported": false
                        }),
                    ))
                }
                Command::InstallPolicy { candidate, digest } => {
                    let installation =
                        load_fixed_installation_config().map_err(CliError::signer)?;
                    let bytes = read_candidate(&candidate, &installation)?;
                    admin
                        .install_policy_candidate(&bytes, &digest)
                        .map_err(CliError::signer)?;
                    Ok(succeeded(
                        "policy-installed",
                        "applied-or-idempotent",
                        serde_json::json!({
                            "candidate_sha256": digest,
                            "signing_enabled": false
                        }),
                    ))
                }
                Command::InstallGrant { candidate, digest } => {
                    let installation =
                        load_fixed_installation_config().map_err(CliError::signer)?;
                    let bytes = read_candidate(&candidate, &installation)?;
                    admin
                        .install_grant_candidate(&bytes, &digest)
                        .map_err(CliError::signer)?;
                    Ok(succeeded(
                        "grant-installed",
                        "applied-or-idempotent",
                        serde_json::json!({ "candidate_sha256": digest }),
                    ))
                }
                Command::RevokeGrant { grant_id } => {
                    admin.revoke_grant(&grant_id).map_err(CliError::signer)?;
                    Ok(succeeded(
                        "grant-revoked",
                        "applied-or-idempotent",
                        serde_json::json!({ "grant_id": grant_id, "grant_record_retained": true }),
                    ))
                }
                Command::SetPurpose { purpose, enabled } => {
                    admin
                        .set_purpose_enabled(&purpose, enabled)
                        .map_err(CliError::signer)?;
                    Ok(succeeded(
                        if enabled {
                            "purpose-enabled"
                        } else {
                            "purpose-disabled"
                        },
                        "applied-or-idempotent",
                        serde_json::json!({ "purpose": purpose, "signing_enabled": enabled }),
                    ))
                }
                Command::InstallDryRun
                | Command::InstallApply
                | Command::BackupPrepare
                | Command::RestorePrepare
                | Command::RestoreCommit => {
                    unreachable!("handled before opening AdminStore")
                }
            }
        }
    }
}

fn parse_command(args: &[String]) -> Result<Command, CliError> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(CliError::invalid());
    };
    let tail = &args[1..];
    match command {
        "install" => {
            let allowed = [
                "--store-dir",
                "--release-manifest",
                "--expected-manifest-sha256",
                "--caller-user",
                "--signer-user",
            ];
            if tail.iter().any(|argument| argument == "--apply") {
                if tail.len() == 1 {
                    return Ok(Command::InstallApply);
                }
                let _ = exact_options(tail, &allowed, &["--apply"])?;
                return Ok(Command::InstallApply);
            }
            if tail.iter().any(|argument| argument == "--dry-run") {
                let _ = exact_options(tail, &allowed, &["--dry-run"])?;
                return Ok(Command::InstallDryRun);
            }
            Err(CliError::invalid())
        }
        "key-create" => {
            let options = exact_options(tail, &["--signer-id", "--purpose"], &[])?;
            let signer_id = checked_id(options["--signer-id"])?;
            let purpose = checked_id(options["--purpose"])?;
            Ok(Command::KeyCreate { signer_id, purpose })
        }
        "key-public" => {
            let options = exact_options(tail, &["--signer-id", "--format"], &[])?;
            let signer_id = checked_id(options["--signer-id"])?;
            if options["--format"] != "hex" {
                return Err(CliError::invalid());
            }
            Ok(Command::KeyPublic {
                signer_id,
                format: options["--format"].to_owned(),
            })
        }
        "policy-install" | "approve-batch" => {
            let options = exact_options(tail, &["--candidate", "--expected-sha256"], &[])?;
            let candidate = checked_candidate_path(options["--candidate"])?;
            let digest = checked_sha256(options["--expected-sha256"])?;
            if command == "policy-install" {
                Ok(Command::InstallPolicy { candidate, digest })
            } else {
                Ok(Command::InstallGrant { candidate, digest })
            }
        }
        "revoke-grant" => {
            let options = exact_options(tail, &["--grant-id"], &[])?;
            Ok(Command::RevokeGrant {
                grant_id: checked_id(options["--grant-id"])?,
            })
        }
        "enable" | "disable" => {
            let options = exact_options(tail, &["--purpose"], &[])?;
            Ok(Command::SetPurpose {
                purpose: checked_id(options["--purpose"])?,
                enabled: command == "enable",
            })
        }
        "backup-prepare" if tail.is_empty() => Ok(Command::BackupPrepare),
        "restore-prepare" => {
            let options = exact_options(tail, &["--approved-snapshot"], &[])?;
            if options["--approved-snapshot"].is_empty() {
                return Err(CliError::invalid());
            }
            Ok(Command::RestorePrepare)
        }
        "restore-commit" => {
            let options = exact_options(tail, &["--plan", "--expected-sha256"], &[])?;
            if options["--plan"].is_empty() {
                return Err(CliError::invalid());
            }
            let _ = checked_sha256(options["--expected-sha256"])?;
            Ok(Command::RestoreCommit)
        }
        _ => Err(CliError::invalid()),
    }
}

fn exact_options<'a>(
    args: &'a [String],
    required: &[&str],
    flags: &[&str],
) -> Result<BTreeMap<&'a str, &'a str>, CliError> {
    let mut values = BTreeMap::new();
    let mut seen_flags = std::collections::BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        let key = args[index].as_str();
        if flags.contains(&key) {
            if !seen_flags.insert(key) {
                return Err(CliError::invalid());
            }
            index += 1;
            continue;
        }
        let value = args.get(index + 1).ok_or_else(CliError::invalid)?.as_str();
        if !required.contains(&key) || values.insert(key, value).is_some() {
            return Err(CliError::invalid());
        }
        index += 2;
    }
    if values.len() != required.len() {
        return Err(CliError::invalid());
    }
    Ok(values)
}

fn checked_id(value: &str) -> Result<String, CliError> {
    validate_id(value).map_err(|_| CliError::invalid())?;
    Ok(value.to_owned())
}

fn checked_sha256(value: &str) -> Result<String, CliError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CliError::invalid());
    }
    Ok(value.to_owned())
}

fn checked_candidate_path(value: &str) -> Result<PathBuf, CliError> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(CliError::invalid());
    }
    Ok(path.to_path_buf())
}

fn read_candidate(path: &Path, installation: &InstallationConfig) -> Result<Vec<u8>, CliError> {
    let expected_owner_uid = candidate_owner_uid(path, installation)?;
    read_candidate_bytes(path, expected_owner_uid, MAX_CANDIDATE_BYTES).map_err(|error| match error {
        SignerError::AuthorizationDenied => CliError::new(
            "AUTHORIZATION_DENIED",
            "candidate owner is not the bound caller for its work root",
            3,
        ),
        SignerError::InvalidInput(_) => CliError::new(
            "INVALID_INPUT",
            "candidate must be a single-linked regular file, not group/other writable, and at most 1 MiB",
            2,
        ),
        SignerError::InstallationDrift | SignerError::PersistenceFailed(_) => CliError::new(
            "INSTALLATION_DRIFT",
            "candidate could not be opened safely from its fixed path",
            11,
        ),
        other => CliError::signer(other),
    })
}

fn candidate_owner_uid(path: &Path, installation: &InstallationConfig) -> Result<u32, CliError> {
    let mut matching = installation.callers.iter().filter_map(|caller| {
        let caller_root = Path::new(&caller.work_dir);
        let relative = path.strip_prefix(caller_root).ok()?;
        let mut components = relative.components();
        let Some(Component::Normal(job_id)) = components.next() else {
            return None;
        };
        let Some(Component::Normal(_candidate_name)) = components.next() else {
            return None;
        };
        let job_id = job_id.to_str()?;
        validate_id(job_id).ok()?;
        Some(caller.uid)
    });

    let Some(owner_uid) = matching.next() else {
        return Err(CliError::new(
            "AUTHORIZATION_DENIED",
            "candidate must be below a configured caller work job directory",
            3,
        ));
    };
    if matching.next().is_some() {
        return Err(CliError::new(
            "INSTALLATION_DRIFT",
            "candidate path matches multiple caller work roots",
            11,
        ));
    }
    Ok(owner_uid)
}

fn succeeded(
    action: &'static str,
    effect: &'static str,
    details: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": RESULT_SCHEMA,
        "status": "SUCCEEDED",
        "action": action,
        "effect": effect,
        "details": details
    })
}

fn blocked(code: &str, reason: &str, exit_code: i32) -> serde_json::Value {
    serde_json::json!({
        "schema_version": RESULT_SCHEMA,
        "status": "BLOCKED",
        "code": code,
        "reason": reason,
        "exit_code": exit_code
    })
}

#[cfg(test)]
mod tests {
    use super::{CliError, Command, blocked, execute, parse_command};
    use oasis7_local_signer::types::{CallerBinding, InstallationConfig};
    use std::path::Path;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn installation(callers: Vec<CallerBinding>) -> InstallationConfig {
        InstallationConfig {
            schema_version: "oasis7.local_signer_installation.v3".to_owned(),
            installation_id: "installation-01".to_owned(),
            deployment_id: "deployment-01".to_owned(),
            store_dir: "/var/lib/oasis7-local-signer".to_owned(),
            store_device_id: 1,
            store_inode: 1,
            signer_uid: 600,
            signer_gid: 600,
            callers,
            release_id: "release-01".to_owned(),
            worker_executable: "/usr/local/libexec/oasis7-local-signer-worker".to_owned(),
            worker_sha256: "a".repeat(64),
            control_schema_version: "oasis7.local_signer_control.v1".to_owned(),
        }
    }

    #[test]
    fn candidate_reads_are_bound_to_exactly_one_caller_work_root() {
        let configured = installation(vec![CallerBinding {
            uid: 501,
            work_dir: "/caller-jobs/caller-501".to_owned(),
            work_device_id: 1,
            work_inode: 2,
        }]);
        let candidate = Path::new("/caller-jobs/caller-501/job-01/policy.json");
        assert_eq!(
            super::candidate_owner_uid(candidate, &configured)
                .expect("configured caller work root"),
            501
        );

        let outside = Path::new("/caller-jobs/caller-501-evil/job-01/policy.json");
        assert_eq!(
            super::candidate_owner_uid(outside, &configured)
                .expect_err("prefix collision is outside the caller root")
                .code,
            "AUTHORIZATION_DENIED"
        );
        let missing_job = Path::new("/caller-jobs/caller-501/policy.json");
        assert!(super::candidate_owner_uid(missing_job, &configured).is_err());

        let ambiguous = installation(vec![
            CallerBinding {
                uid: 501,
                work_dir: "/caller-jobs/caller-shared".to_owned(),
                work_device_id: 1,
                work_inode: 2,
            },
            CallerBinding {
                uid: 502,
                work_dir: "/caller-jobs/caller-shared".to_owned(),
                work_device_id: 1,
                work_inode: 2,
            },
        ]);
        let shared_path = Path::new("/caller-jobs/caller-shared/job-01/policy.json");
        assert_eq!(
            super::candidate_owner_uid(shared_path, &ambiguous)
                .expect_err("ambiguous caller root fails closed")
                .code,
            "INSTALLATION_DRIFT"
        );
    }

    #[test]
    fn admin_mutation_arguments_are_exact_and_fixed() {
        assert!(matches!(
            parse_command(&arguments(&[
                "key-create",
                "--signer-id",
                "rollback-key",
                "--purpose",
                "rollback_strict_audit_v1"
            ])),
            Ok(Command::KeyCreate { .. })
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "key-public",
                "--signer-id",
                "rollback-key",
                "--format",
                "hex"
            ])),
            Ok(Command::KeyPublic { .. })
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "approve-batch",
                "--candidate",
                "/tmp/grant.json",
                "--expected-sha256",
                &"b".repeat(64)
            ])),
            Ok(Command::InstallGrant { .. })
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "enable",
                "--purpose",
                "rollback_strict_audit_v1"
            ])),
            Ok(Command::SetPurpose { enabled: true, .. })
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "disable",
                "--purpose",
                "rollback_strict_audit_v1"
            ])),
            Ok(Command::SetPurpose { enabled: false, .. })
        ));
    }

    #[test]
    fn cli_rejects_caller_selected_output_and_arbitrary_paths() {
        assert!(
            parse_command(&arguments(&[
                "key-public",
                "--signer-id",
                "rollback-key",
                "--format",
                "hex",
                "--out",
                "/tmp/key.hex",
            ]))
            .is_err()
        );
        assert!(
            parse_command(&arguments(&[
                "policy-install",
                "--candidate",
                "relative.json",
                "--expected-sha256",
                &"a".repeat(64),
            ]))
            .is_err()
        );
        assert!(
            parse_command(&arguments(&[
                "policy-install",
                "--candidate",
                "/tmp/../etc/passwd",
                "--expected-sha256",
                &"a".repeat(64),
            ]))
            .is_err()
        );
    }

    #[test]
    fn install_and_recovery_remain_explicitly_gated() {
        assert!(matches!(
            parse_command(&arguments(&["install", "--apply"])),
            Ok(Command::InstallApply)
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "install",
                "--dry-run",
                "--store-dir",
                "/tmp/store",
                "--release-manifest",
                "/tmp/release.json",
                "--expected-manifest-sha256",
                &"d".repeat(64),
                "--caller-user",
                "oasis-caller",
                "--signer-user",
                "oasis-signer"
            ])),
            Ok(Command::InstallDryRun)
        ));
        assert!(matches!(
            parse_command(&arguments(&["backup-prepare"])),
            Ok(Command::BackupPrepare)
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "restore-prepare",
                "--approved-snapshot",
                "/tmp/snapshot"
            ])),
            Ok(Command::RestorePrepare)
        ));
        assert!(matches!(
            parse_command(&arguments(&[
                "restore-commit",
                "--plan",
                "/tmp/plan.json",
                "--expected-sha256",
                &"c".repeat(64)
            ])),
            Ok(Command::RestoreCommit)
        ));
        assert_eq!(CliError::recovery("x").exit_code, 10);
    }

    #[test]
    fn install_apply_execute_returns_safe_block_without_host_access() {
        // This command must hit execute's early gate before opening AdminStore
        // or reading the fixed host installation configuration.
        let error = execute(&arguments(&["install", "--apply"]))
            .expect_err("initial host installation must remain gated");
        assert_eq!(error.code, "UNSUPPORTED_PLATFORM_OR_FS");
        assert_eq!(error.exit_code, 9);
        assert!(error.reason.contains("no host changes were made"));

        // Match the BLOCKED response that main emits for this execution error.
        let result = blocked(error.code, error.reason, error.exit_code);
        assert_eq!(result["status"], "BLOCKED");
        assert_eq!(result["code"], "UNSUPPORTED_PLATFORM_OR_FS");
        assert!(result.get("host_mutated").is_none());
        assert!(result.get("signing_enabled").is_none());
    }

    #[test]
    fn parser_exercises_every_command_form() {
        let parsed = [
            parse_command(&arguments(&[
                "install",
                "--dry-run",
                "--store-dir",
                "/tmp/store",
                "--release-manifest",
                "/tmp/release.json",
                "--expected-manifest-sha256",
                &"d".repeat(64),
                "--caller-user",
                "oasis-caller",
                "--signer-user",
                "oasis-signer",
            ])),
            parse_command(&arguments(&["install", "--apply"])),
            parse_command(&arguments(&[
                "key-create",
                "--signer-id",
                "rollback-key",
                "--purpose",
                "rollback_strict_audit_v1",
            ])),
            parse_command(&arguments(&[
                "key-public",
                "--signer-id",
                "rollback-key",
                "--format",
                "hex",
            ])),
            parse_command(&arguments(&[
                "policy-install",
                "--candidate",
                "/tmp/policy.json",
                "--expected-sha256",
                &"a".repeat(64),
            ])),
            parse_command(&arguments(&[
                "approve-batch",
                "--candidate",
                "/tmp/grant.json",
                "--expected-sha256",
                &"b".repeat(64),
            ])),
            parse_command(&arguments(&["revoke-grant", "--grant-id", "grant-01"])),
            parse_command(&arguments(&[
                "enable",
                "--purpose",
                "rollback_strict_audit_v1",
            ])),
            parse_command(&arguments(&[
                "disable",
                "--purpose",
                "rollback_strict_audit_v1",
            ])),
            parse_command(&arguments(&["backup-prepare"])),
            parse_command(&arguments(&[
                "restore-prepare",
                "--approved-snapshot",
                "/tmp/snapshot",
            ])),
            parse_command(&arguments(&[
                "restore-commit",
                "--plan",
                "/tmp/plan.json",
                "--expected-sha256",
                &"c".repeat(64),
            ])),
        ];
        assert!(parsed.iter().all(Result::is_ok));
        assert_eq!(parsed.len(), 12);
    }
}
