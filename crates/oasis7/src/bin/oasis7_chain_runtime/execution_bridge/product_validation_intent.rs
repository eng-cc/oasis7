use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::write_bytes_atomic;

pub(super) const PRODUCT_VALIDATION_INTENT_SCHEMA_V1: u8 = 1;
const PRODUCT_VALIDATION_INTENT_FILE: &str = "product-validation-intent.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ProductValidationIntentMarkerV1 {
    #[serde(default = "default_schema_version")]
    pub schema_version: u8,
    pub world_id: String,
    pub height: u64,
    #[serde(default)]
    pub action_root: String,
    pub journal_len: usize,
}

fn default_schema_version() -> u8 {
    PRODUCT_VALIDATION_INTENT_SCHEMA_V1
}

pub(super) fn product_validation_intent_path(records_dir: &Path) -> PathBuf {
    records_dir.join(PRODUCT_VALIDATION_INTENT_FILE)
}

pub(super) fn load_product_validation_intent(
    records_dir: &Path,
) -> Result<Option<ProductValidationIntentMarkerV1>, String> {
    let path = product_validation_intent_path(records_dir);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path.as_path()).map_err(|err| {
        format!(
            "read product validation intent marker {} failed: {}",
            path.display(),
            err
        )
    })?;
    let marker = serde_json::from_slice::<ProductValidationIntentMarkerV1>(bytes.as_slice())
        .map_err(|err| {
            format!(
                "parse product validation intent marker {} failed: {}",
                path.display(),
                err
            )
        })?;
    if marker.schema_version != PRODUCT_VALIDATION_INTENT_SCHEMA_V1
        || marker.world_id.trim().is_empty()
        || marker.height == 0
        || marker.journal_len == 0
    {
        return Err(format!(
            "invalid product validation intent marker {}",
            path.display()
        ));
    }
    Ok(Some(marker))
}

pub(super) fn persist_product_validation_intent(
    records_dir: &Path,
    marker: &ProductValidationIntentMarkerV1,
) -> Result<(), String> {
    fs::create_dir_all(records_dir).map_err(|err| {
        format!(
            "create product validation intent records dir {} failed: {}",
            records_dir.display(),
            err
        )
    })?;
    let bytes = serde_json::to_vec_pretty(marker)
        .map_err(|err| format!("serialize product validation intent marker failed: {err}"))?;
    write_bytes_atomic(
        product_validation_intent_path(records_dir).as_path(),
        bytes.as_slice(),
    )
}

pub(super) fn clear_product_validation_intent(records_dir: &Path) -> Result<(), String> {
    let path = product_validation_intent_path(records_dir);
    match fs::remove_file(path.as_path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "remove product validation intent marker {} failed: {}",
            path.display(),
            err
        )),
    }
}
