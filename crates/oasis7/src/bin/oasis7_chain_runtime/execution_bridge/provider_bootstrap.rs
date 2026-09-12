use std::fs;
use std::path::PathBuf;

use oasis7::runtime::{ProviderBackedBootstrapAuthorityV1, World as RuntimeWorld};

use super::driver::NodeRuntimeExecutionDriver;

impl NodeRuntimeExecutionDriver {
    fn execution_world_without_persistence(&self) -> Result<RuntimeWorld, String> {
        let snapshot = self.execution_world.snapshot();
        let has_inline_module_artifacts = !snapshot.module_artifact_bytes.is_empty();
        let mut world = RuntimeWorld::from_snapshot(
            snapshot,
            self.execution_world.journal().clone(),
        )
        .map_err(|error| {
            format!(
                "clone execution world for non-persistent ProviderBacked bootstrap failed: {error:?}"
            )
        })?
        .with_release_security_policy(self.execution_world.release_security_policy().clone());
        if !has_inline_module_artifacts {
            world
                .load_module_store_from_dir(self.world_dir.as_path())
                .map_err(|error| {
                    format!(
                        "load execution module store for ProviderBacked bootstrap failed: {error:?}"
                    )
                })?;
        }
        Ok(world)
    }

    pub(crate) fn stage_provider_backed_bootstrap_authorities(
        &mut self,
        authorities: Vec<ProviderBackedBootstrapAuthorityV1>,
    ) -> Result<(), String> {
        if authorities.is_empty() {
            return Ok(());
        }
        if let Some(existing) = self.pending_provider_backed_bootstrap.as_ref() {
            if existing == &authorities {
                return Ok(());
            }
            return Err(
                "ProviderBacked authority bootstrap already staged with different input"
                    .to_string(),
            );
        }

        // Validate the complete batch against a detached world. This keeps
        // malformed or mismatched authority input fail-closed at startup
        // without writing the execution world before a canonical commit.
        let mut validation_world = self.execution_world_without_persistence()?;
        validation_world
            .bootstrap_provider_backed_authorities(authorities.as_slice())
            .map_err(|error| {
                format!(
                    "validate ProviderBacked authority bootstrap for canonical execution commit failed: {error:?}"
                )
            })?;
        self.pending_provider_backed_bootstrap = Some(authorities);
        Ok(())
    }

    pub(super) fn apply_pending_provider_backed_bootstrap(&mut self) -> Result<(), String> {
        let Some(authorities) = self.pending_provider_backed_bootstrap.as_ref() else {
            return Ok(());
        };
        let mut staged_world = self.execution_world_without_persistence()?;
        staged_world
            .bootstrap_provider_backed_authorities(authorities.as_slice())
            .map_err(|error| {
                format!(
                    "apply ProviderBacked authority bootstrap in canonical execution commit failed: {error:?}"
                )
            })?;
        self.execution_world = staged_world;
        Ok(())
    }
}

/// Load explicit authority bundles before entering the chain runtime loop.
/// The input is entirely caller supplied; an empty list leaves the writer
/// unchanged and no authority or allowance is synthesized.
fn load_provider_backed_bootstrap_authorities(
    paths: &[PathBuf],
) -> Result<Vec<ProviderBackedBootstrapAuthorityV1>, String> {
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|error| {
                format!(
                    "failed to read ProviderBacked authority bundle {}: {error}",
                    path.display()
                )
            })?;
            serde_json::from_slice::<ProviderBackedBootstrapAuthorityV1>(&bytes).map_err(|error| {
                format!(
                    "failed to decode ProviderBacked authority bundle {}: {error}",
                    path.display()
                )
            })
        })
        .collect()
}

/// Stage explicit ProviderBacked authority bundles for the next canonical
/// execution commit. The live world writer lock is held by the chain runtime
/// before this function is called. Runtime validates the whole batch without
/// persistence here; the execution bridge applies and publishes it together
/// with the next per-height commit/replay record.
pub(crate) fn publish_provider_backed_bootstrap_from_paths(
    driver: &mut NodeRuntimeExecutionDriver,
    paths: &[PathBuf],
) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let authorities = load_provider_backed_bootstrap_authorities(paths)?;
    driver.stage_provider_backed_bootstrap_authorities(authorities)
}
