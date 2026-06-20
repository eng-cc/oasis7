use super::super::ModuleInstallTarget;
use super::super::world_model::{InstalledModuleState, ModuleArtifactState};
use super::WorldKernel;
use super::types::{RejectReason, WorldEventKind};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

impl WorldKernel {
    pub(super) fn apply_compile_module_artifact_from_source(
        &mut self,
        publisher_agent_id: String,
        module_id: String,
        manifest_path: String,
        source_files: BTreeMap<String, Vec<u8>>,
    ) -> WorldEventKind {
        if !self.model.agents.contains_key(&publisher_agent_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound {
                    agent_id: publisher_agent_id,
                },
            };
        }

        let module_id = module_id.trim().to_string();
        if module_id.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "compile module source rejected: module_id cannot be empty".to_string(),
                    ],
                },
            };
        }

        let manifest_path = manifest_path.trim().to_string();
        if manifest_path.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "compile module source rejected: manifest_path cannot be empty".to_string(),
                    ],
                },
            };
        }

        if source_files.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "compile module source rejected: source_files cannot be empty".to_string(),
                    ],
                },
            };
        }

        let compiled_bytes =
            match compile_module_source_artifact(module_id.as_str(), manifest_path, source_files) {
                Ok(bytes) => bytes,
                Err(message) => {
                    return WorldEventKind::ActionRejected {
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("compile module source rejected: {message}")],
                        },
                    };
                }
            };
        let wasm_hash = sha256_hex(compiled_bytes.as_slice());

        self.apply_deploy_module_artifact_internal(
            publisher_agent_id,
            wasm_hash,
            compiled_bytes,
            Some(module_id),
        )
    }

    pub(super) fn apply_deploy_module_artifact(
        &mut self,
        publisher_agent_id: String,
        wasm_hash: String,
        wasm_bytes: Vec<u8>,
        module_id_hint: Option<String>,
    ) -> WorldEventKind {
        self.apply_deploy_module_artifact_internal(
            publisher_agent_id,
            wasm_hash,
            wasm_bytes,
            module_id_hint,
        )
    }

    pub(super) fn apply_install_module_from_artifact(
        &mut self,
        installer_agent_id: String,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        activate: bool,
    ) -> WorldEventKind {
        self.apply_install_module_from_artifact_to_target(
            installer_agent_id,
            module_id,
            module_version,
            wasm_hash,
            activate,
            ModuleInstallTarget::SelfAgent,
        )
    }

    pub(super) fn apply_install_module_to_target_from_artifact(
        &mut self,
        installer_agent_id: String,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        activate: bool,
        install_target: ModuleInstallTarget,
    ) -> WorldEventKind {
        self.apply_install_module_from_artifact_to_target(
            installer_agent_id,
            module_id,
            module_version,
            wasm_hash,
            activate,
            install_target,
        )
    }

    fn apply_install_module_from_artifact_to_target(
        &mut self,
        installer_agent_id: String,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        activate: bool,
        install_target: ModuleInstallTarget,
    ) -> WorldEventKind {
        if !self.model.agents.contains_key(&installer_agent_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound {
                    agent_id: installer_agent_id,
                },
            };
        }
        let installer_location_id = self
            .model
            .agents
            .get(&installer_agent_id)
            .map(|agent| agent.location_id.clone())
            .unwrap_or_default();

        let module_id = module_id.trim().to_string();
        if module_id.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec!["install module rejected: module_id cannot be empty".to_string()],
                },
            };
        }

        let module_version = module_version.trim().to_string();
        if module_version.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "install module rejected: module_version cannot be empty".to_string(),
                    ],
                },
            };
        }

        let wasm_hash = wasm_hash.trim().to_string();
        if wasm_hash.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec!["install module rejected: wasm_hash cannot be empty".to_string()],
                },
            };
        }

        let Some(artifact) = self.model.module_artifacts.get(&wasm_hash) else {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "install module rejected: module artifact missing for hash {wasm_hash}"
                    )],
                },
            };
        };

        if artifact.publisher_agent_id != installer_agent_id {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "install module rejected: installer {} is not artifact owner {}",
                        installer_agent_id, artifact.publisher_agent_id
                    )],
                },
            };
        }

        let install_target = match install_target {
            ModuleInstallTarget::SelfAgent => ModuleInstallTarget::SelfAgent,
            ModuleInstallTarget::LocationInfrastructure { location_id } => {
                let location_id = location_id.trim().to_string();
                if location_id.is_empty() {
                    return WorldEventKind::ActionRejected {
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "install module rejected: location infrastructure target requires non-empty location_id".to_string(),
                            ],
                        },
                    };
                }
                if !self.model.locations.contains_key(&location_id) {
                    return WorldEventKind::ActionRejected {
                        reason: RejectReason::LocationNotFound { location_id },
                    };
                }
                if installer_location_id != location_id {
                    return WorldEventKind::ActionRejected {
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "install module rejected: installer {} is at {} but target infrastructure is {}",
                                installer_agent_id, installer_location_id, location_id
                            )],
                        },
                    };
                }
                ModuleInstallTarget::LocationInfrastructure { location_id }
            }
        };

        self.model.installed_modules.insert(
            module_id.clone(),
            InstalledModuleState {
                module_id: module_id.clone(),
                module_version: module_version.clone(),
                wasm_hash: wasm_hash.clone(),
                installer_agent_id: installer_agent_id.clone(),
                install_target: install_target.clone(),
                active: activate,
                installed_at_tick: self.time,
            },
        );

        WorldEventKind::ModuleInstalled {
            installer_agent_id,
            module_id,
            module_version,
            wasm_hash,
            install_target,
            active: activate,
        }
    }

    fn apply_deploy_module_artifact_internal(
        &mut self,
        publisher_agent_id: String,
        wasm_hash: String,
        wasm_bytes: Vec<u8>,
        module_id_hint: Option<String>,
    ) -> WorldEventKind {
        if !self.model.agents.contains_key(&publisher_agent_id) {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::AgentNotFound {
                    agent_id: publisher_agent_id,
                },
            };
        }

        let wasm_hash = wasm_hash.trim().to_string();
        if wasm_hash.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "deploy module artifact rejected: wasm_hash cannot be empty".to_string(),
                    ],
                },
            };
        }
        if wasm_bytes.is_empty() {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "deploy module artifact rejected: wasm_bytes cannot be empty".to_string(),
                    ],
                },
            };
        }

        let computed_hash = sha256_hex(wasm_bytes.as_slice());
        if computed_hash != wasm_hash {
            return WorldEventKind::ActionRejected {
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "deploy module artifact rejected: hash mismatch expected={wasm_hash} got={computed_hash}"
                    )],
                },
            };
        }

        if let Some(existing) = self.model.module_artifacts.get(&wasm_hash) {
            if existing.wasm_bytes != wasm_bytes {
                return WorldEventKind::ActionRejected {
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "deploy module artifact rejected: hash {} already exists with different bytes",
                            wasm_hash
                        )],
                    },
                };
            }
        }

        self.model.module_artifacts.insert(
            wasm_hash.clone(),
            ModuleArtifactState {
                wasm_hash: wasm_hash.clone(),
                publisher_agent_id: publisher_agent_id.clone(),
                module_id_hint: module_id_hint.clone(),
                wasm_bytes: wasm_bytes.clone(),
                deployed_at_tick: self.time,
            },
        );

        WorldEventKind::ModuleArtifactDeployed {
            publisher_agent_id,
            wasm_hash,
            bytes_len: wasm_bytes.len() as u64,
            wasm_bytes,
            module_id_hint,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn compile_module_source_artifact(
    module_id: &str,
    manifest_path: String,
    source_files: BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let source_package = crate::runtime::ModuleSourcePackage {
        manifest_path,
        files: source_files,
    };
    crate::runtime::compile_module_artifact_from_source(module_id, &source_package)
        .map_err(|err| format!("{err:?}"))
}

#[cfg(target_arch = "wasm32")]
fn compile_module_source_artifact(
    _module_id: &str,
    _manifest_path: String,
    _source_files: BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    Err("source compilation is unavailable on wasm32 target".to_string())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
