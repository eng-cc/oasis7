use crate::{
    error::{ContextError, Result},
    format::strict_json,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalConfig {
    pub schema_version: String,
    pub default_root: Option<PathBuf>,
    pub editor: Option<Editor>,
    pub verification_max_age_days: Option<u32>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    pub program: PathBuf,
    pub args: Vec<String>,
}
pub fn load() -> Result<LocalConfig> {
    let base =
        if let Some(p) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(p)
        } else {
            PathBuf::from(std::env::var_os("HOME").ok_or_else(|| {
                ContextError::new("CONTEXT_NOT_CONFIGURED", "HOME is unavailable")
            })?)
            .join(".config")
        };
    if !base.is_absolute() {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "configuration path must be absolute",
        ));
    }
    let path = base.join("oasis7/context.json");
    let parent = path.parent().unwrap();
    let directory = match crate::fs::Directory::open(parent) {
        Ok(d) => d,
        Err(_e) if !parent.exists() => return Ok(LocalConfig::default()),
        Err(e) => return Err(e),
    };
    let bytes = match directory.read("context.json", crate::model::MAX_METADATA_BYTES) {
        Ok(b) => b,
        Err(e) if e.code == "ENTRY_NOT_FOUND" => return Ok(LocalConfig::default()),
        Err(e) => return Err(e),
    };
    let c: LocalConfig = strict_json(&bytes)?;
    if c.schema_version != "oasis7.context_local_config.v1"
        || c.default_root.as_ref().is_some_and(|p| !p.is_absolute())
        || c.verification_max_age_days
            .is_some_and(|n| !(1..=3650).contains(&n))
        || c.editor
            .as_ref()
            .is_some_and(|e| !e.program.is_absolute() || e.args.iter().any(|x| x.contains('\0')))
    {
        return Err(ContextError::new(
            "INVALID_SCHEMA",
            "invalid local configuration",
        ));
    }
    Ok(c)
}
pub fn select_root(explicit: Option<&Path>, config: &LocalConfig) -> Result<PathBuf> {
    let root = explicit
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("OASIS7_CONTEXT_DIR").map(PathBuf::from))
        .or_else(|| config.default_root.clone())
        .ok_or_else(|| {
            ContextError::new(
                "CONTEXT_NOT_CONFIGURED",
                "no private context root configured",
            )
        })?;
    if !root.is_absolute() {
        return Err(ContextError::new("UNSAFE_PATH", "root must be absolute"));
    }
    Ok(root)
}
