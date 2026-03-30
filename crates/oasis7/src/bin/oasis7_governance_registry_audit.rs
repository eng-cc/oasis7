use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

use oasis7::runtime::{
    World, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE,
    MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
};
use serde::{Deserialize, Serialize};

const DEFAULT_FINALITY_SLOT_ID: &str = "governance.finality.v1";
const DEFAULT_EXPECTED_THRESHOLD: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    world_dir: PathBuf,
    public_manifest: Option<PathBuf>,
    finality_slot_id: String,
    default_expected_threshold: u16,
    strict_manifest_match: bool,
    require_single_failure_tolerance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PublicManifestEntry {
    slot_id: String,
    signer_id: String,
    scheme: String,
    public_key_hex: String,
    #[serde(default)]
    threshold: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestSlotExpectation {
    threshold: u16,
    public_keys: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct GovernanceRegistryAuditReport {
    world_dir: String,
    finality: GovernanceSlotAuditRow,
    controllers: Vec<GovernanceSlotAuditRow>,
    overall_single_failure_tolerance_pass: bool,
    manifest_match_pass: Option<bool>,
    overall_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct GovernanceSlotAuditRow {
    slot_id: String,
    threshold: u16,
    signer_count: usize,
    tolerated_failures: usize,
    single_failure_tolerant: bool,
    threshold_matches_expectation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_match: Option<bool>,
    status: String,
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }
    let options = match parse_options(raw_args.iter().map(|arg| arg.as_str())) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            print_help();
            process::exit(1);
        }
    };

    let report = match build_audit_report(&options) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("oasis7_governance_registry_audit failed: {err}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("encode audit report")
    );
    let errors = validate_audit_report(&options, &report);
    if !errors.is_empty() {
        for error in errors {
            eprintln!("{error}");
        }
        process::exit(2);
    }
}

fn build_audit_report(options: &CliOptions) -> Result<GovernanceRegistryAuditReport, String> {
    let world = World::load_from_dir(options.world_dir.as_path())
        .map_err(|err| format!("load world {} failed: {err:?}", options.world_dir.display()))?;
    let manifest_slots = if let Some(path) = options.public_manifest.as_ref() {
        Some(load_manifest_slot_expectations(
            path.as_path(),
            options.default_expected_threshold,
        )?)
    } else {
        None
    };

    let finality_registry = world
        .governance_finality_signer_registry()
        .ok_or_else(|| "world is missing governance finality signer registry".to_string())?;
    let finality_keys = finality_registry
        .signer_bindings
        .values()
        .cloned()
        .collect::<BTreeSet<String>>();
    let finality_expected_threshold = manifest_slots
        .as_ref()
        .and_then(|slots| slots.get(options.finality_slot_id.as_str()))
        .map(|slot| slot.threshold)
        .unwrap_or(options.default_expected_threshold);
    let finality_manifest_match = manifest_slots
        .as_ref()
        .map(|slots| {
            slots.get(options.finality_slot_id.as_str()).is_some_and(|slot| {
                slot.public_keys == finality_keys && slot.threshold == finality_registry.threshold
            })
        });
    let finality = audit_row(
        finality_registry.slot_id.as_str(),
        finality_registry.threshold,
        finality_registry.signer_bindings.len(),
        finality_expected_threshold,
        finality_manifest_match,
    );

    let controller_registry = world
        .governance_main_token_controller_registry()
        .ok_or_else(|| "world is missing governance main token controller registry".to_string())?;
    let mut controllers = controller_registry
        .controller_signer_policies
        .iter()
        .map(|(slot_id, policy)| {
            let expected_threshold = manifest_slots
                .as_ref()
                .and_then(|slots| slots.get(slot_id.as_str()))
                .map(|slot| slot.threshold)
                .unwrap_or(options.default_expected_threshold);
            let manifest_match = manifest_slots
                .as_ref()
                .map(|slots| {
                    slots.get(slot_id.as_str()).is_some_and(|slot| {
                        slot.public_keys == policy.allowed_public_keys
                            && slot.threshold == policy.threshold
                    })
                });
            audit_row(
                slot_id.as_str(),
                policy.threshold,
                policy.allowed_public_keys.len(),
                expected_threshold,
                manifest_match,
            )
        })
        .collect::<Vec<_>>();
    controllers.sort_by(|left, right| left.slot_id.cmp(&right.slot_id));

    let overall_single_failure_tolerance_pass = finality.single_failure_tolerant
        && controllers.iter().all(|row| row.single_failure_tolerant);
    let threshold_expectation_pass = finality.threshold_matches_expectation
        && controllers
            .iter()
            .all(|row| row.threshold_matches_expectation);
    let manifest_match_pass = manifest_slots.as_ref().map(|slots| {
        let world_slot_ids = std::iter::once(finality.slot_id.clone())
            .chain(controllers.iter().map(|row| row.slot_id.clone()))
            .collect::<BTreeSet<String>>();
        let manifest_slot_ids = slots.keys().cloned().collect::<BTreeSet<String>>();
        world_slot_ids == manifest_slot_ids
            && finality.manifest_match.unwrap_or(false)
            && controllers
                .iter()
                .all(|row| row.manifest_match.unwrap_or(false))
    });
    let overall_status = if !threshold_expectation_pass {
        "threshold_mismatch".to_string()
    } else if overall_single_failure_tolerance_pass && manifest_match_pass.unwrap_or(true) {
        "ready_for_ops_drill".to_string()
    } else if !overall_single_failure_tolerance_pass {
        "failover_blocked".to_string()
    } else {
        "manifest_mismatch".to_string()
    };

    Ok(GovernanceRegistryAuditReport {
        world_dir: options.world_dir.display().to_string(),
        finality,
        controllers,
        overall_single_failure_tolerance_pass,
        manifest_match_pass,
        overall_status,
    })
}

fn audit_row(
    slot_id: &str,
    threshold: u16,
    signer_count: usize,
    expected_threshold: u16,
    manifest_match: Option<bool>,
) -> GovernanceSlotAuditRow {
    let tolerated_failures = signer_count.saturating_sub(usize::from(threshold));
    let single_failure_tolerant = tolerated_failures >= 1;
    let threshold_matches_expectation = threshold == expected_threshold;
    let status = if !threshold_matches_expectation {
        "threshold_mismatch".to_string()
    } else if !single_failure_tolerant {
        "single_failure_blocks_slot".to_string()
    } else {
        "single_failure_tolerant".to_string()
    };
    GovernanceSlotAuditRow {
        slot_id: slot_id.to_string(),
        threshold,
        signer_count,
        tolerated_failures,
        single_failure_tolerant,
        threshold_matches_expectation,
        manifest_match,
        status,
    }
}

fn validate_audit_report(
    options: &CliOptions,
    report: &GovernanceRegistryAuditReport,
) -> Vec<String> {
    let mut errors = Vec::new();
    if !report.finality.threshold_matches_expectation
        || report
            .controllers
            .iter()
            .any(|row| !row.threshold_matches_expectation)
    {
        errors.push(format!(
            "governance registry audit failed: at least one slot threshold does not match the expected threshold set"
        ));
    }
    if options.require_single_failure_tolerance && !report.overall_single_failure_tolerance_pass {
        errors.push(
            "governance registry audit failed: at least one slot cannot tolerate a single signer failure"
                .to_string(),
        );
    }
    if options.strict_manifest_match && report.manifest_match_pass != Some(true) {
        errors.push(
            "governance registry audit failed: world-state registry does not exactly match the provided public manifest"
                .to_string(),
        );
    }
    errors
}

fn load_manifest_slot_expectations(
    path: &Path,
    default_threshold: u16,
) -> Result<BTreeMap<String, ManifestSlotExpectation>, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("read public manifest {} failed: {err}", path.display()))?;
    let entries: Vec<PublicManifestEntry> = serde_json::from_slice(bytes.as_slice())
        .map_err(|err| format!("decode public manifest {} failed: {err}", path.display()))?;
    let mut slots = BTreeMap::new();
    for entry in entries {
        validate_manifest_entry(&entry)?;
        let slot_id = entry.slot_id.trim().to_string();
        let resolved_threshold = entry.threshold.unwrap_or(default_threshold);
        let slot = slots
            .entry(slot_id.clone())
            .or_insert_with(|| ManifestSlotExpectation {
                threshold: resolved_threshold,
                public_keys: BTreeSet::new(),
            });
        if slot.threshold != resolved_threshold {
            return Err(format!(
                "manifest slot threshold mismatch slot_id={} expected={} actual={resolved_threshold}",
                entry.slot_id, slot.threshold
            ));
        }
        slot.public_keys
            .insert(entry.public_key_hex.trim().to_string());
    }
    Ok(slots)
}

fn validate_manifest_entry(entry: &PublicManifestEntry) -> Result<(), String> {
    if entry.slot_id.trim().is_empty() || entry.signer_id.trim().is_empty() {
        return Err("manifest entry slot_id/signer_id cannot be empty".to_string());
    }
    if !entry.scheme.trim().eq_ignore_ascii_case("ed25519") {
        return Err(format!(
            "unsupported signer scheme slot_id={} signer_id={} scheme={}",
            entry.slot_id, entry.signer_id, entry.scheme
        ));
    }
    let public_key_hex = entry.public_key_hex.trim();
    if public_key_hex.len() != 64 || !public_key_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "manifest entry public_key_hex must be 32-byte hex slot_id={} signer_id={}",
            entry.slot_id, entry.signer_id
        ));
    }
    if entry.threshold.is_some_and(|value| value == 0) {
        return Err(format!(
            "manifest entry threshold must be > 0 slot_id={} signer_id={}",
            entry.slot_id, entry.signer_id
        ));
    }
    Ok(())
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut world_dir = None;
    let mut public_manifest = None;
    let mut finality_slot_id = DEFAULT_FINALITY_SLOT_ID.to_string();
    let mut default_expected_threshold = DEFAULT_EXPECTED_THRESHOLD;
    let mut strict_manifest_match = false;
    let mut require_single_failure_tolerance = false;
    let mut iter = args.peekable();
    while let Some(arg) = iter.next() {
        match arg {
            "--world-dir" => {
                world_dir = Some(PathBuf::from(parse_required_value(
                    &mut iter,
                    "--world-dir",
                )?));
            }
            "--public-manifest" => {
                public_manifest = Some(PathBuf::from(parse_required_value(
                    &mut iter,
                    "--public-manifest",
                )?));
            }
            "--finality-slot-id" => {
                finality_slot_id = parse_required_value(&mut iter, "--finality-slot-id")?;
            }
            "--expected-threshold" => {
                default_expected_threshold =
                    parse_required_value(&mut iter, "--expected-threshold")?
                    .parse::<u16>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                        "--expected-threshold requires a positive integer".to_string()
                    })?;
            }
            "--strict-manifest-match" => {
                strict_manifest_match = true;
            }
            "--require-single-failure-tolerance" => {
                require_single_failure_tolerance = true;
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }
    Ok(CliOptions {
        world_dir: world_dir.ok_or_else(|| "--world-dir is required".to_string())?,
        public_manifest,
        finality_slot_id,
        default_expected_threshold,
        strict_manifest_match,
        require_single_failure_tolerance,
    })
}

fn parse_required_value<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
    flag: &str,
) -> Result<String, String> {
    iter.next()
        .map(|value| value.to_string())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_help() {
    eprintln!(
        "Usage: oasis7_governance_registry_audit --world-dir <dir> [--public-manifest <file>] [--finality-slot-id <slot>] [--expected-threshold <n>] [--strict-manifest-match] [--require-single-failure-tolerance]"
    );
    eprintln!(
        "  --expected-threshold <n> acts as the default threshold for manifest slots that omit `threshold`"
    );
    eprintln!(
        "Known treasury buckets: {}/{}/{}",
        MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
        MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
        MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE
    );
}

#[cfg(test)]
mod tests {
    use super::{build_audit_report, parse_options, validate_audit_report, CliOptions};
    use oasis7::runtime::{
        GovernanceFinalitySignerRegistry, GovernanceMainTokenControllerRegistry,
        GovernanceThresholdSignerPolicy, World,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-governance-audit-{prefix}-{unique}"))
    }

    fn write_world_and_manifest() -> (PathBuf, PathBuf) {
        let root = temp_dir("fixture");
        std::fs::create_dir_all(&root).expect("create root");
        let world_dir = root.join("world");
        let manifest_path = root.join("public_manifest.json");
        let mut world = World::new();
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "governance.finality.v1.signer01".to_string(),
                        "54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.signer02".to_string(),
                        "38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.signer03".to_string(),
                        "e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8"
                            .to_string(),
                    ),
                ]),
            })
            .expect("set finality registry");
        world
            .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
                genesis_controller_account_id: "msig.genesis.v1".to_string(),
                treasury_bucket_controller_slots: BTreeMap::from([
                    (
                        "staking_reward_pool".to_string(),
                        "msig.staking_governance.v1".to_string(),
                    ),
                    (
                        "ecosystem_pool".to_string(),
                        "msig.ecosystem_governance.v1".to_string(),
                    ),
                    (
                        "security_reserve".to_string(),
                        "msig.security_council.v1".to_string(),
                    ),
                ]),
                restricted_starter_claim_admin_account_ids: BTreeSet::from([
                    "msig.ecosystem_governance.v1".to_string(),
                ]),
                controller_signer_policies: BTreeMap::from([
                    (
                        "msig.genesis.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                                    .to_string(),
                                "7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"
                                    .to_string(),
                                "f4ecbcb4cbff4acb76cd4bf80fd3b6589a5c7ca2ac2f812380acb7b2cfa4a27c"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.staking_governance.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                                    .to_string(),
                                "10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"
                                    .to_string(),
                                "c5c478f1a86b1ecdfa2d09af65f673d2835ee65f35ebd237270295d3773c2ba4"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.ecosystem_governance.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e"
                                    .to_string(),
                                "960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4"
                                    .to_string(),
                                "f01a2f8e033d38b369af6bb9a80814a97d749a89ac9d071cc2fdfde1b1010b8a"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.security_council.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448"
                                    .to_string(),
                                "aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c"
                                    .to_string(),
                                "f852493e575e33647c005fe2fb43eb15963c73e4213c411e3920f25b498a6980"
                                    .to_string(),
                            ]),
                        },
                    ),
                ]),
            })
            .expect("set controller registry");
        world.save_to_dir(&world_dir).expect("save world");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&vec![
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc"}),
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4"}),
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f4ecbcb4cbff4acb76cd4bf80fd3b6589a5c7ca2ac2f812380acb7b2cfa4a27c"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"c5c478f1a86b1ecdfa2d09af65f673d2835ee65f35ebd237270295d3773c2ba4"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f01a2f8e033d38b369af6bb9a80814a97d749a89ac9d071cc2fdfde1b1010b8a"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f852493e575e33647c005fe2fb43eb15963c73e4213c411e3920f25b498a6980"})
            ])
            .expect("encode manifest"),
        )
        .expect("write manifest");
        (world_dir, manifest_path)
    }

    #[test]
    fn parse_options_reads_flags() {
        let options = parse_options(
            [
                "--world-dir",
                "output/world",
                "--public-manifest",
                "manifest.json",
                "--strict-manifest-match",
                "--require-single-failure-tolerance",
            ]
            .into_iter(),
        )
        .expect("parse options");
        assert!(options.strict_manifest_match);
        assert!(options.require_single_failure_tolerance);
        assert_eq!(options.default_expected_threshold, 2);
    }

    #[test]
    fn audit_report_passes_for_matching_two_of_three_registry() {
        let (world_dir, manifest_path) = write_world_and_manifest();
        let options = CliOptions {
            world_dir,
            public_manifest: Some(manifest_path),
            finality_slot_id: "governance.finality.v1".to_string(),
            default_expected_threshold: 2,
            strict_manifest_match: true,
            require_single_failure_tolerance: true,
        };
        let report = build_audit_report(&options).expect("build report");
        assert!(report.overall_single_failure_tolerance_pass);
        assert_eq!(report.manifest_match_pass, Some(true));
        assert_eq!(report.overall_status, "ready_for_ops_drill");
        assert!(validate_audit_report(&options, &report).is_empty());
    }

    #[test]
    fn audit_report_blocks_single_failure_when_threshold_equals_signer_count() {
        let (world_dir, _) = write_world_and_manifest();
        let options = CliOptions {
            world_dir,
            public_manifest: None,
            finality_slot_id: "governance.finality.v1".to_string(),
            default_expected_threshold: 3,
            strict_manifest_match: false,
            require_single_failure_tolerance: true,
        };
        let report = build_audit_report(&options).expect("build report");
        assert!(!report.finality.threshold_matches_expectation);
        assert!(!validate_audit_report(&options, &report).is_empty());
    }

    #[test]
    fn audit_report_accepts_manifest_specific_liveops_threshold() {
        let root = temp_dir("liveops-threshold");
        std::fs::create_dir_all(&root).expect("create root");
        let world_dir = root.join("world");
        let manifest_path = root.join("public_manifest.json");
        let mut world = World::new();
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "governance.finality.v1.signer01".to_string(),
                        "54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.signer02".to_string(),
                        "38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.signer03".to_string(),
                        "e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8"
                            .to_string(),
                    ),
                ]),
            })
            .expect("set finality registry");
        world
            .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
                genesis_controller_account_id: "msig.genesis.v1".to_string(),
                treasury_bucket_controller_slots: BTreeMap::from([
                    (
                        "staking_reward_pool".to_string(),
                        "msig.staking_governance.v1".to_string(),
                    ),
                    (
                        "ecosystem_pool".to_string(),
                        "msig.ecosystem_governance.v1".to_string(),
                    ),
                    (
                        "security_reserve".to_string(),
                        "msig.security_council.v1".to_string(),
                    ),
                ]),
                restricted_starter_claim_admin_account_ids: BTreeSet::from([
                    "liveops".to_string(),
                ]),
                controller_signer_policies: BTreeMap::from([
                    (
                        "msig.genesis.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                                    .to_string(),
                                "7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"
                                    .to_string(),
                                "f4ecbcb4cbff4acb76cd4bf80fd3b6589a5c7ca2ac2f812380acb7b2cfa4a27c"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.staking_governance.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                                    .to_string(),
                                "10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"
                                    .to_string(),
                                "c5c478f1a86b1ecdfa2d09af65f673d2835ee65f35ebd237270295d3773c2ba4"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.ecosystem_governance.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e"
                                    .to_string(),
                                "960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4"
                                    .to_string(),
                                "f01a2f8e033d38b369af6bb9a80814a97d749a89ac9d071cc2fdfde1b1010b8a"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.security_council.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448"
                                    .to_string(),
                                "aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c"
                                    .to_string(),
                                "f852493e575e33647c005fe2fb43eb15963c73e4213c411e3920f25b498a6980"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "liveops".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 1,
                            allowed_public_keys: BTreeSet::from([
                                "14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d"
                                    .to_string(),
                                "b6517819f923b8b25989042b03e00854673b5517be88e3f568141373105ca77f"
                                    .to_string(),
                            ]),
                        },
                    ),
                ]),
            })
            .expect("set controller registry");
        world.save_to_dir(&world_dir).expect("save world");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&vec![
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc"}),
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4"}),
                serde_json::json!({"slot_id":"governance.finality.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"}),
                serde_json::json!({"slot_id":"msig.genesis.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f4ecbcb4cbff4acb76cd4bf80fd3b6589a5c7ca2ac2f812380acb7b2cfa4a27c"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"}),
                serde_json::json!({"slot_id":"msig.staking_governance.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"c5c478f1a86b1ecdfa2d09af65f673d2835ee65f35ebd237270295d3773c2ba4"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4"}),
                serde_json::json!({"slot_id":"msig.ecosystem_governance.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f01a2f8e033d38b369af6bb9a80814a97d749a89ac9d071cc2fdfde1b1010b8a"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer01","scheme":"ed25519","public_key_hex":"d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer02","scheme":"ed25519","public_key_hex":"aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c"}),
                serde_json::json!({"slot_id":"msig.security_council.v1","signer_id":"signer03","scheme":"ed25519","public_key_hex":"f852493e575e33647c005fe2fb43eb15963c73e4213c411e3920f25b498a6980"}),
                serde_json::json!({"slot_id":"liveops","signer_id":"signer01","scheme":"ed25519","threshold":1,"public_key_hex":"14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d"}),
                serde_json::json!({"slot_id":"liveops","signer_id":"signer02","scheme":"ed25519","threshold":1,"public_key_hex":"b6517819f923b8b25989042b03e00854673b5517be88e3f568141373105ca77f"})
            ])
            .expect("encode manifest"),
        )
        .expect("write manifest");

        let options = CliOptions {
            world_dir,
            public_manifest: Some(manifest_path),
            finality_slot_id: "governance.finality.v1".to_string(),
            default_expected_threshold: 2,
            strict_manifest_match: true,
            require_single_failure_tolerance: true,
        };
        let report = build_audit_report(&options).expect("build report");
        let liveops = report
            .controllers
            .iter()
            .find(|row| row.slot_id == "liveops")
            .expect("liveops row");
        assert_eq!(liveops.threshold, 1);
        assert!(liveops.threshold_matches_expectation);
        assert!(liveops.single_failure_tolerant);
        assert_eq!(report.manifest_match_pass, Some(true));
        assert_eq!(report.overall_status, "ready_for_ops_drill");
        assert!(validate_audit_report(&options, &report).is_empty());
    }
}
