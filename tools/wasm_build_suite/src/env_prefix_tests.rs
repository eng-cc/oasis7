use super::*;
use std::env;

struct EnvVarGuard {
    key: String,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn capture(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            previous: env::var(&key).ok(),
            key,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = self.previous.take() {
            env::set_var(&self.key, value);
        } else {
            env::remove_var(&self.key);
        }
    }
}

#[test]
fn compile_time_guard_rejects_workspace_build_script_target() {
    let _guard = EnvVarGuard::capture(wasm_env_key("VALIDATE_WORKSPACE_COMPILETIME"));
    let removed_old_brand_key = removed_old_brand_wasm_env("VALIDATE_WORKSPACE_COMPILETIME");
    let _removed_old_brand_guard = EnvVarGuard::capture(removed_old_brand_key.as_str());
    env::set_var(wasm_env_key("VALIDATE_WORKSPACE_COMPILETIME"), "1");
    env::remove_var(removed_old_brand_key.as_str());

    let metadata = CargoMetadata {
        workspace_root: "/workspace".to_string(),
        target_directory: "/workspace/target".to_string(),
        packages: vec![CargoPackage {
            name: "m1_rule_move".to_string(),
            source: None,
            manifest_path: "/workspace/crates/m1_rule_move/Cargo.toml".to_string(),
            targets: vec![
                CargoTarget {
                    name: "m1_rule_move".to_string(),
                    kind: vec!["cdylib".to_string()],
                },
                CargoTarget {
                    name: "build_script_build".to_string(),
                    kind: vec!["custom-build".to_string()],
                },
            ],
        }],
    };
    let manifest_path = std::path::Path::new("/workspace/crates/m1_rule_move/Cargo.toml");

    let error = validate_workspace_compile_time_determinism(&metadata, manifest_path)
        .expect_err("expected workspace build.rs target to be rejected");
    assert!(matches!(error, BuildError::MetadataInvalid(_)));
}

#[test]
fn compile_time_guard_allows_external_proc_macro_package() {
    let _guard = EnvVarGuard::capture(wasm_env_key("VALIDATE_WORKSPACE_COMPILETIME"));
    let removed_old_brand_key = removed_old_brand_wasm_env("VALIDATE_WORKSPACE_COMPILETIME");
    let _removed_old_brand_guard = EnvVarGuard::capture(removed_old_brand_key.as_str());
    env::set_var(wasm_env_key("VALIDATE_WORKSPACE_COMPILETIME"), "1");
    env::remove_var(removed_old_brand_key.as_str());

    let metadata = CargoMetadata {
        workspace_root: "/workspace".to_string(),
        target_directory: "/workspace/target".to_string(),
        packages: vec![CargoPackage {
            name: "serde_derive".to_string(),
            source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
            manifest_path: "/cargo/registry/src/serde_derive/Cargo.toml".to_string(),
            targets: vec![CargoTarget {
                name: "serde_derive".to_string(),
                kind: vec!["proc-macro".to_string()],
            }],
        }],
    };
    let manifest_path = std::path::Path::new("/workspace/crates/m1_rule_move/Cargo.toml");

    validate_workspace_compile_time_determinism(&metadata, manifest_path)
        .expect("external dependency proc-macro should be ignored by workspace guard");
}

#[test]
fn wasm_env_value_or_default_reads_oasis7_prefix() {
    let _primary = EnvVarGuard::capture(wasm_env_key("BUILD_STD"));
    let removed_old_brand_key = removed_old_brand_wasm_env("BUILD_STD");
    let _removed_old_brand = EnvVarGuard::capture(removed_old_brand_key.as_str());
    env::set_var(wasm_env_key("BUILD_STD"), "1");
    env::set_var(removed_old_brand_key.as_str(), "0");

    assert_eq!(wasm_env_value_or_default("BUILD_STD", "missing"), "1");
}

#[test]
fn wasm_env_value_or_default_rejects_removed_old_brand_prefix() {
    let _primary = EnvVarGuard::capture(wasm_env_key("BUILD_STD_COMPONENTS"));
    let removed_old_brand_key = removed_old_brand_wasm_env("BUILD_STD_COMPONENTS");
    let _removed_old_brand = EnvVarGuard::capture(removed_old_brand_key.as_str());
    env::remove_var(wasm_env_key("BUILD_STD_COMPONENTS"));
    env::set_var(removed_old_brand_key.as_str(), "std,panic_abort");

    assert_eq!(
        wasm_env_value_or_default("BUILD_STD_COMPONENTS", "missing"),
        "missing"
    );
}

fn removed_old_brand_wasm_env(suffix: &str) -> String {
    ["AGENT", "WORLD", "WASM", suffix].join("_")
}
