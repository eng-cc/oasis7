//! Finite private-library IPC. Each worker opens its own connection; snapshots
//! never hold the ordinary project/index mutex.
use oasis7_command_center_core::context::*;
use std::{path::PathBuf, sync::Arc};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Semaphore;

pub struct ContextHost {
    path: PathBuf,
    slots: Arc<Semaphore>,
}
impl ContextHost {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        ContextService::open(&path).map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            slots: Arc::new(Semaphore::new(4)),
        })
    }
    pub(crate) async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(ContextService) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "资料库正在处理请求，请稍后重试".to_string())?;
        let path = self.path.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            work(ContextService::open(path).map_err(|e| e.to_string())?)
        })
        .await
        .map_err(|_| "资料请求未能完成".to_string())?
    }
}
#[tauri::command]
pub async fn context_bindings(host: State<'_, ContextHost>) -> Result<Vec<ContextBinding>, String> {
    host.run(|service| service.bindings().map_err(|e| e.to_string()))
        .await
}
#[tauri::command]
pub async fn context_choose_library(
    host: State<'_, ContextHost>,
    project: State<'_, crate::commands::Host>,
    handle: tauri::AppHandle,
    create: bool,
    name: String,
    operation_id: String,
    tools: State<'_, crate::tool_commands::ToolHost>,
) -> Result<Option<ContextBinding>, String> {
    let project_id = project
        .project
        .lock()
        .map_err(|_| "项目身份不可用")?
        .as_ref()
        .map(|p| p.id.clone())
        .ok_or("请先选择项目")?;
    let selected = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title(if create {
                "选择独立的空资料库目录"
            } else {
                "连接独立的资料库目录"
            })
            .blocking_pick_folder()
    })
    .await
    .map_err(|_| "Directory selection did not complete")?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let selected = selected.into_path().map_err(|e| e.to_string())?;
    let path = host.path.clone();
    let fingerprint = crate::tool_commands::digest(
        &serde_json::to_vec(&(&project_id, &name, create, &selected)).map_err(|e| e.to_string())?,
    );
    let ev_path = selected.clone();
    let ev_project = project_id.clone();
    let evidence = host
        .run(move |service| {
            service
                .binding_evidence(&ev_path, &ev_project, create)
                .map_err(|e| e.to_string())
        })
        .await?;
    let resource = match &evidence {
        ContextRecoveryEvidence::Binding { root, .. } => {
            format!("context:{}", crate::tool_commands::digest(root.as_bytes()))
        }
        _ => unreachable!(),
    };
    tools
        .mutate_context(
            operation_id,
            fingerprint,
            resource,
            "bind",
            evidence,
            move || {
                ContextService::open(path)?
                    .bind(&selected, &project_id, &name, create)
                    .map(Some)
            },
        )
        .await
}
#[tauri::command]
pub async fn context_list(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
    query: String,
    include_archived: bool,
) -> Result<ContextQuery, String> {
    if query.len() > 4096 {
        return Err("搜索文字过长".into());
    }
    host.run(move |s| {
        s.list(&binding_id, binding_version, &query, include_archived)
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn context_get(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
    entry_id: String,
) -> Result<ContextEntry, String> {
    host.run(move |s| {
        s.get_bound(&binding_id, binding_version, &entry_id)
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn context_save(
    host: State<'_, ContextHost>,
    mut request: ContextSaveRequest,
    operation_id: String,
    tools: State<'_, crate::tool_commands::ToolHost>,
) -> Result<serde_json::Value, String> {
    request.actor = "command-center:human".into();
    let path = host.path.clone();
    let fingerprint =
        crate::tool_commands::digest(&serde_json::to_vec(&request).map_err(|e| e.to_string())?);
    let ev_request = request.clone();
    let evidence = host
        .run(move |service| {
            service
                .save_evidence(&ev_request)
                .map_err(|e| e.to_string())
        })
        .await?;
    tools
        .mutate_context(
            operation_id,
            fingerprint,
            format!("context:{}", request.binding_id),
            "save",
            evidence,
            move || ContextService::open(path)?.save(&request),
        )
        .await
}
#[tauri::command]
pub async fn context_archive(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
    entry_id: String,
    revision: String,
    operation_id: String,
    tools: State<'_, crate::tool_commands::ToolHost>,
) -> Result<serde_json::Value, String> {
    let path = host.path.clone();
    let ev_binding = binding_id.clone();
    let ev_entry = entry_id.clone();
    let ev_revision = revision.clone();
    let evidence = host
        .run(move |service| {
            service
                .archive_evidence(&ev_binding, binding_version, &ev_entry, &ev_revision)
                .map_err(|e| e.to_string())
        })
        .await?;
    tools
        .mutate_context(
            operation_id,
            format!("{binding_id}:{binding_version}:{entry_id}:{revision}"),
            format!("context:{binding_id}"),
            "archive",
            evidence,
            move || {
                ContextService::open(path)?.archive(
                    &binding_id,
                    binding_version,
                    &entry_id,
                    &revision,
                    "command-center:human",
                )
            },
        )
        .await
}
#[tauri::command]
pub async fn context_history(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
    entry_id: String,
) -> Result<serde_json::Value, String> {
    host.run(move |s| {
        s.history(&binding_id, binding_version, &entry_id)
            .map_err(|e| e.to_string())
    })
    .await
}
// Tauri injects service/window/app handles separately from the fixed business fields.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn context_restore_entry(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
    entry_id: String,
    commit: String,
    revision: Option<String>,
    operation_id: String,
    tools: State<'_, crate::tool_commands::ToolHost>,
) -> Result<serde_json::Value, String> {
    let path = host.path.clone();
    let ev_binding = binding_id.clone();
    let ev_entry = entry_id.clone();
    let ev_revision = revision.clone();
    let ev_commit = commit.clone();
    let evidence = host
        .run(move |service| {
            service
                .restore_entry_evidence(
                    &ev_binding,
                    binding_version,
                    &ev_entry,
                    &ev_commit,
                    ev_revision.as_deref(),
                )
                .map_err(|e| e.to_string())
        })
        .await?;
    tools
        .mutate_context(
            operation_id,
            format!("{binding_id}:{binding_version}:{entry_id}:{commit}:{revision:?}"),
            format!("context:{binding_id}"),
            "restore_entry",
            evidence,
            move || {
                ContextService::open(path)?.restore_entry(
                    &binding_id,
                    binding_version,
                    &entry_id,
                    &commit,
                    revision.as_deref(),
                    "command-center:human",
                )
            },
        )
        .await
}
#[tauri::command]
pub async fn context_doctor(
    host: State<'_, ContextHost>,
    binding_id: String,
    binding_version: u64,
) -> Result<serde_json::Value, String> {
    host.run(move |s| {
        s.doctor(&binding_id, binding_version)
            .map_err(|e| e.to_string())
    })
    .await
}

// Tauri injects service/window/app handles separately from the fixed business fields.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn context_snapshot_export(
    host: State<'_, ContextHost>,
    tools: State<'_, crate::tool_commands::ToolHost>,
    window: tauri::WebviewWindow,
    handle: tauri::AppHandle,
    binding_id: String,
    binding_version: u64,
    selection: Vec<String>,
    operation_id: String,
    secret_handle: String,
) -> Result<Option<serde_json::Value>, String> {
    let password = tools
        .secrets
        .take(
            &secret_handle,
            window.label(),
            &operation_id,
            "context_snapshot_export",
        )
        .map_err(|e| e.to_string())?;
    let path = host.path.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("保存私有资料加密快照")
            .set_file_name("context-snapshot.enc")
            .blocking_save_file()
    })
    .await
    .map_err(|_| "File selection did not complete")?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let output = selected.into_path().map_err(|e| e.to_string())?;
    let ev_binding = binding_id.clone();
    let ev_selection = selection.clone();
    let ev_output = output.clone();
    let evidence = host
        .run(move |service| {
            service
                .snapshot_export_evidence(&ev_binding, binding_version, &ev_output, &ev_selection)
                .map_err(|e| e.to_string())
        })
        .await?;
    let expected_digest = match &evidence {
        ContextRecoveryEvidence::SnapshotExport { source_digest, .. } => source_digest.clone(),
        _ => unreachable!(),
    };
    let fingerprint =
        crate::tool_commands::digest(&serde_json::to_vec(&evidence).map_err(|e| e.to_string())?);
    tools
        .mutate_context(
            operation_id,
            fingerprint,
            format!("context:{binding_id}"),
            "snapshot_export",
            evidence,
            move || {
                ContextService::open(path)?
                    .snapshot_export_frozen(
                        &binding_id,
                        binding_version,
                        &output,
                        &selection,
                        &password,
                        &expected_digest,
                    )
                    .map(Some)
            },
        )
        .await
}
#[tauri::command]
pub async fn context_snapshot_inspect(
    host: State<'_, ContextHost>,
    tools: State<'_, crate::tool_commands::ToolHost>,
    window: tauri::WebviewWindow,
    handle: tauri::AppHandle,
    operation_id: String,
    secret_handle: String,
) -> Result<Option<serde_json::Value>, String> {
    let password = tools
        .secrets
        .take(
            &secret_handle,
            window.label(),
            &operation_id,
            "context_snapshot_inspect",
        )
        .map_err(|e| e.to_string())?;
    host.run(move |s| {
        let Some(file) = handle
            .dialog()
            .file()
            .set_title("检查私有资料加密快照")
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        s.snapshot_inspect(&file.into_path().map_err(|e| e.to_string())?, &password)
            .map(Some)
            .map_err(|e| e.to_string())
    })
    .await
}
// Tauri injects service/window/app handles separately from the fixed business fields.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn context_snapshot_restore(
    host: State<'_, ContextHost>,
    tools: State<'_, crate::tool_commands::ToolHost>,
    window: tauri::WebviewWindow,
    handle: tauri::AppHandle,
    project: State<'_, crate::commands::Host>,
    binding_id: Option<String>,
    binding_version: Option<u64>,
    operation_id: String,
    secret_handle: String,
    directory_name: String,
) -> Result<Option<ContextBinding>, String> {
    if directory_name.is_empty()
        || directory_name == "."
        || directory_name == ".."
        || directory_name.contains('/')
        || directory_name.contains('\\')
    {
        return Err("A new directory name is required".into());
    }
    let registered_project_id = project
        .project
        .lock()
        .map_err(|_| "Project identity unavailable")?
        .as_ref()
        .map(|p| p.id.clone())
        .ok_or("Select an ordinary project first")?;
    let source = ContextSnapshotRestoreSource {
        binding_id,
        binding_version,
        registered_project_id,
    };
    let password = tools
        .secrets
        .take(
            &secret_handle,
            window.label(),
            &operation_id,
            "context_snapshot_restore",
        )
        .map_err(|e| e.to_string())?;
    let path = host.path.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = handle
            .dialog()
            .file()
            .set_title("选择待恢复的私有资料加密快照")
            .blocking_pick_file()
        else {
            return Ok::<_, String>(None);
        };
        let Some(parent) = handle
            .dialog()
            .file()
            .set_title("选择新资料库目录的父目录")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        Ok(Some((
            file.into_path().map_err(|e| e.to_string())?,
            parent
                .into_path()
                .map_err(|e| e.to_string())?
                .join(directory_name),
        )))
    })
    .await
    .map_err(|_| "File selection did not complete")??;
    let Some((input, target)) = selected else {
        return Ok(None);
    };
    let frozen_input = input.clone();
    let encrypted_digest = tauri::async_runtime::spawn_blocking(move || {
        oasis7_command_center_core::tools::selected_file_digest(&frozen_input, 128 * 1024 * 1024)
            .map(|identity| identity.0)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Snapshot input read did not complete")??;
    let fingerprint = crate::tool_commands::digest(
        &serde_json::to_vec(&(&source, &input, &target, &encrypted_digest))
            .map_err(|e| e.to_string())?,
    );
    let ev_source = source.clone();
    let ev_digest = encrypted_digest.clone();
    let ev_input = input.clone();
    let ev_target = target.clone();
    let (evidence, password) = host
        .run(move |service| {
            let evidence = service
                .restore_snapshot_evidence(&ev_source, &ev_input, &ev_target, &password, &ev_digest)
                .map_err(|e| e.to_string())?;
            Ok((evidence, password))
        })
        .await?;
    let parent_identity = match &evidence {
        oasis7_command_center_core::context::ContextRecoveryEvidence::SnapshotRestore {
            parent_identity,
            ..
        } => parent_identity.clone(),
        _ => return Err("Unexpected restore evidence".into()),
    };
    tools
        .mutate_context(
            operation_id,
            fingerprint,
            format!(
                "context-restore:{}",
                crate::tool_commands::digest(target.to_string_lossy().as_bytes())
            ),
            "snapshot_restore",
            evidence,
            move || {
                if oasis7_command_center_core::tools::selected_file_digest(
                    &input,
                    128 * 1024 * 1024,
                )?
                .0 != encrypted_digest
                {
                    return Err(oasis7_command_center_core::Error::Invalid(
                        "Encrypted input changed".into(),
                    ));
                }
                ContextService::open(path)?
                    .restore_snapshot_frozen_parent(
                        &source,
                        &input,
                        &target,
                        &password,
                        &encrypted_digest,
                        &parent_identity,
                    )
                    .map(Some)
            },
        )
        .await
}

#[tauri::command]
pub fn context_info() -> serde_json::Value {
    oasis7_command_center_core::context::builtin_info()
}
#[tauri::command]
pub async fn context_reconcile(
    host: State<'_, ContextHost>,
    tools: State<'_, crate::tool_commands::ToolHost>,
    window: tauri::WebviewWindow,
    operation_id: String,
    secret_handle: Option<String>,
) -> Result<oasis7_command_center_core::tools::jobs::ToolJob, String> {
    let password = secret_handle
        .map(|token| {
            tools
                .secrets
                .take(&token, window.label(), &operation_id, "context_reconcile")
        })
        .transpose()
        .map_err(|e| e.to_string())?;
    let jobs = tools.jobs.clone();
    host.run(move |service| {
        jobs.reconcile_context(
            &operation_id,
            &service,
            password.as_deref().map(|p| p.as_slice()),
        )
        .map_err(|e| e.to_string())
    })
    .await
}
