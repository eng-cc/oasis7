use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use oasis7::runtime::{
    ChainResourceDerivationContext, LocalTestProviderAuthorityConfig,
    LocalTestProviderModuleArtifact, LocalTestProviderSessionMode,
    ProviderBackedBootstrapAuthorityV1, ReleaseSecurityPolicy, World as RuntimeWorld,
};
use oasis7::viewer::viewer_bootstrap_formal_release_runtime_world;
use serde::Deserialize;

use super::cli::CliOptions;

#[derive(Debug, Deserialize)]
struct LocalTestProviderBuildMetadata {
    source_hash: String,
    build_manifest_hash: String,
}

/// Prepare the explicit DevLocal provider authority before the execution
/// driver opens the world. The writer owns this once-only transition; the
/// viewer remains an observer and consumes the resulting JSON bundle.
pub(super) fn ensure_local_test_provider_authority(
    options: &CliOptions,
    world_dir: &Path,
) -> Result<(), String> {
    let authority_path = options
        .local_test_provider_authority_path
        .as_deref()
        .ok_or_else(|| "local test authority output path is missing".to_string())?;
    let wasm_path = options
        .local_test_provider_wasm_path
        .as_deref()
        .ok_or_else(|| "local test WASM artifact path is missing".to_string())?;
    let metadata_path = options
        .local_test_provider_metadata_path
        .as_deref()
        .ok_or_else(|| "local test build metadata path is missing".to_string())?;
    let finality_block_hash = options
        .local_test_provider_finality_block_hash
        .as_deref()
        .ok_or_else(|| "local test finality marker is missing".to_string())?;

    let snapshot_path = world_dir.join("snapshot.json");
    let journal_path = world_dir.join("journal.json");
    let snapshot_exists = snapshot_path.is_file();
    let journal_exists = journal_path.is_file();
    if snapshot_exists != journal_exists {
        return Err(format!(
            "local test authority setup refuses partial execution world persistence: {} and {}",
            snapshot_path.display(),
            journal_path.display()
        ));
    }

    if snapshot_exists {
        let world = RuntimeWorld::load_from_dir(world_dir).map_err(|error| {
            format!(
                "load existing execution world for local test authority setup failed: {error:?}"
            )
        })?;
        let authority_bytes = fs::read(authority_path).map_err(|error| {
            format!(
                "existing execution world requires local test authority bundle {}: {error}",
                authority_path.display()
            )
        })?;
        let authority = serde_json::from_slice::<ProviderBackedBootstrapAuthorityV1>(
            authority_bytes.as_slice(),
        )
        .map_err(|error| {
            format!(
                "decode existing local test authority bundle {} failed: {error}",
                authority_path.display()
            )
        })?;
        let binding = world.current_cognition_runtime_binding().map_err(|error| {
            format!(
                "existing execution world is missing the local Runtime binding; choose a fresh runtime root: {error:?}"
            )
        })?;
        if authority.world_id != binding.world_id
            || authority.branch_id != binding.branch_id
            || authority.reorg_epoch != binding.reorg_epoch
            || authority.agent_id != options.local_test_provider_agent_id
            || authority.owner_binding != options.local_test_provider_owner_binding
            || authority.authority_record.finality_block_hash != finality_block_hash
        {
            return Err(format!(
                "existing local test authority bundle {} does not match the requested world/session binding",
                authority_path.display()
            ));
        }
        world
            .validate_local_test_main_token_funding(
                options.local_test_provider_agent_id.as_str(),
            )
            .map_err(|error| {
                format!(
                    "existing local execution world is missing canonical W3 starter funding; choose a fresh runtime root: {error:?}"
                )
            })?;
        return Ok(());
    }

    let wasm_bytes = fs::read(wasm_path).map_err(|error| {
        format!(
            "read local test WASM artifact {} failed: {error}",
            wasm_path.display()
        )
    })?;
    let metadata_bytes = fs::read(metadata_path).map_err(|error| {
        format!(
            "read local test build metadata {} failed: {error}",
            metadata_path.display()
        )
    })?;
    let metadata =
        serde_json::from_slice::<LocalTestProviderBuildMetadata>(metadata_bytes.as_slice())
            .map_err(|error| {
                format!(
                    "decode local test build metadata {} failed: {error}",
                    metadata_path.display()
                )
            })?;
    let mut config = LocalTestProviderAuthorityConfig::hosted_public_join(
        options.local_test_provider_agent_id.clone(),
        options.local_test_provider_owner_binding.clone(),
        finality_block_hash.to_string(),
    );
    if options.local_test_provider_session_mode == "loopback" {
        config.session_mode = LocalTestProviderSessionMode::Loopback;
        config.presenter.session_id =
            Some(format!("loopback:{}", options.local_test_provider_agent_id));
    }
    let artifact = LocalTestProviderModuleArtifact {
        wasm_bytes,
        source_hash: metadata.source_hash,
        build_manifest_hash: metadata.build_manifest_hash,
    };

    let (mut world, _) = viewer_bootstrap_formal_release_runtime_world()
        .map_err(|error| format!("formal starter world bootstrap failed: {error}"))?;
    world = world.with_release_security_policy(ReleaseSecurityPolicy::default());

    // The module publication must precede the immutable cognition binding.
    world
        .install_local_test_provider_module(&config, &artifact)
        .map_err(|error| format!("install local test module failed: {error:?}"))?;
    let finality_epoch = world.state().time
        / world
            .governance_execution_policy()
            .epoch_length_ticks
            .max(1);
    world
        .bind_cognition_runtime(
            options.world_id.clone(),
            "main",
            finality_epoch,
            Some(finality_block_hash.to_string()),
            "pending",
            0,
        )
        .map_err(|error| format!("bind local Runtime cognition authority failed: {error:?}"))?;
    let funding_config = config.clone();
    let provisioning = world
        .initialize_local_test_provider_authority(config, artifact)
        .map_err(|error| {
            format!("initialize local test authority through Runtime admission failed: {error:?}")
        })?;
    world
        .initialize_local_test_main_token_funding(&funding_config)
        .map_err(|error| format!("initialize local W3 starter funding failed: {error:?}"))?;

    let world_config_hash = world.current_manifest_hash().map_err(|error| {
        format!("derive local test world manifest hash before persistence failed: {error:?}")
    })?;
    world
        .save_to_dir_with_chain_resource_context(
            world_dir,
            ChainResourceDerivationContext {
                world_id: options.world_id.as_str(),
                chain_id: options.world_id.as_str(),
                genesis_ref: None,
                created_at_height: 0,
                manifest_height: 0,
                commit_block_hash: None,
                tick: world.state().time,
            },
            world_config_hash,
            "local-test-provider-runtime-v1",
        )
        .map_err(|error| {
            format!(
                "persist local test execution world {} failed: {error:?}",
                world_dir.display()
            )
        })?;
    write_authority_atomically(authority_path, &provisioning.bootstrap_authority)?;
    Ok(())
}

fn write_authority_atomically(
    path: &Path,
    authority: &ProviderBackedBootstrapAuthorityV1,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "create local test authority output directory {} failed: {error}",
            parent.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(authority)
        .map_err(|error| format!("encode local test authority failed: {error}"))?;
    let tmp_path = temporary_authority_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path.as_path())
        .map_err(|error| {
            format!(
                "create temporary local test authority {} failed: {error}",
                tmp_path.display()
            )
        })?;
    if let Err(error) = file
        .write_all(bytes.as_slice())
        .and_then(|_| file.sync_all())
    {
        let _ = fs::remove_file(tmp_path.as_path());
        return Err(format!(
            "write temporary local test authority {} failed: {error}",
            tmp_path.display()
        ));
    }
    drop(file);
    if let Err(error) = fs::rename(tmp_path.as_path(), path) {
        let _ = fs::remove_file(tmp_path.as_path());
        return Err(format!(
            "publish local test authority {} failed: {error}",
            path.display()
        ));
    }
    Ok(())
}

fn temporary_authority_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("provider-authority.json");
    path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliOptions;
    use oasis7::runtime::{Action, DomainEvent, WorldEventBody};

    #[test]
    fn local_setup_persists_real_runtime_order_and_reuses_durable_bundle() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-local-provider-setup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let world_dir = root.join("world");
        let authority_path = root.join("authority.json");
        let wasm_path = root.join("provider.wasm");
        let metadata_path = root.join("provider.metadata.json");
        fs::create_dir_all(&root).expect("create setup root");
        fs::write(&wasm_path, b"real-wasm-artifact").expect("write artifact");
        fs::write(
            &metadata_path,
            serde_json::json!({
                "source_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "build_manifest_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            })
            .to_string(),
        )
        .expect("write metadata");
        let mut options = CliOptions::default();
        options.world_id = "local-provider-test-world".to_string();
        options.local_test_provider_authority_path = Some(authority_path.clone());
        options.local_test_provider_wasm_path = Some(wasm_path);
        options.local_test_provider_metadata_path = Some(metadata_path);
        options.local_test_provider_finality_block_hash =
            Some(format!("blake3:{}", "0".repeat(64)));
        ensure_local_test_provider_authority(&options, world_dir.as_path()).expect("setup");
        let authority_bytes = fs::read(&authority_path).expect("authority output");
        let authority = serde_json::from_slice::<ProviderBackedBootstrapAuthorityV1>(
            authority_bytes.as_slice(),
        )
        .expect("decode authority output");
        let mut world = RuntimeWorld::load_from_dir(&world_dir).expect("load world");
        assert!(world.current_cognition_runtime_binding().is_ok());
        assert!(
            world
                .module_registry()
                .active
                .contains_key("module.runtime.local-test-provider")
        );
        assert_eq!(authority.agent_id, options.local_test_provider_agent_id);
        assert_eq!(
            world.state().time,
            2,
            "funding must preserve baseline tick 2"
        );
        assert_eq!(world.main_token_config().initial_supply, 325);
        assert_eq!(world.main_token_supply().total_supply, 325);
        assert_eq!(world.main_token_supply().circulating_supply, 325);
        assert_eq!(world.main_token_liquid_balance("starter-agent-0"), 325);
        assert_eq!(
            world.main_token_last_claim_nonce("starter-agent-0"),
            Some(1)
        );
        let funding_events = world
            .journal()
            .events
            .iter()
            .filter_map(|event| match &event.body {
                WorldEventBody::Domain(
                    event @ (DomainEvent::MainTokenGenesisInitialized { .. }
                    | DomainEvent::MainTokenVestingClaimed { .. }),
                ) => Some(event),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            funding_events.as_slice(),
            [DomainEvent::MainTokenGenesisInitialized { total_supply: 325, allocations },
                DomainEvent::MainTokenVestingClaimed {
                    bucket_id,
                    beneficiary,
                    amount: 325,
                    nonce: 1,
                }]
            if allocations.len() == 1
                && allocations[0].bucket_id == "dev_local_w3_starter_vesting"
                && allocations[0].ratio_bps == 10_000
                && allocations[0].recipient == "starter-agent-0"
                && allocations[0].cliff_epochs == 0
                && allocations[0].linear_unlock_epochs == 0
                && allocations[0].start_epoch == 0
                && bucket_id == "dev_local_w3_starter_vesting"
                && beneficiary == "starter-agent-0"
        ));
        assert_eq!(
            world
                .cognition_economy()
                .expect("cognition economy")
                .available_balance(
                    options.local_test_provider_owner_binding.as_str(),
                    "cognition_units"
                ),
            128,
            "Builtin cognition allowance is independent from OC"
        );
        world.submit_action(Action::TransferMainToken {
            from_account_id: "starter-agent-0".to_string(),
            to_account_id: "local-test-recipient".to_string(),
            amount: 325,
            nonce: 1,
            asset_id: Some("main_token".to_string()),
            memo: Some("w3-local-post-spend-restart".to_string()),
            chain_id: None,
            network_id: None,
            tx_version: None,
            tx_type: None,
            valid_until_unix_ms: None,
            max_fee: None,
            fee_asset_id: None,
            application_payload_hash: None,
            client_request_id: None,
        });
        world.step().expect("normal post-funding spend");
        world
            .save_to_dir(&world_dir)
            .expect("persist post-spend world");
        let restarted = RuntimeWorld::load_from_dir(&world_dir).expect("restart post-spend world");
        assert_eq!(restarted.main_token_liquid_balance("starter-agent-0"), 0);
        assert_eq!(
            restarted.main_token_liquid_balance("local-test-recipient"),
            325
        );
        let journal_len_after_spend = restarted.journal().events.len();
        ensure_local_test_provider_authority(&options, world_dir.as_path())
            .expect("durable setup should be idempotent");
        let reused = RuntimeWorld::load_from_dir(&world_dir).expect("reload reused world");
        assert_eq!(reused.journal().events.len(), journal_len_after_spend);
        assert_eq!(reused.main_token_liquid_balance("starter-agent-0"), 0);
        assert_eq!(
            reused.main_token_liquid_balance("local-test-recipient"),
            325
        );
        let mut mismatched_options = options.clone();
        mismatched_options.local_test_provider_finality_block_hash =
            Some(format!("blake3:{}", "1".repeat(64)));
        assert!(
            ensure_local_test_provider_authority(&mismatched_options, world_dir.as_path())
                .expect_err("changed finality must not reuse durable authority")
                .contains("does not match")
        );
        let _ = fs::remove_dir_all(root);
    }
}
