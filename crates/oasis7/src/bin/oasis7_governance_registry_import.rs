use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

use oasis7::runtime::{
    GovernanceFinalitySignerRegistry, GovernanceMainTokenControllerRegistry,
    GovernanceThresholdSignerPolicy, World, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE, MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
};
use serde::{Deserialize, Serialize};

const DEFAULT_FINALITY_SLOT_ID: &str = "governance.finality.v1";
const DEFAULT_GENESIS_CONTROLLER_ACCOUNT_ID: &str = "msig.genesis.v1";
const DEFAULT_CONTROLLER_THRESHOLD: u16 = 2;
const DEFAULT_STAKING_CONTROLLER_ACCOUNT_ID: &str = "msig.staking_governance.v1";
const DEFAULT_ECOSYSTEM_CONTROLLER_ACCOUNT_ID: &str = "msig.ecosystem_governance.v1";
const DEFAULT_SECURITY_CONTROLLER_ACCOUNT_ID: &str = "msig.security_council.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    world_dir: PathBuf,
    public_manifest: PathBuf,
    finality_slot_id: String,
    controller_threshold: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PublicManifestEntry {
    slot_id: String,
    signer_id: String,
    scheme: String,
    public_key_hex: String,
    #[serde(default)]
    awt_account_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ImportSummary {
    world_dir: String,
    public_manifest: String,
    imported_finality_slot_id: String,
    finality_signer_count: usize,
    controller_policy_count: usize,
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

    let summary = match run_import(options) {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("oasis7_governance_registry_import failed: {err}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("encode import summary")
    );
}

fn run_import(options: CliOptions) -> Result<ImportSummary, String> {
    let manifest_bytes = std::fs::read(options.public_manifest.as_path()).map_err(|err| {
        format!(
            "read public manifest {} failed: {err}",
            options.public_manifest.display()
        )
    })?;
    let entries: Vec<PublicManifestEntry> = serde_json::from_slice(manifest_bytes.as_slice())
        .map_err(|err| {
            format!(
                "decode public manifest {} failed: {err}",
                options.public_manifest.display()
            )
        })?;
    let mut world = load_or_create_world(options.world_dir.as_path())?;
    let finality_registry = build_finality_registry(
        entries.as_slice(),
        options.finality_slot_id.as_str(),
        options.controller_threshold,
    )?;
    let controller_registry = build_main_token_controller_registry(
        entries.as_slice(),
        options.finality_slot_id.as_str(),
        options.controller_threshold,
    )?;
    world
        .set_governance_finality_signer_registry(finality_registry.clone())
        .map_err(|err| format!("write finality registry failed: {err:?}"))?;
    world
        .set_governance_main_token_controller_registry(controller_registry.clone())
        .map_err(|err| format!("write controller registry failed: {err:?}"))?;
    world
        .save_to_dir(options.world_dir.as_path())
        .map_err(|err| format!("save world {} failed: {err:?}", options.world_dir.display()))?;
    Ok(ImportSummary {
        world_dir: options.world_dir.display().to_string(),
        public_manifest: options.public_manifest.display().to_string(),
        imported_finality_slot_id: finality_registry.slot_id,
        finality_signer_count: finality_registry.signer_bindings.len(),
        controller_policy_count: controller_registry.controller_signer_policies.len(),
    })
}

fn load_or_create_world(world_dir: &Path) -> Result<World, String> {
    let snapshot_path = world_dir.join("snapshot.json");
    let journal_path = world_dir.join("journal.json");
    if !snapshot_path.exists() || !journal_path.exists() {
        return Ok(World::new());
    }
    World::load_from_dir(world_dir)
        .map_err(|err| format!("load world {} failed: {err:?}", world_dir.display()))
}

fn build_finality_registry(
    entries: &[PublicManifestEntry],
    finality_slot_id: &str,
    threshold: u16,
) -> Result<GovernanceFinalitySignerRegistry, String> {
    let mut signer_bindings = BTreeMap::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.slot_id == finality_slot_id)
    {
        validate_manifest_entry(entry)?;
        let signer_id = entry.signer_id.trim();
        if signer_id.is_empty() {
            return Err(format!(
                "finality manifest entry has empty signer_id slot_id={finality_slot_id}"
            ));
        }
        signer_bindings.insert(
            format!("{finality_slot_id}.{signer_id}"),
            entry.public_key_hex.trim().to_string(),
        );
    }
    if signer_bindings.is_empty() {
        return Err(format!(
            "public manifest does not contain finality slot {finality_slot_id}"
        ));
    }
    Ok(GovernanceFinalitySignerRegistry {
        slot_id: finality_slot_id.to_string(),
        threshold,
        threshold_bps: 0,
        signer_bindings,
    })
}

fn build_main_token_controller_registry(
    entries: &[PublicManifestEntry],
    finality_slot_id: &str,
    threshold: u16,
) -> Result<GovernanceMainTokenControllerRegistry, String> {
    let mut controller_signer_policies = BTreeMap::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.slot_id != finality_slot_id)
    {
        validate_manifest_entry(entry)?;
        controller_signer_policies
            .entry(entry.slot_id.trim().to_string())
            .or_insert_with(|| GovernanceThresholdSignerPolicy {
                threshold,
                allowed_public_keys: BTreeSet::new(),
            })
            .allowed_public_keys
            .insert(entry.public_key_hex.trim().to_string());
    }
    if controller_signer_policies.is_empty() {
        return Err("public manifest does not contain any controller signer slots".to_string());
    }
    Ok(GovernanceMainTokenControllerRegistry {
        genesis_controller_account_id: DEFAULT_GENESIS_CONTROLLER_ACCOUNT_ID.to_string(),
        treasury_bucket_controller_slots: default_treasury_bucket_controller_slots(),
        restricted_starter_claim_admin_account_ids: BTreeSet::new(),
        controller_signer_policies,
    })
}

fn default_treasury_bucket_controller_slots() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
            DEFAULT_STAKING_CONTROLLER_ACCOUNT_ID.to_string(),
        ),
        (
            MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
            DEFAULT_ECOSYSTEM_CONTROLLER_ACCOUNT_ID.to_string(),
        ),
        (
            MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE.to_string(),
            DEFAULT_SECURITY_CONTROLLER_ACCOUNT_ID.to_string(),
        ),
    ])
}

fn validate_manifest_entry(entry: &PublicManifestEntry) -> Result<(), String> {
    if !entry.scheme.trim().eq_ignore_ascii_case("ed25519") {
        return Err(format!(
            "unsupported signer scheme slot_id={} signer_id={} scheme={}",
            entry.slot_id, entry.signer_id, entry.scheme
        ));
    }
    let slot_id = entry.slot_id.trim();
    let public_key_hex = entry.public_key_hex.trim();
    if slot_id.is_empty() || public_key_hex.is_empty() {
        return Err("manifest entry slot_id/public_key_hex cannot be empty".to_string());
    }
    if public_key_hex.len() != 64 || !public_key_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "manifest entry public_key_hex must be 32-byte hex slot_id={} signer_id={}",
            entry.slot_id, entry.signer_id
        ));
    }
    Ok(())
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut world_dir: Option<PathBuf> = None;
    let mut public_manifest: Option<PathBuf> = None;
    let mut finality_slot_id = DEFAULT_FINALITY_SLOT_ID.to_string();
    let mut controller_threshold = DEFAULT_CONTROLLER_THRESHOLD;
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
            "--controller-threshold" => {
                controller_threshold = parse_required_value(&mut iter, "--controller-threshold")?
                    .parse::<u16>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                        "--controller-threshold requires a positive integer".to_string()
                    })?;
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    let world_dir = world_dir.ok_or_else(|| "--world-dir is required".to_string())?;
    let public_manifest =
        public_manifest.ok_or_else(|| "--public-manifest is required".to_string())?;
    Ok(CliOptions {
        world_dir,
        public_manifest,
        finality_slot_id,
        controller_threshold,
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
        "Usage: oasis7_governance_registry_import --world-dir <dir> --public-manifest <file> [--finality-slot-id <slot>] [--controller-threshold <n>]"
    );
}

#[cfg(test)]
mod tests {
    use super::{parse_options, run_import};
    use oasis7::runtime::World;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-governance-import-{prefix}-{unique}"))
    }

    #[test]
    fn parse_options_accepts_required_flags() {
        let options = parse_options(
            [
                "--world-dir",
                "output/world",
                "--public-manifest",
                "manifest.json",
            ]
            .into_iter(),
        )
        .expect("parse options");
        assert_eq!(options.finality_slot_id, "governance.finality.v1");
        assert_eq!(options.controller_threshold, 2);
    }

    #[test]
    fn import_writes_governance_registries_into_world() {
        let temp_dir = temp_dir("import");
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let manifest_path = temp_dir.join("public_manifest.json");
        std::fs::write(
            manifest_path.as_path(),
            serde_json::to_vec_pretty(&vec![
                serde_json::json!({
                    "slot_id": "governance.finality.v1",
                    "signer_id": "signer01",
                    "scheme": "ed25519",
                    "public_key_hex": "54e7a02919fff2d49a9c325def8cb0211ea7f7a75a9011b9d0678b9e2a7af6bc"
                }),
                serde_json::json!({
                    "slot_id": "governance.finality.v1",
                    "signer_id": "signer02",
                    "scheme": "ed25519",
                    "public_key_hex": "38dac17ff403cc19de033e47be7cf7b5354635fbc5c1976d7c532e20494aace4"
                }),
                serde_json::json!({
                    "slot_id": "governance.finality.v1",
                    "signer_id": "signer03",
                    "scheme": "ed25519",
                    "public_key_hex": "e22bd5029176296712fb1a477f91c15775e5ab858181cb4172839ced526f12c8"
                }),
                serde_json::json!({
                    "slot_id": "msig.genesis.v1",
                    "signer_id": "signer01",
                    "scheme": "ed25519",
                    "public_key_hex": "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                }),
                serde_json::json!({
                    "slot_id": "msig.genesis.v1",
                    "signer_id": "signer02",
                    "scheme": "ed25519",
                    "public_key_hex": "7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"
                }),
                serde_json::json!({
                    "slot_id": "msig.staking_governance.v1",
                    "signer_id": "signer01",
                    "scheme": "ed25519",
                    "public_key_hex": "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                }),
                serde_json::json!({
                    "slot_id": "msig.staking_governance.v1",
                    "signer_id": "signer02",
                    "scheme": "ed25519",
                    "public_key_hex": "10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"
                }),
                serde_json::json!({
                    "slot_id": "msig.ecosystem_governance.v1",
                    "signer_id": "signer01",
                    "scheme": "ed25519",
                    "public_key_hex": "0241f2e23305407676f2a5cec6d154da74944b2a366b2b2b6913cb746d402d0e"
                }),
                serde_json::json!({
                    "slot_id": "msig.ecosystem_governance.v1",
                    "signer_id": "signer02",
                    "scheme": "ed25519",
                    "public_key_hex": "960137cd5d675a517daed5f14ea6bea460e196fda4310a581ecd448f3bcd20b4"
                }),
                serde_json::json!({
                    "slot_id": "msig.security_council.v1",
                    "signer_id": "signer01",
                    "scheme": "ed25519",
                    "public_key_hex": "d09de9413371ae42f643e4f8f31e2139611d1617809375b1ad884df3fb089448"
                }),
                serde_json::json!({
                    "slot_id": "msig.security_council.v1",
                    "signer_id": "signer02",
                    "scheme": "ed25519",
                    "public_key_hex": "aa738a832b0d3bf371d231a0bd8502fd411f2a9723246e5d7d215e8fb0ecbb7c"
                })
            ])
            .expect("encode manifest"),
        )
        .expect("write manifest");

        let summary = run_import(super::CliOptions {
            world_dir: temp_dir.join("world"),
            public_manifest: manifest_path,
            finality_slot_id: "governance.finality.v1".to_string(),
            controller_threshold: 2,
        })
        .expect("run import");
        assert_eq!(summary.finality_signer_count, 3);

        let world = World::load_from_dir(temp_dir.join("world")).expect("load world");
        assert!(world.governance_finality_signer_registry().is_some());
        assert!(world.governance_main_token_controller_registry().is_some());
    }
}
