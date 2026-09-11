use std::fs;
use std::path::PathBuf;

use oasis7::runtime::ProviderBackedBootstrapAuthorityV1;

use super::driver::NodeRuntimeExecutionDriver;

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

/// Publish explicit ProviderBacked authority bundles through the chain
/// execution world's Runtime transaction boundary. The live world writer lock
/// is held by the chain runtime before this function is called. Runtime stages
/// the whole batch, persists it atomically, and makes exact replay idempotent.
pub(crate) fn publish_provider_backed_bootstrap_from_paths(
    driver: &mut NodeRuntimeExecutionDriver,
    paths: &[PathBuf],
) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let authorities = load_provider_backed_bootstrap_authorities(paths)?;
    driver
        .execution_world
        .bootstrap_provider_backed_authorities(authorities.as_slice())
        .map(|_| ())
        .map_err(|error| {
            format!(
                "publish ProviderBacked authority bundles through chain Runtime failed: {error:?}"
            )
        })
}
